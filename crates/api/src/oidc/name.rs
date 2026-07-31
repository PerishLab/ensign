use super::Oidc;
use keel::Wire;
use keel::query::{Op, form};

impl<W: Wire + 'static> Oidc<W> {
    pub(crate) async fn named(&self, sub: &str) -> Result<Option<i64>, keel::adapt::Error> {
        let ask = form("Actor").when("sub", Op::Eq, sub);
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.map(|row| row.key()))
    }

    pub(crate) async fn subject(&self, id: i64) -> Result<Option<String>, keel::adapt::Error> {
        let ask = form("Actor").when("id", Op::Eq, &id.to_string());
        let held = self.core.of(self.svc).one(&ask).await?;
        Ok(held.and_then(|row| row.text("sub").map(str::to_string)))
    }
}
