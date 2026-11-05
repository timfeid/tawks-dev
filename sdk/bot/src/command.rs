use serde::Serialize;
use serde_json::json;

use crate::{TawksBot, TawksSdk};
use workflow::{InputType, WorkflowInput};

/// A slash command in the chat composer, such as `/roll 1d20`. Running it
/// starts `workflow_id` with the text after the command as the `text` input,
/// and the chat it was typed in as `chat_id` in the context.
#[derive(Debug, Clone, Serialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command: String,
    pub inputs: Vec<WorkflowInput>,
    pub workflow_id: String,
    pub icon: Option<String>,
}

impl Command {
    /// `command` is what people type after the slash; `name` describes it.
    pub fn new(command: impl Into<String>, name: impl Into<String>, workflow_id: impl Into<String>) -> Self {
        let command = command.into();
        Self {
            id: command.clone(),
            name: name.into(),
            command,
            inputs: Vec::new(),
            workflow_id: workflow_id.into(),
            icon: None,
        }
    }

    /// Shown after the command in the composer, such as `formula`.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.inputs = vec![WorkflowInput {
            readonly: None,
            r#type: InputType::Text {
                placeholder: Some(text.into()),
            },
            default_value: None,
            required: false,
            on_change_action: None,
        }];
        self
    }

    /// A Lucide icon name.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl TawksBot {
    /// Registering again updates the command.
    pub async fn register_command(&self, command: &Command) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, serde_json::Value>("command_register", command)
            .await?;
        tracing::info!("Registered command /{}", command.command);
        Ok(())
    }
}

impl TawksSdk {
    /// Lights up the lobby stage tiles of the people whose turn it is, with
    /// an optional short label such as a character's name. An empty list
    /// clears it. Only the bot running the lobby's game may set it.
    pub async fn set_lobby_turn(&self, lobby_id: &str, user_ids: &[String], label: Option<&str>) -> anyhow::Result<()> {
        self.web_client
            .mutate::<_, serde_json::Value>(
                "lobby_set_turn",
                &json!({ "lobby_id": lobby_id, "user_ids": user_ids, "label": label }),
            )
            .await?;
        Ok(())
    }
}
