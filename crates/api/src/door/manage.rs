use crate::booth::Booth;
use crate::oidc::{origin, safe};
use crate::util::{Sound, fault, parse, private};
use axum::extract::Path as Route;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::{Op, Operator, Row, Wire, form};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Craft {
    name: String,
    slug: String,
    home: Option<String>,
    redirect: Option<String>,
    mode: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Amend {
    name: Option<String>,
    home: Option<String>,
    redirect: Option<String>,
    barred: Option<bool>,
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn apps(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let rows = self.core.of(me).live("App").await.sound()?;
        let out: Vec<Value> = rows.iter().map(shape).collect();
        Ok((StatusCode::OK, private(), Json(json!(out))))
    }

    pub(crate) async fn register(
        &self,
        op: Option<Extension<Operator>>,
        Json(raw): Json<Value>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Craft = parse(raw)?;
        let name = body.name.trim();
        let len = name.chars().count();
        if len == 0 || len > 120 || body.name.chars().any(char::is_control) {
            return Err(StatusCode::BAD_REQUEST);
        }
        if !slug(&body.slug) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let home = body.home.unwrap_or_default();
        let redirect = body.redirect.unwrap_or_default();
        if !valid(&body.mode, &home, &redirect) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let face = self.core.of(me);
        let fields = [
            ("name", name),
            ("slug", body.slug.as_str()),
            ("home", home.as_str()),
            ("redirect", redirect.as_str()),
            ("secret", ""),
            ("mode", body.mode.as_str()),
            ("barred", "false"),
        ];
        let id = face.put("App", &fields).await.map_err(|err| fault(&err))?;
        let row = face
            .one(&form("App").when("id", Op::Eq, &id.to_string()))
            .await
            .sound()?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok((StatusCode::CREATED, Json(shape(&row))))
    }

    pub(crate) async fn amend(
        &self,
        op: Option<Extension<Operator>>,
        Route(id): Route<i64>,
        Json(raw): Json<Value>,
    ) -> Result<Json<Value>, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Amend = parse(raw)?;
        let ask = form("App").when("id", Op::Eq, &id.to_string());
        let face = self.core.of(me);
        let existing = face.one(&ask).await.sound()?.ok_or(StatusCode::NOT_FOUND)?;
        let mode = existing.text("mode").unwrap_or_default();
        let pairs = collect(body, mode)?;
        if pairs.is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        let refs: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        face.set("App", id, &refs)
            .await
            .map_err(|err| fault(&err))?;
        let row = face.one(&ask).await.sound()?.ok_or(StatusCode::NOT_FOUND)?;
        Ok(Json(shape(&row)))
    }
}

fn collect(body: Amend, mode: &str) -> Result<Vec<(&'static str, String)>, StatusCode> {
    let mut pairs = Vec::new();
    if let Some(name) = body.name {
        let trimmed = name.trim();
        let len = trimmed.chars().count();
        if len == 0 || len > 120 || name.chars().any(char::is_control) {
            return Err(StatusCode::BAD_REQUEST);
        }
        pairs.push(("name", trimmed.to_string()));
    }
    if let Some(home) = body.home {
        if !site(mode, &home) {
            return Err(StatusCode::BAD_REQUEST);
        }
        pairs.push(("home", home));
    }
    if let Some(redirect) = body.redirect {
        if !back(mode, &redirect) {
            return Err(StatusCode::BAD_REQUEST);
        }
        pairs.push(("redirect", redirect));
    }
    if let Some(barred) = body.barred {
        pairs.push(("barred", barred.to_string()));
    }
    Ok(pairs)
}

fn shape(row: &Row) -> Value {
    json!({
        "id": row.key(),
        "name": row.text("name").unwrap_or_default(),
        "slug": row.text("slug").unwrap_or_default(),
        "home": row.text("home").unwrap_or_default(),
        "redirect": row.text("redirect").unwrap_or_default(),
        "mode": row.text("mode").unwrap_or_default(),
        "barred": row.flag("barred") == Some(true),
        "created": row.created(),
    })
}

fn slug(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    let edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    if !edge(bytes[0]) || !edge(bytes[bytes.len() - 1]) {
        return false;
    }
    bytes.iter().all(|&byte| edge(byte) || byte == b'-')
}

fn site(mode: &str, home: &str) -> bool {
    match mode {
        "oidc" | "forward" => origin(home),
        "resource" => home.is_empty() || origin(home),
        _ => false,
    }
}

fn back(mode: &str, redirect: &str) -> bool {
    match mode {
        "oidc" => safe(redirect),
        "forward" | "resource" => redirect.is_empty() || safe(redirect),
        _ => false,
    }
}

fn valid(mode: &str, home: &str, redirect: &str) -> bool {
    matches!(mode, "oidc" | "forward" | "resource") && site(mode, home) && back(mode, redirect)
}
