use serde::Serialize;

use crate::TawksBot;

/// A game in lobbies' game menus. The game itself is a workflow the bot
/// registered. With a `client_url`, its page shows an `activity` display that
/// runs that web app in a sandboxed frame; without one, tawks draws the
/// workflow's own displays.
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub min_players: Option<i32>,
    pub max_players: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_url: Option<String>,
    /// Where people can read the game's code; the Play page links to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

impl Activity {
    /// `workflow_id` is the id `register_workflow` returned.
    pub fn new(workflow_id: impl Into<String>, name: impl Into<String>, client_url: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            name: name.into(),
            description: None,
            icon: None,
            min_players: None,
            max_players: None,
            client_url: Some(client_url.into()),
            source_url: None,
        }
    }

    /// A game drawn by its workflow's displays, with no web app of its own.
    pub fn workflow(workflow_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            name: name.into(),
            description: None,
            icon: None,
            min_players: None,
            max_players: None,
            client_url: None,
            source_url: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A Lucide icon name, shown in the lobby's game menu.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn source(mut self, url: impl Into<String>) -> Self {
        self.source_url = Some(url.into());
        self
    }

    pub fn players(mut self, min: i32, max: i32) -> Self {
        self.min_players = Some(min);
        self.max_players = Some(max);
        self
    }
}

impl TawksBot {
    pub async fn register_activity(&self, activity: &Activity) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, serde_json::Value>("activity_register", activity)
            .await?;
        tracing::info!("Registered activity {}", activity.workflow_id);
        Ok(())
    }
}
