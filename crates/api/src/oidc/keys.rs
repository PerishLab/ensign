use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use jsonwebtoken::{DecodingKey, EncodingKey};
use serde_json::{Value, json};
use std::path::Path;

pub(crate) struct Keys {
    pub(crate) enc: EncodingKey,
    pub(crate) dec: DecodingKey,
    pub(crate) jwk: Value,
    pub(crate) kid: String,
}

pub(crate) fn keys(path: &Path) -> Keys {
    use p256::SecretKey;
    use p256::pkcs8::DecodePrivateKey;
    let secret = match std::fs::read_to_string(path) {
        Ok(pem) => SecretKey::from_pkcs8_pem(&pem).expect("key pem"),
        Err(_) => born(path),
    };
    shape(secret)
}

fn born(path: &Path) -> p256::SecretKey {
    use p256::SecretKey;
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    let secret = SecretKey::generate();
    let pem = secret.to_pkcs8_pem(LineEnding::LF).expect("pkcs8");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).expect("key dir");
    }
    std::fs::write(path, pem.as_bytes()).expect("write key");
    eprintln!("ensign: signing key born at {}", path.display());
    secret
}

fn shape(secret: p256::SecretKey) -> Keys {
    use p256::elliptic_curve::sec1::ToSec1Point;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    let pem = secret.to_pkcs8_pem(LineEnding::LF).expect("pkcs8");
    let enc = EncodingKey::from_ec_pem(pem.as_bytes()).expect("enc");
    let point = secret.public_key().to_sec1_point(false);
    let x = B64.encode(point.x().expect("x"));
    let y = B64.encode(point.y().expect("y"));
    let dec = DecodingKey::from_ec_components(&x, &y).expect("dec");
    let kid = tag(&x, &y);
    let jwk = json!({
        "kty": "EC",
        "crv": "P-256",
        "use": "sig",
        "alg": "ES256",
        "kid": kid,
        "x": x,
        "y": y,
    });
    Keys { enc, dec, jwk, kid }
}

fn tag(x: &str, y: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut sponge = Sha256::new();
    sponge.update(x.as_bytes());
    sponge.update(y.as_bytes());
    sponge
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
