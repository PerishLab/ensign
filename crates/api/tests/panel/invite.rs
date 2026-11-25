use super::wire::bare;
use super::{admin, open};
use serde_json::json;

#[test]
fn invites() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    assert_eq!(wire.get("/admin/invites", &bare()).status, 401);
    assert_eq!(wire.get("/admin/invites", &crown).status, 401);
    assert_eq!(wire.post("/admin/invites", json!({}), &bare()).status, 401);
    assert_eq!(wire.post("/admin/invites", json!({}), &crown).status, 401);
    assert_eq!(
        wire.post("/admin/invites", json!({}), &ada.head).status,
        403
    );
    let empty = wire.get("/admin/invites", &boss.head);
    assert_eq!(empty.status, 200);
    assert_eq!(empty.header("cache-control"), Some("no-store"));
    assert_eq!(empty.body, json!([]));
    let quiet = wire.get("/admin/invites", &ada.head);
    assert_eq!(quiet.status, 200);
    assert_eq!(quiet.header("cache-control"), Some("no-store"));
    assert_eq!(quiet.body, json!([]));
    for bad in [
        json!({ "ttl": 0 }),
        json!({ "ttl": -5 }),
        json!({ "ttl": 2_592_001 }),
        json!({ "note": "a".repeat(121) }),
        json!({ "note": "bad\nnote" }),
        json!({ "note": "x", "hash": "forged" }),
    ] {
        assert_eq!(wire.post("/admin/invites", bad, &boss.head).status, 400);
    }
    let first = wire.post(
        "/admin/invites",
        json!({ "note": "guest", "ttl": 3600 }),
        &boss.head,
    );
    assert_eq!(first.status, 201, "create {}", first.body);
    assert_eq!(first.header("cache-control"), Some("no-store"));
    assert_eq!(first.header("pragma"), Some("no-cache"));
    let code = first.text("code");
    assert!(!code.is_empty());
    assert_eq!(first.text("note"), "guest");
    assert!(first.body["created"].as_i64().is_some());
    assert!(first.body["expires"].as_i64().is_some());
    let id = first.id();
    let second = wire.post("/admin/invites", json!({ "note": "open" }), &boss.head);
    assert_eq!(second.status, 201);
    assert!(second.body["expires"].is_null());
    let seen = wire.get("/admin/invites", &boss.head);
    assert_eq!(seen.status, 200);
    assert_eq!(seen.header("cache-control"), Some("no-store"));
    let list = seen.body.as_array().expect("invites");
    assert_eq!(list.len(), 2);
    let text = seen.body.to_string();
    assert!(!text.contains("hash"), "leaked hash: {text}");
    assert!(!text.contains(&code), "leaked code: {text}");
    assert!(!text.contains(&second.text("code")), "leaked code: {text}");
    let blind = wire.get("/admin/invites", &ada.head);
    assert_eq!(blind.status, 200);
    assert_eq!(blind.body, json!([]));
    assert_eq!(
        wire.cut(&format!("/admin/invites/{id}"), &bare()).status,
        401
    );
    assert_eq!(
        wire.cut(&format!("/admin/invites/{id}"), &crown).status,
        401
    );
    assert_eq!(
        wire.cut(&format!("/admin/invites/{id}"), &ada.head).status,
        404,
        "uniform 404 prevents veiled invite existence oracle"
    );
    let viewer = wire.enrol(&crown, "viewer");
    wire.grant(&crown, &viewer.id.to_string(), ("see", "Invite"));
    assert_eq!(
        wire.cut(&format!("/admin/invites/{id}"), &viewer.head)
            .status,
        403,
        "see-only viewer cannot cancel invite"
    );
    assert_eq!(wire.cut("/admin/invites/999999", &ada.head).status, 404);
    assert_eq!(wire.cut("/admin/invites/999999", &boss.head).status, 404);
    assert_eq!(
        wire.cut(&format!("/admin/invites/{id}"), &boss.head).status,
        204
    );
    let join = wire.post(
        "/join",
        json!({ "code": code, "login": "bob", "name": "Bob", "pass": "anchor-pass-8" }),
        &bare(),
    );
    assert_eq!(join.status, 404, "cancelled invite redeemed");
    let valid = wire.post(
        "/join",
        json!({
            "code": second.text("code"),
            "login": "bob",
            "name": "Bob",
            "pass": "anchor-pass-8",
        }),
        &bare(),
    );
    assert_eq!(valid.status, 201, "redeem {}", valid.body);
    let legacy = wire.post("/invite", json!({ "note": "cli" }), &crown);
    assert_eq!(legacy.status, 201, "legacy sudo invite intact");
}
