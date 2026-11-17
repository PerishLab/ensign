use crate::wire::{Reply, ask, base, call, fault, field, seat, send};
use serde_json::{Value, json};

pub(crate) fn logout() -> Reply {
    seat()?.logout()?;
    println!("signed out");
    Ok(())
}

pub(crate) fn whoami() -> Reply {
    let client = seat()?;
    let (code, body) = client.call("GET", "/whoami", None)?;
    if code != 200 {
        return Err(fault("whoami", code));
    }
    let login = field(&body, "login");
    let name = field(&body, "name");
    let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
    println!("{login} ({name}) — operator {id}");
    Ok(())
}

pub(crate) fn list(unit: &str, cols: &[&str]) -> Reply {
    let client = seat()?;
    let (code, body) = client.call("GET", &format!("/{unit}"), None)?;
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

pub(crate) struct Rest<'a>(pub(crate) &'a [String]);

impl Rest<'_> {
    pub(crate) fn login(&self) -> Reply {
        let who = self.0.first().ok_or("login takes a login name")?;
        let pass = ask("password")?;
        let client = ensign::Client::login(who, pass.trim())?;
        let base = client.base();
        println!("signed in to {base} as {who}");
        Ok(())
    }

    pub(crate) fn join(&self) -> Reply {
        if self.0.len() < 3 {
            return Err("join takes <code> <login> <name>".into());
        }
        let base = base()?;
        let pass = ask("password")?;
        let (code, body) = call(
            "POST",
            &format!("{base}/join"),
            None,
            Some(json!({
                "code": self.0[0],
                "login": self.0[1],
                "name": self.0[2],
                "pass": pass.trim(),
            })),
        )?;
        if code != 201 {
            return Err(fault("redeem the invite", code));
        }
        let id = body.get("id").and_then(Value::as_i64).unwrap_or(0);
        println!("joined {} as actor {id}", self.0[1]);
        Ok(())
    }

    pub(crate) fn recover(&self) -> Reply {
        if self.0.len() < 2 {
            return Err("recover takes <login> <code>".into());
        }
        let base = base()?;
        let pass = ask("password")?;
        let (code, _) = call(
            "POST",
            &format!("{base}/revive"),
            None,
            Some(json!({
                "login": self.0[0],
                "code": self.0[1],
                "pass": pass.trim(),
            })),
        )?;
        if code != 204 {
            return Err(fault("burn the rescue code", code));
        }
        println!("recovered {}", self.0[0]);
        Ok(())
    }

    pub(crate) fn crown(&self) -> Reply {
        let who = self.0.first().ok_or("crown takes a login")?;
        let base = base()?;
        let held = ask("sudo")?;
        let sudo = held.trim();
        if sudo.is_empty() {
            return Err("crown reads the sudo token from stdin".into());
        }
        let worn = format!("sudo {sudo}");
        let (code, body) = send("GET", &format!("{base}/handle/{who}"), Some(&worn), None)?;
        if code == 404 {
            return Err(format!("unknown handle: {who}"));
        }
        if code != 200 {
            return Err(fault("resolve handle", code));
        }
        let id = body
            .get("id")
            .and_then(Value::as_i64)
            .ok_or("handle has no actor")?;
        let (made, _) = send(
            "POST",
            &format!("{base}/@grant"),
            Some(&worn),
            Some(json!({
                "who": id.to_string(),
                "verb": "*",
                "unit": "*",
                "scope": "all",
            })),
        )?;
        if made != 201 {
            return Err(fault("grant site admin", made));
        }
        println!("crowned {who} as actor {id}");
        Ok(())
    }

    pub(crate) fn invite(&self) -> Reply {
        let client = seat()?;
        let note = self.0.first().map(String::as_str).unwrap_or("");
        let (code, body) = client.call("POST", "/invite", Some(json!({ "note": note })))?;
        if code != 201 {
            return Err(fault("open an invite", code));
        }
        println!("{}", field(&body, "code"));
        Ok(())
    }

    pub(crate) fn team(&self) -> Reply {
        let client = seat()?;
        let name = self.0.first().ok_or("team takes a name")?;
        let (code, _) = client.call("POST", "/Team", Some(json!({ "name": name })))?;
        if code != 201 {
            return Err(fault("found a team", code));
        }
        println!("founded {name}");
        Ok(())
    }

    pub(crate) fn app(&self) -> Reply {
        let client = seat()?;
        if self.0.len() < 4 {
            return Err("app takes <name> <slug> <home> <redirect>".into());
        }
        let (code, _) = client.call(
            "POST",
            "/App",
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
