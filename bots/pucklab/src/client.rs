//! No inventory, wallet or game rules live in this bot. Each action calls the
//! same PuckLab API as the game, as the linked, verified workflow owner.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const SERVICE_SUBJECT: &str = "@service";

#[derive(Clone)]
pub struct PuckLab {
    http: Client,
    base: Url,
    site: Url,
    secret: Vec<u8>,
    build: u32,
    protocol: u32,
}

impl std::fmt::Debug for PuckLab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PuckLab")
            .field("base", &self.base)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: String,
    #[serde(skip)]
    pub status: u16,
    #[serde(default)]
    pub pucklab_link_required: bool,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}
impl std::error::Error for ApiError {}

#[derive(Serialize)]
struct ActorProof<'a> {
    v: u8,
    aud: &'static str,
    sub: &'a str,
    method: &'a str,
    path: &'a str,
    body_sha256: String,
    iat: u64,
    exp: u64,
    nonce: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn proof(secret: &[u8], subject: &str, method: &str, path: &str, body: &[u8]) -> Result<String> {
    ensure!(
        !subject.is_empty() && subject.len() <= 128 && !subject.chars().any(char::is_control),
        "Invalid Tawks actor"
    );
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let claims = ActorProof {
        v: 1,
        aud: "pucklab.tawks-bot.v1",
        sub: subject,
        method,
        path,
        body_sha256: hex(&Sha256::digest(body)),
        iat: now,
        exp: now + 60,
        nonce: hex(&rand::random::<[u8; 16]>()),
    };
    let encoded = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)?;
    mac.update(encoded.as_bytes());
    Ok(format!(
        "TawksBot {encoded}.{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    ))
}

fn origin(value: &str) -> Result<Url> {
    tawks_bot::web_client::require_tls(value)?;
    let url = Url::parse(value)?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "PuckLab URL must be an HTTP(S) origin"
    );
    Ok(url)
}

impl PuckLab {
    pub fn new(base: &str, secret: String, build: u32, protocol: u32) -> Result<Self> {
        let base = origin(base)?;
        ensure!(
            secret.len() >= 32,
            "PUCKLAB_BOT_SECRET must contain at least 32 bytes"
        );
        ensure!(
            build > 0 && protocol > 0,
            "Configure the supported PuckLab build and protocol"
        );
        Ok(Self {
            http: Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            site: base.clone(),
            base,
            secret: secret.into_bytes(),
            build,
            protocol,
        })
    }

    /// Links shown to people use this origin; API calls keep using the service origin.
    pub fn with_site(mut self, site: &str) -> Result<Self> {
        self.site = origin(site)?;
        Ok(self)
    }

    pub fn origin(&self) -> String {
        self.base.to_string()
    }

    pub fn page(&self, path: &str) -> String {
        // Only the bot's fixed site page names are accepted, not arbitrary URLs.
        assert!(
            path.starts_with('/') && !path.starts_with("//") && !path.contains(['?', '#', '\\'])
        );
        let mut url = self.site.clone();
        url.set_path(path);
        url.to_string()
    }

    pub async fn get(
        &self,
        subject: &str,
        path: &str,
        query: &[(&str, &str)],
        roster_hash: Option<&str>,
    ) -> Result<Value> {
        self.request(subject, Method::GET, path, query, None, roster_hash)
            .await
    }

    pub async fn post(
        &self,
        subject: &str,
        path: &str,
        body: &Value,
        roster_hash: Option<&str>,
    ) -> Result<Value> {
        self.request(subject, Method::POST, path, &[], Some(body), roster_hash)
            .await
    }

