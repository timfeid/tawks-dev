use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, instrument, warn};

#[derive(Debug, Deserialize)]
struct JsonRpcEnvelope {
    #[serde(rename = "jsonrpc")]
    _jsonrpc: String,
    result: RspcInner,
}

#[derive(Debug, Deserialize)]
struct RspcInner {
    #[serde(rename = "type")]
    kind: String,
    data: Value,
}

pub fn parse_rspc<T: DeserializeOwned>(text: &str) -> anyhow::Result<T> {
    // Try JSON-RPC envelope first
    if let Ok(env) = serde_json::from_str::<JsonRpcEnvelope>(text) {
        if env.result.kind != "response" {
            anyhow::bail!(
                "rspc {} response: {}",
                env.result.kind,
                summarize_rspc_data(&env.result.data)
            );
        }
        return Ok(serde_json::from_value::<T>(env.result.data)?);
    }

    // Fallback: raw rspc { type, data }
    if let Ok(inner) = serde_json::from_str::<RspcInner>(text) {
        if inner.kind != "response" {
            anyhow::bail!(
                "rspc {} response: {}",
                inner.kind,
                summarize_rspc_data(&inner.data)
            );
        }
        return Ok(serde_json::from_value::<T>(inner.data)?);
    }

    anyhow::bail!("unexpected RSPC response: {}", text);
}

fn summarize_rspc_data(data: &Value) -> String {
    if let Some(msg) = data.get("message").and_then(|v| v.as_str()) {
        return msg.to_string();
    }
    if let Some(s) = data.as_str() {
        return s.to_string();
    }
    data.to_string()
}

/// Minimal RSPC HTTP client supporting `mutate` calls.
pub struct WebClient {
    client: Client,
    pub base_url: String,
    auth: String,
}

impl std::fmt::Debug for WebClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebClient").field("base_url", &self.base_url).finish_non_exhaustive()
    }
}

/// A bot token must not cross the internet in the clear. Plain http is for
/// this machine, a private network, or a container name on a Docker network.
pub fn require_tls(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url).map_err(|e| anyhow::anyhow!("invalid URL {url}: {e}"))?;
    let private = match parsed.host() {
        Some(url::Host::Domain(name)) => name == "localhost" || !name.contains('.'),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    match parsed.scheme() {
        "https" | "wss" => Ok(()),
        "http" | "ws" if private => Ok(()),
        scheme => anyhow::bail!("refusing to send the bot token over {scheme}:// to a public host; use https or wss"),
    }
}


impl WebClient {
    #[instrument(skip(base_url, auth_header))]
    pub fn new(base_url: impl Into<String>, auth_header: impl Into<String>) -> Result<Self> {
        let base_url = base_url.into();
        require_tls(&base_url)?;
        let client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        Ok(Self {
            client,
            base_url,
            auth: auth_header.into(),
        })
    }

    #[instrument(skip(self, input))]
    pub async fn mutate<TIn: Serialize + std::fmt::Debug, TOut: DeserializeOwned>(
        &self,
        path: &str,
        input: &TIn,
    ) -> Result<TOut> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        debug!(
            "Sending mutate request to URL: {} \n\ninput:\n{:?}\n\n",
            url, input
        );

        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth.clone())
            .header("content-type", "application/json")
            .json(&input)
            .send()
            .await?;

        let text = resp.text().await?;
        if path == "game_session_launch" {
            debug!("Received private game launch response (redacted)");
        } else {
            debug!("Received response: {}", text);
        }
        match parse_rspc(&text) {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!("RSPC error response: {:?}", e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self, input))]
    pub async fn query<TIn: Serialize + std::fmt::Debug, TOut: DeserializeOwned>(
        &self,
        path: &str,
        input: &TIn,
    ) -> Result<TOut> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        debug!(
            "Sending query request to URL: {} \n\ninput:\n{:?}\n\n",
            url, input
        );

        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, self.auth.clone())
            .header("content-type", "application/json")
            .json(&input)
            .send()
            .await?;

        let text = resp.text().await?;
        debug!("Received response: {}", text);

        match parse_rspc(&text) {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!("RSPC error response: {:?}", e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self))]
    pub async fn fetch_access_token(&self) -> anyhow::Result<String> {
        let url = format!("{}/auth_access_token", self.base_url.trim_end_matches('/'));
        let auth_value = self.auth.clone();

        debug!("Fetching access token from URL: {}", url);

        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, &auth_value)
            .send()
            .await
            .map_err(anyhow::Error::from)?;
        let status = resp.status();
        let text = resp.text().await?;

        debug!(%status, "Access token response received");

        let v: Value = serde_json::from_str(&text)
            .map_err(|_| anyhow::anyhow!("access token request failed with {status}"))?;
        v.pointer("/result/data/access_token")
            .or_else(|| v.get("access_token"))
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .ok_or_else(|| anyhow::anyhow!("access token request failed with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::require_tls;

    #[test]
    fn tokens_only_travel_in_the_clear_on_private_networks() {
        assert!(require_tls("https://rspc.tawks.app/rspc").is_ok());
        assert!(require_tls("http://localhost:8787/rspc").is_ok());
        assert!(require_tls("http://tawks-rspc:3000/rspc").is_ok());
        assert!(require_tls("ws://10.0.0.5:8085/").is_ok());
        assert!(require_tls("http://rspc.tawks.app/rspc").is_err());
        assert!(require_tls("ws://203.0.113.9:8085/").is_err());
    }
}
