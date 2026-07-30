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

pub(crate) fn load() -> Option<(String, String)> {
    let text = std::fs::read_to_string(home().ok()?.join("credential")).ok()?;
    let mut lines = text.lines();
    let base = lines.next()?.to_string();
    let token = lines.next()?.to_string();
    Some((base, token))
}

pub(crate) fn save(base: &str, token: &str) -> Reply {
    let dir = home()?;
    std::fs::create_dir_all(&dir).map_err(|err| format!("credential dir: {err}"))?;
    let path = dir.join("credential");
    let mut file = std::fs::File::create(&path).map_err(|err| format!("credential: {err}"))?;
    file.write_all(format!("{base}\n{token}\n").as_bytes())
        .map_err(|err| format!("credential: {err}"))?;
    guard(&path);
    Ok(())
}

pub(crate) fn wipe() -> Reply {
    if let Ok(path) = home().map(|dir| dir.join("credential")) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
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
