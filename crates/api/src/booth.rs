pub(crate) const PASS: &str = "pass";

use crate::util::{Sound, fits};
use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use keel::{Core, Op, Wire, form};
use keel_gate::{Gate, bake, digest};
use serde_json::{Value, json};
use std::sync::Arc;

pub struct Booth<W: Wire> {
    pub(crate) core: Arc<Core<W>>,
    pub(crate) gate: Gate<W>,
    pub(crate) svc: i64,
    pub(crate) secure: bool,
}

impl<W: Wire> Clone for Booth<W> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            gate: self.gate.clone(),
            svc: self.svc,
            secure: self.secure,
        }
    }
}

impl<W: Wire + 'static> Booth<W> {
    pub fn new(core: Arc<Core<W>>, gate: Gate<W>, svc: i64, secure: bool) -> Self {
        Self {
            core,
            gate,
            svc,
            secure,
        }
    }

    pub(crate) async fn card(&self, code: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Invite").when("hash", Op::Eq, &digest(code));
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn source(&self, handle: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Source")
            .when("kind", Op::Eq, PASS)
            .when("handle", Op::Eq, handle);
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn seat(&self, actor: i64) -> Result<Option<i64>, keel::adapt::Error> {
        let ask =
            form("Source")
                .when("kind", Op::Eq, PASS)
                .when("actor", Op::Eq, &actor.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn owner(&self, source: i64) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Source").when("id", Op::Eq, &source.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.and_then(|row| row.int("actor")))
    }

    pub(crate) async fn barred(&self, key: i64) -> Result<Option<bool>, keel::adapt::Error> {
        let ask = form("Actor").when("id", Op::Eq, &key.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.flag("barred") == Some(true)))
    }

    pub async fn verify(&self, handle: &str, pass: &str) -> Result<i64, StatusCode> {
        let seat = self
            .source(handle)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let key = self
            .owner(seat)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let held = match self.guard(seat).await {
            Ok(Some(held)) => held,
            Ok(None) => return Err(StatusCode::UNAUTHORIZED),
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };
        let owned = pass.to_string();
        let fit = tokio::task::spawn_blocking(move || fits(&owned, &held))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !fit {
            return Err(StatusCode::UNAUTHORIZED);
        }
        match self.barred(key).await {
            Ok(Some(false)) => Ok(key),
            Ok(Some(true)) => Err(StatusCode::FORBIDDEN),
            Ok(None) => Err(StatusCode::UNAUTHORIZED),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    pub(crate) async fn guard(&self, source: i64) -> Result<Option<String>, keel::adapt::Error> {
        let ask = form("Pass").when("source", Op::Eq, &source.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.and_then(|row| row.text("hash").map(str::to_string)))
    }

    pub(crate) async fn spare(
        &self,
        key: i64,
        code: &str,
    ) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Rescue").when("actor", Op::Eq, &key.to_string()).when(
            "hash",
            Op::Eq,
            &digest(code),
        );
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn crews(&self, id: i64) -> Result<Vec<String>, keel::adapt::Error> {
        let ask = form("Team").when("members", Op::Has, &id.to_string());
        let pack = self.core.of(self.svc).ask(&ask).await?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| row.text("name").map(str::to_string))
            .collect())
    }

    pub async fn shield(
        &self,
        key: i64,
        source: i64,
        hash: &str,
    ) -> Result<(), keel::adapt::Error> {
        let hash = hash.to_string();
        let seat = source.to_string();
        self.core
            .of(key)
            .batch(async |tx| {
                let ask = form("Pass").when("source", Op::Eq, &seat);
                let held = tx.ask(&ask).await?;
                for row in held.rows() {
                    tx.end("Pass", row.key()).await?;
                }
                tx.put("Pass", &[("hash", &hash), ("source", &seat)])
                    .await?;
                Ok(())
            })
            .await
    }

    pub(crate) async fn recover(
        &self,
        key: i64,
        source: i64,
        hash: &str,
    ) -> Result<(), keel::adapt::Error> {
        let hash = hash.to_string();
        let seat = source.to_string();
        self.core
            .of(key)
            .batch(async |tx| {
                let ask = form("Pass").when("source", Op::Eq, &seat);
                let held = tx.ask(&ask).await?;
                for row in held.rows() {
                    tx.end("Pass", row.key()).await?;
                }
                tx.put("Pass", &[("hash", &hash), ("source", &seat)])
                    .await?;
                for unit in ["Rescue", "Session", "Renew"] {
                    let live = tx
                        .ask(&form(unit).when("actor", Op::Eq, &key.to_string()))
                        .await?;
                    for row in live.rows() {
                        tx.end(unit, row.key()).await?;
                    }
                }
                Ok(())
            })
            .await
    }

    pub(crate) async fn known(&self, handle: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Profile").when("handle", Op::Eq, handle);
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.and_then(|row| row.int("actor")))
    }

    pub(crate) async fn shown(
        &self,
        sub: &str,
    ) -> Result<Option<(String, String)>, keel::adapt::Error> {
        let ask = form("Actor").when("sub", Op::Eq, sub);
        let Some(row) = self.core.of(self.svc).one(&ask).await? else {
            return Ok(None);
        };
        self.tag(row.key()).await
    }

    pub(crate) async fn tag(
        &self,
        id: i64,
    ) -> Result<Option<(String, String)>, keel::adapt::Error> {
        let ask = form("Profile").when("actor", Op::Eq, &id.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        let Some(row) = held else {
            return Ok(None);
        };
        let Some(login) = row.text("handle").map(str::to_string) else {
            return Ok(None);
        };
        let name = row.text("name").unwrap_or_default().to_string();
        Ok(Some((login, name)))
    }

    pub(crate) async fn session(
        &self,
        key: i64,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let (row, sid) = self.gate.session(key).await.sound()?;
        let mut headers = HeaderMap::new();
        let jar = bake(&sid, self.secure);
        headers.insert(
            "set-cookie",
            jar.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
        Ok((StatusCode::CREATED, headers, Json(json!({ "id": row }))))
    }
}
