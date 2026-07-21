use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use keel::Cell;
use serde_json::{Value, json};

pub(crate) fn scrub(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(crate) fn safe(redirect: &str) -> bool {
    redirect.starts_with("https://")
        || redirect.starts_with("http://127.0.0.1")
        || redirect.starts_with("http://localhost")
}

pub(crate) fn pct(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub(crate) fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::to_string)
}

pub(crate) fn cell(row: &keel::Row, name: &str) -> String {
    row.cells().get(name).map(Cell::show).unwrap_or_default()
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

pub(crate) fn wild() -> String {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("os entropy");
    seed.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|gap| gap.as_secs() as i64)
        .unwrap_or(0)
}
