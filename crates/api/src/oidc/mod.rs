mod flow;
mod issue;
mod keys;
mod plain;

use flow::{authorize, disco, jwks, token, userinfo};

pub(crate) use keys::{Keys, keys};

use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use jsonwebtoken::{Algorithm, Validation, decode};
use keel::{Cell, Core, Wire};
use plain::{cell, now, pct, safe, scrub, seal, wild};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) const LIFE: i64 = 60 * 60;
pub(crate) const GRACE: i64 = 60;
pub(crate) const RENEW: i64 = 60 * 60 * 24 * 30;

pub(crate) struct Code {
    actor: i64,
    client: String,
    redirect: String,
    challenge: String,
    scope: String,
    nonce: Option<String>,
    dies: i64,
}

pub(crate) struct Oidc<W: Wire> {
    pub(crate) core: Arc<Core<W>>,
    pub(crate) svc: i64,
    pub(crate) keys: Arc<Keys>,
    pub(crate) iss: String,
    pub(crate) codes: Arc<Mutex<HashMap<String, Code>>>,
}

impl<W: Wire> Clone for Oidc<W> {
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

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) fn new(core: Arc<Core<W>>, svc: i64, iss: String, keys: Keys) -> Self {
        Self {
            core,
            svc,
            keys: Arc::new(keys),
            iss,
            codes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn plate(self) -> Router {
        Router::new()
            .route("/.well-known/openid-configuration", get(disco::<W>))
            .route("/.well-known/jwks.json", get(jwks::<W>))
            .route("/authorize", get(authorize::<W>))
            .route("/token", post(token::<W>))
            .route("/userinfo", get(userinfo::<W>))
            .with_state(self)
    }

    async fn open(&self, ask: Ask, actor: i64) -> Response {
        let opens = ask.scope.split_whitespace().any(|word| word == "openid");
        if ask.kind != "code" || ask.method != "S256" || !opens {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let Some(app) = self.client(&ask.client).await else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        if cell(&app, "redirect") != ask.redirect || cell(&app, "mode") != "oidc" {
            return StatusCode::BAD_REQUEST.into_response();
        }
        if !safe(&ask.redirect) {
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
                nonce: ask.nonce,
                dies: now() + GRACE,
            },
        );
        let back = format!(
            "{}?code={}&state={}",
            ask.redirect,
            pct(&grant),
            pct(&ask.state)
        );
        Redirect::to(&back).into_response()
    }

    async fn alive(&self, actor: i64) -> bool {
        let q = format!(r#"from Actor where id = "{actor}""#);
        let Ok(pack) = self.core.of(self.svc).query(&q).await else {
            return false;
        };
        match pack.rows().first() {
            Some(row) => row.cells().get("barred").map(Cell::show).as_deref() != Some("true"),
            None => false,
        }
    }

    async fn look(&self, token: &str) -> Result<Json<Value>, StatusCode> {
        let mut rule = Validation::new(Algorithm::ES256);
        rule.set_issuer(&[&self.iss]);
        rule.set_audience(&[&self.iss]);
        let data =
            decode::<Value>(token, &self.keys.dec, &rule).map_err(|_| StatusCode::UNAUTHORIZED)?;
        let claims = &data.claims;
        if claims.get("kind").and_then(Value::as_str) != Some("access") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let sub = claims.get("sub").and_then(Value::as_str).unwrap_or("");
        let id: i64 = sub.parse().map_err(|_| StatusCode::UNAUTHORIZED)?;
        let who = self.person(id).await.ok_or(StatusCode::NOT_FOUND)?;
        let wide = claims
            .get("scope")
            .and_then(Value::as_str)
            .unwrap_or("")
            .split_whitespace()
            .any(|word| word == "profile");
        let mut out = json!({ "sub": sub });
        if wide {
            out["login"] = json!(who.login);
            out["name"] = json!(who.name);
            out["teams"] = json!(who.teams);
        }
        Ok(Json(out))
    }

    async fn person(&self, id: i64) -> Option<Who> {
        let q = format!(r#"from Actor where id = "{id}""#);
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        let row = pack.rows().first()?;
        Some(Who {
            login: row.cells().get("login").map(Cell::show)?,
            name: row.cells().get("name").map(Cell::show).unwrap_or_default(),
            teams: self.crews(id).await?,
        })
    }

    async fn crews(&self, id: i64) -> Option<Vec<String>> {
        let q = format!(r#"from Team where members has "{id}""#);
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        Some(
            pack.rows()
                .iter()
                .filter_map(|row| row.cells().get("name").map(Cell::show))
                .collect(),
        )
    }

    async fn client(&self, slug: &str) -> Option<keel::Row> {
        let q = format!(r#"from App where slug = "{}""#, scrub(slug));
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
        pack.rows().first().cloned()
    }

    async fn warrant(&self, token: &str) -> Option<Ward> {
        let q = format!(r#"from Renew where hash = "{}""#, seal(token));
        let pack = self.core.of(self.svc).query(&q).await.ok()?;
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
}

pub(crate) struct Ticket<'a> {
    sub: String,
    aud: String,
    kind: &'a str,
    scope: String,
    nonce: Option<String>,
    wide: bool,
    who: &'a Who,
}

#[derive(Deserialize)]
pub(crate) struct Ask {
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
    nonce: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Grant {
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

pub(crate) type Grip = Result<Json<Value>, (StatusCode, Json<Value>)>;

pub(crate) struct Who {
    login: String,
    name: String,
    teams: Vec<String>,
}

pub(crate) struct Ward {
    row: i64,
    actor: i64,
    slug: String,
    scope: String,
}

#[derive(Serialize)]
pub(crate) struct Claims {
    iss: String,
    sub: String,
    aud: String,
    kind: String,
    exp: i64,
    iat: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    login: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    teams: Option<Vec<String>>,
}
