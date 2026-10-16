use super::open;
use super::wire::{Head, Wire, bare};
use serde_json::json;

struct Door<'a> {
    wire: &'a Wire,
}

impl Door<'_> {
    fn login(&self, login: &str, pass: &str) -> u16 {
        self.wire
            .post("/login", json!({ "login": login, "pass": pass }), &bare())
            .status
    }

    fn revive(&self, login: &str, code: &str, pass: &str) -> u16 {
        self.wire
            .post(
                "/revive",
                json!({ "login": login, "code": code, "pass": pass }),
                &bare(),
            )
            .status
    }

    fn codes(&self, head: &Head) -> Vec<String> {
        let minted = self.wire.post("/mint", json!({}), head);
        assert_eq!(minted.status, 201, "mint");
        minted.body["codes"]
            .as_array()
            .expect("codes")
            .iter()
            .map(|code| code.as_str().expect("code").to_string())
            .collect()
    }

    fn join(&self, crown: &Head, login: &str, pass: &str) -> u16 {
        let card = self.wire.post("/invite", json!({ "note": login }), crown);
        self.wire
            .post(
                "/join",
                json!({ "code": card.text("code"), "login": login, "name": login, "pass": pass }),
                &bare(),
            )
            .status
    }
}

#[test]
fn rescue() {
    let (stage, wire) = open();
    let door = Door { wire: &wire };
    let face = wire.enrol(&stage.crown(), "ada");
    let codes = door.codes(&face.head);
    assert_eq!(codes.len(), 3, "mint count");
    assert_eq!(
        door.revive("ada", &codes[0], "refloor-anchor-8"),
        204,
        "revive"
    );
    let stale = wire.get("/whoami", &face.head);
    assert_eq!(stale.status, 401, "old session outlived recovery");
    let sibling = door.revive("ada", &codes[1], "refloor-anchor-8");
    assert_ne!(sibling, 204, "sibling rescue code survived re-floor");
    let spent = door.revive("ada", &codes[0], "refloor-anchor-8");
    assert_eq!(spent, 401, "spent code");
    assert_eq!(
        door.login("ada", "pass-ada"),
        401,
        "old password outlived recovery"
    );
    assert_eq!(door.login("ada", "refloor-anchor-8"), 201, "relog");
}

#[test]
fn clash() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let card = wire.post("/invite", json!({ "note": "clash" }), &crown);
    let code = card.text("code");
    wire.enrol(&crown, "keeper");
    let pass = "rope-ladder-9";
    let clash = wire.post(
        "/join",
        json!({ "code": code, "login": "keeper", "name": "Second Keeper", "pass": pass }),
        &bare(),
    );
    assert_eq!(clash.status, 409, "clash");
    let fresh = wire.post(
        "/join",
        json!({ "code": code, "login": "keeper2", "name": "Keeper Two", "pass": pass }),
        &bare(),
    );
    assert_eq!(fresh.status, 201, "invite spent by the clash");
}

#[test]
fn repass() {
    let (stage, wire) = open();
    let door = Door { wire: &wire };
    door.join(&stage.crown(), "galer", "first-anchor-8");
    let held = wire.post(
        "/login",
        json!({ "login": "galer", "pass": "first-anchor-8" }),
        &bare(),
    );
    let jar = vec![("cookie".to_string(), held.cookie())];
    let wrong = wire.post(
        "/repass",
        json!({ "old": "not-the-password", "pass": "second-anchor-8" }),
        &jar,
    );
    assert_eq!(wrong.status, 403, "repass without the current password");
    let right = wire.post(
        "/repass",
        json!({ "old": "first-anchor-8", "pass": "second-anchor-8" }),
        &jar,
    );
    assert_eq!(right.status, 204, "repass with the current password");
    assert_eq!(
        door.login("galer", "first-anchor-8"),
        401,
        "old password survived repass"
    );
    assert_eq!(
        door.login("galer", "second-anchor-8"),
        201,
        "new password refused"
    );
}

#[test]
fn recovery() {
    let (stage, wire) = open();
    let door = Door { wire: &wire };
    let joined = door.join(&stage.crown(), "drifter", "old-anchor-8");
    assert_eq!(joined, 201, "join");
    let held = wire.post(
        "/login",
        json!({ "login": "drifter", "pass": "old-anchor-8" }),
        &bare(),
    );
    let jar = vec![("cookie".to_string(), held.cookie())];
    let codes = door.codes(&jar);
    assert_eq!(
        door.revive("drifter", &codes[0], "new-harbor-9"),
        204,
        "revive"
    );
    assert_eq!(
        door.login("drifter", "old-anchor-8"),
        401,
        "old password survived"
    );
    assert_eq!(
        door.login("drifter", "new-harbor-9"),
        201,
        "new password refused"
    );
}
