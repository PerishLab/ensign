use super::oidc::REDIRECT;
use super::wire::bare;
use super::{admin, open};
use serde_json::json;

#[test]
fn apps() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    assert_eq!(wire.get("/admin/apps", &bare()).status, 401);
    assert_eq!(wire.get("/admin/apps", &crown).status, 401);
    assert_eq!(wire.post("/admin/apps", json!({}), &bare()).status, 401);
    assert_eq!(wire.post("/admin/apps", json!({}), &crown).status, 401);
    let payload = json!({
        "name": "Portal",
        "slug": "portal",
        "home": "https://portal.perish.top",
        "redirect": REDIRECT,
        "mode": "oidc",
    });
    assert_eq!(
        wire.post("/admin/apps", payload.clone(), &ada.head).status,
        403
    );
    for bad in [
        json!({ "name": "", "slug": "a", "mode": "resource" }),
        json!({ "name": "a".repeat(121), "slug": "a", "mode": "resource" }),
        json!({ "name": "Bad", "slug": "Bad_Slug", "mode": "resource" }),
        json!({ "name": "Bad", "slug": "-", "mode": "resource" }),
        json!({ "name": "Bad", "slug": "-foo", "mode": "resource" }),
        json!({ "name": "Bad", "slug": "foo-", "mode": "resource" }),
        json!({ "name": "Bad", "slug": "a".repeat(65), "mode": "resource" }),
        json!({ "name": "Bad", "slug": "bad", "mode": "unknown" }),
        json!({ "name": "Bad", "slug": "bad", "mode": "forward" }),
        json!({ "name": "Bad", "slug": "bad", "mode": "resource", "secret": "x" }),
    ] {
        assert_eq!(wire.post("/admin/apps", bad, &boss.head).status, 400);
    }
    for link in [
        "http://evil.example.com/cb",
        "http://localhost.evil.com/cb",
        "http://127.0.0.1.evil.com/cb",
        "http://localhost@evil.com/cb",
        "https://",
        "https://app.perish.top/cb?a=1",
        "https://app.perish.top/cb#frag",
        "javascript:alert(1)",
    ] {
        let bad = json!({
            "name": "Bad",
            "slug": "bad",
            "home": "https://ok.perish.top",
            "redirect": link,
            "mode": "oidc",
        });
        assert_eq!(
            wire.post("/admin/apps", bad, &boss.head).status,
            400,
            "{link}"
        );
    }
    assert_eq!(
        wire.post(
            "/admin/apps",
            json!({
                "name": "Frag",
                "slug": "frag",
                "home": "https://ok.perish.top/#frag",
                "mode": "forward",
            }),
            &boss.head,
        )
        .status,
        400
    );
    let oidc = wire.post("/admin/apps", payload.clone(), &boss.head);
    assert_eq!(oidc.status, 201, "create oidc {}", oidc.body);
    assert_eq!(oidc.body["barred"], json!(false));
    assert!(oidc.body.get("secret").is_none(), "leaked secret");
    let oid = oidc.id();
    assert_eq!(wire.post("/admin/apps", payload, &boss.head).status, 409);
    let forward = wire.post(
        "/admin/apps",
        json!({ "name": "Dash", "slug": "dash", "home": "https://dash.perish.top", "mode": "forward" }),
        &boss.head,
    );
    assert_eq!(forward.status, 201);
    let resource = wire.post(
        "/admin/apps",
        json!({ "name": "Vault", "slug": "vault", "mode": "resource" }),
        &boss.head,
    );
    assert_eq!(resource.status, 201);
    let legacy = wire.post(
        "/App",
        json!({
            "name": "Legacy",
            "slug": "legacy",
            "home": "https://legacy.perish.top",
            "redirect": "https://legacy.perish.top/cb",
            "secret": "hidden-value",
            "mode": "oidc",
        }),
        &crown,
    );
    assert_eq!(legacy.status, 201, "legacy POST /App without barred");
    let peer = wire.get("/admin/apps", &ada.head);
    assert_eq!(peer.status, 200);
    assert_eq!(peer.header("cache-control"), Some("no-store"));
    let listed = wire.get("/admin/apps", &boss.head);
    assert_eq!(listed.status, 200);
    assert_eq!(listed.header("cache-control"), Some("no-store"));
    let text = listed.body.to_string();
    assert!(!text.contains("secret"), "leaked secret key: {text}");
    assert!(
        !text.contains("hidden-value"),
        "leaked secret value: {text}"
    );
    assert_eq!(listed.body.as_array().expect("apps").len(), 4);
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{oid}"),
            json!({ "barred": true }),
            &bare()
        )
        .status,
        401
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{oid}"),
            json!({ "barred": true }),
            &crown
        )
        .status,
        401
    );
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{oid}"),
            json!({ "barred": true }),
            &ada.head
        )
        .status,
        403
    );
    assert_eq!(
        wire.patch("/admin/apps/999999", json!({ "barred": true }), &boss.head)
            .status,
        404
    );
    for bad in [
        json!({}),
        json!({ "slug": "other" }),
        json!({ "mode": "forward" }),
        json!({ "secret": "x" }),
        json!({ "redirect": "http://localhost.evil.com/cb" }),
        json!({ "redirect": "https://main.perish.top/cb?q=1" }),
        json!({ "home": "http://localhost@evil.com/" }),
        json!({ "name": "   " }),
        json!({ "name": "a".repeat(121) }),
    ] {
        assert_eq!(
            wire.patch(&format!("/admin/apps/{oid}"), bad, &boss.head)
                .status,
            400
        );
    }
    let edited = wire.patch(
        &format!("/admin/apps/{oid}"),
        json!({
            "name": "Main Portal",
            "home": "https://main.perish.top",
            "redirect": "https://main.perish.top/cb",
            "barred": true,
        }),
        &boss.head,
    );
    assert_eq!(edited.status, 200);
    assert_eq!(edited.text("name"), "Main Portal");
    assert_eq!(edited.text("home"), "https://main.perish.top");
    assert_eq!(edited.text("redirect"), "https://main.perish.top/cb");
    assert_eq!(edited.body["barred"], json!(true));
    assert!(edited.body.get("secret").is_none());
}
