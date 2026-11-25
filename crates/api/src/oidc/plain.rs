use axum::Json;
use axum::http::{HeaderMap, StatusCode, Uri};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Value, json};

pub(crate) fn origin(text: &str) -> bool {
    if text.contains('#') || text.chars().any(|ch| ch.is_control() || ch.is_whitespace()) {
        return false;
    }
    let Ok(uri) = text.parse::<Uri>() else {
        return false;
    };
    let Some(auth) = uri.authority() else {
        return false;
    };
    if auth.as_str().contains('@') || auth.host().is_empty() {
        return false;
    }
    match uri.scheme_str() {
        Some("https") => true,
        Some("http") => matches!(auth.host(), "localhost" | "127.0.0.1"),
        _ => false,
    }
}

pub(crate) fn safe(redirect: &str) -> bool {
    origin(redirect) && !redirect.contains('?')
}

pub(crate) fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_string)
}

pub(crate) fn seal(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut sponge = Sha256::new();
    sponge.update(text.as_bytes());
    B64.encode(sponge.finalize())
}

pub(crate) fn sour(note: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": note })))
}

pub(crate) fn spoil() -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "server_error" })),
    )
}
