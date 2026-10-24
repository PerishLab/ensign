use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use keel::adapt::{Error, Store};
use serde_json::{Map, Value, json};
use std::cell::RefCell;

tokio::task_local! {
    static HELD: RefCell<Option<(StatusCode, String)>>;
}

pub(crate) trait Sound<T> {
    fn sound(self) -> Result<T, StatusCode>;
}

impl<T> Sound<T> for Result<T, Error> {
    fn sound(self) -> Result<T, StatusCode> {
        self.map_err(|err| lift(&err, StatusCode::INTERNAL_SERVER_ERROR))
    }
}

pub(crate) fn lift(error: &Error, other: StatusCode) -> StatusCode {
    let Error::Store(store) = error else {
        return other;
    };
    eprintln!("ensign: {store}: {}", store.detail());
    let status = match store {
        Store::Down(_) => StatusCode::SERVICE_UNAVAILABLE,
        Store::Taken(_) => StatusCode::CONFLICT,
        Store::Fault(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    let _ = HELD.try_with(|held| *held.borrow_mut() = Some((status, store.to_string())));
    status
}

fn answer(res: Response) -> Response {
    let Some((status, note)) = HELD.with(|held| held.borrow_mut().take()) else {
        return res;
    };
    if res.status() != status {
        return res;
    }
    (status, Json(json!({ "error": note }))).into_response()
}

pub(crate) async fn voice(req: Request, next: Next) -> Response {
    HELD.scope(RefCell::new(None), async { answer(next.run(req).await) })
        .await
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
