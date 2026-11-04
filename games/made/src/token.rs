//! Passes for one person in one lobby's room, for one version of one game.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

pub const TTL_SECS: i64 = 6 * 60 * 60;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub lobby_id: String,
    pub activity_id: String,
    pub version: i32,
    pub seat: Option<u8>,
    pub user_id: String,
    pub name: String,
    pub owner: bool,
    pub exp: i64,
}

pub fn sign(claims: &Claims, secret: &[u8]) -> String {
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).expect("claims serialize"));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac key");
    mac.update(b"made:");
    mac.update(payload.as_bytes());
    format!(
        "{payload}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    )
}

pub fn verify(token: &str, secret: &[u8], now: i64) -> Option<Claims> {
    if token.len() > 2048 {
        return None;
    }
    let (payload, signature) = token.split_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(b"made:");
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature).ok()?;
    let claims: Claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
    (claims.exp > now).then_some(claims)
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> Claims {
        Claims {
            lobby_id: "L".into(),
            activity_id: "act_1".into(),
            version: 2,
            seat: Some(1),
            user_id: "u".into(),
            name: "U".into(),
            owner: true,
            exp: 100,
        }
    }

    #[test]
    fn a_pass_checks_out_only_with_its_secret_and_in_time() {
        let pass = sign(&claims(), b"secret");
        assert_eq!(verify(&pass, b"secret", 50), Some(claims()));
        assert_eq!(verify(&pass, b"other", 50), None);
        assert_eq!(verify(&pass, b"secret", 100), None);
        let (payload, sig) = pass.split_once('.').unwrap();
        let mut forged = claims();
        forged.lobby_id = "M".into();
        let forged_payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged).unwrap());
        assert_eq!(
            verify(&format!("{forged_payload}.{sig}"), b"secret", 50),
            None
        );
        assert!(verify(&format!("{payload}x.{sig}"), b"secret", 50).is_none());
    }

    #[test]
    fn a_party_pass_is_not_a_made_pass() {
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims()).unwrap());
        mac.update(payload.as_bytes());
        let unprefixed = format!(
            "{payload}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        );
        assert!(verify(&unprefixed, b"secret", 50).is_none());
    }
}