    /// The bot itself, not a person: PuckLab accepts this subject only on the
    /// read-only service routes it lists, such as the leaderboards.
    pub async fn service_get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        self.request(SERVICE_SUBJECT, Method::GET, path, query, None, None)
            .await
    }

    /// A page anyone may read, sent without an actor proof.
    pub async fn public_get(&self, path: &str) -> Result<Value> {
        self.send(None, Method::GET, path, &[], None, None).await
    }

    async fn request(
        &self,
        subject: &str,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
        roster_hash: Option<&str>,
    ) -> Result<Value> {
        self.send(Some(subject), method, path, query, body, roster_hash)
            .await
    }

    async fn send(
        &self,
        subject: Option<&str>,
        method: Method,
        path: &str,
        query: &[(&str, &str)],
        body: Option<&Value>,
        roster_hash: Option<&str>,
    ) -> Result<Value> {
        ensure!(
            path.starts_with("/v1/")
                && !path.contains(['?', '#', '\\'])
                && !path.split('/').any(|p| matches!(p, "." | "..")),
            "Invalid PuckLab API path"
        );
        let mut url = self.base.clone();
        url.set_path(path);
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.iter().copied());
        }
        let target = match url.query() {
            Some(query) => format!("{}?{query}", url.path()),
            None => url.path().to_owned(),
        };
        let bytes = body
            .map(serde_json::to_vec)
            .transpose()?
            .unwrap_or_default();
        let auth = subject
            .map(|subject| proof(&self.secret, subject, method.as_str(), &target, &bytes))
            .transpose()?;
        let mut request = self
            .http
            .request(method, url)
            .header("X-PuckLab-Build", self.build)
            .header("X-PuckLab-Protocol", self.protocol)
            .header("Accept", "application/json");
        if let Some(auth) = auth {
            request = request.header("Authorization", auth);
        }
        if let Some(hash) = roster_hash {
            request = request.header("X-Roster-Hash", hash);
        }
        if body.is_some() {
            request = request
                .header("Content-Type", "application/json")
                .body(bytes);
        }
        let mut response = request.send().await?;
        let status = response.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            ensure!(
                bytes.len() + chunk.len() <= 8 * 1024 * 1024,
                "PuckLab response is too large"
            );
            bytes.extend_from_slice(&chunk);
        }
        if !status.is_success() {
            if let Ok(mut error) = serde_json::from_slice::<ApiError>(&bytes) {
                error.status = status.as_u16();
                return Err(error.into());
            }
            anyhow::bail!("PuckLab service returned {status}");
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn roster_hash(&self, subject: &str, roster: &str) -> Result<String> {
        let listing = self.get(subject, "/v1/rosters", &[], None).await?;
        listing["rosters"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["id"] == roster))
            .and_then(|row| row["hash"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                anyhow::anyhow!("This PuckLab service does not have the configured NHL roster")
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_rejects_public_plaintext_embedded_credentials_and_weak_secrets() {
        let secret = "a".repeat(32);
        for base in [
            "http://example.com",
            "https://user:pass@example.com",
            "https://example.com/other",
            "https://example.com?secret=yes",
            "wss://example.com",
        ] {
            assert!(PuckLab::new(base, secret.clone(), 1, 1).is_err(), "{base}");
        }
        assert!(PuckLab::new("http://127.0.0.1:1234", secret.clone(), 1, 1).is_ok());
        assert!(PuckLab::new("https://pucklab.example.com", "weak".into(), 1, 1).is_err());
        assert!(
            !format!(
                "{:?}",
                PuckLab::new("https://pucklab.example.com", secret.clone(), 1, 1).unwrap()
            )
            .contains(&secret)
        );
    }

    #[test]
    fn links_use_the_site_and_requests_keep_the_service() {
        let secret = "s".repeat(32);
        let api = PuckLab::new("https://beta.pucklab.example.com", secret.clone(), 1, 1).unwrap();
        assert_eq!(
            api.page("/account"),
            "https://beta.pucklab.example.com/account"
        );
        let api = api.with_site("https://pucklab.example.com").unwrap();
        assert_eq!(api.page("/account"), "https://pucklab.example.com/account");
        assert_eq!(api.origin(), "https://beta.pucklab.example.com/");
        let api = PuckLab::new("https://beta.pucklab.example.com", secret, 1, 1).unwrap();
        for site in [
            "http://pucklab.example.com",
            "https://pucklab.example.com/x",
            "https://u@pucklab.example.com",
        ] {
            assert!(api.clone().with_site(site).is_err(), "{site}");
        }
    }
}
