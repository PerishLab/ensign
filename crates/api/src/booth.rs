use crate::util::{Sound, digest, fits, now, scrub, wild};
use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use keel::{Cell, Core, Wire};
use keel_gate::TTL;
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct Booth<W: Wire> {
    pub(crate) core: Arc<Core<W>>,
    pub(crate) svc: i64,
    pub(crate) secure: bool,
}

impl<W: Wire> Clone for Booth<W> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            svc: self.svc,
            secure: self.secure,
        }
    }
}

impl<W: Wire + 'static> Booth<W> {
    pub(crate) async fn card(&self, code: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let q = format!(r#"from Invite where hash = "{}""#, digest(code));
        let pack = self.core.of(self.svc).query(&q).await?;
        Ok(pack.rows().first().map(keel::Row::key))
    }

    pub(crate) async fn birth(&self, login: &str, name: &str) -> Result<i64, keel::adapt::Error> {
        let sudo = self.core.sudo();
        let key = sudo
            .put(
                "Actor",
                &[
                    ("login", login),
                    ("name", name),
                    ("kind", "user"),
                    ("barred", "false"),
                ],
            )
            .await?;
        sudo.put(
            "@grant",
            &[
                ("who", &key.to_string()),
                ("verb", "*"),
                ("unit", "Actor"),
                ("scope", &format!("row {key}")),
            ],
        )
        .await?;
        Ok(key)
    }

    pub(crate) async fn actor(&self, login: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let q = format!(r#"from Actor where login = "{}""#, scrub(login));
        let pack = self.core.of(self.svc).query(&q).await?;
        Ok(pack.rows().first().map(keel::Row::key))
    }

    pub(crate) async fn barred(&self, key: i64) -> Result<Option<bool>, keel::adapt::Error> {
        let q = format!(r#"from Actor where id = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).await?;
        let Some(row) = pack.rows().first() else {
            return Ok(None);
        };
        Ok(Some(
            row.cells().get("barred").map(Cell::show) == Some("true".into()),
        ))
    }

    pub(crate) async fn verify(&self, login: &str, pass: &str) -> Result<i64, StatusCode> {
        let key = self
            .actor(login)
            .await
            .sound()?
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let held = match self.guard(key).await {
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

    pub(crate) async fn guard(&self, key: i64) -> Result<Option<String>, keel::adapt::Error> {
        let q = format!(r#"from Pass where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).await?;
        Ok(pack
            .rows()
            .first()
            .and_then(|row| row.cells().get("hash").map(Cell::show)))
    }

    pub(crate) async fn spare(
        &self,
        key: i64,
        code: &str,
    ) -> Result<Option<i64>, keel::adapt::Error> {
        let q = format!(r#"from Rescue where actor = "{key}""#);
        let pack = self.core.of(self.svc).query(&q).await?;
        let mark = digest(code);
        Ok(pack
            .rows()
            .iter()
            .find(|row| row.cells().get("hash").map(Cell::show) == Some(mark.clone()))
            .map(keel::Row::key))
    }

    pub(crate) async fn face(
        &self,
        headers: &HeaderMap,
        op: Option<i64>,
    ) -> Option<keel::Face<'_, W>> {
        let told = headers.get("authorization").and_then(|v| v.to_str().ok());
        if let Some(token) = told.and_then(|v| v.strip_prefix("sudo ")) {
            return self
                .core
                .seal(token)
                .await
                .ok()
                .filter(|ok| *ok)
                .map(|_| self.core.sudo());
        }
        op.map(|id| self.core.of(id))
    }

    pub(crate) async fn crews(&self, id: i64) -> Result<Vec<String>, keel::adapt::Error> {
        let q = format!(r#"from Team where members has "{id}""#);
        let pack = self.core.of(self.svc).query(&q).await?;
        Ok(pack
            .rows()
            .iter()
            .filter_map(|row| row.cells().get("name").map(Cell::show))
            .collect())
    }

    pub(crate) async fn shield(&self, key: i64, hash: &str) -> Result<(), keel::adapt::Error> {
        let hash = hash.to_string();
        self.core
            .of(key)
            .batch(async |tx| {
                let held = tx
                    .query(&format!(r#"from Pass where actor = "{key}""#))
                    .await?;
                for row in held.rows() {
                    tx.end("Pass", row.key()).await?;
                }
                tx.put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
                    .await?;
                Ok(())
            })
            .await
    }

    pub(crate) async fn recover(&self, key: i64, hash: &str) -> Result<(), keel::adapt::Error> {
        let hash = hash.to_string();
        self.core
            .batch(async |tx| {
                let held = tx
                    .query(&format!(r#"from Pass where actor = "{key}""#))
                    .await?;
                for row in held.rows() {
                    tx.end("Pass", row.key()).await?;
                }
                tx.put("Pass", &[("hash", &hash), ("actor", &key.to_string())])
                    .await?;
                for unit in ["Rescue", "Session", "Renew"] {
                    let live = tx
                        .query(&format!(r#"from {unit} where actor = "{key}""#))
                        .await?;
                    for row in live.rows() {
                        tx.end(unit, row.key()).await?;
                    }
                }
                Ok(())
            })
            .await
    }

    pub(crate) async fn rearm(&self, hash: &str) {
        let sudo = self.core.sudo();
        let armed = sudo
            .put("Invite", &[("hash", hash), ("note", "rearmed")])
            .await;
        if let Err(err) = armed {
            eprintln!("ensign: join unwind lost the invite: {err}");
        }
    }

    pub(crate) async fn unbirth(&self, key: i64) {
        self.sweep(key).await;
        let felled = self.core.sudo().end("Actor", key).await;
        if let Err(err) = felled {
            eprintln!("ensign: join unwind left actor {key}: {err}");
        }
    }

    pub(crate) async fn sweep(&self, key: i64) {
        let sudo = self.core.sudo();
        let q = format!(r#"from @grant where who = "{key}""#);
        let pack = match sudo.query(&q).await {
            Ok(pack) => pack,
            Err(err) => {
                eprintln!("ensign: join unwind blind to grants: {err}");
                return;
            }
        };
        for row in pack.rows() {
            if let Err(err) = sudo.end("@grant", row.key()).await {
                eprintln!("ensign: join unwind left a grant: {err}");
            }
        }
    }

    pub(crate) async fn tag(
        &self,
        id: i64,
    ) -> Result<Option<(String, String)>, keel::adapt::Error> {
        let q = format!(r#"from Actor where id = "{id}""#);
        let pack = self.core.of(id).query(&q).await?;
        let Some(row) = pack.rows().first() else {
            return Ok(None);
        };
        let Some(login) = row.cells().get("login").map(Cell::show) else {
            return Ok(None);
        };
        let name = row.cells().get("name").map(Cell::show).unwrap_or_default();
        Ok(Some((login, name)))
    }

    pub(crate) async fn session(
        &self,
        key: i64,
    ) -> Result<(StatusCode, HeaderMap, Json<Value>), StatusCode> {
        let sid = wild();
        let row = self
            .core
            .of(self.svc)
            .put(
                "Session",
                &[("hash", &digest(&sid)), ("actor", &key.to_string())],
            )
            .await
            .sound()?;
        self.core.lease("Session", row, now() + TTL).await.sound()?;
        let mut headers = HeaderMap::new();
        let mut jar = format!("session={sid}; HttpOnly; SameSite=Lax; Path=/");
        if self.secure {
            jar.push_str("; Secure");
        }
        headers.insert(
            "set-cookie",
            jar.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
        Ok((StatusCode::CREATED, headers, Json(json!({ "id": row }))))
    }
}
