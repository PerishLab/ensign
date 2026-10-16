use super::open;
use super::wire::{Face, Head, Wire, app};
use serde_json::json;

fn crew(wire: &Wire, crown: &Head) -> [Face; 3] {
    let boss = wire.enrol(crown, "boss");
    let hand = wire.enrol(crown, "hand");
    let guest = wire.enrol(crown, "guest");
    wire.grant(crown, &boss.id.to_string(), ("*", "App"));
    wire.grant(crown, &boss.id.to_string(), ("*", "Team"));
    [boss, hand, guest]
}

#[test]
fn register() {
    let (stage, wire) = open();
    let [boss, hand, guest] = crew(&wire, &stage.crown());
    let made = wire.post(
        "/App",
        json!({
            "name": "Wiki",
            "slug": "wiki",
            "home": "https://wiki.perish.top",
            "redirect": "https://wiki.perish.top/callback",
            "secret": "",
            "mode": "oidc",
        }),
        &boss.head,
    );
    assert_eq!(made.status, 201, "register");
    let seen = wire.get("/App", &hand.head);
    let rows = seen.body.as_array().map(Vec::len);
    assert_eq!(rows, Some(1), "member sees {}", seen.body);
    let barred = wire.post(
        "/App",
        json!({
            "name": "Rogue",
            "slug": "rogue",
            "home": "https://x",
            "redirect": "https://x/cb",
            "secret": "",
            "mode": "oidc",
        }),
        &guest.head,
    );
    assert_eq!(barred.status, 403, "stranger app");
}

#[test]
fn grant() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let [boss, hand, _] = crew(&wire, &crown);
    let team = wire.post("/Team", json!({ "name": "editors" }), &boss.head);
    let id = team.id();
    wire.grant(&crown, &format!("Team {id}"), ("put", "App"));
    let before = wire.post("/App", app("one"), &hand.head);
    assert_eq!(before.status, 403, "pre-member write");
    let tie = wire.post(
        &format!("/Team/{id}/members"),
        json!({ "right": hand.id }),
        &boss.head,
    );
    assert_eq!(tie.status, 201, "enrol");
    let during = wire.post("/App", app("two"), &hand.head);
    assert_eq!(during.status, 201, "member write");
    let cut = wire.cut(&format!("/Team/{id}/members/{}", tie.id()), &boss.head);
    assert_eq!(cut.status, 204, "revoke");
    let after = wire.post("/App", app("three"), &hand.head);
    assert_eq!(after.status, 403, "access outlived membership");
}
