use super::wire::{Head, Wire, bare};
use super::{admin, keyed, open};
use serde_json::json;

fn token(wire: &Wire, login: &str, name: &str) -> (String, Head) {
    let body = json!({ "login": login, "pass": format!("pass-{login}"), "name": name });
    let made = wire.post("/bearer", body, &bare());
    assert_eq!(made.status, 201, "bearer");
    let raw = made.text("token");
    let head = vec![("authorization".into(), format!("token {raw}"))];
    (raw, head)
}

#[test]
fn profile() {
    let (stage, wire) = keyed(&["ledger"]);
    let crown = stage.crown();
    let ada = wire.enrol(&crown, "ada");
    assert_eq!(
        wire.patch("/profile", json!({ "name": "Anon" }), &bare())
            .status,
        401
    );
    assert_eq!(
        wire.patch("/profile", json!({ "name": "Sudo" }), &crown)
            .status,
        401
    );
    for bad in [
        json!({ "name": "" }),
        json!({ "name": "   " }),
        json!({ "name": "a".repeat(121) }),
        json!({ "name": "bad\nname" }),
        json!({ "name": "Ada", "handle": "other" }),
        json!({ "name": "Ada", "actor": 999 }),
    ] {
        assert_eq!(
            wire.patch("/profile", bad, &ada.head).status,
            400,
            "invalid profile patch"
        );
    }
    let updated = wire.patch("/profile", json!({ "name": "  Ada Lovelace  " }), &ada.head);
    assert_eq!(updated.status, 200, "rename {}", updated.body);
    assert_eq!(updated.body["id"], json!(ada.id));
    assert_eq!(updated.text("login"), "ada");
    assert_eq!(updated.text("name"), "Ada Lovelace");
    assert!(!updated.text("sub").is_empty());
    let who = wire.get("/whoami", &ada.head);
    assert_eq!(who.text("login"), "ada");
    assert_eq!(who.text("name"), "Ada Lovelace");
    let svc = vec![(
        "authorization".into(),
        format!("token {}", stage.key("ledger")),
    )];
    assert_eq!(
        wire.patch("/profile", json!({ "name": "Service" }), &svc)
            .status,
        404
    );
}

#[test]
fn sessions() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    let second = wire.post(
        "/login",
        json!({ "login": "ada", "pass": "pass-ada" }),
        &bare(),
    );
    assert_eq!(second.status, 201);
    let other = vec![("cookie".into(), second.cookie())];
    let (pat, bearer) = token(&wire, "ada", "laptop");
    assert_eq!(wire.get("/sessions", &bare()).status, 401);
    assert_eq!(wire.get("/sessions", &crown).status, 401);
    let seen = wire.get("/sessions", &ada.head);
    assert_eq!(seen.status, 200);
    assert_eq!(seen.header("cache-control"), Some("no-store"));
    let text = seen.body.to_string();
    assert!(!text.contains("hash"), "leaked hash: {text}");
    assert!(!text.contains(&pat), "leaked token: {text}");
    let list = seen.body["sessions"].as_array().expect("sessions");
    assert_eq!(list.len(), 2, "two sessions");
    assert_eq!(
        list.iter()
            .filter(|row| row["current"] == json!(true))
            .count(),
        1
    );
    let tokens = seen.body["tokens"].as_array().expect("tokens");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["name"], json!("laptop"));
    assert_eq!(tokens[0]["current"], json!(false));
    let via = wire.get("/sessions", &bearer);
    assert_eq!(via.status, 200);
    assert_eq!(via.body["tokens"][0]["current"], json!(true));
    assert!(
        via.body["sessions"]
            .as_array()
            .expect("sessions")
            .iter()
            .all(|row| row["current"] == json!(false))
    );
    let current = list
        .iter()
        .find(|row| row["current"] == json!(true))
        .expect("current")["id"]
        .as_i64()
        .expect("id");
    let remote = list
        .iter()
        .find(|row| row["current"] == json!(false))
        .expect("remote")["id"]
        .as_i64()
        .expect("id");
    let tid = tokens[0]["id"].as_i64().expect("token id");
    assert_eq!(
        wire.cut(&format!("/sessions/{remote}"), &bare()).status,
        401
    );
    assert_eq!(wire.cut(&format!("/sessions/{remote}"), &crown).status, 401);
    assert_eq!(wire.cut(&format!("/tokens/{tid}"), &bare()).status, 401);
    assert_eq!(wire.cut(&format!("/tokens/{tid}"), &crown).status, 401);
    assert_eq!(wire.cut("/sessions/999999", &ada.head).status, 404);
    assert_eq!(wire.cut("/tokens/999999", &ada.head).status, 404);
    assert_eq!(
        wire.cut(&format!("/sessions/{remote}"), &boss.head).status,
        404,
        "admin cannot delete another user session"
    );
    assert_eq!(
        wire.cut(&format!("/tokens/{tid}"), &boss.head).status,
        404,
        "admin cannot delete another user token"
    );
    let cut = wire.cut(&format!("/sessions/{remote}"), &ada.head);
    assert_eq!(cut.status, 204);
    assert!(cut.header("set-cookie").is_none());
    assert_eq!(wire.get("/whoami", &other).status, 401);
    assert_eq!(wire.get("/whoami", &ada.head).status, 200);
    let drop = wire.cut(&format!("/tokens/{tid}"), &ada.head);
    assert_eq!(drop.status, 204);
    assert_eq!(wire.get("/whoami", &bearer).status, 401);
    let selfcut = wire.cut(&format!("/sessions/{current}"), &ada.head);
    assert_eq!(selfcut.status, 204);
    let cleared = selfcut.header("set-cookie").expect("cleared cookie");
    assert!(cleared.starts_with("session=;"));
    assert!(cleared.contains("Max-Age=0"));
    assert_eq!(wire.get("/whoami", &ada.head).status, 401);
}

#[test]
fn rescue() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let ada = wire.enrol(&crown, "ada");
    assert_eq!(wire.get("/rescue", &bare()).status, 401);
    assert_eq!(wire.get("/rescue", &crown).status, 401);
    assert_eq!(wire.post("/mint", json!({}), &bare()).status, 401);
    assert_eq!(wire.post("/mint", json!({}), &crown).status, 401);
    let empty = wire.get("/rescue", &ada.head);
    assert_eq!(empty.status, 200);
    assert_eq!(empty.body, json!({ "count": 0, "created": null }));
    let first = wire.post("/mint", json!({}), &ada.head);
    assert_eq!(first.status, 201);
    assert_eq!(first.header("cache-control"), Some("no-store"));
    assert_eq!(first.header("pragma"), Some("no-cache"));
    let old: Vec<String> = first.body["codes"]
        .as_array()
        .expect("codes")
        .iter()
        .map(|item| item.as_str().expect("code").to_string())
        .collect();
    assert_eq!(old.len(), 3);
    let meta = wire.get("/rescue", &ada.head);
    assert_eq!(meta.status, 200);
    assert_eq!(meta.body["count"], json!(3));
    assert!(meta.body["created"].as_i64().is_some());
    let text = meta.body.to_string();
    assert!(!text.contains("hash"));
    for code in &old {
        assert!(!text.contains(code), "leaked code in GET /rescue");
    }
    let next = wire.post("/mint", json!({}), &ada.head);
    assert_eq!(next.status, 201);
    assert_eq!(wire.get("/rescue", &ada.head).body["count"], json!(3));
    let stale = wire.post(
        "/revive",
        json!({ "login": "ada", "code": old[0], "pass": "new-anchor-8" }),
        &bare(),
    );
    assert_eq!(stale.status, 401, "rotated rescue code still worked");
}
