use super::oidc::Flow;
use super::open;
use super::wire::bare;
use serde_json::json;

#[test]
fn rotation() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let sailor = wire.enrol(&crown, "sailor");
    let token = flow
        .mint(&sailor.head, "openid profile")
        .text("refresh_token");
    assert_ne!(
        flow.renew(&token, None).status,
        200,
        "refresh without client_id accepted"
    );
    let wrong = flow.renew(&token, Some("intruder"));
    assert_ne!(wrong.status, 200, "refresh accepted a mismatched client");
    let fresh = flow.renew(&token, Some("portal"));
    assert_eq!(fresh.status, 200, "refresh");
    assert_eq!(
        fresh.text("scope"),
        "openid profile",
        "refresh widened the scope"
    );
    let stale = flow.renew(&token, Some("portal"));
    assert_ne!(stale.status, 200, "old refresh survived rotation");
}

#[test]
fn revocation() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let castaway = wire.enrol(&crown, "castaway");
    let token = flow
        .mint(&castaway.head, "openid profile")
        .text("refresh_token");
    let minted = wire.post("/mint", json!({}), &castaway.head);
    let code = minted.body["codes"][0]
        .as_str()
        .expect("rescue code")
        .to_string();
    let back = wire.post(
        "/revive",
        json!({ "login": "castaway", "code": code, "pass": "landfall-anchor-8" }),
        &bare(),
    );
    assert_eq!(back.status, 204, "revive");
    let dead = flow.renew(&token, Some("portal"));
    assert_ne!(dead.status, 200, "refresh grant outlived recovery");
}

#[test]
fn barred() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let rope = wire.enrol(&crown, "roper");
    let held = flow
        .mint(&rope.head, "openid profile")
        .text("refresh_token");
    wire.patch(
        &format!("/Actor/{}", rope.id),
        json!({ "barred": "true" }),
        &crown,
    );
    let shut = flow.renew(&held, Some("portal"));
    assert_ne!(shut.status, 200, "barred owner still refreshed");
}
