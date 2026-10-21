use super::keyed;
use super::oidc::Flow;
use super::stage::Stage;
use super::wire::{Reply, Wire, bare};
use serde_json::json;

const SWAP: &str = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS: &str = "urn:ietf:params:oauth:token-type:access_token";

fn swap(wire: &Wire, key: &str, sort: &str, resource: &str) -> Reply {
    wire.form(
        "/token",
        &[
            ("grant_type", SWAP),
            ("subject_token", key),
            ("subject_token_type", sort),
            ("resource", resource),
        ],
    )
}

fn refused(reply: &Reply, error: &str, case: &str) {
    assert_eq!(reply.status, 400, "{case}: {}", reply.body);
    assert_eq!(reply.text("error"), error, "{case}: {}", reply.body);
    assert!(reply.body.get("access_token").is_none(), "{case} minted");
}

fn spec(stage: &Stage, slug: &str, file: &str) -> String {
    format!("key:{slug}=file:{}", stage.nest.join(file).display())
}

fn actor(stage: &Stage, wire: &Wire, sub: &str) -> i64 {
    let rows = wire.get("/Actor", &stage.crown()).body;
    rows.as_array()
        .and_then(|rows| rows.iter().find(|row| row["sub"] == json!(sub)))
        .and_then(|row| row["id"].as_i64())
        .unwrap_or_else(|| panic!("no Actor {sub} in {rows}"))
}

#[test]
fn exchange() {
    let (stage, wire) = keyed(&["ledger"]);
    let key = stage.key("ledger");
    assert_eq!(key.len(), 64, "key shape");
    let grip = swap(&wire, &key, ACCESS, "ledger");
    assert_eq!(grip.status, 200, "exchange {}", grip.body);
    assert_eq!(grip.header("cache-control"), Some("no-store"), "cacheable");
    assert_eq!(grip.text("issued_token_type"), ACCESS, "{}", grip.body);
    assert_eq!(grip.text("token_type"), "Bearer", "{}", grip.body);
    assert_eq!(grip.body["expires_in"], json!(3600), "{}", grip.body);
    assert!(grip.body.get("refresh_token").is_none(), "refresh minted");
    assert!(grip.body.get("id_token").is_none(), "id token minted");
    let flow = Flow { wire: &wire };
    let claims = flow.attest(&grip.text("access_token"));
    assert_eq!(claims["aud"], json!("ledger"), "claims {claims}");
    assert_eq!(claims["kind"], json!("access"), "claims {claims}");
    assert_eq!(claims["iss"], json!(stage.base), "claims {claims}");
    let sub = claims["sub"].as_str().unwrap_or_default();
    assert!(!sub.is_empty(), "claims {claims}");
}

#[test]
fn listed() {
    let (_stage, wire) = keyed(&[]);
    let meta = wire.get("/.well-known/openid-configuration", &bare()).body;
    let kinds = meta["grant_types_supported"].as_array().expect("grants");
    for kind in ["authorization_code", "refresh_token", SWAP] {
        assert!(kinds.contains(&json!(kind)), "{kind} unlisted: {meta}");
    }
}

#[test]
fn target() {
    let (stage, wire) = keyed(&["ledger"]);
    let flow = Flow { wire: &wire };
    flow.portal(&stage.crown());
    let key = stage.key("ledger");
    refused(
        &swap(&wire, &key, ACCESS, "ghost"),
        "invalid_target",
        "unknown",
    );
    refused(
        &swap(&wire, &key, ACCESS, "portal"),
        "invalid_target",
        "oidc",
    );
    refused(&swap(&wire, &key, ACCESS, ""), "invalid_target", "empty");
    let bare = wire.form(
        "/token",
        &[
            ("grant_type", SWAP),
            ("subject_token", &key),
            ("subject_token_type", ACCESS),
        ],
    );
    refused(&bare, "invalid_target", "absent");
}

#[test]
fn subject() {
    let (stage, wire) = keyed(&["ledger"]);
    let key = stage.key("ledger");
    let jwt = "urn:ietf:params:oauth:token-type:jwt";
    refused(&swap(&wire, &key, jwt, "ledger"), "invalid_request", "sort");
    let blind = wire.form(
        "/token",
        &[
            ("grant_type", SWAP),
            ("subject_token", &key),
            ("resource", "ledger"),
        ],
    );
    refused(&blind, "invalid_request", "untyped");
    let ghost = "0".repeat(64);
    refused(
        &swap(&wire, &ghost, ACCESS, "ledger"),
        "invalid_grant",
        "stranger",
    );
}

#[test]
fn revoked() {
    let (stage, wire) = keyed(&["ledger"]);
    let key = stage.key("ledger");
    let head = vec![("authorization".to_string(), format!("token {key}"))];
    assert_eq!(wire.cut("/bearer", &head).status, 204, "revoke");
    refused(
        &swap(&wire, &key, ACCESS, "ledger"),
        "invalid_grant",
        "revoked",
    );
}

#[test]
fn barred() {
    let (stage, wire) = keyed(&["ledger"]);
    let key = stage.key("ledger");
    let id = actor(&stage, &wire, "ledger");
    let shut = wire.patch(
        &format!("/Actor/{id}"),
        json!({ "barred": "true" }),
        &stage.crown(),
    );
    assert!(shut.status < 300, "bar {}", shut.body);
    refused(
        &swap(&wire, &key, ACCESS, "ledger"),
        "invalid_grant",
        "barred",
    );
}

#[test]
fn replay() {
    let (stage, wire) = keyed(&["ledger"]);
    let held = std::fs::read(stage.nest.join("ledger.key")).expect("key");
    let again = stage.again(&[spec(&stage, "ledger", "ledger.key")]);
    assert!(
        again.status.success(),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert_eq!(
        std::fs::read(stage.nest.join("ledger.key")).expect("key"),
        held
    );
    let grip = swap(&wire, &stage.key("ledger"), ACCESS, "ledger");
    assert_eq!(grip.status, 200, "replayed key {}", grip.body);
}

#[test]
fn drift() {
    let (stage, wire) = keyed(&["ledger", "vault"]);
    let flow = Flow { wire: &wire };
    let crown = stage.crown();
    flow.portal(&crown);
    wire.enrol(&crown, "sailor");
    std::fs::write(stage.nest.join("stray.key"), "not-a-key").expect("stray");
    let vault = std::fs::read(stage.nest.join("vault.key")).expect("vault");
    let cases = [
        (spec(&stage, "portal", "portal.key"), "conflicting App"),
        (spec(&stage, "sailor", "sailor.key"), "conflicting Actor"),
        (spec(&stage, "ensign", "ensign.key"), "conflicting Actor"),
        (spec(&stage, "ledger", "stray.key"), "holds no live key"),
        (spec(&stage, "ledger", "vault.key"), "holds no live key"),
    ];
    for (case, note) in cases {
        let out = stage.again(std::slice::from_ref(&case));
        let told = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{case} accepted");
        assert!(told.contains(note), "{case}: {told}");
    }
    for file in ["portal.key", "sailor.key", "ensign.key"] {
        assert!(!stage.nest.join(file).exists(), "{file} written on drift");
    }
    let stray = std::fs::read(stage.nest.join("stray.key")).expect("stray");
    assert_eq!(stray, b"not-a-key", "drift rewrote a foreign destination");
    assert_eq!(
        std::fs::read(stage.nest.join("vault.key")).expect("vault"),
        vault
    );
}
