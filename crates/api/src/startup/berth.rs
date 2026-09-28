use keel::{Core, Op, Wire, form};
use keel_gate::{Gate, Seed};
use std::sync::Arc;

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

fn seeds(who: &str) -> [Seed<'_>; 15] {
    [
        Seed {
            who: "all",
            verb: "see",
            unit: "Actor",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Profile",
            scope: "all",
        },
        Seed {
            who,
            verb: "put",
            unit: "Profile",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Source",
            scope: "all",
        },
        Seed {
            who: "all",
            verb: "see",
            unit: "Team",
            scope: "all",
        },
        Seed {
            who: "all",
            verb: "see",
            unit: "App",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Invite",
            scope: "all",
        },
        Seed {
            who,
            verb: "end",
            unit: "Invite",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Pass",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Rescue",
            scope: "all",
        },
        Seed {
            who,
            verb: "end",
            unit: "Rescue",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "App",
            scope: "all",
        },
        Seed {
            who,
            verb: "see",
            unit: "Renew",
            scope: "all",
        },
        Seed {
            who,
            verb: "put",
            unit: "Renew",
            scope: "all",
        },
        Seed {
            who,
            verb: "end",
            unit: "Renew",
            scope: "all",
        },
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
