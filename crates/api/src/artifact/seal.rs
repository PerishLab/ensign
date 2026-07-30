use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde_json::{Value, json};

const SEAT: &str = "/var/run/secrets/kubernetes.io/serviceaccount";
const KEY: &str = "value";

struct Reach {
    host: String,
    token: String,
    space: String,
    root: Vec<u8>,
}

fn reach() -> Result<Reach, String> {
    let token = read("token")?;
    let space = read("namespace")?;
    let root = std::fs::read(format!("{SEAT}/ca.crt"))
        .map_err(|err| format!("cannot read the cluster root: {err}"))?;
    Ok(Reach {
        host: "https://kubernetes.default.svc".to_string(),
        token,
        space,
        root,
    })
}

fn read(leaf: &str) -> Result<String, String> {
    std::fs::read_to_string(format!("{SEAT}/{leaf}"))
        .map(|held| held.trim().to_string())
        .map_err(|err| format!("cannot read the service account {leaf}: {err}"))
}

fn agent(reach: &Reach) -> Result<ureq::Agent, String> {
    let root = ureq::tls::Certificate::from_pem(&reach.root)
        .map_err(|err| format!("malformed cluster root: {err}"))?;
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::Specific(std::sync::Arc::new(vec![
            root,
        ])))
        .build();
    let config = ureq::config::Config::builder()
        .http_status_as_error(false)
        .tls_config(tls)
        .build();
    Ok(ureq::Agent::new_with_config(config))
}

pub(crate) fn load(name: &str) -> Result<Option<Vec<u8>>, String> {
    let reach = reach()?;
    let agent = agent(&reach)?;
    let mut held = agent
        .get(&format!(
            "{}/api/v1/namespaces/{}/secrets/{name}",
            reach.host, reach.space
        ))
        .header("authorization", &format!("Bearer {}", reach.token))
        .call()
        .map_err(|err| format!("cannot reach the cluster for secret {name}: {err}"))?;
    let code = held.status().as_u16();
    if code == 404 {
        return Ok(None);
    }
    if code != 200 {
        return Err(format!("cluster refused secret {name}: {code}"));
    }
    let body: Value = held
        .body_mut()
        .read_json()
        .map_err(|err| format!("malformed secret {name}: {err}"))?;
    let coded = body
        .get("data")
        .and_then(|data| data.get(KEY))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("secret {name} has no {KEY}"))?;
    B64.decode(coded)
        .map(Some)
        .map_err(|err| format!("secret {name} is not base64: {err}"))
}

pub(crate) fn keep(name: &str, bytes: &[u8]) -> Result<bool, String> {
    let reach = reach()?;
    let agent = agent(&reach)?;
    let body = json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "type": "Opaque",
        "metadata": { "name": name, "namespace": reach.space },
        "data": { KEY: B64.encode(bytes) },
    });
    let held = agent
        .post(&format!(
            "{}/api/v1/namespaces/{}/secrets",
            reach.host, reach.space
        ))
        .header("authorization", &format!("Bearer {}", reach.token))
        .send_json(body)
        .map_err(|err| format!("cannot reach the cluster to keep secret {name}: {err}"))?;
    match held.status().as_u16() {
        200 | 201 => Ok(true),
        409 => Ok(false),
        code => Err(format!("cluster refused to keep secret {name}: {code}")),
    }
}
