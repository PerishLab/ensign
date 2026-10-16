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

pub struct Wire {
    base: String,
    agent: Agent,
}

impl Wire {
    pub fn new(base: &str) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Self {
            base: base.to_string(),
            agent,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    pub fn post(&self, path: &str, body: Value, head: &Head) -> Reply {
        let mut ask = self.agent.post(self.url(path));
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(
            ask.header("content-type", "application/json")
                .send(body.to_string()),
        )
    }

    pub fn patch(&self, path: &str, body: Value, head: &Head) -> Reply {
        let mut ask = self.agent.patch(self.url(path));
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(
            ask.header("content-type", "application/json")
                .send(body.to_string()),
        )
    }

    pub fn get(&self, path: &str, head: &Head) -> Reply {
        self.fetch(&self.url(path), head)
    }

    pub fn fetch(&self, url: &str, head: &Head) -> Reply {
        let mut ask = self.agent.get(url);
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(ask.call())
    }

    pub fn query(&self, path: &str, head: &Head, pairs: &[(&str, &str)]) -> Reply {
        let mut ask = self
            .agent
            .get(self.url(path))
            .query_pairs(pairs.iter().copied());
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(ask.call())
    }

    pub fn cut(&self, path: &str, head: &Head) -> Reply {
        let mut ask = self.agent.delete(self.url(path));
        for (key, value) in head {
            ask = ask.header(key, value);
        }
        reply(ask.call())
    }

    pub fn form(&self, path: &str, fields: &[(&str, &str)]) -> Reply {
        reply(
            self.agent
                .post(self.url(path))
                .send_form(fields.iter().copied()),
        )
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
    let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
    Reply { status, body, head }
}
