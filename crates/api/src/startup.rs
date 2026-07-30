use crate::artifact::{self, Artifact};
use crate::config::{self, Kind};
use crate::door::{auth, invite, join, login, logout, mint, repass, revive, token, untoken, who};
use crate::{Berth, Booth, oidc, shape};
use axum::Router;
use axum::routing::{get, post};
use keel::adapt::db::Sqlite;
use keel::{Core, Status, Wire, app, bind, config as keel_config};
use std::path::Path;
use std::sync::Arc;

const PREFIX: &str = "/api";

pub async fn bootstrap(root: &str, specs: &[String]) {
    let runtime = match config::load(Path::new(root)) {
        Ok(runtime) => runtime,
        Err(err) => halt("config", &err.to_string()),
    };
    let (_, home) = match keel_config::load(Path::new(root)) {
        Ok(found) => found,
        Err(err) => halt("config", &err.to_string()),
    };
    let artifacts = match artifact::bootstrap(specs) {
        Ok(artifacts) => artifacts,
        Err(err) => halt("artifact", &err),
    };
    match runtime.store.kind {
        Kind::Pg => {
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
            if let Err(err) = provision(store, &artifacts).await {
                halt("bootstrap", &err);
            }
        }
        Kind::File => {
            let held = keel::adapt::db::Store {
                kind: keel::adapt::db::Kind::File,
                path: runtime.store.path.clone(),
            };
            let store = match held.open(&home).await {
                Ok(store) => store,
                Err(err) => halt("store", &err.to_string()),
            };
            if runtime.fresh {
                halt("fresh", "file store wipe is not supported");
            }
            if let Err(err) = provision(store, &artifacts).await {
                halt("bootstrap", &err);
            }
        }
        Kind::Memory => halt("store", "bootstrap requires a durable store"),
    }
}

pub async fn serve(root: &str, specs: &[String]) {
    let runtime = match config::load(Path::new(root)) {
        Ok(runtime) => runtime,
        Err(err) => halt("config", &err.to_string()),
    };
    if runtime.fresh {
        halt("fresh", "wipe is allowed only during explicit bootstrap");
    }
    let (cfg, home) = match keel_config::load(Path::new(root)) {
        Ok(found) => found,
        Err(err) => halt("config", &err.to_string()),
    };
    let signing = match artifact::signing(Path::new(root), specs) {
        Ok(signing) => signing,
        Err(err) => halt("artifact", &err),
    };
    match runtime.store.kind {
        Kind::Pg => {
            let held = keel::adapt::pg::Store {
                url: runtime.store.url.clone(),
            };
            let store = match held.open().await {
                Ok(store) => store,
                Err(err) => halt("pg", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            listen(core, &cfg, &signing, &runtime.iss).await;
        }
        Kind::File => {
            let held = keel::adapt::db::Store {
                kind: keel::adapt::db::Kind::File,
                path: runtime.store.path.clone(),
            };
            let store = match held.open(&home).await {
                Ok(store) => store,
                Err(err) => halt("store", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            listen(core, &cfg, &signing, &runtime.iss).await;
        }
        Kind::Memory => {
            let store = match Sqlite::memory().await {
                Ok(store) => store,
                Err(err) => halt("store", &err.to_string()),
            };
            let core = raise(bind(shape(), store).await, &cfg);
            listen(core, &cfg, &signing, &runtime.iss).await;
        }
    }
}

async fn provision<W: Wire + 'static>(
    store: W,
    artifacts: &artifact::Bootstrap,
) -> Result<(), String> {
    let mut estate = keel::bootstrap(shape(), store).map_err(|err| err.to_string())?;
    let sudo = match artifacts.sudo.load()? {
        Some(bytes) => custody(&bytes)?,
        None => {
            if estate.status().await.map_err(|err| err.to_string())? != Status::Vacant {
                return Err("sudo custody is absent for an occupied estate".to_string());
            }
            let minted = estate.mint().await.map_err(|err| err.to_string())?;
            if artifacts.sudo.keep(minted.as_bytes())? {
                minted
            } else {
                let bytes = artifacts
                    .sudo
                    .load()?
                    .ok_or_else(|| "sudo artifact lost during creation".to_string())?;
                custody(&bytes)?
            }
        }
    };
    let core = estate
        .seal(&sudo)
        .await
        .and_then(|core| core.identify("Actor"))
        .map_err(|err| err.to_string())?
        .share();
    Berth(&core).seed().await.map_err(|err| err.to_string())?;
    oidc::provision(&artifacts.signing)?;
    Berth(&core).rig().await.map_err(|err| err.to_string())?;
    Ok(())
}

fn custody(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "sudo artifact is not text".to_string())?;
    let token = text.trim();
    if token.is_empty() {
        return Err("sudo artifact is empty".to_string());
    }
    Ok(token.to_string())
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

async fn listen<W: Wire + 'static>(
    core: Arc<Core<W>>,
    cfg: &keel_config::Config,
    signing: &Artifact,
    issuer: &str,
) {
    let (door, svc) = match Berth(&core).rig().await {
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
    let keys = match oidc::keys(signing.path()) {
        Ok(keys) => keys,
        Err(err) => halt("signing", &err),
    };
    let flags = oidc::Oidc::new(core.clone(), svc, iss, keys).plate();
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
