use super::staged;
use super::wire::bare;
use serde_json::json;

#[test]
fn served() {
    let (stage, wire) = staged(&[]);
    let health = wire.get("/health", &bare());
    assert_eq!(health.status, 200, "served health {}", health.body);
    let meta = wire.get("/.well-known/openid-configuration", &bare()).body;
    assert_eq!(meta["issuer"], json!(stage.base), "issuer");
    assert_eq!(
        meta["token_endpoint"],
        json!(format!("{}/token", stage.base)),
        "token endpoint {meta}"
    );
}
