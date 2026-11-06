//! The signed pass the lobby's join action hands a player, checked when the
//! table socket says hello.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub const PASS_TTL_SECS: i64 = 12 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pass {
    pub lobby_id: String,
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
    /// Owns the lobby, so hosts the drive whenever they are in it.
    #[serde(default)]
    pub owner: bool,
    pub exp: i64,
}

pub fn sign(pass: &Pass, secret: &[u8]) -> String {
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(pass).expect("pass serializes"));
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(payload.as_bytes());
    format!("{payload}.{}", URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

pub fn verify(token: &str, secret: &[u8], now: i64) -> Option<Pass> {
    let (payload, signature) = token.split_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature).ok()?;
    let pass: Pass = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    (pass.exp > now).then_some(pass)
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// `OUTROADS_SECRET`, else one derived from `PARTY_SECRET` so a stack that
/// already sets the party's needs nothing new. Passes outlive a restart only
/// when one of them is set.
pub fn secret_from_env() -> [u8; 32] {
    let read = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    match (read("OUTROADS_SECRET"), read("PARTY_SECRET")) {
        (Some(secret), _) => Sha256::digest(secret.as_bytes()).into(),
        (None, Some(party)) => Sha256::digest(format!("outroads:{party}").as_bytes()).into(),
        (None, None) => rand::random(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(exp: i64) -> Pass {
        Pass {
            lobby_id: "lobby".into(),
            user_id: "u1".into(),
            name: "Pat".into(),
            avatar_url: String::new(),
            owner: false,
            exp,
        }
    }

    #[test]
    fn a_pass_checks_out_until_it_expires() {
        let token = sign(&pass(100), b"secret");
        assert_eq!(verify(&token, b"secret", 99), Some(pass(100)));
        assert_eq!(verify(&token, b"secret", 100), None);
        assert_eq!(verify(&token, b"other", 99), None);
        let (payload, signature) = token.split_once('.').unwrap();
        let forged = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(&Pass {
                owner: true,
                ..pass(100)
            })
            .unwrap(),
        );
        assert_ne!(forged, payload);
        assert_eq!(verify(&format!("{forged}.{signature}"), b"secret", 1), None);
        assert_eq!(verify("garbage", b"secret", 1), None);
    }
}
