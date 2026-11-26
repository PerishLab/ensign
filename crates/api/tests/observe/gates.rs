use super::{atom, compress, send, spec, upstream};
use api::observe::{Relay, router};
use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn origin() {
    let (addr, _rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let mut cfg = spec(addr, &dir);
    cfg.origins = vec!["https://ENSIGN.EXAMPLE:443".into()];
    let app = router(Relay::open(cfg).expect("relay"));

    let badsite = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "cross-site")
        .header("origin", "https://ensign.example")
        .body(Body::from(atom("a1", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, badsite).await.0, StatusCode::FORBIDDEN);

    let missing = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(atom("a2", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, missing).await.0, StatusCode::FORBIDDEN);

    let wrong = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "same-origin")
        .header("origin", "https://evil.example")
        .body(Body::from(atom("a3", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, wrong).await.0, StatusCode::FORBIDDEN);

    let ok = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "Same-Origin")
        .header("origin", "https://ensign.example")
        .body(Body::from(atom("a4", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, ok).await.0, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn ratelimit() {
    let (addr, _rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let mut cfg = spec(addr, &dir);
    cfg.rpm = 60;
    cfg.burst = 2;
    cfg.forwarded = true;
    let app = router(Relay::open(cfg).expect("relay"));

    for idx in 0..2 {
        let req = Request::builder()
            .method("POST")
            .uri("/")
            .header("x-forwarded-for", "198.51.100.1, 203.0.113.7")
            .body(Body::from(atom(&format!("r{idx}"), "ensign-web")))
            .expect("req");
        assert_eq!(send(&app, req).await.0, StatusCode::NO_CONTENT);
    }
    let blocked = Request::builder()
        .method("POST")
        .uri("/")
        .header("x-forwarded-for", "203.0.113.7")
        .body(Body::from(atom("r2", "ensign-web")))
        .expect("req");
    let (status, heads) = send(&app, blocked).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let retry: u64 = heads
        .get("retry-after")
        .expect("retry-after")
        .parse()
        .expect("num");
    assert!(retry >= 1);

    let other = Request::builder()
        .method("POST")
        .uri("/")
        .header("x-real-ip", "203.0.113.8")
        .body(Body::from(atom("r3", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, other).await.0, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn ordering() {
    let (addr, _rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let mut cfg = spec(addr, &dir);
    cfg.rpm = 60;
    cfg.burst = 1;
    let app = router(Relay::open(cfg).expect("relay"));

    for idx in 0..3 {
        let denied = Request::builder()
            .method("POST")
            .uri("/")
            .header("sec-fetch-site", "cross-site")
            .body(Body::from(atom(&format!("o{idx}"), "ensign-web")))
            .expect("req");
        assert_eq!(send(&app, denied).await.0, StatusCode::FORBIDDEN);
    }

    let ok = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(atom("o3", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, ok).await.0, StatusCode::NO_CONTENT);

    let limited = Request::builder()
        .method("POST")
        .uri("/")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(atom("o4", "ensign-web")))
        .expect("req");
    assert_eq!(send(&app, limited).await.0, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn encoding() {
    let (addr, _rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let app = router(Relay::open(spec(addr, &dir)).expect("relay"));

    let line = format!("{}\n", atom("g1", "ensign-web"));
    let gz = compress(line.as_bytes());
    let ok = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .body(Body::from(gz.clone()))
        .expect("req");
    assert_eq!(send(&app, ok).await.0, StatusCode::NO_CONTENT);

    let bomb = compress(&vec![b' '; 256 * 1024 + 32]);
    let big = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .body(Body::from(bomb))
        .expect("req");
    assert_eq!(send(&app, big).await.0, StatusCode::PAYLOAD_TOO_LARGE);

    let corrupt = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .body(Body::from(vec![0x1f, 0x8b, 0x00, 0xff]))
        .expect("req");
    assert_eq!(send(&app, corrupt).await.0, StatusCode::BAD_REQUEST);

    let mut trailing = gz;
    trailing.extend_from_slice(b"junk");
    let junk = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .body(Body::from(trailing))
        .expect("req");
    assert_eq!(send(&app, junk).await.0, StatusCode::BAD_REQUEST);

    let bad = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "br")
        .body(Body::from(line.clone()))
        .expect("req");
    assert_eq!(send(&app, bad).await.0, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let dup = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .header("content-encoding", "gzip")
        .body(Body::from(line))
        .expect("req");
    assert_eq!(send(&app, dup).await.0, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let rawbig = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(vec![b'a'; 256 * 1024 + 1]))
        .expect("req");
    assert_eq!(send(&app, rawbig).await.0, StatusCode::PAYLOAD_TOO_LARGE);

    let gzbig = Request::builder()
        .method("POST")
        .uri("/")
        .header("content-encoding", "gzip")
        .body(Body::from(vec![0u8; 128 * 1024 + 1]))
        .expect("req");
    assert_eq!(send(&app, gzbig).await.0, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn validation() {
    let (addr, _rx) = upstream();
    let dir = tempfile::tempdir().expect("dir");
    let app = router(Relay::open(spec(addr, &dir)).expect("relay"));

    let empty = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(""))
        .expect("req");
    assert_eq!(send(&app, empty).await.0, StatusCode::BAD_REQUEST);

    let mismatch = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(atom("v1", "other-web")))
        .expect("req");
    assert_eq!(send(&app, mismatch).await.0, StatusCode::BAD_REQUEST);

    let unknown =
        r#"{"at":1,"id":"v2","producer":"ensign-web","context":{},"choices":[],"extra":1}"#;
    let req = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(unknown))
        .expect("req");
    assert_eq!(send(&app, req).await.0, StatusCode::BAD_REQUEST);

    let many = (0..513)
        .map(|idx| atom(&format!("m{idx}"), "ensign-web"))
        .collect::<Vec<_>>()
        .join("\n");
    let over = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(many))
        .expect("req");
    assert_eq!(send(&app, over).await.0, StatusCode::PAYLOAD_TOO_LARGE);

    let role = r#"{"at":1,"id":"r1","producer":"ensign-web","context":{},"choices":[{"role":"auth.token","key":"k","origin":"explicit"}]}"#;
    let badrole = Request::builder()
        .method("POST")
        .uri("/")
        .body(Body::from(role))
        .expect("req");
    assert_eq!(send(&app, badrole).await.0, StatusCode::BAD_REQUEST);

    for key in [
        "password",
        "pass",
        "oldPass",
        "token",
        "csrfToken",
        "secret",
        "authorization",
        "cookie",
        "code_verifier",
        "rescue",
        "rescueCodes",
        "sudo",
    ] {
        let body = format!(
            r#"{{"at":1,"id":"s1","producer":"ensign-web","context":{{}},"choices":[],"payload":{{"nested":[{{"{key}":"x"}}]}}}}"#
        );
        let req = Request::builder()
            .method("POST")
            .uri("/")
            .body(Body::from(body))
            .expect("req");
        assert_eq!(send(&app, req).await.0, StatusCode::BAD_REQUEST, "{key}");
    }
}
