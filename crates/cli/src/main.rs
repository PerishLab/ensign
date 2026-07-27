mod config;

use clap::Parser;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::exit;

#[derive(Parser)]
#[command(
    disable_help_flag = true,
    disable_version_flag = true,
    trailing_var_arg = true
)]
struct Cli {
    #[arg(allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    let args = Cli::parse().args;
    let verb = args.first().map(String::as_str).unwrap_or("help");
    let rest = Rest(&args[args.len().min(1)..]);
    let done = match verb {
        "login" => rest.login(),
        "logout" => logout(),
        "whoami" => whoami(),
        "invite" => rest.invite(),
        "actors" => list("Actor", &["login", "name", "kind"]),
        "teams" => list("Team", &["name"]),
        "team" => rest.team(),
        "apps" => list("App", &["name", "slug", "mode"]),
        "app" => rest.app(),
        _ => usage(),
    };
    if let Err(note) = done {
        eprintln!("ensign: {note}");
        exit(1);
    }
}

fn usage() -> Reply {
    eprintln!("ensign — a client for the ensign identity provider");
    eprintln!();
    eprintln!("  login <login>        sign in; reads the password from stdin");
    eprintln!("  logout               revoke this token and forget it");
    eprintln!("  whoami               show the signed-in identity");
    eprintln!("  invite [note]        open an invite code");
    eprintln!("  actors               list actors");
    eprintln!("  teams                list teams");
    eprintln!("  team <name>          found a team");
    eprintln!("  apps                 list apps");
    eprintln!("  app <name> <slug> <home> <redirect>   register an oidc app");
    Ok(())
}

fn logout() -> Reply {
    let (base, token) = seat()?;
    let (code, _) = call("DELETE", &format!("{base}/bearer"), Some(&token), None)?;
    wipe()?;
    if code != 204 {
        return Err(fault("revoke", code));
    }
    println!("signed out");
    Ok(())
}

fn whoami() -> Reply {
    let (base, token) = seat()?;
    let (code, body) = call("GET", &format!("{base}/whoami"), Some(&token), None)?;
    if code != 200 {
        return Err(fault("whoami", code));
    }
    let login = field(&body, "login");
    let name = field(&body, "name");
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    println!("{login} ({name}) — operator {id}");
    Ok(())
}

fn list(unit: &str, cols: &[&str]) -> Reply {
    let (base, token) = seat()?;
    let (code, body) = call("GET", &format!("{base}/{unit}"), Some(&token), None)?;
    if code != 200 {
        return Err(fault("list", code));
    }
    let rows = body.as_array().ok_or("expected a list")?;
    for row in rows {
        let line: Vec<String> = cols.iter().map(|col| field(row, col)).collect();
        println!("{}", line.join("\t"));
    }
    Ok(())
}

struct Rest<'a>(&'a [String]);

impl Rest<'_> {
    fn login(&self) -> Reply {
        let who = self.0.first().ok_or("login takes a login name")?;
        let base = base()?;
        let pass = ask()?;
        let (code, body) = call(
            "POST",
            &format!("{base}/bearer"),
            None,
            Some(json!({ "login": who, "pass": pass.trim(), "name": "cli" })),
        )?;
        if code != 201 {
            return Err(fault("sign in", code));
        }
        let token = body
            .get("token")
            .and_then(Value::as_str)
            .ok_or("no token in the reply")?;
        save(&base, token)?;
        println!("signed in to {base} as {who}");
        Ok(())
    }

    fn invite(&self) -> Reply {
        let (base, token) = seat()?;
        let note = self.0.first().map(String::as_str).unwrap_or("");
        let (code, body) = call(
            "POST",
            &format!("{base}/invite"),
            Some(&token),
            Some(json!({ "note": note })),
        )?;
        if code != 201 {
            return Err(fault("open an invite", code));
        }
        println!("{}", field(&body, "code"));
        Ok(())
    }

    fn team(&self) -> Reply {
        let (base, token) = seat()?;
        let name = self.0.first().ok_or("team takes a name")?;
        let (code, _) = call(
            "POST",
            &format!("{base}/Team"),
            Some(&token),
            Some(json!({ "name": name })),
        )?;
        if code != 201 {
            return Err(fault("found a team", code));
        }
        println!("founded {name}");
        Ok(())
    }

    fn app(&self) -> Reply {
        let (base, token) = seat()?;
        if self.0.len() < 4 {
            return Err("app takes <name> <slug> <home> <redirect>".into());
        }
        let (code, _) = call(
            "POST",
            &format!("{base}/App"),
            Some(&token),
            Some(json!({
                "name": self.0[0],
                "slug": self.0[1],
                "home": self.0[2],
                "redirect": self.0[3],
                "secret": "",
                "mode": "oidc",
            })),
        )?;
        if code != 201 {
            return Err(fault("register an app", code));
        }
        println!("registered {}", self.0[1]);
        Ok(())
    }
}

type Reply = Result<(), String>;
type Deal = (u16, Value);

fn call(verb: &str, url: &str, token: Option<&str>, body: Option<Value>) -> Result<Deal, String> {
    let config = ureq::config::Config::builder()
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut res = match verb {
        "POST" => {
            let mut req = agent.post(url);
            if let Some(token) = token {
                req = req.header("authorization", &format!("token {token}"));
            }
            req.send_json(body.unwrap_or(Value::Null))
        }
        "DELETE" => {
            let mut req = agent.delete(url);
            if let Some(token) = token {
                req = req.header("authorization", &format!("token {token}"));
            }
            req.call()
        }
        _ => {
            let mut req = agent.get(url);
            if let Some(token) = token {
                req = req.header("authorization", &format!("token {token}"));
            }
            req.call()
        }
    }
    .map_err(|err| format!("unreachable: {err}"))?;
    let code = res.status().as_u16();
    let read: Value = res.body_mut().read_json().unwrap_or(Value::Null);
    Ok((code, read))
}

fn base() -> Result<String, String> {
    config::base()
}

fn seat() -> Result<(String, String), String> {
    load().ok_or_else(|| "not signed in — run `ensign login <login>`".into())
}

fn home() -> Result<std::path::PathBuf, String> {
    config::home()
}

fn load() -> Option<(String, String)> {
    let text = std::fs::read_to_string(home().ok()?.join("credential")).ok()?;
    let mut lines = text.lines();
    let base = lines.next()?.to_string();
    let token = lines.next()?.to_string();
    Some((base, token))
}

fn save(base: &str, token: &str) -> Reply {
    let dir = home()?;
    std::fs::create_dir_all(&dir).map_err(|err| format!("credential dir: {err}"))?;
    let path = dir.join("credential");
    let mut file = std::fs::File::create(&path).map_err(|err| format!("credential: {err}"))?;
    file.write_all(format!("{base}\n{token}\n").as_bytes())
        .map_err(|err| format!("credential: {err}"))?;
    guard(&path);
    Ok(())
}

fn wipe() -> Reply {
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

fn ask() -> Result<String, String> {
    eprint!("password: ");
    std::io::stderr().flush().ok();
    let mut pass = String::new();
    std::io::stdin()
        .read_to_string(&mut pass)
        .map_err(|err| format!("read password: {err}"))?;
    Ok(pass)
}

fn field(row: &Value, col: &str) -> String {
    row.get(col)
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default()
}

fn fault(deed: &str, code: u16) -> String {
    match code {
        401 => format!("{deed}: not signed in or wrong credentials"),
        403 => format!("{deed}: refused"),
        404 => format!("{deed}: not found"),
        409 => format!("{deed}: already taken"),
        _ => format!("{deed}: failed ({code})"),
    }
}
