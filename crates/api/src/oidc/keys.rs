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

pub(crate) fn keys(path: &Path) -> Result<Keys, String> {
    use p256::SecretKey;
    use p256::pkcs8::DecodePrivateKey;
    let pem = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read signing artifact {}: {err}", path.display()))?;
    let secret = SecretKey::from_pkcs8_pem(&pem)
        .map_err(|_| format!("malformed signing artifact {}", path.display()))?;
    shape(secret)
}

pub(crate) fn provision(artifact: &crate::artifact::Artifact) -> Result<Keys, String> {
    use p256::SecretKey;
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    match keys(artifact.path()) {
        Ok(keys) => return Ok(keys),
        Err(_) if artifact.path().exists() => {
            return Err(format!(
                "signing artifact is inaccessible or malformed: {}",
                artifact.path().display()
            ));
        }
        Err(_) => {}
    }
    let secret = SecretKey::generate();
    let pem = secret
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|_| "cannot encode signing key".to_string())?;
    if !artifact.keep(pem.as_bytes())? {
        return keys(artifact.path());
    }
    shape(secret)
}

fn shape(secret: p256::SecretKey) -> Result<Keys, String> {
    use p256::elliptic_curve::sec1::ToSec1Point;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    let pem = secret
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|_| "cannot encode signing key".to_string())?;
    let enc = EncodingKey::from_ec_pem(pem.as_bytes())
        .map_err(|_| "signing key cannot encode tokens".to_string())?;
    let point = secret.public_key().to_sec1_point(false);
    let x = B64.encode(
        point
            .x()
            .ok_or_else(|| "signing key has no x coordinate".to_string())?,
    );
    let y = B64.encode(
        point
            .y()
            .ok_or_else(|| "signing key has no y coordinate".to_string())?,
    );
    let dec = DecodingKey::from_ec_components(&x, &y)
        .map_err(|_| "signing key cannot verify tokens".to_string())?;
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
    Ok(Keys { enc, dec, jwk, kid })
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
