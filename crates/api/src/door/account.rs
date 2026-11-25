use crate::booth::Booth;
use crate::util::{Sound, fault, parse};
use axum::http::StatusCode;
use axum::{Extension, Json};
use keel::{Op, Operator, Wire, form};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rename {
    name: String,
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn rename(
        &self,
        op: Option<Extension<Operator>>,
        Json(raw): Json<Value>,
    ) -> Result<Json<Value>, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let body: Rename = parse(raw)?;
        let trimmed = body.name.trim();
        let len = trimmed.chars().count();
        if len == 0 || len > 120 || body.name.chars().any(char::is_control) {
            return Err(StatusCode::BAD_REQUEST);
        }
        let seat = me.to_string();
        let face = self.core.of(me);
        let profile = face
            .one(&form("Profile").when("actor", Op::Eq, &seat))
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
        face.set("Profile", profile.key(), &[("name", trimmed)])
            .await
            .map_err(|err| fault(&err))?;
        let login = profile
            .text("handle")
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let actor = face
            .one(&form("Actor").when("id", Op::Eq, &seat))
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let sub = actor.text("sub").ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(json!({
            "id": me,
            "sub": sub,
            "login": login,
            "name": trimmed,
        })))
    }
}
