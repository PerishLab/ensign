#![allow(dead_code)]

use serde_json::{Value, json};
use ureq::http::Response;
use ureq::{Agent, Body};

pub type Head = Vec<(String, String)>;

pub struct Reply {
    pub status: u16,
    pub body: Value,
    head: Head,
}

impl Reply {
    pub fn new(status: u16, text: &str, head: Head) -> Self {
        let body = serde_json::from_str(text).unwrap_or(Value::String(text.to_string()));
        Self { status, body, head }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.head
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn text(&self, key: &str) -> String {
        self.body[key].as_str().unwrap_or_default().to_string()
    }

    pub fn id(&self) -> i64 {
        self.body["id"].as_i64().expect("row id")
    }

    pub fn cookie(&self) -> String {
        self.head
            .iter()
            .filter(|(key, _)| key.eq_ignore_ascii_case("set-cookie"))
            .filter_map(|(_, value)| value.split(';').next())
            .find(|value| value.starts_with("session="))
            .expect("no session cookie")
            .to_string()
    }
}

pub struct Face {
    pub id: i64,
    pub head: Head,
}

pub struct Ask {
    pub verb: &'static str,
    pub url: String,
    pub head: Head,
    pub body: Option<(&'static str, String)>,
}

pub trait Line {
    fn send(&self, ask: Ask) -> Reply;
}

struct Http(Agent);

impl Line for Http {
    fn send(&self, ask: Ask) -> Reply {
        let mut held = ask.head;
        if let Some((kind, _)) = &ask.body {
            held.push(("content-type".into(), kind.to_string()));
        }
        let text = ask.body.map(|(_, text)| text).unwrap_or_default();
        let agent = &self.0;
        let res = match ask.verb {
            "POST" => dress(agent.post(&ask.url), &held).send(text),
            "PATCH" => dress(agent.patch(&ask.url), &held).send(text),
            "DELETE" => dress(agent.delete(&ask.url), &held).call(),
            _ => dress(agent.get(&ask.url), &held).call(),
        };
        reply(res)
    }
}

fn dress<B>(mut ask: ureq::RequestBuilder<B>, head: &Head) -> ureq::RequestBuilder<B> {
    for (key, value) in head {
        ask = ask.header(key, value);
    }
    ask
}

pub struct Wire {
    base: String,
    line: Box<dyn Line>,
}

impl Wire {
    pub fn new(base: &str) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Self::on(base, Box::new(Http(agent)))
    }

    pub fn on(base: &str, line: Box<dyn Line>) -> Self {
        Self {
            base: base.to_string(),
            line,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn send(
        &self,
        verb: &'static str,
        url: String,
        head: &Head,
        body: Option<(&'static str, String)>,
    ) -> Reply {
        self.line.send(Ask {
            verb,
            url,
            head: head.clone(),
            body,
        })
    }

    pub fn post(&self, path: &str, body: Value, head: &Head) -> Reply {
        let body = Some(("application/json", body.to_string()));
        self.send("POST", self.url(path), head, body)
    }

    pub fn patch(&self, path: &str, body: Value, head: &Head) -> Reply {
        let body = Some(("application/json", body.to_string()));
        self.send("PATCH", self.url(path), head, body)
    }

    pub fn get(&self, path: &str, head: &Head) -> Reply {
        self.fetch(&self.url(path), head)
    }

    pub fn fetch(&self, url: &str, head: &Head) -> Reply {
        self.send("GET", url.to_string(), head, None)
    }

    pub fn query(&self, path: &str, head: &Head, pairs: &[(&str, &str)]) -> Reply {
        let url = format!("{}?{}", self.url(path), encode(pairs));
        self.send("GET", url, head, None)
    }

    pub fn cut(&self, path: &str, head: &Head) -> Reply {
        self.send("DELETE", self.url(path), head, None)
    }

    pub fn form(&self, path: &str, fields: &[(&str, &str)]) -> Reply {
        let body = Some(("application/x-www-form-urlencoded", encode(fields)));
        self.send("POST", self.url(path), &Vec::new(), body)
    }

    pub fn enrol(&self, crown: &Head, login: &str) -> Face {
        let seat = self.post("/invite", json!({ "note": login }), crown);
        let pass = format!("pass-{login}");
        let made = self.post(
            "/join",
            json!({ "code": seat.text("code"), "login": login, "name": login, "pass": pass }),
            &Vec::new(),
        );
        let back = self.post(
            "/login",
            json!({ "login": login, "pass": pass }),
            &Vec::new(),
        );
        Face {
            id: made.id(),
            head: vec![("cookie".into(), back.cookie())],
        }
    }

    pub fn grant(&self, crown: &Head, who: &str, rule: (&str, &str)) {
        let (verb, unit) = rule;
        let made = self.post(
            "/@grant",
            json!({ "who": who, "verb": verb, "unit": unit, "scope": "all" }),
            crown,
        );
        assert_eq!(made.status, 201, "grant");
    }
}

pub fn app(slug: &str) -> Value {
    json!({
        "name": slug,
        "slug": slug,
        "home": format!("https://{slug}.perish.top"),
        "redirect": format!("https://{slug}.perish.top/cb"),
        "secret": "",
        "mode": "oidc",
    })
}

pub fn encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", pct(key), pct(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn pct(text: &str) -> String {
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

pub fn bare() -> Head {
    Vec::new()
}

fn reply(result: Result<Response<Body>, ureq::Error>) -> Reply {
    let mut res = result.expect("request");
    let status = res.status().as_u16();
    let head = res
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let text = res.body_mut().read_to_string().unwrap_or_default();
    Reply::new(status, &text, head)
}
