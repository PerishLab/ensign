mod oidc;

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use keel::Wire;
use keel::adapt::pg::Postgres;
use keel::atom::string;
use keel::atom::url as link;
use keel::config;
use keel::resource;
use keel::{Cell, Core, Graph, Operator, app, bind};
use keel_gate::{Gate, TTL};
use serde_json::{Map, Value, json};
use std::env;
use std::path::Path;
use std::sync::Arc;

#[resource]
struct Actor {
    #[field(string, unique)]
    login: string,
    #[field(string)]
    name: string,
    #[field(string)]
    kind: string,
    #[field(bool)]
    barred: bool,
}

#[resource(veil)]
struct Pass {
    #[field(string)]
    hash: string,
    #[relation(Actor, one2one, root)]
    actor: Actor,
}

#[resource(veil)]
struct Invite {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    note: string,
}

#[resource(veil)]
struct Rescue {
    #[field(string)]
    hash: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource]
struct Team {
    #[field(string, unique)]
    name: string,
    #[relation(Actor, many2many, crew)]
    members: Actor,
}

#[resource]
struct App {
    #[field(string)]
    name: string,
    #[field(string, unique)]
    slug: string,
    #[field(url)]
    home: link,
    #[field(url)]
    redirect: link,
    #[field(string)]
    secret: string,
    #[field(string)]
    mode: string,
}

#[resource(veil)]
struct Renew {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    slug: string,
    #[field(string)]
    scope: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

keel_gate::gate!(Actor);

struct Booth<W: Wire> {
    core: Arc<Core<W>>,
    svc: i64,
    secure: bool,
}

impl<W: Wire> Clone for Booth<W> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            svc: self.svc,
            secure: self.secure,
        }
    }
}

