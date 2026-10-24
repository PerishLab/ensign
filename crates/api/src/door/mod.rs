mod face;
mod plate;

use face::{known, shown};

use crate::booth::{Booth, PASS};
use crate::util::{Sound, lift, lock, pct, text};
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use keel::serve::admit;
use keel::{Op, Operator, Wire, form};
use keel_gate::{bearer, crumb, digest, wild};
use serde_json::{Map, Value, json};

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn invite(
        &self,
        headers: HeaderMap,
        op: Option<Extension<Operator>>,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let who = op.map(|Extension(op)| op);
        let face = admit(&self.core, &headers, who.as_ref(), "invite").await?;
        let note = body.get("note").and_then(Value::as_str).unwrap_or("");
        let code = wild();
        face.put("Invite", &[("hash", &digest(&code)), ("note", note)])
            .await
            .map_err(|err| lift(&err, StatusCode::FORBIDDEN))?;
        Ok((StatusCode::CREATED, Json(json!({ "code": code }))))
    }

    pub(crate) async fn join(
        &self,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let code = text(&body, "code")?;
        let login = text(&body, "login")?;
        let name = text(&body, "name")?;
        let pass = text(&body, "pass")?;
        if pass.chars().count() < 8 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let card = self
            .card(&code)
            .await
            .sound()?
            .ok_or(StatusCode::NOT_FOUND)?;
        let taken = self.source(&login).await.sound()?;
        if taken.is_some() {
            return Err(StatusCode::CONFLICT);
        }
        let hash = tokio::task::spawn_blocking(move || lock(&pass))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        #[allow(clippy::disallowed_methods)]
        let made = self
            .core
            .sudo()
            .batch(async |tx| {
                tx.end("Invite", card).await?;
                let key = tx
                    .birth(&[("sub", &wild()), ("kind", "user"), ("barred", "false")])
                    .await?;
                let seat = key.to_string();
                tx.put(
                    "Profile",
                    &[("handle", &login), ("name", &name), ("actor", &seat)],
                )
                .await?;
                let source = tx
                    .put(
                        "Source",
                        &[("kind", PASS), ("handle", &login), ("actor", &seat)],
                    )
                    .await?;
                tx.put("Pass", &[("hash", &hash), ("source", &source.to_string())])
                    .await?;
                Ok(key)
            })
            .await;
        match made {
            Ok(key) => Ok((StatusCode::CREATED, Json(json!({ "id": key })))),
            Err(err) => Err(lift(&err, gauge(&err.to_string()))),
        }
    }

    pub(crate) async fn login(
        &self,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let login = text(&body, "login")?;
        let pass = text(&body, "pass")?;
        let key = self.verify(&login, &pass).await?;
        self.session(key).await
    }

    pub(crate) async fn token(
        &self,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let login = text(&body, "login")?;
        let pass = text(&body, "pass")?;
        let name = body.get("name").and_then(Value::as_str).unwrap_or("cli");
        let key = self.verify(&login, &pass).await?;
        let pat = self.gate.token(key, name).await.sound()?;
        Ok((StatusCode::CREATED, Json(json!({ "token": pat }))))
    }

    pub(crate) async fn untoken(&self, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
        let pat = bearer(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
        match self.gate.revoke(&pat).await {
            Ok(()) => Ok(StatusCode::NO_CONTENT),
            Err(deny) => Err(deny.status()),
        }
    }

    pub(crate) async fn logout(&self, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
        let sid = crumb(&headers).ok_or(StatusCode::BAD_REQUEST)?;
        match self.gate.logout(&sid).await {
            Ok(()) => Ok(StatusCode::NO_CONTENT),
            Err(deny) => Err(deny.status()),
        }
    }

    pub(crate) async fn who(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<Json<Value>, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let (login, name) = self.tag(me).await.sound()?.ok_or(StatusCode::NOT_FOUND)?;
        Ok(Json(json!({ "id": me, "login": login, "name": name })))
    }

    pub(crate) async fn auth(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, HeaderMap), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let (login, _) = self
            .tag(me)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let teams = self
            .crews(me)
            .await
            .sound()?
            .iter()
            .map(|team| pct(team))
            .collect::<Vec<_>>()
            .join(",");
        let mut headers = HeaderMap::new();
        let stamp = |value: &str| value.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        headers.insert("x-ensign-user", stamp(&me.to_string())?);
        headers.insert("x-ensign-login", stamp(&login)?);
        headers.insert("x-ensign-teams", stamp(&teams)?);
        Ok((StatusCode::OK, headers))
    }

    pub(crate) async fn mint(
        &self,
        op: Option<Extension<Operator>>,
    ) -> Result<(StatusCode, Json<Value>), StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let face = self.core.of(me);
        let ask = form("Rescue").when("actor", Op::Eq, &me.to_string());
        let pack = face.ask(&ask).await.sound()?;
        for row in pack.rows() {
            face.end("Rescue", row.key()).await.sound()?;
        }
        let mut codes = Vec::new();
        for _ in 0..3 {
            let code = wild();
            face.put(
                "Rescue",
                &[("hash", &digest(&code)), ("actor", &me.to_string())],
            )
            .await
            .sound()?;
            codes.push(code);
        }
        Ok((StatusCode::CREATED, Json(json!({ "codes": codes }))))
    }

    pub(crate) async fn repass(
        &self,
        op: Option<Extension<Operator>>,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<StatusCode, StatusCode> {
        let Some(Extension(Operator(me))) = op else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let old = text(&body, "old")?;
        let pass = text(&body, "pass")?;
        if pass.chars().count() < 8 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let seat = self.seat(me).await.sound()?.ok_or(StatusCode::FORBIDDEN)?;
        let held = match self.guard(seat).await {
            Ok(Some(held)) => held,
            Ok(None) => return Err(StatusCode::FORBIDDEN),
            Err(err) => return Err(lift(&err, StatusCode::INTERNAL_SERVER_ERROR)),
        };
        let fit = tokio::task::spawn_blocking(move || crate::util::fits(&old, &held))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !fit {
            return Err(StatusCode::FORBIDDEN);
        }
        let hash = tokio::task::spawn_blocking(move || lock(&pass))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        self.shield(me, seat, &hash).await.sound()?;
        Ok(StatusCode::NO_CONTENT)
    }

    pub(crate) async fn revive(
        &self,
        Json(body): Json<Map<String, Value>>,
    ) -> Result<StatusCode, StatusCode> {
        let login = text(&body, "login")?;
        let code = text(&body, "code")?;
        let pass = text(&body, "pass")?;
        if pass.chars().count() < 8 {
            return Err(StatusCode::BAD_REQUEST);
        }
        let seat = self
            .source(&login)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let key = self
            .owner(seat)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let spare = self
            .spare(key, &code)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        match self.barred(key).await {
            Ok(Some(false)) => {}
            Ok(Some(true)) => return Err(StatusCode::FORBIDDEN),
            Ok(None) => return Err(StatusCode::UNAUTHORIZED),
            Err(err) => return Err(lift(&err, StatusCode::INTERNAL_SERVER_ERROR)),
        }
        let _ = spare;
        let hash = tokio::task::spawn_blocking(move || lock(&pass))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Err(err) = self.recover(key, seat, &hash).await {
            return Err(lift(&err, StatusCode::INTERNAL_SERVER_ERROR));
        }
        Ok(StatusCode::NO_CONTENT)
    }
}

fn gauge(note: &str) -> StatusCode {
    if note.contains("missing row") {
        return StatusCode::NOT_FOUND;
    }
    if note.contains("taken") {
        return StatusCode::CONFLICT;
    }
    StatusCode::INTERNAL_SERVER_ERROR
}
