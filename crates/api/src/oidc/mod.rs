mod bear;
mod flow;
mod issue;
mod keys;
mod name;
mod plain;

pub(crate) use bear::{Bearing, Code, Ward};
use flow::{authorize, disco, jwks, token, userinfo};

pub(crate) use keys::{Keys, keys, provision};

use crate::util::{Sound, pct};
use axum::Json;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use jsonwebtoken::{Algorithm, Validation, decode};
use keel::life::tick;
use keel::{Core, Op, Wire, form};
use keel_gate::wild;
use plain::{safe, seal};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) const LIFE: i64 = 60 * 60;
pub(crate) const GRACE: i64 = 60;
pub(crate) const RENEW: i64 = 60 * 60 * 24 * 30;

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
        let app = match self.client(&ask.client).await {
            Ok(Some(app)) => app,
            Ok(None) => return StatusCode::BAD_REQUEST.into_response(),
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        if app.text("redirect") != Some(&ask.redirect) || app.text("mode") != Some("oidc") {
            return StatusCode::BAD_REQUEST.into_response();
        }
        if !safe(&ask.redirect) {
            return StatusCode::BAD_REQUEST.into_response();
        }
        let audience = match self.heard(&ask.resource).await {
            Ok(Some(audience)) => audience,
            Ok(None) => return StatusCode::BAD_REQUEST.into_response(),
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        let grant = wild();
        self.codes.lock().expect("codes").insert(
            grant.clone(),
            Code {
                bearing: Bearing {
                    actor,
                    client: ask.client,
                    scope: ask.scope,
                    audience,
                },
                redirect: ask.redirect.clone(),
                challenge: ask.challenge,
                nonce: ask.nonce,
                dies: tick() + GRACE,
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

    async fn alive(&self, actor: i64) -> Result<bool, keel::adapt::Error> {
        let ask = form("Actor").when("id", Op::Eq, &actor.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        match held {
            Some(row) => Ok(row.flag("barred") != Some(true)),
            None => Ok(false),
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
        let id = self
            .named(sub)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let who = self
            .person(id)
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
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

    async fn person(&self, id: i64) -> Result<Option<Who>, keel::adapt::Error> {
        let ask = form("Profile").when("actor", Op::Eq, &id.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        let Some(row) = held else {
            return Ok(None);
        };
        let Some(login) = row.text("handle").map(str::to_string) else {
            return Ok(None);
        };
        Ok(Some(Who {
            login,
            name: row.text("name").unwrap_or_default().to_string(),
            teams: self.crews(id).await?,
        }))
    }

    async fn crews(&self, id: i64) -> Result<Vec<String>, keel::adapt::Error> {
        let ask = form("Team").when("members", Op::Has, &id.to_string());
        let pack = self.core.of(self.svc).ask(&ask).await?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| row.text("name").map(str::to_string))
            .collect())
    }

    async fn client(&self, slug: &str) -> Result<Option<keel::Row>, keel::adapt::Error> {
        let ask = form("App").when("slug", Op::Eq, slug);
        self.core.of(self.svc).one(&ask).await
    }

    async fn warrant(&self, token: &str) -> Result<Option<Ward>, keel::adapt::Error> {
        let ask = form("Renew").when("hash", Op::Eq, &seal(token));
        let held = self.core.of(self.svc).one(&ask).await?;
        let Some(row) = held else {
            return Ok(None);
        };
        let Some(actor) = row.int("actor") else {
            return Ok(None);
        };
        let Some(slug) = row.text("slug").map(str::to_string) else {
            return Ok(None);
        };
        Ok(Some(Ward {
            row: row.key(),
            bearing: Bearing {
                actor,
                client: slug,
                scope: row.text("scope").unwrap_or_default().to_string(),
                audience: row.text("audience").unwrap_or_default().to_string(),
            },
        }))
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
    #[serde(default)]
    resource: String,
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
