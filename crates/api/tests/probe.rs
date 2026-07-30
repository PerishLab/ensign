use api::{Booth, lock};
use axum::http::StatusCode;
use keel::adapt::Error;
use keel::adapt::db::Sqlite;
use keel::ddl::Grain;
use keel::{Core, Op, Val, Wire, form};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

struct Faint {
    real: Sqlite,
    live: Arc<AtomicBool>,
}

impl Faint {
    fn beat(&self) -> Result<(), Error> {
        if self.live.load(Ordering::Relaxed) {
            return Ok(());
        }
        Err(Error::Adapt("connection closed".into()))
    }
}

impl Wire for Faint {
    fn grain(&self) -> Grain {
        self.real.grain()
    }

    async fn run(&mut self, sql: &str, args: &[Val]) -> Result<u64, Error> {
        self.beat()?;
        self.real.run(sql, args).await
    }

    async fn plant(&mut self, sql: &str, args: &[Val]) -> Result<i64, Error> {
        self.beat()?;
        self.real.plant(sql, args).await
    }

    async fn rows(&mut self, sql: &str, args: &[Val]) -> Result<Vec<Vec<Val>>, Error> {
        self.beat()?;
        self.real.rows(sql, args).await
    }

    async fn script(&mut self, sql: &str) -> Result<(), Error> {
        self.beat()?;
        self.real.script(sql).await
    }
}

async fn core() -> (Arc<Core<Faint>>, Arc<AtomicBool>) {
    let live = Arc::new(AtomicBool::new(true));
    let real = Sqlite::memory().await.expect("db");
    let wire = Faint {
        real,
        live: live.clone(),
    };
    let mut estate = keel::bootstrap(api::shape(), wire).expect("bootstrap");
    let sudo = estate.mint().await.expect("mint");
    let core = estate
        .seal(&sudo)
        .await
        .expect("seal")
        .identify("Actor")
        .expect("identify")
        .share();
    (core, live)
}

async fn rig() -> (Booth<Faint>, Arc<AtomicBool>) {
    let (core, live) = core().await;
    assert!(api::Berth(&core).rig().await.is_err());
    assert!(
        core.one(&form("Actor").when("login", Op::Eq, "ensign"))
            .await
            .expect("service query")
            .is_none()
    );
    api::Berth(&core).seed().await.expect("seed");
    let (gate, svc) = api::Berth(&core).rig().await.expect("rig");
    let booth = Booth::new(core, gate.clone(), svc, false);
    let key = gate
        .birth(&[
            ("login", "ada"),
            ("name", "Ada"),
            ("kind", "user"),
            ("barred", "false"),
        ])
        .await
        .expect("ada");
    let hash = lock("seaworthy").expect("hash");
    booth.shield(key, &hash).await.expect("pass");
    (booth, live)
}

#[tokio::test]
async fn frozen() {
    let (core, _live) = core().await;
    api::Berth(&core).seed().await.expect("seed");
    let ask = form("@grant")
        .when("who", Op::Eq, "all")
        .when("verb", Op::Eq, "see")
        .when("unit", Op::Eq, "Actor")
        .when("scope", Op::Eq, "all");
    let grant = core
        .sudo()
        .one(&ask)
        .await
        .expect("grant query")
        .expect("grant");
    core.sudo()
        .end("@grant", grant.key())
        .await
        .expect("remove grant");

    assert!(api::Berth(&core).rig().await.is_err());
    assert!(
        core.sudo()
            .one(&ask)
            .await
            .expect("grant recheck")
            .is_none()
    );
}

#[tokio::test]
async fn refused() {
    let (booth, _live) = rig().await;
    assert!(booth.verify("ada", "seaworthy").await.is_ok());
    let wrong = booth.verify("ada", "adrift").await;
    assert_eq!(wrong, Err(StatusCode::UNAUTHORIZED));
    let absent = booth.verify("bob", "seaworthy").await;
    assert_eq!(absent, Err(StatusCode::UNAUTHORIZED));
}

#[tokio::test]
async fn outage() {
    let (booth, live) = rig().await;
    live.store(false, Ordering::Relaxed);
    let held = booth.verify("ada", "seaworthy").await;
    assert_eq!(held, Err(StatusCode::INTERNAL_SERVER_ERROR));
    let wrong = booth.verify("ada", "adrift").await;
    assert_eq!(wrong, Err(StatusCode::INTERNAL_SERVER_ERROR));
    live.store(true, Ordering::Relaxed);
    assert!(booth.verify("ada", "seaworthy").await.is_ok());
}
