use axum::Json;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Extension, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use keel::store::Store;
use keel::{Cell, Core, Operator};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

const LIFE: i64 = 60 * 60;
const GRACE: i64 = 60;
const RENEW: i64 = 60 * 60 * 24 * 30;

pub struct Keys {
    enc: EncodingKey,
    dec: DecodingKey,
    jwk: Value,
    kid: String,
}

pub fn keys(path: &Path) -> Keys {
    use p256::SecretKey;
    use p256::pkcs8::DecodePrivateKey;
    let secret = match std::fs::read_to_string(path) {
        Ok(pem) => SecretKey::from_pkcs8_pem(&pem).expect("key pem"),
        Err(_) => born(path),
    };
    shape(secret)
}

fn born(path: &Path) -> p256::SecretKey {
    use p256::SecretKey;
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    let secret = SecretKey::generate();
    let pem = secret.to_pkcs8_pem(LineEnding::LF).expect("pkcs8");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("key dir");
    }
    std::fs::write(path, pem.as_bytes()).expect("write key");
    eprintln!("ensign: signing key born at {}", path.display());
    secret
}

fn shape(secret: p256::SecretKey) -> Keys {
    use p256::elliptic_curve::sec1::ToSec1Point;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    let pem = secret.to_pkcs8_pem(LineEnding::LF).expect("pkcs8");
    let enc = EncodingKey::from_ec_pem(pem.as_bytes()).expect("enc");
    let point = secret.public_key().to_sec1_point(false);
    let x = B64.encode(point.x().expect("x"));
    let y = B64.encode(point.y().expect("y"));
    let dec = DecodingKey::from_ec_components(&x, &y).expect("dec");
    let kid = tag(&x, &y);
    let jwk = json!({
        "kty": "EC",
        "crv": "P-256",
        "use": "sig",
        "alg": "ES256",
        "kid": kid,
        "x": x,
        "y": y,
    });
    Keys { enc, dec, jwk, kid }
}

fn tag(x: &str, y: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut sponge = Sha256::new();
    sponge.update(x.as_bytes());
    sponge.update(y.as_bytes());
    sponge
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct Code {
    actor: i64,
    client: String,
    redirect: String,
    challenge: String,
    scope: String,
    dies: i64,
}

pub struct Oidc<S: Store> {
    core: Arc<Core<S>>,
    svc: i64,
    keys: Arc<Keys>,
    iss: String,
    codes: Arc<Mutex<HashMap<String, Code>>>,
}

impl<S: Store> Clone for Oidc<S> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            svc: self.svc,
            keys: self.keys.clone(),
            iss: self.iss.clone(),
            codes: self.codes.clone(),
        }
    }
}

