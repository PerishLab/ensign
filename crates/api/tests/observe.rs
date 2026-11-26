#[path = "observe/gates.rs"]
mod gates;

use api::config::Locus;
use api::observe::{Config, Relay, router};
use api::{Artifact, Berth, Booth};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use flate2::Compression;
use flate2::write::GzEncoder;
use keel::adapt::db::Sqlite;
use locus::Atom;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;
use tower::ServiceExt;

pub(crate) struct Received {
    pub(crate) line: String,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: String,
}

pub(crate) fn upstream() -> (SocketAddr, Receiver<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = listener.local_addr().expect("addr");
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            if let Some(req) = handle(stream) {
                let _ = sender.send(req);
            }
        }
    });
    (address, receiver)
}

fn handle(mut stream: TcpStream) -> Option<Received> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let split = loop {
        let count = stream.read(&mut chunk).ok()?;
        if count == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..count]);
        if let Some(pos) = buffer.windows(4).position(|win| win == b"\r\n\r\n") {
            break pos;
        }
    };
    let head = String::from_utf8(buffer[..split].to_vec()).ok()?;
    let mut lines = head.split("\r\n");
    let line = lines.next()?.to_owned();
    let mut headers = HashMap::new();
    for item in lines {
        if let Some((name, value)) = item.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }
    let length: usize = headers
        .get("content-length")
        .and_then(|val| val.parse().ok())
        .unwrap_or(0);
    let mut body = buffer[split + 4..].to_vec();
    while body.len() < length {
        let count = stream.read(&mut chunk).ok()?;
        if count == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..count]);
    }
    body.truncate(length);
    let reply = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(reply.as_bytes());
    Some(Received {
        line,
        headers,
        body: String::from_utf8(body).ok()?,
    })
}

pub(crate) fn atom(id: &str, producer: &str) -> String {
    format!(
        r#"{{"at":1700000000000000001,"id":"{id}","producer":"{producer}","context":{{"locus.schema":"locus.log","route":"/login"}},"choices":[]}}"#
    )
}

pub(crate) fn compress(raw: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(raw).expect("write");
    enc.finish().expect("finish")
}

pub(crate) fn spec(addr: SocketAddr, dir: &tempfile::TempDir) -> Config {
    Config::new(
        format!("http://{addr}"),
        "writer-secret",
        "ensign-web",
        dir.path().join("relay"),
    )
}

pub(crate) async fn send(
    app: &Router,
    req: Request<Body>,
) -> (StatusCode, HashMap<String, String>) {
    let res = app.clone().oneshot(req).await.expect("oneshot");
    let status = res.status();
    let heads = res
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    (status, heads)
}

async fn estate(dir: &tempfile::TempDir) -> (Booth<Sqlite>, api::Keys) {
    let mut boot = keel::bootstrap(api::shape(), Sqlite::memory().await.expect("db")).expect("b");
    let sudo = boot.mint().await.expect("mint");
    let core = boot
        .seal(&sudo)
        .await
        .expect("seal")
        .identify("Actor")
        .expect("id")
        .share();
    Berth(&core).seed().await.expect("seed");
    let keys = api::provision(&Artifact::Kept(dir.path().join("sign.pem"))).expect("keys");
    let (door, svc) = Berth(&core).rig().await.expect("rig");
    (Booth::new(core, door, svc, false), keys)
}

#[tokio::test]
async fn disabled() {
    let cfg = Locus::default();
    assert!(!cfg.enabled());
    let redacted = Locus {
        key: "super-secret".into(),
        ..Locus::default()
    };
    let text = format!("{redacted:?}");
    assert!(text.contains("[redacted]"));
    assert!(!text.contains("super-secret"));

    let dir = tempfile::tempdir().expect("dir");
    let (booth, keys) = estate(&dir).await;
    let app = api::router(booth, "http://ensign.test/api".into(), keys, "");
    let req = Request::builder()
        .method("POST")
        .uri("/api/observe")
        .body(Body::from(atom("a1", "ensign-web")))
        .expect("req");
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delivery() {
    let (addr, rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let (booth, keys) = estate(&dir).await;
    let relay = Relay::open(spec(addr, &dir)).expect("relay");
    let app = api::observed(booth, "http://ensign.test/api".into(), keys, relay.clone());

    let first = atom("d1", "ensign-web");
    let second = atom("d2", "ensign-web");
    let payload = format!("{first}\n{second}\n");
    let req = Request::builder()
        .method("POST")
        .uri("/api/observe")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(payload.clone()))
        .expect("req");
    assert_eq!(send(&app, req).await.0, StatusCode::NO_CONTENT);

    let flush = relay.clone();
    tokio::task::spawn_blocking(move || flush.settle())
        .await
        .expect("join")
        .expect("settle");

    let got = rx.recv_timeout(Duration::from_secs(5)).expect("delivered");
    assert_eq!(got.line, "POST /api/v1/atoms?producer=ensign-web HTTP/1.1");
    assert_eq!(
        got.headers.get("authorization").map(String::as_str),
        Some("Bearer writer-secret")
    );
    assert_eq!(
        got.headers.get("content-type").map(String::as_str),
        Some("application/x-ndjson")
    );
    assert_eq!(got.body, payload);
    for line in got.body.lines() {
        let parsed: Atom = serde_json::from_str(line).expect("atom");
        assert_eq!(parsed.producer(), Some("ensign-web"));
    }
    let _ = router(relay);
}
