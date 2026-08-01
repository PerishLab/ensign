use crate::config;
use serde_json::Value;
use std::io::{Read, Write};

pub(crate) type Reply = Result<(), String>;
pub(crate) type Deal = (u16, Value);

pub(crate) fn call(
    verb: &str,
    url: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<Deal, String> {
    let worn = token.map(|held| format!("token {held}"));
    send(verb, url, worn.as_deref(), body)
}

pub(crate) fn send(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<Value>,
) -> Result<Deal, String> {
    let config = ureq::config::Config::builder()
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut res = match verb {
        "POST" => {
            let mut req = agent.post(url);
            if let Some(token) = auth {
                req = req.header("authorization", token);
            }
            req.send_json(body.unwrap_or(Value::Null))
        }
        "DELETE" => {
            let mut req = agent.delete(url);
            if let Some(token) = auth {
                req = req.header("authorization", token);
            }
            req.call()
        }
        _ => {
            let mut req = agent.get(url);
            if let Some(token) = auth {
                req = req.header("authorization", token);
            }
            req.call()
        }
    }
    .map_err(|err| format!("unreachable: {err}"))?;
    let code = res.status().as_u16();
    let read: Value = res.body_mut().read_json().unwrap_or(Value::Null);
    Ok((code, read))
}

pub(crate) fn base() -> Result<String, String> {
    config::base()
}

pub(crate) fn seat() -> Result<(String, String), String> {
    load().ok_or_else(|| "not signed in — run `ensign login <login>`".into())
}

pub(crate) fn home() -> Result<std::path::PathBuf, String> {
    config::home()
}

const SCHEMA: u64 = 1;

pub(crate) fn kept() -> Result<std::path::PathBuf, String> {
    Ok(home()?.join("state").join("pat.json"))
}

pub(crate) fn load() -> Option<(String, String)> {
    let text = std::fs::read_to_string(kept().ok()?).ok()?;
    let held: Value = serde_json::from_str(&text).ok()?;
    if held.get("schema").and_then(Value::as_u64) != Some(SCHEMA) {
        return None;
    }
    let base = held.get("base")?.as_str()?.to_string();
    let token = held.get("token")?.as_str()?.to_string();
    Some((base, token))
}

pub(crate) fn save(base: &str, token: &str) -> Reply {
    let path = kept()?;
    let dir = path
        .parent()
        .ok_or_else(|| "credential has no directory".to_string())?;
    std::fs::create_dir_all(dir).map_err(|err| format!("credential dir: {err}"))?;
    let held = serde_json::json!({ "schema": SCHEMA, "base": base, "token": token });
    let beside = path.with_extension("json.new");
    let mut file = std::fs::File::create(&beside).map_err(|err| format!("credential: {err}"))?;
    guard(&beside);
    file.write_all(held.to_string().as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|err| format!("credential: {err}"))?;
    std::fs::rename(&beside, &path).map_err(|err| format!("credential: {err}"))?;
    guard(&path);
    shed();
    Ok(())
}

pub(crate) fn wipe() -> Reply {
    if let Ok(path) = kept() {
        let _ = std::fs::remove_file(path);
    }
    shed();
    Ok(())
}

fn shed() {
    if let Ok(dir) = home() {
        let _ = std::fs::remove_file(dir.join("credential"));
    }
}

#[cfg(unix)]
fn guard(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn guard(_: &std::path::Path) {}

pub(crate) fn ask(label: &str) -> Result<String, String> {
    eprint!("{label}: ");
    std::io::stderr().flush().ok();
    let mut held = String::new();
    std::io::stdin()
        .read_to_string(&mut held)
        .map_err(|err| format!("read {label}: {err}"))?;
    Ok(held)
}

pub(crate) fn field(row: &Value, col: &str) -> String {
    row.get(col)
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default()
}

pub(crate) fn fault(deed: &str, code: u16) -> String {
    match code {
        401 => format!("{deed}: not signed in or wrong credentials"),
        403 => format!("{deed}: refused"),
        404 => format!("{deed}: not found"),
        409 => format!("{deed}: already taken"),
        _ => format!("{deed}: failed ({code})"),
    }
}
