pub mod config;
mod custody;
mod transport;

use radicula_oidc::{Secret, exchange};
use serde::Deserialize;
use serde_json::{Value, json};

pub struct Client {
    base: String,
    token: Secret,
}

#[derive(Debug, Deserialize)]
pub struct Discovery {
    pub issuer: String,
    #[serde(rename = "token_endpoint")]
    pub endpoint: String,
    #[serde(rename = "jwks_uri")]
    pub keys: String,
}

#[derive(Debug, Deserialize)]
pub struct Identity {
    pub sub: String,
    pub login: String,
    pub name: String,
}

impl Client {
    pub fn retained() -> Result<Self, String> {
        let (base, token) = custody::load()?;
        Ok(Self { base, token })
    }

    pub fn login(login: &str, password: &str) -> Result<Self, String> {
        let base = config::base()?;
        let (code, body) = transport::call(
            "POST",
            &format!("{base}/bearer"),
            None,
            Some(json!({"login": login, "pass": password, "name": "cli"})),
        )?;
        if code != 201 {
            return Err(format!("sign in: failed ({code})"));
        }
        let token = body
            .get("token")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or("sign in: no token")?;
        custody::save(&base, token)?;
        Ok(Self {
            base,
            token: Secret::new(token.into()),
        })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn call(
        &self,
        verb: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<(u16, Value), String> {
        if !path.starts_with('/') || path.starts_with("//") || path.contains(['?', '#']) {
            return Err("invalid Ensign resource path".into());
        }
        transport::call(
            verb,
            &format!("{}{path}", self.base),
            Some(self.token.expose()),
            body,
        )
    }

    pub fn identity(&self) -> Result<Identity, String> {
        let (code, body) = self.call("GET", "/whoami", None)?;
        if code != 200 {
            return Err(format!("whoami: failed ({code})"));
        }
        let identity: Identity =
            serde_json::from_value(body).map_err(|_| "invalid identity reply")?;
        if identity.sub.is_empty() {
            return Err("identity has no subject".into());
        }
        Ok(identity)
    }

    pub fn logout(self) -> Result<(), String> {
        let (code, _) = self.call("DELETE", "/bearer", None)?;
        if code != 204 {
            return Err(format!("revoke: failed ({code})"));
        }
        custody::wipe()
    }

    pub fn discovery(&self) -> Result<Discovery, String> {
        let mut reply = transport::agent()
            .get(format!("{}/.well-known/openid-configuration", self.base))
            .call()
            .map_err(|_| "discovery transport failed")?;
        if reply.status().as_u16() != 200 {
            return Err(format!("discovery: failed ({})", reply.status().as_u16()));
        }
        let document: Discovery = reply
            .body_mut()
            .read_json()
            .map_err(|_| "invalid discovery reply")?;
        if document.issuer.is_empty() || document.endpoint.is_empty() || document.keys.is_empty() {
            return Err("incomplete discovery reply".into());
        }
        Ok(document)
    }

    pub fn exchange(&self, resource: &str) -> Result<exchange::Response, String> {
        let request =
            exchange::Request::new(Secret::new(self.token.expose().into()), resource.into());
        request
            .validate()
            .map_err(|_| "invalid token exchange request")?;
        let mut reply = transport::agent()
            .post(format!("{}/token", self.base))
            .send_form([
                ("grant_type", request.grant.as_str()),
                ("subject_token", request.subject.expose()),
                ("subject_token_type", request.sort.as_str()),
                ("resource", request.resource.as_str()),
            ])
            .map_err(|_| "token exchange transport failed")?;
        if reply.status().as_u16() != 200 {
            return Err(format!(
                "token exchange: failed ({})",
                reply.status().as_u16()
            ));
        }
        let token: exchange::Response = reply
            .body_mut()
            .read_json()
            .map_err(|_| "invalid token exchange reply")?;
        token
            .validate()
            .map_err(|_| "invalid token exchange reply")?;
        Ok(token)
    }
}

#[cfg(test)]
mod tests;
