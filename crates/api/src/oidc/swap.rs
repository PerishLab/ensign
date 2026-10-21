use crate::oidc::plain::{sour, spoil};
use crate::oidc::{Grant, Grip, LIFE, Oidc, Ticket, Who};
use axum::Json;
use axum::http::{HeaderMap, HeaderValue};
use keel::Wire;
use serde_json::json;

pub(crate) const SWAP: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) async fn swap(&self, grant: Grant) -> Grip {
        if grant.sort.as_deref() != Some(ACCESS) {
            return Err(sour("invalid_request"));
        }
        let pat = grant
            .subject
            .filter(|pat| !pat.is_empty())
            .ok_or_else(|| sour("invalid_request"))?;
        let want = grant.resource.unwrap_or_default();
        if want.is_empty() {
            return Err(sour("invalid_target"));
        }
        let audience = self
            .heard(&want)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(|| sour("invalid_target"))?;
        let actor = self
            .bearer(&pat)
            .await
            .ok_or_else(|| sour("invalid_grant"))?;
        if !self.alive(actor).await.map_err(|_| spoil())? {
            return Err(sour("invalid_grant"));
        }
        let sub = self
            .subject(actor)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(|| sour("invalid_grant"))?;
        let who = Who {
            login: String::new(),
            name: String::new(),
            teams: Vec::new(),
        };
        let reach = self
            .sign(&Ticket {
                sub,
                aud: audience,
                kind: "access",
                scope: String::new(),
                nonce: None,
                wide: false,
                who: &who,
            })
            .map_err(|_| spoil())?;
        Ok(Json(json!({
            "access_token": reach,
            "issued_token_type": ACCESS,
            "token_type": "Bearer",
            "expires_in": LIFE,
        })))
    }

    async fn bearer(&self, pat: &str) -> Option<i64> {
        let seat = HeaderValue::from_str(&format!("token {pat}")).ok()?;
        let mut headers = HeaderMap::new();
        headers.insert("authorization", seat);
        self.gate.whom(&headers).await
    }
}
