use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use jsonwebtoken::{DecodingKey, EncodingKey};
use serde_json::{Value, json};

pub(crate) struct Keys {
    pub(crate) enc: EncodingKey,
    pub(crate) dec: DecodingKey,
    pub(crate) jwk: Value,
    pub(crate) kid: String,
}

pub(crate) fn keys(artifact: &crate::artifact::Artifact) -> Result<Keys, String> {
    let held = artifact
        .load()?
        .ok_or_else(|| "signing artifact is absent".to_string())?;
    read(&held)
}

fn read(bytes: &[u8]) -> Result<Keys, String> {
    use p256::SecretKey;
    use p256::pkcs8::DecodePrivateKey;
    let pem = std::str::from_utf8(bytes).map_err(|_| "signing artifact is not text".to_string())?;
    let secret =
        SecretKey::from_pkcs8_pem(pem).map_err(|_| "malformed signing artifact".to_string())?;
    shape(secret)
}

pub(crate) fn provision(artifact: &crate::artifact::Artifact) -> Result<Keys, String> {
    use p256::SecretKey;
    use p256::elliptic_curve::Generate;
    use p256::pkcs8::EncodePrivateKey;
    use p256::pkcs8::LineEnding;
    if let Some(held) = artifact.load()? {
        return read(&held)
            .map_err(|note| format!("signing artifact is inaccessible or malformed: {note}"));
    }
    let secret = SecretKey::generate();
    let pem = secret
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|_| "cannot encode signing key".to_string())?;
    if !artifact.keep(pem.as_bytes())? {
        return keys(artifact);
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
