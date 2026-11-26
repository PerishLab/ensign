use axum::http::StatusCode;
use locus::Atom;
use serde_json::{Map, Value};

const LINES: usize = 512;
const FIELDS: [&str; 8] = [
    "at",
    "id",
    "producer",
    "context",
    "choices",
    "collections",
    "source",
    "payload",
];
const BANNED: [&str; 10] = [
    "password",
    "passphrase",
    "pass",
    "token",
    "secret",
    "authorization",
    "cookie",
    "verifier",
    "rescue",
    "sudo",
];

pub(crate) fn parse(bytes: &[u8], producer: &str) -> Result<Vec<String>, StatusCode> {
    let text = std::str::from_utf8(bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    if trimmed.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut out = Vec::new();
    for part in trimmed.split('\n') {
        let line = part.strip_suffix('\r').unwrap_or(part);
        if line.is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        if out.len() >= LINES {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        validate(line, producer)?;
        out.push(line.to_string());
    }
    Ok(out)
}

fn validate(line: &str, producer: &str) -> Result<(), StatusCode> {
    let value: Value = serde_json::from_str(line).map_err(|_| StatusCode::BAD_REQUEST)?;
    let Some(map) = value.as_object() else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if map.keys().any(|key| !FIELDS.contains(&key.as_str())) {
        return Err(StatusCode::BAD_REQUEST);
    }
    inspect(&value)?;
    let atom: Atom = serde_json::from_value(value).map_err(|_| StatusCode::BAD_REQUEST)?;
    if atom.producer() != Some(producer) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if atom.id().is_none_or(|id| id.trim().is_empty()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    roles(&atom)
}

fn roles(atom: &Atom) -> Result<(), StatusCode> {
    for choice in atom.choices() {
        if sensitive(choice.role()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    for item in atom.collections() {
        if sensitive(item.role()) {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    Ok(())
}

fn inspect(value: &Value) -> Result<(), StatusCode> {
    match value {
        Value::Object(map) => object(map),
        Value::Array(items) => array(items),
        _ => Ok(()),
    }
}

fn object(map: &Map<String, Value>) -> Result<(), StatusCode> {
    for (key, item) in map {
        if sensitive(key) {
            return Err(StatusCode::BAD_REQUEST);
        }
        inspect(item)?;
    }
    Ok(())
}

fn array(items: &[Value]) -> Result<(), StatusCode> {
    for item in items {
        inspect(item)?;
    }
    Ok(())
}

fn sensitive(key: &str) -> bool {
    if key.to_ascii_lowercase().contains("code_verifier") {
        return true;
    }
    let mut word = String::new();
    let mut prev = None;
    for ch in key.chars() {
        if !ch.is_ascii_alphanumeric() {
            if banned(&word) {
                return true;
            }
            word.clear();
            prev = None;
            continue;
        }
        if split(prev, ch) && banned(&word) {
            return true;
        }
        if split(prev, ch) {
            word.clear();
        }
        word.push(ch.to_ascii_lowercase());
        prev = Some(ch);
    }
    banned(&word)
}

fn split(prev: Option<char>, curr: char) -> bool {
    let lower = prev.is_some_and(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit());
    lower && curr.is_ascii_uppercase()
}

fn banned(word: &str) -> bool {
    BANNED.contains(&word)
}
