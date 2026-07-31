use super::Oidc;
use keel::Wire;
use keel::query::{Op, form};

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) async fn named(&self, sub: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Actor").when("sub", Op::Eq, sub);
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn heard(&self, want: &str) -> Result<Option<String>, keel::adapt::Error> {
        if want.is_empty() {
            return Ok(Some(self.iss.clone()));
        }
        let ask = form("App").when("slug", Op::Eq, want);
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held
            .filter(|row| row.text("mode") == Some("resource"))
            .map(|_| want.to_string()))
    }

    pub(crate) async fn subject(&self, id: i64) -> Result<Option<String>, keel::adapt::Error> {
        let ask = form("Actor").when("id", Op::Eq, &id.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.and_then(|row| row.text("sub").map(str::to_string)))
    }
}