impl<S: Store + 'static> Oidc<S> {
    pub fn new(core: Arc<Core<S>>, svc: i64, iss: String, keys: Keys) -> Self {
        Self {
            core,
            svc,
            keys: Arc::new(keys),
            iss,
            codes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn plate(self) -> Router {
        Router::new()
            .route("/.well-known/openid-configuration", get(disco::<S>))
            .route("/.well-known/jwks.json", get(jwks::<S>))
            .route("/authorize", get(authorize::<S>))
            .route("/token", post(token::<S>))
            .route("/userinfo", get(userinfo::<S>))
            .with_state(self)
    }

    fn open(&self, ask: Ask, actor: i64) -> Response {
        if ask.kind != "code" || ask.method != "S256" {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let Some(app) = self.client(&ask.client) else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if cell(&app, "redirect") != ask.redirect || cell(&app, "mode") != "oidc" {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let grant = wild();
        self.codes.lock().expect("codes").insert(
            grant.clone(),
            Code {
                actor,
                client: ask.client,
                redirect: ask.redirect.clone(),
                challenge: ask.challenge,
                scope: ask.scope,
                dies: now() + GRACE,
            },
        );
        let back = format!("{}?code={}&state={}", ask.redirect, grant, ask.state);
        Redirect::to(&back).into_response()
    }

    fn trade(&self, grant: Grant) -> Grip {
        let key = grant.code.ok_or_else(|| sour("invalid_request"))?;
        let held = self.codes.lock().expect("codes").remove(&key);
        let code = held.ok_or_else(|| sour("invalid_grant"))?;
        if code.dies < now() {
            return Err(sour("invalid_grant"));
        }
        if grant.client.as_deref() != Some(&code.client)
            || grant.redirect.as_deref() != Some(&code.redirect)
        {
            return Err(sour("invalid_grant"));
        }
        let verifier = grant.verifier.ok_or_else(|| sour("invalid_request"))?;
        if seal(&verifier) != code.challenge {
            return Err(sour("invalid_grant"));
        }
        self.grip(code.actor, &code.client, &code.scope)
    }

    fn renew(&self, grant: Grant) -> Grip {
        let token = grant.renew.ok_or_else(|| sour("invalid_request"))?;
        let ward = self.warrant(&token).ok_or_else(|| sour("invalid_grant"))?;
        if grant.client.as_deref() != Some(&ward.slug) {
            return Err(sour("invalid_grant"));
        }
        if !self.alive(ward.actor) || self.client(&ward.slug).is_none() {
            return Err(sour("invalid_grant"));
        }
        self.core
            .of(self.svc)
            .end("Renew", ward.row)
            .map_err(|_| sour("invalid_grant"))?;
        self.grip(ward.actor, &ward.slug, &ward.scope)
    }

    fn grip(&self, actor: i64, client: &str, scope: &str) -> Grip {
        let who = self.person(actor).ok_or_else(|| sour("server_error"))?;
        let sub = actor.to_string();
        let wide = scope.split_whitespace().any(|word| word == "profile");
        let id = self
            .sign(&sub, &who, client, wide)
            .map_err(|_| sour("server_error"))?;
        let reach = self
            .sign(&sub, &who, &self.iss, false)
            .map_err(|_| sour("server_error"))?;
        let fresh = wild();
        self.mint(actor, client, scope, &fresh)
            .map_err(|_| sour("server_error"))?;
        Ok(Json(json!({
            "access_token": reach,
            "id_token": id,
            "refresh_token": fresh,
            "token_type": "Bearer",
            "expires_in": LIFE,
            "scope": scope,
        })))
    }

    fn alive(&self, actor: i64) -> bool {
        let q = format!(r#"from Actor where id = "{actor}""#);
        let Ok(pack) = self.core.of(self.svc).query(&q) else {
            return false;
        };
        match pack.rows().first() {
            Some(row) => row.cells().get("barred").map(Cell::show).as_deref() != Some("true"),
            None => false,
        }
    }

    fn look(&self, token: &str) -> Result<Json<Value>, StatusCode> {
        let mut rule = Validation::new(Algorithm::ES256);
        rule.set_issuer(&[&self.iss]);
        rule.set_audience(&[&self.iss]);
        let data =
            decode::<Value>(token, &self.keys.dec, &rule).map_err(|_| StatusCode::UNAUTHORIZED)?;
        let sub = data.claims.get("sub").and_then(Value::as_str).unwrap_or("");
        let id: i64 = sub.parse().map_err(|_| StatusCode::UNAUTHORIZED)?;
        let who = self.person(id).ok_or(StatusCode::NOT_FOUND)?;
        Ok(Json(
            json!({ "sub": sub, "login": who.login, "name": who.name }),
        ))
    }

    fn person(&self, id: i64) -> Option<Who> {
        let q = format!(r#"from Actor where id = "{id}""#);
        let pack = self.core.of(self.svc).query(&q).ok()?;
        let row = pack.rows().first()?;
        Some(Who {
            login: row.cells().get("login").map(Cell::show)?,
            name: row.cells().get("name").map(Cell::show).unwrap_or_default(),
        })
    }

    fn client(&self, slug: &str) -> Option<keel::Row> {
        let q = format!(r#"from App where slug = "{slug}""#);
        let pack = self.core.of(self.svc).query(&q).ok()?;
        pack.rows().first().cloned()
    }

    fn warrant(&self, token: &str) -> Option<Ward> {
        let q = format!(r#"from Renew where hash = "{}""#, seal(token));
        let pack = self.core.of(self.svc).query(&q).ok()?;
        let row = pack.rows().first()?;
        let actor = match row.cells().get("actor") {
            Some(Cell::Int(key)) => *key,
            _ => return None,
        };
        Some(Ward {
            row: row.key(),
            actor,
            slug: row.cells().get("slug").map(Cell::show)?,
            scope: row.cells().get("scope").map(Cell::show).unwrap_or_default(),
        })
    }

    fn mint(
        &self,
        actor: i64,
        client: &str,
        scope: &str,
        token: &str,
    ) -> Result<(), keel::adapt::Error> {
        let face = self.core.of(self.svc);
        let row = face.put(
            "Renew",
            &[
                ("hash", &seal(token)),
                ("slug", client),
                ("scope", scope),
                ("actor", &actor.to_string()),
            ],
        )?;
        face.lease("Renew", row, now() + RENEW)
    }

    fn sign(
        &self,
        sub: &str,
        who: &Who,
        aud: &str,
        wide: bool,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let mut head = Header::new(Algorithm::ES256);
        head.kid = Some(self.keys.kid.clone());
        let claims = Claims {
            iss: self.iss.clone(),
            sub: sub.to_string(),
            aud: aud.to_string(),
            exp: now() + LIFE,
            iat: now(),
            login: wide.then(|| who.login.clone()),
            name: wide.then(|| who.name.clone()),
        };
        encode(&head, &claims, &self.keys.enc)
    }
}

async fn disco<S: Store>(State(oidc): State<Oidc<S>>) -> Json<Value> {
    let iss = &oidc.iss;
    Json(json!({
        "issuer": iss,
        "authorization_endpoint": format!("{iss}/authorize"),
        "token_endpoint": format!("{iss}/token"),
        "userinfo_endpoint": format!("{iss}/userinfo"),
        "jwks_uri": format!("{iss}/.well-known/jwks.json"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["ES256"],
        "scopes_supported": ["openid", "profile"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

async fn jwks<S: Store>(State(oidc): State<Oidc<S>>) -> Json<Value> {
    Json(json!({ "keys": [oidc.keys.jwk] }))
}

#[derive(Deserialize)]
struct Ask {
    #[serde(rename = "response_type")]
    kind: String,
    #[serde(rename = "client_id")]
    client: String,
    #[serde(rename = "redirect_uri")]
    redirect: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    state: String,
    #[serde(rename = "code_challenge")]
    challenge: String,
    #[serde(rename = "code_challenge_method")]
    method: String,
}

async fn authorize<S: Store + 'static>(
    State(oidc): State<Oidc<S>>,
    Query(ask): Query<Ask>,
    op: Option<Extension<Operator>>,
) -> Response {
    let Some(Extension(Operator(actor))) = op else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    oidc.open(ask, actor)
}

#[derive(Deserialize)]
struct Grant {
    #[serde(rename = "grant_type")]
    kind: String,
    code: Option<String>,
    #[serde(rename = "redirect_uri")]
    redirect: Option<String>,
    #[serde(rename = "client_id")]
    client: Option<String>,
    #[serde(rename = "code_verifier")]
    verifier: Option<String>,
    #[serde(rename = "refresh_token")]
    renew: Option<String>,
}

type Grip = Result<Json<Value>, (StatusCode, Json<Value>)>;

async fn token<S: Store + 'static>(State(oidc): State<Oidc<S>>, Form(grant): Form<Grant>) -> Grip {
    match grant.kind.as_str() {
        "authorization_code" => oidc.trade(grant),
        "refresh_token" => oidc.renew(grant),
        _ => Err(sour("unsupported_grant_type")),
    }
}

async fn userinfo<S: Store + 'static>(
    State(oidc): State<Oidc<S>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let token = bearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    oidc.look(&token)
}

struct Who {
    login: String,
    name: String,
}

struct Ward {
    row: i64,
    actor: i64,
    slug: String,
    scope: String,
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_string)
}

fn cell(row: &keel::Row, name: &str) -> String {
    row.cells().get(name).map(Cell::show).unwrap_or_default()
}

fn seal(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut sponge = Sha256::new();
    sponge.update(text.as_bytes());
    B64.encode(sponge.finalize())
}

fn sour(note: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": note })))
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
        .map(|gap| gap.as_secs() as i64)
        .unwrap_or(0)
}
