//! Join tokens handed out by the lobby workflow and checked when the socket
//! says hello. The secret lives only in this process: a restart drops every
//! room anyway, and clients fetch a fresh token when theirs stops working.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
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
