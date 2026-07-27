use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::http::StatusCode;
use serde_json::{Map, Value};

pub(crate) trait Sound<T> {
    fn sound(self) -> Result<T, StatusCode>;
}

impl<T> Sound<T> for Result<T, keel::adapt::Error> {
    fn sound(self) -> Result<T, StatusCode> {
        self.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub(crate) fn text(body: &Map<String, Value>, name: &str) -> Result<String, StatusCode> {
    body.get(name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(StatusCode::BAD_REQUEST)
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

pub fn lock(pass: &str) -> Result<String, argon2::password_hash::Error> {
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
