use crate::booth::Booth;
use crate::util::Sound;
use axum::extract::{Path as Route, State};
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::{Extension, Json};
use keel::{Operator, Wire};
use serde_json::{Value, json};

pub(crate) async fn shown<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    Route(sub): Route<String>,
) -> Result<Json<Value>, StatusCode> {
    let (handle, name) = booth
        .shown(&sub)
        .await
        .sound()?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "sub": sub, "handle": handle, "name": name })))
}

pub(crate) async fn known<W: Wire + 'static>(
    State(booth): State<Booth<W>>,
    op: Option<Extension<Operator>>,
    headers: HeaderMap,
    Route(handle): Route<String>,
) -> Result<Json<Value>, StatusCode> {
    if op.is_none() && !crowned(&booth, &headers).await? {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let id = booth
        .known(&handle)
        .await
        .sound()?
        .ok_or(StatusCode::NOT_FOUND)?;
    let (handle, name) = booth.tag(id).await.sound()?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(json!({ "id": id, "handle": handle, "name": name })))
}

async fn crowned<W: Wire + 'static>(
    booth: &Booth<W>,
    headers: &HeaderMap,
) -> Result<bool, StatusCode> {
    let Some(token) = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("sudo "))
    else {
        return Ok(false);
    };
    booth.core.seal(token).await.sound()
}