impl<W: Wire + 'static> Booth<W> {
    async fn card(&self, code: &str) -> Option<i64> {
        let q = format!(r#"from Invite where hash = "{}""#, digest(code));
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        pack.rows().first().map(keel::Row::key)
    }

    async fn birth(&self, login: &str, name: &str) -> Result<i64, keel::adapt::Error> {
        let sudo = self.core.sudo();
        let key = sudo
            .put(
                "Actor",
                &[
                    ("login", login),
                    ("name", name),
                    ("kind", "user"),
                    ("barred", "false"),
                ],
            )
            .await?;
        sudo.put(
            "@grant",
            &[
                ("who", &key.to_string()),
                ("verb", "*"),
                ("unit", "Actor"),
                ("scope", &format!("row {key}")),
            ],
        )
        .await?;
        Ok(key)
    }

    async fn actor(&self, login: &str) -> Option<i64> {
        let q = format!(r#"from Actor where login = "{}""#, scrub(login));
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        pack.rows().first().map(keel::Row::key)
    }

    async fn refloor(&self, key: i64) {
        let face = self.core.of(self.svc);
        for unit in ["Rescue", "Session"] {
            let q = format!(r#"from {unit} where actor = "{key}""#);
            if let Ok(pack) = face.query(&q).await {
                for row in pack.rows() {
                    let _ = face.end(unit, row.key()).await;
                }
            }
        }
    }

    async fn shield(&self, key: i64) -> Option<String> {
        let q = format!(r#"from Pass where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        let row = pack.rows().first()?;
        row.cells().get("hash").map(Cell::show)
    }

    async fn spare(&self, key: i64, code: &str) -> Option<i64> {
        let q = format!(r#"from Rescue where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        let mark = digest(code);
        let hit = pack
            .rows()
            .iter()
            .find(|row| row.cells().get("hash").map(Cell::show) == Some(mark.clone()))?;
        Some(hit.key())
    }

    async fn face(&self, headers: &HeaderMap, op: Option<i64>) -> Option<keel::Face<'_, W>> {
        let told = headers.get("authorization").and_then(|v| v.to_str().ok());
        if let Some(token) = told.and_then(|v| v.strip_prefix("sudo ")) {
            return self
                .core
                .seal(token)
                .await
                .ok()
                .filter(|ok| *ok)
                .map(|_| self.core.sudo());
        }
        op.map(|id| self.core.of(id))
    }

    async fn tag(&self, id: i64) -> Option<(String, String)> {
        let q = format!(r#"from Actor where id = "{id}""#);
        let pack = self.core.of(id).query(&q).await.ok()?;
        let row = pack.rows().first()?;
        let login = row.cells().get("login").map(Cell::show)?;
        let name = row.cells().get("name").map(Cell::show).unwrap_or_default();
        Some((login, name))
    }

    async fn session(&self, key: i64) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let sid = wild();
        let row = self
            .core
            .of(self.svc)
            .put(
                "Session",
                &[("hash", &digest(&sid)), ("actor", &key.to_string())],
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        self.core
            .lease("Session", row, now() + TTL)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut headers = HeaderMap::new();
        let mut jar = format!("session={sid}; HttpOnly; SameSite=Lax; Path=/");
        if self.secure {
            jar.push_str("; Secure");
        }
        headers.insert(
            "set-cookie",
            jar.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
        Ok((StatusCode::CREATED, headers, Json(json!({ "id": row }))))
    }
}

fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Pass>()
        .plug::<Invite>()
        .plug::<Rescue>()
        .plug::<Team>()
        .plug::<App>()
        .plug::<Renew>();
    plug(&mut graph);
    graph
}

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let cfg = config::load(Path::new(&root));
    match env::var("KEEL_PG") {
        Ok(url) => {
            if env::var("KEEL_FRESH").is_ok() {
                fresh(&url).await;
            }
            let store = match Postgres::at(url).await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &root).await;
        }
        Err(_) => {
            let store = match cfg.open().await {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &root).await;
        }
    }
}

async fn fresh(url: &str) {
    let mut store = match Postgres::at(url).await {
        Ok(store) => store,
        Err(err) => halt("fresh", &err.to_string()),
    };
    let wipe = store
        .script("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .await;
    if let Err(err) = wipe {
        halt("fresh", &err.to_string());
    }
}

fn raise<W: Wire + 'static>(
    made: Result<Core<W>, keel::adapt::Error>,
    cfg: &config::Config,
) -> Arc<Core<W>> {
    let built = made
        .and_then(|core| core.identify("Actor"))
        .map(|core| match cfg.cache.kind {
            config::Hold::Memory => core,
            config::Hold::None => core.bare(),
        });
    match built {
        Ok(core) => core.share(),
        Err(err) => halt("bind", &err.to_string()),
    }
}

async fn serve<W: Wire + 'static>(core: Arc<Core<W>>, cfg: &config::Config, root: &str) {
    let (door, svc) = match rig(&core).await {
        Ok(pair) => pair,
        Err(err) => halt("rise", &err.to_string()),
    };
    let iss = env::var("ENSIGN_ISS")
        .unwrap_or_else(|_| format!("http://{}:{}", cfg.listen.host, cfg.listen.port));
    let booth = Booth {
        core: core.clone(),
        svc,
        secure: iss.starts_with("https://"),
    };
    let plate = Router::new()
        .route("/invite", post(invite::<W>))
        .route("/join", post(join::<W>))
        .route("/login", post(login::<W>))
        .route("/logout", post(logout::<W>))
        .route("/whoami", get(who::<W>))
        .route("/auth", get(auth::<W>))
        .route("/mint", post(mint::<W>))
        .route("/revive", post(revive::<W>))
        .with_state(booth);
    let vault = Path::new(root).join(".local").join("sign.pem");
    let flags = oidc::Oidc::new(core.clone(), svc, iss, oidc::keys(&vault)).plate();
    let base = app(core.clone(), &cfg.listen.prefix)
        .merge(plate)
        .merge(flags);
    let router = door.screen(base);
    let addr = format!("{}:{}", cfg.listen.host, cfg.listen.port);
    let bound = match tokio::net::TcpListener::bind(&addr).await {
        Ok(bound) => bound,
        Err(err) => halt("listen", &err.to_string()),
    };
    eprintln!("ensign: ready on http://{addr}");
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("ensign: {seat}: {note}");
    std::process::exit(1)
}

async fn rig<W: Wire + 'static>(core: &Arc<Core<W>>) -> Result<(Gate<W>, i64), keel::adapt::Error> {
    let svc = hail(core).await?;
    let gate = Gate::rise(core.clone(), svc).await?.bar("barred");
    seed(core, svc).await?;
    Ok((gate, svc))
}

async fn hail<W: Wire>(core: &Arc<Core<W>>) -> Result<i64, keel::adapt::Error> {
    let held = core.query(r#"from Actor where login = "ensign""#).await?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => {
            core.put(
                "Actor",
                &[
                    ("login", "ensign"),
                    ("name", "ensign"),
                    ("kind", "svc"),
                    ("barred", "false"),
                ],
            )
            .await
        }
    }
}

