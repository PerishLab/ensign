use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde_json::{Value, json};

pub(crate) fn safe(redirect: &str) -> bool {
    redirect.starts_with("https://")
        || redirect.starts_with("http://127.0.0.1")
        || redirect.starts_with("http://localhost")
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
