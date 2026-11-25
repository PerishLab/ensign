use super::oidc::{Flow, REDIRECT, pkce};
use super::wire::Wire;
use super::{admin, keyed, open};
use serde_json::json;

const SWAP: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

fn swap(wire: &Wire, key: &str, resource: &str) -> u16 {
    wire.form(
        "/token",
        &[
            ("grant_type", SWAP),
            ("subject_token", key),
            ("subject_token_type", ACCESS),
            ("resource", resource),
        ],
    )
    .status
}

#[test]
fn bypass() {
    let (stage, wire) = open();
    let crown = stage.crown();
    let ada = wire.enrol(&crown, "ada");
    let pk = pkce();
    for (at, evil) in [
        "http://localhost.evil.com/cb",
        "http://127.0.0.1.evil.com/cb",
        "http://localhost@evil.com/cb",
        "https://app.perish.top/cb?a=1",
    ]
    .iter()
    .enumerate()
    {
        let slug = format!("evil-{at}");
        let made = wire.post(
            "/App",
            json!({
                "name": slug,
                "slug": slug,
                "home": "https://app.perish.top",
                "redirect": evil,
                "secret": "",
                "mode": "oidc",
            }),
            &crown,
        );
        assert_eq!(made.status, 201);
        let res = wire.query(
            "/authorize",
            &ada.head,
            &[
                ("response_type", "code"),
                ("client_id", &slug),
                ("redirect_uri", evil),
                ("scope", "openid"),
                ("state", "s"),
                ("code_challenge", &pk.challenge),
                ("code_challenge_method", "S256"),
            ],
        );
        assert_eq!(res.status, 400, "authorize accepted {evil}");
    }
}

#[test]
fn barred() {
    let (stage, wire) = keyed(&["ledger"]);
    let crown = stage.crown();
    let boss = admin(&wire, &crown, "boss");
    let ada = wire.enrol(&crown, "ada");
    let flow = Flow { wire: &wire };
    let portal = wire.post(
        "/admin/apps",
        json!({
            "name": "Portal",
            "slug": "portal",
            "home": "http://127.0.0.1:9999/",
            "redirect": REDIRECT,
            "mode": "oidc",
        }),
        &boss.head,
    );
    assert_eq!(portal.status, 201);
    let pid = portal.id();
    let minted = flow.mint(&ada.head, "openid profile");
    assert_eq!(minted.status, 200);
    let refresh = minted.text("refresh_token");
    let pk = pkce();
    let mid = flow.dance(&ada.head, &pk.challenge, &[("scope", "openid")]);
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{pid}"),
            json!({ "barred": true }),
            &boss.head
        )
        .status,
        200
    );
    let traded = flow.trade(&mid, &pk.verifier);
    assert_eq!(traded.status, 400, "barred oidc app traded code");
    assert_eq!(traded.text("error"), "invalid_grant");
    let blocked = flow.walk(&ada.head, &pk.challenge, &[("scope", "openid profile")]);
    assert_eq!(blocked.status, 400, "barred oidc app authorized");
    let renew = flow.renew(&refresh, Some("portal"));
    assert_eq!(renew.status, 400, "barred oidc app refreshed");
    assert_eq!(renew.text("error"), "invalid_grant");
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{pid}"),
            json!({ "barred": false }),
            &boss.head
        )
        .status,
        200
    );
    let key = stage.key("ledger");
    assert_eq!(swap(&wire, &key, "ledger"), 200);
    let apps = wire.get("/admin/apps", &boss.head);
    let ledger = apps
        .body
        .as_array()
        .expect("apps")
        .iter()
        .find(|row| row["slug"] == json!("ledger"))
        .expect("ledger")["id"]
        .as_i64()
        .expect("ledger id");
    let aud = flow.dance(
        &ada.head,
        &pk.challenge,
        &[("scope", "openid"), ("resource", "ledger")],
    );
    let held = flow.dance(
        &ada.head,
        &pk.challenge,
        &[("scope", "openid"), ("resource", "ledger")],
    );
    let prior = flow.trade(&held, &pk.verifier);
    assert_eq!(prior.status, 200);
    let token = prior.text("refresh_token");
    assert_eq!(
        wire.patch(
            &format!("/admin/apps/{ledger}"),
            json!({ "barred": true }),
            &boss.head,
        )
        .status,
        200
    );
    assert_eq!(swap(&wire, &key, "ledger"), 400, "barred resource swapped");
    let target = flow.walk(
        &ada.head,
        &pk.challenge,
        &[("scope", "openid"), ("resource", "ledger")],
    );
    assert_eq!(target.status, 400, "barred resource audience authorized");
    let stale = flow.trade(&aud, &pk.verifier);
    assert_eq!(stale.status, 400, "barred resource traded code");
    assert_eq!(stale.text("error"), "invalid_grant");
    let spent = flow.renew(&token, Some("portal"));
    assert_eq!(spent.status, 400, "barred resource refreshed");
    assert_eq!(spent.text("error"), "invalid_grant");
    let who = flow.dance(&ada.head, &pk.challenge, &[("scope", "openid")]);
    assert_eq!(
        wire.patch(
            &format!("/admin/actors/{}", ada.id),
            json!({ "barred": true }),
            &boss.head,
        )
        .status,
        200
    );
    let dead = flow.trade(&who, &pk.verifier);
    assert_eq!(dead.status, 400, "barred actor traded code");
    assert_eq!(dead.text("error"), "invalid_grant");
}
