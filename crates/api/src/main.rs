mod oidc;

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use keel::adapt::pg::Postgres;
use keel::atom::string;
use keel::atom::url as link;
use keel::config;
use keel::resource;
use keel::store::Store;
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

#[resource]
struct Pass {
    #[field(string)]
    hash: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

#[resource]
struct Invite {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    note: string,
}

#[resource]
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

#[resource]
struct Renew {
    #[field(string, unique)]
    hash: string,
    #[field(string)]
    slug: string,
    #[relation(Actor, many2one, root)]
    actor: Actor,
}

keel_gate::gate!(Actor);

struct Booth<S: Store> {
    core: Arc<Core<S>>,
    svc: i64,
}

impl<S: Store> Clone for Booth<S> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            svc: self.svc,
        }
    }
}

impl<S: Store + 'static> Booth<S> {
    fn card(&self, code: &str) -> Option<i64> {
        let q = format!(r#"from Invite where hash = "{}""#, digest(code));
        let pack = self.core.of(self.svc).query(&q).ok()?;
        pack.rows().first().map(keel::Row::key)
    }

    fn birth(&self, login: &str, name: &str) -> Result<i64, keel::adapt::Error> {
        let sudo = self.core.sudo();
        let key = sudo.put(
            "Actor",
            &[
                ("login", login),
                ("name", name),
                ("kind", "user"),
                ("barred", "false"),
            ],
        )?;
        sudo.put(
            "@grant",
            &[
                ("who", &key.to_string()),
                ("verb", "*"),
                ("unit", "Actor"),
                ("scope", &format!("row {key}")),
            ],
        )?;
        Ok(key)
    }

    fn actor(&self, login: &str) -> Option<i64> {
        let q = format!(r#"from Actor where login = "{login}""#);
        let pack = self.core.of(self.svc).query(&q).ok()?;
        pack.rows().first().map(keel::Row::key)
    }

    fn shield(&self, key: i64) -> Option<String> {
        let q = format!(r#"from Pass where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).ok()?;
        let row = pack.rows().first()?;
        row.cells().get("hash").map(Cell::show)
    }

    fn spare(&self, key: i64, code: &str) -> Option<i64> {
        let q = format!(r#"from Rescue where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).ok()?;
        let mark = digest(code);
        let hit = pack
            .rows()
            .iter()
            .find(|row| row.cells().get("hash").map(Cell::show) == Some(mark.clone()))?;
        Some(hit.key())
    }

    fn tag(&self, id: i64) -> Option<(String, String)> {
        let q = format!(r#"from Actor where id = "{id}""#);
        let pack = self.core.of(id).query(&q).ok()?;
        let row = pack.rows().first()?;
        let login = row.cells().get("login").map(Cell::show)?;
        let name = row.cells().get("name").map(Cell::show).unwrap_or_default();
        Some((login, name))
    }

    fn session(&self, key: i64) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let sid = wild();
        let row = self
            .core
            .of(self.svc)
            .put(
                "Session",
                &[("hash", &digest(&sid)), ("actor", &key.to_string())],
            )
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        self.core
            .lease("Session", row, now() + TTL)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut headers = HeaderMap::new();
        let jar = format!("session={sid}; HttpOnly; Path=/");
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
                fresh(&url);
            }
            let core = raise(bind(shape(), Postgres::at(url)), &cfg);
            serve(core, &cfg).await;
        }
        Err(_) => {
            let store = match cfg.open() {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store), &cfg);
            serve(core, &cfg).await;
        }
    }
}

fn fresh(url: &str) {
    let url = url.to_string();
    let done = std::thread::spawn(move || {
        let mut client = postgres::Client::connect(&url, postgres::NoTls)?;
        client.batch_execute("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
    })
    .join();
    match done {
        Ok(Ok(())) => {}
        Ok(Err(err)) => halt("fresh", &err.to_string()),
        Err(_) => halt("fresh", "reset thread panicked"),
    }
}

fn raise<S: Store + 'static>(
    made: Result<Core<S>, keel::adapt::Error>,
    cfg: &config::Config,
) -> Arc<Core<S>> {
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

async fn serve<S: Store + 'static>(core: Arc<Core<S>>, cfg: &config::Config) {
    let (door, svc) = match rig(&core) {
        Ok(pair) => pair,
        Err(err) => halt("rise", &err.to_string()),
    };
    let booth = Booth {
        core: core.clone(),
        svc,
    };
    let plate = Router::new()
        .route("/join", post(join::<S>))
        .route("/login", post(login::<S>))
        .route("/logout", post(logout::<S>))
        .route("/whoami", get(who::<S>))
        .route("/auth", get(auth::<S>))
        .route("/mint", post(mint::<S>))
        .route("/revive", post(revive::<S>))
        .with_state(booth);
    let iss = format!("http://{}:{}", cfg.listen.host, cfg.listen.port);
    let flags = oidc::Oidc::new(core.clone(), svc, iss).plate();
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

fn rig<S: Store + 'static>(core: &Arc<Core<S>>) -> Result<(Gate<S>, i64), keel::adapt::Error> {
    let svc = hail(core)?;
    let gate = Gate::rise(core.clone(), svc)?.bar("barred");
    seed(core, svc)?;
    Ok((gate, svc))
}

