mod booth;
mod config;
mod door;
mod model;
mod oidc;
mod util;

pub use booth::Booth;
pub use util::lock;

use axum::Router;
use axum::routing::{get, post};
use door::{auth, invite, join, login, logout, mint, repass, revive, token, untoken, who};
use keel::adapt::db::Sqlite;
use keel::{Core, Graph, Op, Wire, app, bind, config as keel_config, form};
use keel_gate::Gate;
use model::{Actor, App, Invite, Pass, Renew, Rescue, Team, plug};
use std::path::Path;
use std::sync::Arc;

const PREFIX: &str = "/api";

pub fn shape() -> Graph {
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

pub async fn sail(root: &str) {
    let runtime = match config::load(Path::new(root)) {
        Ok(runtime) => runtime,
        Err(err) => halt("config", &err.to_string()),
    };
    let (cfg, home) = match keel_config::load(Path::new(root)) {
        Ok(found) => found,
        Err(err) => halt("config", &err.to_string()),
    };
    match runtime.store.kind {
        config::Kind::Pg => {
            let held = keel::adapt::pg::Store {
                url: runtime.store.url.clone(),
            };
            let mut store = match held.open().await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            if runtime.fresh
                && let Err(err) = store.wipe().await
            {
                halt("fresh", &err.to_string());
            }
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, root, &runtime.iss).await;
        }
        config::Kind::File => {
            let held = keel::adapt::db::Store {
                kind: keel::adapt::db::Kind::File,
                path: runtime.store.path.clone(),
            };
            let store = match held.open(&home).await {
                Ok(store) => store,
                Err(err) => halt("store", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, root, &runtime.iss).await;
        }
        config::Kind::Memory => {
            let store = match Sqlite::memory().await {
                Ok(store) => store,
                Err(err) => halt("store", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            serve(core, &cfg, root, &runtime.iss).await;
        }
    }
}

fn raise<W: Wire + 'static>(
    made: Result<Core<W>, keel::adapt::Error>,
    cfg: &keel_config::Config,
) -> Arc<Core<W>> {
    let built = made
        .and_then(|core| core.identify("Actor"))
        .map(|core| match cfg.cache.kind {
            keel_config::Hold::Memory => core,
            keel_config::Hold::None => core.bare(),
        });
    match built {
        Ok(core) => core.share(),
        Err(err) => halt("bind", &err.to_string()),
    }
}

async fn serve<W: Wire + 'static>(
    core: Arc<Core<W>>,
    cfg: &keel_config::Config,
    root: &str,
    issuer: &str,
) {
    let (door, svc) = match rig(&core).await {
        Ok(pair) => pair,
        Err(err) => halt("rise", &err.to_string()),
    };
    let iss = match issuer.is_empty() {
        true => format!("http://{}:{}{PREFIX}", cfg.listen.host, cfg.listen.port),
        false => issuer.trim_end_matches('/').to_string(),
    };
    let booth = Booth::new(core.clone(), door.clone(), svc, iss.starts_with("https://"));
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
    let router = Router::new().nest(PREFIX, door.screen(base));
    let addr = format!("{}:{}", cfg.listen.host, cfg.listen.port);
    let bound = match tokio::net::TcpListener::bind(&addr).await {
        Ok(bound) => bound,
        Err(err) => halt("listen", &err.to_string()),
    };
    let live = match bound.local_addr() {
        Ok(live) => live,
        Err(err) => halt("listen", &err.to_string()),
    };
    eprintln!(
        "{}",
        serde_json::json!({ "role": "api", "endpoint": format!("http://{live}") })
    );
    if let Err(err) = axum::serve(bound, router).await {
        halt("serve", &err.to_string());
    }
}

fn halt(seat: &str, note: &str) -> ! {
    eprintln!("ensign: {seat}: {note}");
    std::process::exit(1)
}

pub async fn rig<W: Wire + 'static>(
    core: &Arc<Core<W>>,
) -> Result<(Gate<W>, i64), keel::adapt::Error> {
    let svc = hail(core).await?;
    let gate = Gate::rise(core.clone(), svc).await?.bar("barred");
    let who = svc.to_string();
    gate.sow(&[
        ("all", "see", "Actor", "all"),
        ("all", "see", "Team", "all"),
        ("all", "see", "App", "all"),
        (&who, "see", "Invite", "all"),
        (&who, "end", "Invite", "all"),
        (&who, "see", "Pass", "all"),
        (&who, "see", "Rescue", "all"),
        (&who, "end", "Rescue", "all"),
        (&who, "see", "App", "all"),
        (&who, "see", "Renew", "all"),
        (&who, "put", "Renew", "all"),
        (&who, "end", "Renew", "all"),
    ])
    .await?;
    Ok((gate, svc))
}

pub async fn hail<W: Wire>(core: &Arc<Core<W>>) -> Result<i64, keel::adapt::Error> {
    let held = core
        .one(&form("Actor").when("login", Op::Eq, "ensign"))
        .await?;
    match held {
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
