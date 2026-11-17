use crate::oidc::plain::{seal, sour, spoil};
use crate::oidc::{Bearing, Grant, Grip, LIFE, Oidc, RENEW, Ticket};
use axum::Json;
use keel::Wire;
use keel::life::tick;
use keel_gate::wild;
use radicula_oidc::jwt::{Algorithm, Audience, Claims, Signer};
use serde_json::json;

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) async fn trade(&self, grant: Grant) -> Grip {
        let key = grant.code.ok_or_else(|| sour("invalid_request"))?;
        let held = self.codes.lock().expect("codes").remove(&key);
        let code = held.ok_or_else(|| sour("invalid_grant"))?;
        if code.dies < tick() {
            return Err(sour("invalid_grant"));
        }
        if grant.client.as_deref() != Some(&code.bearing.client)
            || grant.redirect.as_deref() != Some(&code.redirect)
        {
            return Err(sour("invalid_grant"));
        }
        let verifier = grant.verifier.ok_or_else(|| sour("invalid_request"))?;
        if radicula_oidc::pkce::verify(&verifier, &code.challenge).is_err() {
            return Err(sour("invalid_grant"));
        }
        self.grip(&code.bearing, code.nonce).await
    }

    pub(crate) async fn renew(&self, grant: Grant) -> Grip {
        let token = grant.renew.ok_or_else(|| sour("invalid_request"))?;
        let ward = self
            .warrant(&token)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(|| sour("invalid_grant"))?;
        if grant.client.as_deref() != Some(&ward.bearing.client) {
            return Err(sour("invalid_grant"));
        }
        let lives = self.alive(ward.bearing.actor).await.map_err(|_| spoil())?;
        let known = self
            .client(&ward.bearing.client)
            .await
            .map_err(|_| spoil())?;
        if !lives || known.is_none() {
            return Err(sour("invalid_grant"));
        }
        self.core
            .of(self.svc)
            .end("Renew", ward.row)
            .await
            .map_err(|_| spoil())?;
        self.grip(&ward.bearing, None).await
    }

    async fn grip(&self, bearing: &Bearing, nonce: Option<String>) -> Grip {
        let actor = bearing.actor;
        let client = bearing.client.as_str();
        let scope = bearing.scope.as_str();
        let who = self
            .person(actor)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(spoil)?;
        let sub = self
            .subject(actor)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(spoil)?;
        let wide = scope.split_whitespace().any(|word| word == "profile");
        let id = self
            .sign(&Ticket {
                sub: sub.clone(),
                aud: client.to_string(),
                kind: "id",
                scope: String::new(),
                nonce,
                wide,
                who: &who,
            })
            .map_err(|_| spoil())?;
        let reach = self
            .sign(&Ticket {
                sub,
                aud: bearing.audience.clone(),
                kind: "access",
                scope: scope.to_string(),
                nonce: None,
                wide,
                who: &who,
            })
            .map_err(|_| spoil())?;
        let fresh = wild();
        self.mint(bearing, &fresh).await.map_err(|_| spoil())?;
        Ok(Json(json!({
            "access_token": reach,
            "id_token": id,
            "refresh_token": fresh,
            "token_type": "Bearer",
            "expires_in": LIFE,
            "scope": scope,
        })))
    }

    async fn mint(&self, bearing: &Bearing, token: &str) -> Result<(), keel::adapt::Error> {
        let face = self.core.of(self.svc);
        let row = face
            .put(
                "Renew",
                &[
                    ("hash", &seal(token)),
                    ("slug", &bearing.client),
                    ("scope", &bearing.scope),
                    ("audience", &bearing.audience),
                    ("actor", &bearing.actor.to_string()),
                ],
            )
            .await?;
        face.lease("Renew", row, tick() + RENEW).await
    }

    pub(crate) fn sign(&self, ticket: &Ticket) -> Result<String, radicula_oidc::Fault> {
        let mut extra = serde_json::Map::new();
        extra.insert("kind".into(), json!(ticket.kind));
        if ticket.kind == "access" {
            extra.insert("scope".into(), json!(ticket.scope));
        }
        if let Some(nonce) = &ticket.nonce {
            extra.insert("nonce".into(), json!(nonce));
        }
        if ticket.wide {
            extra.insert("login".into(), json!(ticket.who.login));
            extra.insert("name".into(), json!(ticket.who.name));
            extra.insert("teams".into(), json!(ticket.who.teams));
        }
        let now = tick() as u64;
        let claims = Claims {
            iss: self.iss.clone(),
            sub: ticket.sub.clone(),
            aud: Audience::One(ticket.aud.clone()),
            exp: now + LIFE as u64,
            iat: now,
            nbf: None,
            extra,
        };
        let signer = Signer {
            algorithm: Algorithm::ES256,
            kid: self.keys.kid.clone(),
            key: self.keys.enc.clone(),
        };
        signer.sign(&claims).map(|token| token.expose().to_string())
    }
}
