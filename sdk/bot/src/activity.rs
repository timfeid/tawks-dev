use serde::Serialize;

use crate::TawksBot;

/// A game whose client is a web app tawks runs in a sandboxed frame. The game
/// itself is a workflow the bot registered; its page shows an `activity`
/// display, which loads `client_url` and relays state and moves.
#[derive(Debug, Clone, Serialize)]
pub struct Activity {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub min_players: Option<i32>,
    pub max_players: Option<i32>,
    pub client_url: String,
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
            client_url: client_url.into(),
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