fn hail<S: Store>(core: &Arc<Core<S>>) -> Result<i64, keel::adapt::Error> {
    let held = core.query(r#"from Actor where login = "ensign""#)?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => core.put(
            "Actor",
            &[
                ("login", "ensign"),
                ("name", "ensign"),
                ("kind", "svc"),
                ("barred", "false"),
            ],
        ),
    }
}

fn seed<S: Store>(core: &Arc<Core<S>>, svc: i64) -> Result<(), keel::adapt::Error> {
    let sown = core.query(r#"from @grant where who = "all" count"#)?;
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
        )?;
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
        )?;
    }
    Ok(())
}

async fn join<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let code = text(&body, "code")?;
    let login = text(&body, "login")?;
    let name = text(&body, "name")?;
    let pass = text(&body, "pass")?;
    let card = booth.card(&code).ok_or(StatusCode::NOT_FOUND)?;
    let key = booth
        .birth(&login, &name)
        .map_err(|_| StatusCode::CONFLICT)?;
    let hash = lock(&pass).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(booth.svc)
        .end("Invite", card)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(json!({ "id": key }))))
}

async fn login<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let pass = text(&body, "pass")?;
    let key = booth.actor(&login).ok_or(StatusCode::UNAUTHORIZED)?;
    let hash = booth.shield(key).ok_or(StatusCode::UNAUTHORIZED)?;
    if !fits(&pass, &hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    booth.session(key)
}

async fn logout<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let sid = crumb(&headers).ok_or(StatusCode::BAD_REQUEST)?;
    let q = format!(r#"from Session where hash = "{}""#, digest(&sid));
    let face = booth.core.of(booth.svc);
    let pack = face
        .query(&q)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = pack.rows().first().ok_or(StatusCode::NOT_FOUND)?;
    let key = owner(row).ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    booth
        .core
        .of(key)
        .end("Session", row.key())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn who<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    op: Option<Extension<Operator>>,
) -> Result<Json<Value>, StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, name) = booth.tag(me).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "id": me, "login": login, "name": name })))
}

async fn auth<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, HeaderMap), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let (login, _) = booth.tag(me).ok_or(StatusCode::UNAUTHORIZED)?;
    let mut headers = HeaderMap::new();
    let stamp = |value: &str| value.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    headers.insert("x-ensign-user", stamp(&me.to_string())?);
    headers.insert("x-ensign-login", stamp(&login)?);
    Ok((StatusCode::OK, headers))
}

async fn mint<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    op: Option<Extension<Operator>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    let Some(Extension(Operator(me))) = op else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let face = booth.core.of(me);
    let q = format!(r#"from Rescue where actor = "{me}""#);
    let pack = face
        .query(&q)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for row in pack.rows() {
        face.end("Rescue", row.key())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let mut codes = Vec::new();
    for _ in 0..3 {
        let code = wild();
        face.put(
            "Rescue",
            &[("hash", &digest(&code)), ("actor", &me.to_string())],
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        codes.push(code);
    }
    Ok((StatusCode::CREATED, Json(json!({ "codes": codes }))))
}

async fn revive<S: Store + 'static>(
    State(booth): State<Booth<S>>,
    Json(body): Json<Map<String, Value>>,
) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
    let login = text(&body, "login")?;
    let code = text(&body, "code")?;
    let key = booth.actor(&login).ok_or(StatusCode::UNAUTHORIZED)?;
    let spare = booth.spare(key, &code).ok_or(StatusCode::UNAUTHORIZED)?;
    booth
        .core
        .of(booth.svc)
        .end("Rescue", spare)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    booth.session(key)
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
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for _ in 0..4 {
        let word = RandomState::new().build_hasher().finish();
        out.push_str(&format!("{word:016x}"));
    }
    out
}

fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
