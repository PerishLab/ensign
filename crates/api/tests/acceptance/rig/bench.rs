#![allow(dead_code)]

use super::stage::{Unit, serial, store};
use super::wire::{Ask, Head, Line, Reply, Wire};
use api::{Artifact, Berth, Booth};
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use keel::adapt::db::Sqlite;
use keel::adapt::pg::Postgres;
use std::path::{Path, PathBuf};
use std::sync::{Arc, MutexGuard};
use tokio::runtime::Runtime;
use tower::ServiceExt;

pub const ISSUER: &str = "http://ensign.test/api";

pub struct Bench {
    pub origin: String,
    pub base: String,
    pub sudo: String,
    pub nest: PathBuf,
    router: Router,
    spin: Arc<Runtime>,
    room: tempfile::TempDir,
    hold: Option<MutexGuard<'static, Unit>>,
}

impl Bench {
    pub fn new() -> Self {
        Self::with(&[])
    }

    pub fn with(slugs: &[&str]) -> Self {
        let hold = serial();
        let room = tempfile::tempdir().expect("bench room");
        let nest = room.path().to_path_buf();
        let spin = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("bench runtime");
        let (router, sudo) = spin.block_on(async {
            match store() {
                Some(url) => {
                    let mut held = Postgres::at(url).await.expect("pg");
                    held.wipe().await.expect("wipe");
                    raise(held, &nest, slugs).await
                }
                None => raise(Sqlite::memory().await.expect("db"), &nest, slugs).await,
            }
        });
        Self {
            origin: ISSUER.trim_end_matches("/api").to_string(),
            base: ISSUER.to_string(),
            sudo,
            nest,
            router,
            spin: Arc::new(spin),
            room,
            hold,
        }
    }

    pub fn wire(&self) -> Wire {
        let line = Loop {
            router: self.router.clone(),
            spin: self.spin.clone(),
        };
        Wire::on(&self.base, Box::new(line))
    }

    pub fn key(&self, slug: &str) -> String {
        let held = std::fs::read_to_string(self.nest.join(format!("{slug}.key")));
        held.expect("key custody").trim().to_string()
    }

    pub fn crown(&self) -> Vec<(String, String)> {
        vec![("authorization".into(), format!("sudo {}", self.sudo))]
    }
}

async fn raise<W: keel::Wire + 'static>(store: W, nest: &Path, slugs: &[&str]) -> (Router, String) {
    let mut estate = keel::bootstrap(api::shape(), store).expect("bootstrap");
    let sudo = estate.mint().await.expect("mint");
    let core = estate
        .seal(&sudo)
        .await
        .expect("seal")
        .identify("Actor")
        .expect("identify")
        .share();
    let (gate, _) = Berth(&core).seed().await.expect("seed");
    let keys = api::provision(&Artifact::Kept(nest.join("sign.pem"))).expect("signing");
    for slug in slugs {
        let held = Artifact::Kept(nest.join(format!("{slug}.key")));
        Berth(&core).key(&gate, slug, &held).await.expect("key");
    }
    let (door, svc) = Berth(&core).rig().await.expect("rig");
    let booth = Booth::new(core, door, svc, ISSUER.starts_with("https://"));
    (api::router(booth, ISSUER.to_string(), keys, ""), sudo)
}

struct Loop {
    router: Router,
    spin: Arc<Runtime>,
}

impl Line for Loop {
    fn send(&self, ask: Ask) -> Reply {
        let mut made = Request::builder().method(ask.verb).uri(path(&ask.url));
        for (key, value) in &ask.head {
            made = made.header(key, value);
        }
        let text = match ask.body {
            Some((kind, text)) => {
                made = made.header("content-type", kind);
                text
            }
            None => String::new(),
        };
        let asked = made.body(Body::from(text)).expect("request");
        let router = self.router.clone();
        self.spin.block_on(async move {
            let res = router.oneshot(asked).await.expect("router");
            let status = res.status().as_u16();
            let head: Head = res
                .headers()
                .iter()
                .map(|(key, value)| {
                    (
                        key.as_str().to_string(),
                        value.to_str().unwrap_or_default().to_string(),
                    )
                })
                .collect();
            let bytes = to_bytes(res.into_body(), usize::MAX).await.expect("body");
            Reply::new(status, &String::from_utf8_lossy(&bytes), head)
        })
    }
}

fn path(url: &str) -> &str {
    match url.split_once("://") {
        Some((_, rest)) => rest.find('/').map_or("/", |at| &rest[at..]),
        None => url,
    }
}
