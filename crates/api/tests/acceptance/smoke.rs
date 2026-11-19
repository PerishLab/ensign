use super::wire::bare;
use super::{open, staged};
use serde_json::json;

#[test]
fn served() {
    let (stage, wire) = staged(&[]);
    let health = wire.get("/health", &bare());
    assert_eq!(health.status, 200, "served health {}", health.body);
    let meta = wire.get("/.well-known/openid-configuration", &bare()).body;
    assert_eq!(meta["issuer"], json!(stage.base), "issuer");
    assert_eq!(
        meta["token_endpoint"],
        json!(format!("{}/token", stage.base)),
        "token endpoint {meta}"
    );
}

#[test]
fn pages() {
    let (stage, wire) = open();
    let root = wire.fetch(&format!("{}/", stage.origin), &bare());
    assert_eq!(root.status, 303);
    assert_eq!(root.header("location"), Some("/login"));
    for path in ["/login", "/logout"] {
        let page = wire.fetch(&format!("{}{path}", stage.origin), &bare());
        assert_eq!(page.status, 200, "{path}");
        assert_eq!(
            page.header("content-type"),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(page.header("cache-control"), Some("no-store"));
        assert_eq!(page.header("x-content-type-options"), Some("nosniff"));
        assert_eq!(page.header("referrer-policy"), Some("no-referrer"));
        let policy = page.header("content-security-policy").expect("CSP");
        assert!(policy.contains("script-src 'self'"));
        assert!(policy.contains("frame-ancestors 'none'"));
        assert!(!policy.contains("unsafe-inline"));
    }
}

#[test]
fn assets() {
    let (stage, wire) = open();
    for (path, kind) in [
        ("/assets/client.mjs", "text/javascript; charset=utf-8"),
        ("/assets/protocol.mjs", "text/javascript; charset=utf-8"),
        ("/assets/style.css", "text/css; charset=utf-8"),
    ] {
        let held = wire.fetch(&format!("{}{path}", stage.origin), &bare());
        assert_eq!(held.status, 200);
        assert_eq!(held.header("content-type"), Some(kind));
        assert_eq!(held.header("cache-control"), Some("no-store"));
    }
    let absent = wire.fetch(&format!("{}/assets/absent", stage.origin), &bare());
    assert_eq!(absent.status, 404);
}
