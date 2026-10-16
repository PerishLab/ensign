use super::open;
use super::wire::bare;
use serde_json::json;

#[test]
fn namespace() {
    let (stage, wire) = open();
    let root = wire.fetch(&format!("{}/health", stage.origin), &bare());
    assert_eq!(root.status, 404, "root health");
}

#[test]
fn possession() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let seat = wire.post("/invite", json!({ "note": "first" }), &crown);
    let code = seat.text("code");
    assert_eq!(seat.status, 201, "invite");
    assert!(!code.is_empty(), "invite code");
    let open = wire.post("/invite", json!({ "note": "open" }), &bare());
    assert!(
        matches!(open.status, 401 | 403),
        "anon invite {}",
        open.status
    );
    let made = wire.post(
        "/join",
        json!({ "code": code, "login": "ada", "name": "Ada", "pass": "seaworthy" }),
        &bare(),
    );
    assert_eq!(made.status, 201, "join");
    let again = wire.post(
        "/join",
        json!({ "code": code, "login": "bob", "name": "Bob", "pass": "driftwood" }),
        &bare(),
    );
    assert_eq!(again.status, 404, "burned invite");
    let chit = wire.post("/invite", json!({ "note": "short" }), &crown);
    let feeble = wire.post(
        "/join",
        json!({ "code": chit.text("code"), "login": "wisp", "name": "Wisp", "pass": "short" }),
        &bare(),
    );
    assert_eq!(feeble.status, 400, "weak password admitted");
}

#[test]
fn floor() {
    let (stage, wire) = open();
    wire.enrol(&stage.crown(), "ada");
    let wrong = wire.post(
        "/login",
        json!({ "login": "ada", "pass": "adrift" }),
        &bare(),
    );
    assert_eq!(wrong.status, 401, "wrong pass");
    let right = wire.post(
        "/login",
        json!({ "login": "ada", "pass": "pass-ada" }),
        &bare(),
    );
    assert_eq!(right.status, 201, "login");
    let jar = vec![("cookie".to_string(), right.cookie())];
    let me = wire.get("/whoami", &jar);
    assert_eq!(me.status, 200, "whoami");
    assert_eq!(me.text("login"), "ada", "whoami login");
    let anon = wire.get("/whoami", &bare());
    assert_eq!(anon.status, 401, "anon whoami");
}

#[test]
fn logout() {
    let (stage, wire) = open();
    let face = wire.enrol(&stage.crown(), "ada");
    let out = wire.post("/logout", json!({}), &face.head);
    assert_eq!(out.status, 204, "logout");
    let gone = wire.get("/whoami", &face.head);
    assert_eq!(gone.status, 401, "dead session");
}
