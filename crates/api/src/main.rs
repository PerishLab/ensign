mod booth;
mod door;
mod model;
mod oidc;
mod util;

use axum::Router;
use axum::routing::{get, post};
use booth::Booth;
use door::{auth, invite, join, login, logout, mint, repass, revive, token, untoken, who};
use keel::adapt::pg::Postgres;
use keel::{Core, Graph, Wire, app, bind, config};
use keel_gate::Gate;
use model::{Actor, App, Invite, Pass, Renew, Rescue, Team, plug};
use std::env;
use std::path::Path;
use std::sync::Arc;

fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<Pass>()
        .plug::<Invite>()
        .plug::<Rescue>()
        .plug::<Team>()
        .plug::<App>()
        .plug::<Renew>();
    plug(&mut graph);
    graph
}

#[tokio::main]
async fn main() {
    let root = env::args().nth(1).unwrap_or_else(|| ".".into());
    let cfg = config::load(Path::new(&root));
    match env::var("KEEL_PG") {
        Ok(url) => {
            if env::var("KEEL_FRESH").is_ok() {
                fresh(&url).await;
            }
            let store = match Postgres::at(url).await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &root).await;
        }
        Err(_) => {
            let store = match cfg.open().await {
                Ok(store) => store,
                Err(err) => halt("config", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, &root).await;
        }
    }
}

async fn fresh(url: &str) {
    let mut store = match Postgres::at(url).await {
        Ok(store) => store,
        Err(err) => halt("fresh", &err.to_string()),
    };
    let wipe = store
        .script("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .await;
    if let Err(err) = wipe {
        halt("fresh", &err.to_string());
    }
}

fn raise<W: Wire + 'static>(
    made: Result<Core<W>, keel::adapt::Error>,
    cfg: &config::Config,
) -> Arc<Core<W>> {
    let built = made
        .and_then(|core| core.identify("Actor"))
        .map(|core| match cfg.cache.kind {
            config::Hold::Memory => core,
            config::Hold::None => core.bare(),
        });
    match built {
        Ok(core) => core.share(),
        Err(err) => halt("bind", &err.to_string()),
    }
}

async fn serve<W: Wire + 'static>(core: Arc<Core<W>>, cfg: &config::Config, root: &str) {
    let (door, svc) = match rig(&core).await {
        Ok(pair) => pair,
        Err(err) => halt("rise", &err.to_string()),
    };
    let iss = env::var("ENSIGN_ISS")
        .unwrap_or_else(|_| format!("http://{}:{}", cfg.listen.host, cfg.listen.port));
    let booth = Booth {
        core: core.clone(),
        svc,
        secure: iss.starts_with("https://"),
    };
    let plate = Router::new()
        .route("/invite", post(invite::<W>))
        .route("/join", post(join::<W>))
        .route("/login", post(login::<W>))
        .route("/logout", post(logout::<W>))
        .route("/whoami", get(who::<W>))
        .route("/auth", get(auth::<W>))
        .route("/mint", post(mint::<W>))
        .route("/repass", post(repass::<W>))
        .route("/revive", post(revive::<W>))
        .route("/bearer", post(token::<W>).delete(untoken::<W>))
        .with_state(booth);
    let vault = Path::new(root).join(".local").join("sign.pem");
    let flags = oidc::Oidc::new(core.clone(), svc, iss, oidc::keys(&vault)).plate();
    let base = app(core.clone(), &cfg.listen.prefix)
        .merge(plate)
        .merge(flags);
    let router = door.screen(base);
    let addr = format!("{}:{}", cfg.listen.host, cfg.listen.port);
    let bound = match tokio::net::TcpListener::bind(&addr).await {
        Ok(bound) => bound,
        Err(err) => halt("listen", &err.to_string()),
    };
    eprintln!("ensign: ready on http://{addr}");
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("ensign: {seat}: {note}");
    std::process::exit(1)
}

async fn rig<W: Wire + 'static>(core: &Arc<Core<W>>) -> Result<(Gate<W>, i64), keel::adapt::Error> {
    let svc = hail(core).await?;
    let gate = Gate::rise(core.clone(), svc).await?.bar("barred");
    seed(core, svc).await?;
    Ok((gate, svc))
}

async fn hail<W: Wire>(core: &Arc<Core<W>>) -> Result<i64, keel::adapt::Error> {
    let held = core.query(r#"from Actor where login = "ensign""#).await?;
    match held.rows().first() {
        Some(row) => Ok(row.key()),
        None => {
            core.put(
                "Actor",
                &[
                    ("login", "ensign"),
                    ("name", "ensign"),
                    ("kind", "svc"),
                    ("barred", "false"),
                ],
            )
            .await
        }
    }
}

async fn seed<W: Wire>(core: &Arc<Core<W>>, svc: i64) -> Result<(), keel::adapt::Error> {
    let sown = core.query(r#"from @grant where who = "all" count"#).await?;
    if sown.count() != Some(0) {
        return Ok(());
    }
    let sudo = core.sudo();
    let who = svc.to_string();
    for unit in ["Actor", "Team", "App"] {
        sudo.put(
            "@grant",
            &[
                ("who", "all"),
                ("verb", "see"),
                ("unit", unit),
                ("scope", "all"),
            ],
        )
        .await?;
    }
    for (verb, unit) in [
        ("see", "Invite"),
        ("end", "Invite"),
        ("see", "Pass"),
        ("see", "Rescue"),
        ("end", "Rescue"),
        ("see", "App"),
        ("see", "Renew"),
        ("put", "Renew"),
        ("end", "Renew"),
    ] {
        sudo.put(
            "@grant",
            &[
                ("who", &who),
                ("verb", verb),
                ("unit", unit),
                ("scope", "all"),
            ],
        )
        .await?;
    }
    Ok(())
}
