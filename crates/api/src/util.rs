use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::http::{HeaderMap, StatusCode};
use keel::Cell;
use serde_json::{Map, Value};

pub(crate) fn owner(row: &keel::Row) -> Option<i64> {
    match row.cells().get("actor") {
        Some(Cell::Int(key)) => Some(*key),
        _ => None,
    }
}

pub(crate) fn text(body: &Map<String, Value>, name: &str) -> Result<String, StatusCode> {
    body.get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(StatusCode::BAD_REQUEST)
}

pub(crate) fn scrub(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

pub(crate) fn wearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("token ")
        .map(str::to_string)
}

pub(crate) fn crumb(headers: &HeaderMap) -> Option<String> {
    let jar = headers.get("cookie")?.to_str().ok()?;
    jar.split(';')
        .filter_map(|part| part.trim().strip_prefix("session="))
        .next()
        .map(str::to_string)
}

pub(crate) fn lock(pass: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default().hash_password(pass.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub(crate) fn fits(pass: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(pass.as_bytes(), &parsed)
        .is_ok()
}

pub(crate) fn digest(code: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    format!("{:x}", hasher.finalize())
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
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
