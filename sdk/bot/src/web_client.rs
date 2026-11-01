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

/// A call tawks answered with an error, or with something that was not an
/// rspc response at all (such as a proxy's 502 page).
#[derive(Debug, Clone)]
pub struct RspcError {
    pub path: String,
    /// The HTTP status, when the call was made over HTTP.
    pub status: Option<u16>,
    /// The rspc error code: 400 for input the API could not read, 401 and
    /// 403 for auth, 404 for an unknown procedure, 500 for a failure inside it.
    pub code: Option<i32>,
    pub message: String,
}

impl RspcError {
    /// The API is up and refused this request, so retrying the same request
    /// will not help: usually the bot and the API disagree about a type.
    pub fn is_rejected(&self) -> bool {
        let client_error = |code: i64| (400..500).contains(&code);
        self.code.is_some_and(|code| client_error(code.into()))
            || self.status.is_some_and(|status| client_error(status.into()))
    }
}

impl std::fmt::Display for RspcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = if self.path.is_empty() { "rspc call" } else { &self.path };
        match (self.code, self.status) {
            (Some(code), _) => write!(f, "{path} failed with error {code}: {}", self.message),
            (None, Some(status)) => write!(f, "{path} got HTTP {status} and no rspc response: {}", self.message),
            (None, None) => write!(f, "{path} got no rspc response: {}", self.message),
        }
    }
}

impl std::error::Error for RspcError {}

pub fn parse_rspc<T: DeserializeOwned>(text: &str) -> anyhow::Result<T> {
    parse_rspc_response("", None, text)
}

fn parse_rspc_response<T: DeserializeOwned>(path: &str, status: Option<u16>, text: &str) -> anyhow::Result<T> {
    let inner = serde_json::from_str::<JsonRpcEnvelope>(text)
        .map(|env| env.result)
        .or_else(|_| serde_json::from_str::<RspcInner>(text));
    let error = |code, message| RspcError { path: path.to_string(), status, code, message };
    match inner {
        Ok(inner) if inner.kind == "response" => Ok(serde_json::from_value::<T>(inner.data)?),
        Ok(inner) => {
            let code = inner.data.get("code").and_then(Value::as_i64).and_then(|code| i32::try_from(code).ok());
            Err(error(code, summarize_rspc_data(&inner.data)).into())
        }
        Err(_) => Err(error(None, snippet(text)).into()),
    }
}

fn snippet(text: &str) -> String {
    const MAX: usize = 300;
    let text = text.trim();
    match text.char_indices().nth(MAX) {
        Some((end, _)) => format!("{}...", &text[..end]),
        None => text.to_string(),
    }
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

        let status = resp.status().as_u16();
        let text = resp.text().await?;
        if path == "game_session_launch" {
            debug!("Received private game launch response (redacted)");
        } else {
            debug!("Received response: {}", text);
        }
        match parse_rspc_response(path, Some(status), &text) {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!("{e:#}");
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

        let status = resp.status().as_u16();
        let text = resp.text().await?;
        debug!("Received response: {}", text);

        match parse_rspc_response(path, Some(status), &text) {
            Ok(res) => Ok(res),
            Err(e) => {
                warn!("{e:#}");
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
    use super::{RspcError, parse_rspc_response, require_tls};

    #[test]
    fn an_api_error_keeps_its_code_and_message() {
        let body = r#"{"jsonrpc":"2.0","id":null,"result":{"type":"error","data":{"code":400,"message":"invalid input: unknown variant `WordCloud`","data":null}}}"#;
        let err = parse_rspc_response::<String>("workflow_register_workflow", Some(400), body).unwrap_err();
        let err = err.downcast_ref::<RspcError>().expect("an RspcError");
        assert!(err.is_rejected());
        assert_eq!(err.code, Some(400));
        assert_eq!(
            err.to_string(),
            "workflow_register_workflow failed with error 400: invalid input: unknown variant `WordCloud`"
        );
    }

    #[test]
    fn a_proxy_error_page_is_an_outage_not_a_rejection() {
        let err = parse_rspc_response::<String>("workflow_register_workflow", Some(502), "<html>Bad Gateway</html>").unwrap_err();
        let err = err.downcast_ref::<RspcError>().expect("an RspcError");
        assert!(!err.is_rejected());
        assert!(err.to_string().contains("HTTP 502"), "{err}");
    }

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
