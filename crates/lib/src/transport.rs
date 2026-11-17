use serde_json::Value;

pub(crate) fn agent() -> ureq::Agent {
    ureq::Agent::new_with_config(
        ureq::config::Config::builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build(),
    )
}

pub(crate) fn call(
    verb: &str,
    url: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<(u16, Value), String> {
    let worn = token.map(|held| format!("token {held}"));
    send(verb, url, worn.as_deref(), body)
}

pub(crate) fn send(
    verb: &str,
    url: &str,
    auth: Option<&str>,
    body: Option<Value>,
) -> Result<(u16, Value), String> {
    let agent = agent();
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
        "GET" => {
            let mut req = agent.get(url);
            if let Some(token) = auth {
                req = req.header("authorization", token);
            }
            req.call()
        }
        _ => return Err("unsupported Ensign HTTP method".into()),
    }
    .map_err(|err| format!("unreachable: {err}"))?;
    let code = res.status().as_u16();
    let read: Value = res.body_mut().read_json().unwrap_or(Value::Null);
    Ok((code, read))
}
