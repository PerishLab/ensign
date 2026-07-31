mod artifact;
mod booth;
mod config;
mod door;
mod model;
mod oidc;
mod startup;
mod util;

pub use booth::Booth;
pub use startup::{bootstrap, serve};
pub use util::lock;

use crate::model::{Actor, App, Invite, Pass, Profile, Renew, Rescue, Source, Team, plug};
use keel::{Core, Graph, Op, Wire, form};
use keel_gate::Gate;
use std::sync::Arc;

pub fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Profile>()
        .plug::<Source>()
        .plug::<Pass>()
        .plug::<Invite>()
        .plug::<Rescue>()
        .plug::<Team>()
        .plug::<App>()
        .plug::<Renew>();
    plug(&mut graph);
    graph
}

pub struct Berth<'a, W: Wire>(pub &'a Arc<Core<W>>);

impl<W: Wire + 'static> Berth<'_, W> {
    pub async fn rig(&self) -> Result<(Gate<W>, i64), keel::adapt::Error> {
        let core = self.0;
        let svc = self.service().await?;
        let gate = Gate::rise(core.clone(), svc)?.bar("barred");
        if !gate.ready().await? {
            return Err(keel::adapt::Error::Adapt(
                "missing keel-gate bootstrap grants".into(),
            ));
        }
        let who = svc.to_string();
        if !gate.sown(&seeds(&who)).await? {
            return Err(keel::adapt::Error::Adapt(
                "missing ensign bootstrap grants".into(),
            ));
        }
        Ok((gate, svc))
    }

    pub async fn seed(&self) -> Result<(Gate<W>, i64), keel::adapt::Error> {
        let core = self.0;
        let svc = self.hail().await?;
        let gate = Gate::rise(core.clone(), svc)?.bar("barred");
        gate.seed().await?;
        let who = svc.to_string();
        gate.sow(&seeds(&who)).await?;
        Ok((gate, svc))
    }

    async fn hail(&self) -> Result<i64, keel::adapt::Error> {
        let core = self.0;
        let held = core
            .one(&form("Actor").when("sub", Op::Eq, SERVICE))
            .await?;
        match held {
            Some(row) => canonical(&row).map(|()| row.key()),
            None => {
                core.put(
                    "Actor",
                    &[("sub", SERVICE), ("kind", "svc"), ("barred", "false")],
                )
                .await
            }
        }
    }

    async fn service(&self) -> Result<i64, keel::adapt::Error> {
        let core = self.0;
        let held = core
            .one(&form("Actor").when("sub", Op::Eq, SERVICE))
            .await?;
        let row = held.ok_or_else(|| {
            keel::adapt::Error::Adapt("missing canonical ensign service Actor".into())
        })?;
        canonical(&row)?;
        Ok(row.key())
    }
}

pub(crate) const SERVICE: &str = "ensign";

fn seeds(who: &str) -> [(&str, &str, &str, &str); 15] {
    [
        ("all", "see", "Actor", "all"),
        (who, "see", "Profile", "all"),
        (who, "put", "Profile", "all"),
        (who, "see", "Source", "all"),
        ("all", "see", "Team", "all"),
        ("all", "see", "App", "all"),
        (who, "see", "Invite", "all"),
        (who, "end", "Invite", "all"),
        (who, "see", "Pass", "all"),
        (who, "see", "Rescue", "all"),
        (who, "end", "Rescue", "all"),
        (who, "see", "App", "all"),
        (who, "see", "Renew", "all"),
        (who, "put", "Renew", "all"),
        (who, "end", "Renew", "all"),
    ]
}

fn canonical(row: &keel::Row) -> Result<(), keel::adapt::Error> {
    if row.text("sub") == Some(SERVICE)
        && row.text("kind") == Some("svc")
        && row.flag("barred") == Some(false)
    {
        return Ok(());
    }
    Err(keel::adapt::Error::Adapt(
        "conflicting ensign service Actor".into(),
    ))
}
