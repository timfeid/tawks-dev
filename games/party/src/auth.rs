//! Join tokens handed out by the lobby workflow and checked when the socket
//! says hello. The secret lives only in this process: a restart drops every
//! room anyway, and clients fetch a fresh token when theirs stops working.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const TOKEN_TTL_SECS: i64 = 12 * 60 * 60;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    pub match_id: String,
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
    pub exp: i64,
    /// Watches only; guests get this so they cannot take a seat.
    #[serde(default)]
    pub spectator: bool,
    /// Signed out; may play but not record.
    #[serde(default)]
    pub guest: bool,
    /// Save drives this person may pick, one per friend group.
    #[serde(default)]
    pub drives: Vec<String>,
    /// Owns the lobby, so hosts the room while in it.
    #[serde(default)]
    pub owner: bool,
    /// The game the lobby entry was for, so the room is set to it on arrival.
    #[serde(default)]
    pub game: Option<String>,
}

pub fn sign(claims: &Claims, secret: &[u8]) -> String {
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).expect("claims serialize"));
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(payload.as_bytes());
    format!(
        "{payload}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    )
}

pub fn verify(token: &str, secret: &[u8], now: i64) -> Option<Claims> {
    let (payload, signature) = token.split_once('.')?;
    let signature = URL_SAFE_NO_PAD.decode(signature).ok()?;
    let mut mac = HmacSha256::new_from_slice(secret).ok()?;
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

/// Short-lived handoff tickets are opaque, single-use and never stored on disk.
/// The room server consumes them atomically on the first WebSocket hello.
#[derive(Default)]
pub struct NativePasses(std::sync::Mutex<std::collections::HashMap<String, Claims>>);
pub const NATIVE_PASS_TTL_SECS: i64 = 60;
const MAX_NATIVE_PASSES: usize = 4096;
impl NativePasses {
    pub fn exchange(
        &self,
        token: &str,
        user_id: &str,
        match_id: &str,
        secret: &[u8],
        now: i64,
    ) -> Option<String> {
        let mut claims = verify(token, secret, now)?;
        if claims.user_id != user_id || claims.match_id != match_id {
            return None;
        }
        let mut passes = self.0.lock().ok()?;
        passes.retain(|_, claims| claims.exp > now);
        if passes.len() >= MAX_NATIVE_PASSES {
            return None;
        }
        let random: [u8; 32] = rand::random();
        let ticket = format!("native-{}", URL_SAFE_NO_PAD.encode(random));
        claims.exp = now + NATIVE_PASS_TTL_SECS;
        passes.insert(ticket.clone(), claims);
        Some(ticket)
    }
    pub fn consume(&self, token: &str, now: i64) -> Option<Claims> {
        let claims = self.0.lock().ok()?.remove(token)?;
        (claims.exp > now).then_some(claims)
    }
}

#[cfg(test)]
mod native_pass_tests {
    use super::*;
    fn claims() -> Claims {
        Claims {
            match_id: "room".into(),
            user_id: "user".into(),
            name: "Player".into(),
            avatar_url: String::new(),
            exp: 10000,
            spectator: false,
            guest: false,
            drives: vec![],
            owner: true,
            game: Some("city".into()),
        }
    }
    #[test]
    fn native_tickets_expire_and_cannot_be_replayed_or_exchanged_again() {
        let passes = NativePasses::default();
        let original = sign(&claims(), b"secret");
        let ticket = passes
            .exchange(&original, "user", "room", b"secret", 100)
            .unwrap();
        assert!(verify(&ticket, b"secret", 100).is_none());
        assert!(passes
            .exchange(&ticket, "user", "room", b"secret", 100)
            .is_none());
        let claims = passes.consume(&ticket, 159).unwrap();
        assert_eq!(claims.exp, 160);
        assert_eq!(claims.user_id, "user");
        assert!(claims.owner);
        assert_eq!(claims.game.as_deref(), Some("city"));
        assert!(passes.consume(&ticket, 159).is_none());
        let expired = passes
            .exchange(&original, "user", "room", b"secret", 100)
            .unwrap();
        assert!(passes.consume(&expired, 160).is_none());
    }
    #[test]
    fn native_exchange_rejects_wrong_principal_match_signature_and_expired_source() {
        let passes = NativePasses::default();
        let original = sign(&claims(), b"secret");
        assert!(passes
            .exchange(&original, "other", "room", b"secret", 100)
            .is_none());
        assert!(passes
            .exchange(&original, "user", "other-room", b"secret", 100)
            .is_none());
        assert!(passes
            .exchange(&original, "user", "room", b"wrong", 100)
            .is_none());
        assert!(passes
            .exchange(&original, "user", "room", b"secret", 10000)
            .is_none());
    }
    #[test]
    fn expired_native_tickets_are_pruned_and_the_registry_is_bounded() {
        let passes = NativePasses::default();
        let original = sign(&claims(), b"secret");
        for _ in 0..MAX_NATIVE_PASSES {
            assert!(passes
                .exchange(&original, "user", "room", b"secret", 100)
                .is_some());
        }
        assert!(passes
            .exchange(&original, "user", "room", b"secret", 100)
            .is_none());
        assert!(passes
            .exchange(&original, "user", "room", b"secret", 161)
            .is_some());
    }
}
