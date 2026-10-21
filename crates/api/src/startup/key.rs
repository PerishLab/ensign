use super::Berth;
use super::berth::SERVICE;
use crate::artifact::Artifact;
use keel::query::Tree;
use keel::{Op, Wire, form};
use keel_gate::{Gate, digest};

impl<W: Wire + 'static> Berth<'_, W> {
    pub(crate) async fn key(
        &self,
        gate: &Gate<W>,
        slug: &str,
        held: &Artifact,
    ) -> Result<(), String> {
        let app = self.resource(slug).await?;
        let actor = self.consumer(slug).await?;
        let kept = held.load()?;
        let foreign = format!("key artifact holds no live key of its Actor: key:{slug}");
        if kept.is_some() && actor.is_none() {
            return Err(foreign);
        }
        if !app {
            self.register(slug).await?;
        }
        let actor = match actor {
            Some(actor) => actor,
            None => gate
                .birth(&[("sub", slug), ("kind", "svc"), ("barred", "false")])
                .await
                .map_err(|err| err.to_string())?,
        };
        let bytes = match kept {
            Some(bytes) => bytes,
            None => match self.issue(gate, actor, held).await? {
                Some(bytes) => bytes,
                None => return Ok(()),
            },
        };
        match self.vouch(actor, &bytes).await? {
            true => Ok(()),
            false => Err(foreign),
        }
    }

    async fn resource(&self, slug: &str) -> Result<bool, String> {
        let held = self
            .0
            .one(&form("App").when("slug", Op::Eq, slug))
            .await
            .map_err(|err| err.to_string())?;
        match held {
            Some(row) if row.text("mode") == Some("resource") => Ok(true),
            Some(_) => Err(format!("conflicting App for key artifact: key:{slug}")),
            None => Ok(false),
        }
    }

    async fn register(&self, slug: &str) -> Result<(), String> {
        let fields = [
            ("name", slug),
            ("slug", slug),
            ("home", ""),
            ("redirect", ""),
            ("secret", ""),
            ("mode", "resource"),
        ];
        self.0
            .put("App", &fields)
            .await
            .map(|_| ())
            .map_err(|err| err.to_string())
    }

    async fn consumer(&self, slug: &str) -> Result<Option<i64>, String> {
        let drift = || format!("conflicting Actor for key artifact: key:{slug}");
        let named = form("Profile").when("handle", Op::Eq, slug);
        if slug == SERVICE || self.exists(&named).await? {
            return Err(drift());
        }
        let held = self
            .0
            .one(&form("Actor").when("sub", Op::Eq, slug))
            .await
            .map_err(|err| err.to_string())?;
        let Some(row) = held else {
            return Ok(None);
        };
        let seat = row.key().to_string();
        let person = self
            .exists(&form("Profile").when("actor", Op::Eq, &seat))
            .await?;
        if row.text("kind") != Some("svc") || row.flag("barred") != Some(false) || person {
            return Err(drift());
        }
        Ok(Some(row.key()))
    }

    async fn issue(
        &self,
        gate: &Gate<W>,
        actor: i64,
        held: &Artifact,
    ) -> Result<Option<Vec<u8>>, String> {
        let pat = gate
            .token(actor, "key")
            .await
            .map_err(|err| err.to_string())?;
        if held.keep(pat.as_bytes())? {
            return Ok(None);
        }
        gate.revoke(&pat)
            .await
            .map_err(|deny| deny.note().to_string())?;
        let bytes = held.load()?;
        bytes
            .map(Some)
            .ok_or_else(|| "key artifact lost during creation".to_string())
    }

    async fn exists(&self, ask: &Tree) -> Result<bool, String> {
        let held = self.0.one(ask).await.map_err(|err| err.to_string())?;
        Ok(held.is_some())
    }

    async fn vouch(&self, actor: i64, bytes: &[u8]) -> Result<bool, String> {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return Ok(false);
        };
        let pat = text.trim();
        if pat.is_empty() {
            return Ok(false);
        }
        let held = self
            .0
            .one(&form("Token").when("hash", Op::Eq, &digest(pat)))
            .await
            .map_err(|err| err.to_string())?;
        Ok(held.and_then(|row| row.int("actor")) == Some(actor))
    }
}
