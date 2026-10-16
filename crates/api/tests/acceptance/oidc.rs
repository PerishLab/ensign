use super::wire::{Head, Reply, Wire};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const REDIRECT: &str = "http://127.0.0.1:9999/cb";

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

pub fn pkce() -> Pkce {
    let verifier = format!("{}{}", digest("voyage"), digest("anchor"));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    Pkce {
        verifier,
        challenge,
    }
}

pub fn digest(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub struct Flow<'a> {
    pub wire: &'a Wire,
}

impl Flow<'_> {
    pub fn portal(&self, crown: &Head) {
        let made = self.wire.post(
            "/App",
            json!({
                "name": "Portal",
                "slug": "portal",
                "home": "http://127.0.0.1:9999/",
                "redirect": REDIRECT,
                "secret": "",
                "mode": "oidc",
            }),
            crown,
        );
        assert_eq!(made.status, 201, "portal");
    }

    pub fn walk(&self, head: &Head, challenge: &str, extra: &[(&str, &str)]) -> Reply {
        let mut pairs = vec![
            ("response_type", "code"),
            ("client_id", "portal"),
            ("redirect_uri", REDIRECT),
            ("state", "voyage & drift"),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
        ];
        pairs.extend_from_slice(extra);
        self.wire.query("/authorize", head, &pairs)
    }

    pub fn dance(&self, head: &Head, challenge: &str, extra: &[(&str, &str)]) -> String {
        let res = self.walk(head, challenge, extra);
        let place = res.header("location").unwrap_or_default().to_string();
        let query = place
            .split_once('?')
            .map(|(_, rest)| rest)
            .unwrap_or_default();
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("code="))
            .map(str::to_string)
            .unwrap_or_else(|| panic!("no code in {place} {}", res.status))
    }

    pub fn trade(&self, code: &str, verifier: &str) -> Reply {
        self.wire.form(
            "/token",
            &[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", REDIRECT),
                ("client_id", "portal"),
                ("code_verifier", verifier),
            ],
        )
    }

    pub fn renew(&self, token: &str, client: Option<&str>) -> Reply {
        let mut fields = vec![("grant_type", "refresh_token"), ("refresh_token", token)];
        if let Some(id) = client {
            fields.push(("client_id", id));
        }
        self.wire.form("/token", &fields)
    }

    pub fn mint(&self, head: &Head, scope: &str) -> Reply {
        let pk = pkce();
        let code = self.dance(head, &pk.challenge, &[("scope", scope)]);
        self.trade(&code, &pk.verifier)
    }

    pub fn attest(&self, token: &str) -> Value {
        let set = self.wire.get("/.well-known/jwks.json", &Vec::new());
        let jwk = &set.body["keys"][0];
        let mut point = vec![4u8];
        for axis in ["x", "y"] {
            let raw = jwk[axis].as_str().expect("jwk axis");
            point.extend(URL_SAFE_NO_PAD.decode(raw).expect("jwk base64"));
        }
        let key = VerifyingKey::from_sec1_bytes(&point).expect("jwk point");
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "id token shape");
        let seal = URL_SAFE_NO_PAD.decode(parts[2]).expect("seal base64");
        let signature = Signature::from_slice(&seal).expect("seal shape");
        let data = format!("{}.{}", parts[0], parts[1]);
        key.verify(data.as_bytes(), &signature)
            .expect("id token signature invalid");
        let load = URL_SAFE_NO_PAD.decode(parts[1]).expect("load base64");
        serde_json::from_slice(&load).expect("claims json")
    }
}
