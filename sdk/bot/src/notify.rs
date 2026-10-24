use anyhow::Result;
use serde::Serialize;

use crate::web_client::WebClient;

/// A notification kind a channel declares so users can opt in or out of it.
#[derive(Debug, Clone, Serialize)]
pub struct KindInput {
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    pub kind: String,
    pub default_state: String,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
struct RegisterKindsArgs<'a> {
    channel_id: &'a str,
    kinds: &'a [KindInput],
}

#[derive(Debug, Serialize)]
struct SubscribersArgs<'a> {
    channel_id: &'a str,
    event_key: &'a str,
}

#[derive(Debug, Serialize)]
struct SendArgs<'a> {
    channel_id: &'a str,
    event_key: &'a str,
    dedupe_key: &'a str,
    user_ids: &'a [String],
    title: &'a str,
    body: Option<&'a str>,
    url: Option<&'a str>,
    icon: Option<&'a str>,
}

/// Sends channel notifications through the bot notification procedures.
pub struct Notifier {
    client: WebClient,
    channel_id: String,
    channel_url: String,
    kinds: Vec<KindInput>,
}

impl Notifier {
    pub fn new(
        rspc_url: &str,
        auth_header: &str,
        channel_id: impl Into<String>,
        channel_url: impl Into<String>,
        kinds: Vec<KindInput>,
    ) -> Result<Self> {
        Ok(Self {
            client: WebClient::new(rspc_url, auth_header)?,
            channel_id: channel_id.into(),
            channel_url: channel_url.into(),
            kinds,
        })
    }

    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    pub fn client(&self) -> &WebClient {
        &self.client
    }

    pub fn channel_url(&self) -> &str {
        &self.channel_url
    }

    pub async fn register_kinds(&self) -> Result<usize> {
        self.client
            .mutate(
                "bot_notifications_register_kinds",
                &RegisterKindsArgs {
                    channel_id: &self.channel_id,
                    kinds: &self.kinds,
                },
            )
            .await
    }

    pub async fn subscribers(&self, event_key: &str) -> Result<Vec<String>> {
        self.client
            .query(
                "bot_notifications_subscribers",
                &SubscribersArgs {
                    channel_id: &self.channel_id,
                    event_key,
                },
            )
            .await
    }

    pub async fn send(
        &self,
        event_key: &str,
        dedupe_key: &str,
        user_ids: &[String],
        title: &str,
        body: Option<&str>,
        url: Option<&str>,
    ) -> Result<usize> {
        self.send_with_icon(event_key, dedupe_key, user_ids, title, body, url, None)
            .await
    }

    /// `icon` is a push icon URL; `Some("none")` shows no icon at all.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_with_icon(
        &self,
        event_key: &str,
        dedupe_key: &str,
        user_ids: &[String],
        title: &str,
        body: Option<&str>,
        url: Option<&str>,
        icon: Option<&str>,
    ) -> Result<usize> {
        if user_ids.is_empty() {
            return Ok(0);
        }
        self.client
            .mutate(
                "bot_notifications_send",
                &SendArgs {
                    channel_id: &self.channel_id,
                    event_key,
                    dedupe_key,
                    user_ids,
                    title,
                    body,
                    url,
                    icon,
                },
            )
            .await
    }
}
