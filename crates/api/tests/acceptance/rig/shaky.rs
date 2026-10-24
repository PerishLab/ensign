use keel::adapt::{Error, Store};
use keel::wire::Val;
use std::sync::{Arc, Mutex};

pub const DETAIL: &str = "driver detail: relation \"ledger\" at db.internal:5432";

#[derive(Clone, Copy)]
pub enum Kind {
    Down,
    Taken,
    Fault,
    Plain,
}

#[derive(Default)]
pub struct Knob(Mutex<Option<(String, Kind)>>);

impl Knob {
    pub fn arm(&self, needle: &str, kind: Kind) {
        *self.0.lock().expect("knob") = Some((needle.to_string(), kind));
    }

    pub fn clear(&self) {
        *self.0.lock().expect("knob") = None;
    }

    fn check(&self, sql: &str) -> Result<(), Error> {
        let held = self.0.lock().expect("knob");
        let Some((needle, kind)) = held.as_ref() else {
            return Ok(());
        };
        if !sql.contains(needle.as_str()) {
            return Ok(());
        }
        Err(Error::Store(match kind {
            Kind::Down => Store::Down(DETAIL.into()),
            Kind::Taken => Store::Taken(DETAIL.into()),
            Kind::Fault => Store::Fault(DETAIL.into()),
            Kind::Plain => return Err(Error::Adapt(DETAIL.into())),
        }))
    }
}

pub struct Shaky<W> {
    wire: W,
    knob: Arc<Knob>,
}

impl<W> Shaky<W> {
    pub fn new(wire: W, knob: Arc<Knob>) -> Self {
        Self { wire, knob }
    }
}

impl<W: keel::Wire> keel::Wire for Shaky<W> {
    fn grain(&self) -> keel::ddl::Grain {
        self.wire.grain()
    }

    async fn run(&mut self, sql: &str, args: &[Val]) -> Result<u64, Error> {
        self.knob.check(sql)?;
        self.wire.run(sql, args).await
    }

    async fn plant(&mut self, sql: &str, args: &[Val]) -> Result<i64, Error> {
        self.knob.check(sql)?;
        self.wire.plant(sql, args).await
    }

    async fn rows(&mut self, sql: &str, args: &[Val]) -> Result<Vec<Vec<Val>>, Error> {
        self.knob.check(sql)?;
        self.wire.rows(sql, args).await
    }

    async fn script(&mut self, sql: &str) -> Result<(), Error> {
        self.knob.check(sql)?;
        self.wire.script(sql).await
    }

    async fn revive(&mut self) -> Result<(), Error> {
        self.wire.revive().await
    }

    async fn epoch(&mut self) -> Result<Option<String>, Error> {
        self.wire.epoch().await
    }
}
