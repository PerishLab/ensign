use crate::oidc::plain::{seal, sour, spoil};
use crate::oidc::{Claims, Grant, Grip, LIFE, Oidc, RENEW, Ticket};
use axum::Json;
use jsonwebtoken::{Algorithm, Header, encode};
use keel::Wire;
use keel::life::tick;
use keel_gate::wild;
use serde_json::json;

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) async fn trade(&self, grant: Grant) -> Grip {
        let key = grant.code.ok_or_else(|| sour("invalid_request"))?;
        let held = self.codes.lock().expect("codes").remove(&key);
        let code = held.ok_or_else(|| sour("invalid_grant"))?;
        if code.dies < tick() {
            return Err(sour("invalid_grant"));
        }
        if grant.client.as_deref() != Some(&code.client)
            || grant.redirect.as_deref() != Some(&code.redirect)
        {
            return Err(sour("invalid_grant"));
        }
        let verifier = grant.verifier.ok_or_else(|| sour("invalid_request"))?;
        if seal(&verifier) != code.challenge {
            return Err(sour("invalid_grant"));
        }
        self.grip(code.actor, &code.client, &code.scope, code.nonce)
            .await
    }

    pub(crate) async fn renew(&self, grant: Grant) -> Grip {
        let token = grant.renew.ok_or_else(|| sour("invalid_request"))?;
        let ward = self
            .warrant(&token)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(|| sour("invalid_grant"))?;
        if grant.client.as_deref() != Some(&ward.slug) {
            return Err(sour("invalid_grant"));
        }
        let lives = self.alive(ward.actor).await.map_err(|_| spoil())?;
        let known = self.client(&ward.slug).await.map_err(|_| spoil())?;
        if !lives || known.is_none() {
            return Err(sour("invalid_grant"));
        }
        self.core
            .of(self.svc)
            .end("Renew", ward.row)
            .await
            .map_err(|_| spoil())?;
        self.grip(ward.actor, &ward.slug, &ward.scope, None).await
    }

    async fn grip(&self, actor: i64, client: &str, scope: &str, nonce: Option<String>) -> Grip {
        let who = self
            .person(actor)
            .await
            .map_err(|_| spoil())?
            .ok_or_else(spoil)?;
        let sub = actor.to_string();
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
                aud: self.iss.clone(),
                kind: "access",
                scope: scope.to_string(),
                nonce: None,
                wide: false,
                who: &who,
            })
            .map_err(|_| spoil())?;
        let fresh = wild();
        self.mint(actor, client, scope, &fresh)
            .await
            .map_err(|_| spoil())?;
        Ok(Json(json!({
            "access_token": reach,
            "id_token": id,
            "refresh_token": fresh,
            "token_type": "Bearer",
            "expires_in": LIFE,
            "scope": scope,
        })))
    }

    async fn mint(
        &self,
        actor: i64,
        client: &str,
        scope: &str,
        token: &str,
    ) -> Result<(), keel::adapt::Error> {
        let face = self.core.of(self.svc);
        let row = face
            .put(
                "Renew",
                &[
                    ("hash", &seal(token)),
                    ("slug", client),
                    ("scope", scope),
                    ("actor", &actor.to_string()),
                ],
            )
            .await?;
        face.lease("Renew", row, tick() + RENEW).await
    }

    fn sign(&self, ticket: &Ticket) -> Result<String, jsonwebtoken::errors::Error> {
        let mut head = Header::new(Algorithm::ES256);
        head.kid = Some(self.keys.kid.clone());
        let claims = Claims {
            iss: self.iss.clone(),
            sub: ticket.sub.clone(),
            aud: ticket.aud.clone(),
            kind: ticket.kind.to_string(),
            exp: tick() + LIFE,
            iat: tick(),
            scope: (ticket.kind == "access").then(|| ticket.scope.clone()),
            nonce: ticket.nonce.clone(),
            login: ticket.wide.then(|| ticket.who.login.clone()),
            name: ticket.wide.then(|| ticket.who.name.clone()),
            teams: ticket.wide.then(|| ticket.who.teams.clone()),
        };
        encode(&head, &claims, &self.keys.enc)
    }
}