async fn seed<W: Wire>(core: &Arc<Core<W>>, svc: i64) -> Result<(), keel::adapt::Error> {
    let sown = core.query(r#"from @grant where who = "all" count"#).await?;
    if sown.count() != Some(0) {
        return Ok(());
    }
    let sudo = core.sudo();
    let who = svc.to_string();
    for unit in ["Actor", "Team", "App"] {
        sudo.put(
            "@grant",
            &[
                ("who", "all"),
                ("verb", "see"),
                ("unit", unit),
                ("scope", "all"),
            ],
        )
        .await?;
    }
    for (verb, unit) in [
        ("see", "Invite"),
        ("end", "Invite"),
        ("see", "Pass"),
        ("see", "Rescue"),
        ("end", "Rescue"),
        ("see", "App"),
        ("see", "Renew"),
        ("put", "Renew"),
        ("end", "Renew"),
    ] {
        sudo.put(
            "@grant",
            &[
                ("who", &who),
                ("verb", verb),
                ("unit", unit),
                ("scope", "all"),
            ],
        )
        .await?;
    }
    Ok(())
}

async fn invite<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
    op: Option<Extension<Operator>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let who = op.map(|Extension(Operator(id))| id);
    let face = booth
        .face(&headers, who)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let note = body.get("note").and_then(Value::as_str).unwrap_or("");
    let code = wild();
    face.put("Invite", &[("hash", &digest(&code)), ("note", note)])
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok((StatusCode::CREATED, Json(json!({ "code": code }))))
}

async fn join<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let code = text(&body, "code")?;
    let login = text(&body, "login")?;
    let name = text(&body, "name")?;
    let pass = text(&body, "pass")?;
    if pass.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let card = booth.card(&code).await.ok_or(StatusCode::NOT_FOUND)?;
    booth
        .core
        .of(booth.svc)
        .end("Invite", card)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let key = booth
        .birth(&login, &name)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    let hash = tokio::task::spawn_blocking(move || lock(&pass))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": key }))))
}

async fn login<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let pass = text(&body, "pass")?;
    let key = booth.actor(&login).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let hash = booth.shield(key).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let fit = tokio::task::spawn_blocking(move || fits(&pass, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !fit {
        return Err(StatusCode::UNAUTHORIZED);
    }
    booth.session(key).await
}

async fn logout<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let sid = crumb(&headers).ok_or(StatusCode::BAD_REQUEST)?;
    let q = format!(r#"from Session where hash = "{}""#, digest(&sid));
    let face = booth.core.of(booth.svc);
    let pack = face
        .query(&q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = pack.rows().first().ok_or(StatusCode::NOT_FOUND)?;
    let key = owner(row).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .end("Session", row.key())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn who<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, name) = booth.tag(me).await.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "id": me, "login": login, "name": name })))
}

async fn auth<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, HeaderMap), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, _) = booth.tag(me).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let mut headers = HeaderMap::new();
    let stamp = |value: &str| value.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    headers.insert("x-ensign-user", stamp(&me.to_string())?);
    headers.insert("x-ensign-login", stamp(&login)?);
    Ok((StatusCode::OK, headers))
}

async fn mint<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let face = booth.core.of(me);
    let q = format!(r#"from Rescue where actor = "{me}""#);
    let pack = face
        .query(&q)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for row in pack.rows() {
        face.end("Rescue", row.key())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let mut codes = Vec::new();
    for _ in 0..3 {
        let code = wild();
        face.put(
            "Rescue",
            &[("hash", &digest(&code)), ("actor", &me.to_string())],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        codes.push(code);
    }
    Ok((StatusCode::CREATED, Json(json!({ "codes": codes }))))
}

async fn revive<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let code = text(&body, "code")?;
    let key = booth.actor(&login).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let spare = booth
        .spare(key, &code)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    booth
        .core
        .of(booth.svc)
        .end("Rescue", spare)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    booth.refloor(key).await;
    booth.session(key).await
}

fn owner(row: &keel::Row) -> Option<i64> {
    match row.cells().get("actor") {
        Some(Cell::Int(key)) => Some(*key),
        _ => None,
    }
}

fn text(body: &Map<String, Value>, name: &str) -> Result<String, StatusCode> {
    body.get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(StatusCode::BAD_REQUEST)
}

fn scrub(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn crumb(headers: &HeaderMap) -> Option<String> {
    let jar = headers.get("cookie")?.to_str().ok()?;
    jar.split(';')
        .filter_map(|part| part.trim().strip_prefix("session="))
        .next()
        .map(str::to_string)
}

fn lock(pass: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(pass.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

fn fits(pass: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(pass.as_bytes(), &parsed)
        .is_ok()
}

fn digest(code: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn wild() -> String {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("os entropy");
    seed.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
