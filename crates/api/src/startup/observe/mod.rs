mod check;
mod decode;
mod limit;

use crate::config::Locus;
use axum::Router;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use limit::{Meter, Source};
use locus::relay::{Config, Relay as Queue};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

pub struct Spec {
    pub endpoint: String,
    pub key: String,
    pub producer: String,
    pub buffer: PathBuf,
    pub origins: Vec<String>,
    pub rpm: u32,
    pub burst: u32,
    pub forwarded: bool,
}

struct Inner {
    queue: Arc<Queue>,
    meter: Meter,
    origins: HashSet<String>,
    producer: String,
    forwarded: bool,
}

#[derive(Clone)]
pub struct Relay {
    inner: Arc<Inner>,
}

impl Relay {
    pub fn open(spec: Spec) -> Result<Self, String> {
        let origins = allowlist(&spec.origins)?;
        let meter = Meter::new(spec.rpm, spec.burst)?;
        let producer = spec.producer.trim().to_string();
        let config = Config::new(spec.endpoint.trim(), spec.key, &producer, spec.buffer);
        let queue = Queue::open(config).map_err(|err| err.to_string())?;
        Ok(Self {
            inner: Arc::new(Inner {
                queue: Arc::new(queue),
                meter,
                origins,
                producer,
                forwarded: spec.forwarded,
            }),
        })
    }

    pub(crate) fn from(cfg: &Locus, root: &Path) -> Result<Self, String> {
        let key = cfg.secret(root)?;
        let buffer = cfg.dir(root)?;
        let origins = cfg
            .origins
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect();
        Self::open(Spec {
            endpoint: cfg.endpoint.clone(),
            key,
            producer: cfg.producer.clone(),
            buffer,
            origins,
            rpm: cfg.rpm,
            burst: cfg.burst,
            forwarded: cfg.forwarded,
        })
    }

    pub fn settle(&self) -> Result<(), String> {
        self.inner.queue.settle()
    }
}

pub fn router(relay: Relay) -> Router {
    Router::new().route("/", post(ingest)).with_state(relay)
}

pub fn absent() -> Router {
    Router::new().route("/", post(|| async { StatusCode::NOT_FOUND }))
}

async fn ingest(State(relay): State<Relay>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let headers = &parts.headers;
    if let Err(status) = origin(headers, &relay.inner.origins) {
        return status.into_response();
    }
    let addr = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr);
    let key = Source(headers).caller(addr, relay.inner.forwarded);
    if let Err(wait) = relay.inner.meter.take(key) {
        return throttled(wait);
    }
    let bytes = match decode::read(headers, body).await {
        Ok(bytes) => bytes,
        Err(status) => return status.into_response(),
    };
    let lines = match check::parse(&bytes, &relay.inner.producer) {
        Ok(lines) => lines,
        Err(status) => return status.into_response(),
    };
    let queue = relay.inner.queue.clone();
    match tokio::task::spawn_blocking(move || queue.enqueue(&lines)).await {
        Ok(Ok(())) => StatusCode::NO_CONTENT.into_response(),
        Ok(Err(_)) | Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

fn throttled(wait: u64) -> Response {
    let mut headers = HeaderMap::new();
    if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
        headers.insert(header::RETRY_AFTER, value);
    }
    (StatusCode::TOO_MANY_REQUESTS, headers).into_response()
}

fn origin(headers: &HeaderMap, allowed: &HashSet<String>) -> Result<(), StatusCode> {
    site(headers)?;
    if allowed.is_empty() {
        return Ok(());
    }
    let mut items = headers.get_all(header::ORIGIN).iter();
    let Some(first) = items.next() else {
        return Err(StatusCode::FORBIDDEN);
    };
    if items.next().is_some() {
        return Err(StatusCode::FORBIDDEN);
    }
    let raw = first.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
    let parsed = canonical(raw).map_err(|_| StatusCode::FORBIDDEN)?;
    if !allowed.contains(&parsed) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

fn site(headers: &HeaderMap) -> Result<(), StatusCode> {
    let mut sites = headers.get_all("sec-fetch-site").iter();
    let Some(first) = sites.next() else {
        return Ok(());
    };
    if sites.next().is_some() {
        return Err(StatusCode::FORBIDDEN);
    }
    let raw = first.to_str().map_err(|_| StatusCode::FORBIDDEN)?;
    if !raw.trim().eq_ignore_ascii_case("same-origin") {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

fn allowlist(raw: &[String]) -> Result<HashSet<String>, String> {
    let mut out = HashSet::new();
    for item in raw {
        out.insert(canonical(item)?);
    }
    Ok(out)
}

fn canonical(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let parsed = Url::parse(trimmed).map_err(|err| format!("invalid origin {trimmed}: {err}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("invalid origin {trimmed}"));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!("invalid origin {trimmed}"));
    }
    if parsed.host_str().is_none() {
        return Err(format!("invalid origin {trimmed}"));
    }
    if !bare(&parsed) {
        return Err(format!("invalid origin {trimmed}"));
    }
    Ok(parsed.origin().ascii_serialization())
}

fn bare(parsed: &Url) -> bool {
    let root = parsed.path().is_empty() || parsed.path() == "/";
    root && parsed.query().is_none() && parsed.fragment().is_none()
}
