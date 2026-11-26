use crate::config::Locus;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::post;
pub use locus_relay::{Config, Relay, router};
use std::path::Path;

pub(crate) fn open(cfg: &Locus, root: &Path) -> Result<Relay, String> {
    let key = cfg.secret(root)?;
    let buffer = cfg.dir(root)?;
    let origins = cfg
        .origins
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    let config = Config::new(cfg.endpoint.trim(), key, cfg.producer.trim(), buffer)
        .origins(origins)
        .rpm(cfg.rpm)
        .burst(cfg.burst)
        .forwarded(cfg.forwarded);
    Relay::open(config).map_err(|err| err.to_string())
}

pub fn absent() -> Router {
    Router::new().route("/", post(|| async { StatusCode::NOT_FOUND }))
}
