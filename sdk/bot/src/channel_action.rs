use serde::Serialize;

use crate::TawksBot;

/// A button on a channel's row in the sidebar, shown while the bot is online.
/// It opens a link or runs one of the bot's commands in that channel.
#[derive(Debug, Clone, Serialize)]
pub struct ChannelAction {
    pub channel_id: String,
    pub key: String,
    pub label: String,
    pub icon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i32>,
}

impl ChannelAction {
    /// Opens `url`: a path in the app such as `/niche/hockey/tonight`, or an https link.
    pub fn link(
        channel_id: impl Into<String>,
        key: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self::new(channel_id, key, label, icon, Some(url.into()), None)
    }

    /// Runs `command` in the channel as if it were typed, such as `/standings`.
    pub fn command(
        channel_id: impl Into<String>,
        key: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self::new(channel_id, key, label, icon, None, Some(command.into()))
    }

    fn new(
        channel_id: impl Into<String>,
        key: impl Into<String>,
        label: impl Into<String>,
        icon: impl Into<String>,
        url: Option<String>,
        command: Option<String>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            key: key.into(),
            label: label.into(),
            icon: icon.into(),
            url,
            command,
            position: None,
            expires_in: None,
        }
    }

    pub fn position(mut self, position: i32) -> Self {
        self.position = Some(position);
        self
    }

    /// Removes the action on its own after this many seconds.
    pub fn expires_in(mut self, seconds: i32) -> Self {
        self.expires_in = Some(seconds);
        self
    }
}

impl TawksBot {
    /// Adds or updates a channel action; the same channel and key replace it.
    pub async fn set_channel_action(&self, action: &ChannelAction) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, serde_json::Value>("channel_action_set", action)
            .await?;
        Ok(())
    }

    pub async fn remove_channel_action(&self, channel_id: &str, key: &str) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, serde_json::Value>(
                "channel_action_remove",
                &serde_json::json!({ "channel_id": channel_id, "key": key }),
            )
            .await?;
        Ok(())
    }
}
