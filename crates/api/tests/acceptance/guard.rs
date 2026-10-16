use super::oidc::digest;
use super::open;
use super::wire::{Head, Wire, bare};
use serde_json::json;

fn bar(wire: &Wire, crown: &Head, id: i64) -> u16 {
    wire.patch(&format!("/Actor/{id}"), json!({ "barred": "true" }), crown)
        .status
}

#[test]
fn stamp() {
    let (stage, wire) = open();
    let rider = wire.enrol(&stage.crown(), "rider");
    let pass = wire.get("/auth", &rider.head);
    assert_eq!(pass.status, 200, "probe");
    assert_eq!(
        pass.header("x-ensign-login"),
        Some("rider"),
        "no identity header"
    );
    let anon = wire.get("/auth", &bare());
    assert_eq!(anon.status, 401, "anon probe");
}

#[test]
fn barring() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let rider = wire.enrol(&crown, "rider");
    let status = bar(&wire, &crown, rider.id);
    assert!(matches!(status, 200 | 204), "bar {status}");
    let shut = wire.get("/auth", &rider.head);
    assert_eq!(shut.status, 401, "barred probe");
    let door = wire.post(
        "/login",
        json!({ "login": "rider", "pass": "pass-rider" }),
        &bare(),
    );
    assert_eq!(door.status, 403, "barred login");
}

#[test]
fn veil() {
    let (stage, wire) = open();
    let mallory = wire.enrol(&stage.crown(), "mallory");
    let forged = wire.post(
        "/Renew",
        json!({ "hash": digest("forged"), "slug": "portal", "actor": mallory.id.to_string() }),
        &mallory.head,
    );
    assert_eq!(forged.status, 404, "Renew authorable on the wire");
    let redeem = wire.form(
        "/token",
        &[("grant_type", "refresh_token"), ("refresh_token", "forged")],
    );
    assert_ne!(redeem.status, 200, "forged refresh redeemed");
    for unit in ["Pass", "Rescue", "Invite", "Session"] {
        let shut = wire.post(&format!("/{unit}"), json!({ "hash": "x" }), &mallory.head);
        assert_eq!(shut.status, 404, "{unit} authorable on the wire");
        let read = wire.get(&format!("/{unit}"), &mallory.head);
        assert_eq!(read.status, 404, "{unit} readable on the wire");
    }
    let query = wire.post("/query", json!({ "q": "from Pass" }), &mallory.head);
    assert_eq!(query.status, 404, "veiled unit queryable");
}
