use super::oidc::{Flow, pkce};
use super::open;
use super::wire::bare;
use serde_json::json;

#[test]
fn discovery() {
    let (stage, wire) = open();
    let meta = wire.get("/.well-known/openid-configuration", &bare()).body;
    assert_eq!(meta["issuer"], json!(stage.base), "issuer");
    let methods = meta["code_challenge_methods_supported"]
        .as_array()
        .expect("methods");
    assert!(methods.contains(&json!("S256")), "no S256");
}

#[test]
fn code() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let sailor = wire.enrol(&crown, "sailor");
    let pk = pkce();
    let extra = [("scope", "openid profile"), ("nonce", "salt")];
    let code = flow.dance(&sailor.head, &pk.challenge, &extra);
    let grip = flow.trade(&code, &pk.verifier);
    assert_eq!(grip.status, 200, "token");
    assert!(!grip.text("id_token").is_empty(), "no id token");
    assert_eq!(
        grip.header("cache-control"),
        Some("no-store"),
        "token response is cacheable"
    );
    let claims = flow.attest(&grip.text("id_token"));
    assert_eq!(claims["iss"], json!(stage.base), "claims {claims}");
    assert_eq!(claims["aud"], json!("portal"), "claims {claims}");
    let sub = claims["sub"].as_str().unwrap_or_default().to_string();
    assert!(
        !sub.is_empty() && sub != sailor.id.to_string(),
        "subject {claims}"
    );
    assert_eq!(claims["login"], json!("sailor"), "subject {claims}");
    let shown = wire.get(&format!("/profile/{sub}"), &bare());
    assert_eq!(shown.status, 200, "profile");
    assert_eq!(shown.text("handle"), "sailor", "profile {}", shown.body);
    assert!(
        shown.body.get("teams").is_none(),
        "the anonymous profile face leaked teams"
    );
    assert_eq!(claims["nonce"], json!("salt"), "nonce {claims}");
    assert_eq!(claims["kind"], json!("id"), "kind {claims}");
    let seen = wire.get("/userinfo", &bearer(&grip.text("access_token")));
    assert_eq!(
        seen.text("login"),
        "sailor",
        "userinfo profile {}",
        seen.body
    );
}

fn bearer(token: &str) -> Vec<(String, String)> {
    vec![("authorization".to_string(), format!("Bearer {token}"))]
}

#[test]
fn scope() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let sailor = wire.enrol(&crown, "sailor");
    let pk = pkce();
    let plain = flow.walk(&sailor.head, &pk.challenge, &[("scope", "profile")]);
    let place = plain.header("location").unwrap_or_default();
    assert!(
        !place.contains("code="),
        "authorize issued a code without openid"
    );
    let grip = flow.mint(&sailor.head, "openid");
    let face = wire
        .get("/userinfo", &bearer(&grip.text("access_token")))
        .body;
    let sub = face["sub"].as_str().unwrap_or_default();
    assert!(
        !sub.is_empty() && sub != sailor.id.to_string(),
        "scope leaked profile {face}"
    );
    assert!(face.get("login").is_none(), "scope leaked profile {face}");
}

#[test]
fn bounce() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    flow.portal(&stage.crown());
    let pk = pkce();
    let res = flow.walk(&bare(), &pk.challenge, &[("scope", "openid profile")]);
    assert_eq!(res.status, 303, "anon authorize");
    let place = res.header("location").unwrap_or_default();
    assert!(
        place.starts_with("/login?return=%2Fapi%2Fauthorize"),
        "bounce {place}"
    );
}

#[test]
fn claims() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let mate = wire.enrol(&crown, "mate");
    let deck = wire.post("/Team", json!({ "name": "deck" }), &crown);
    let tie = wire.post(
        &format!("/Team/{}/members", deck.id()),
        json!({ "right": mate.id }),
        &crown,
    );
    assert_eq!(tie.status, 201, "crew tie");
    let grip = flow.mint(&mate.head, "openid profile");
    let claims = flow.attest(&grip.text("id_token"));
    let teams = claims["teams"].as_array().cloned().unwrap_or_default();
    assert!(teams.contains(&json!("deck")), "id token teams {claims}");
    let face = wire
        .get("/userinfo", &bearer(&grip.text("access_token")))
        .body;
    let seen = face["teams"].as_array().cloned().unwrap_or_default();
    assert!(seen.contains(&json!("deck")), "userinfo teams {face}");
    let pass = wire.get("/auth", &mate.head);
    assert_eq!(pass.header("x-ensign-teams"), Some("deck"), "probe teams");
}

#[test]
fn verifier() {
    let (stage, wire) = open();
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    let sailor = wire.enrol(&crown, "sailor");
    let pk = pkce();
    let code = flow.dance(&sailor.head, &pk.challenge, &[("scope", "openid profile")]);
    let grip = flow.trade(&code, &format!("{}-tamper", pk.verifier));
    assert_ne!(grip.status, 200, "PKCE bypassed");
}
