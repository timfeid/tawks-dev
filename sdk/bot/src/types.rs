use serde::{Deserialize, Serialize};
use specta::Type;
use workflow::service::{WorkflowResource, WorkflowResourceWithContext};

#[derive(Clone, Serialize, Deserialize)]
pub struct BotOptions {
    pub rspc_url: String,
    pub events_ws_url: String,
    pub token: String,
    pub token_type: Option<String>,
}

impl BotOptions {
    /// Reads RSPC_URL, EVENTS_WS_URL, BOT_TOKEN and BOT_TOKEN_TYPE (default `Bot`).
    pub fn from_env() -> anyhow::Result<Self> {
        let var = |key: &str| std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"));
        Ok(Self {
            rspc_url: var("RSPC_URL")?,
            events_ws_url: var("EVENTS_WS_URL")?,
            token: var("BOT_TOKEN")?,
            token_type: Some(std::env::var("BOT_TOKEN_TYPE").unwrap_or_else(|_| "Bot".to_string())),
        })
    }
}

impl std::fmt::Debug for BotOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotOptions")
            .field("rspc_url", &self.rspc_url)
            .field("events_ws_url", &self.events_ws_url)
            .field("token", &"<redacted>")
            .field("token_type", &self.token_type)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Connect,
    Disconnect,
    Message { message: MessageEvent },
    Error { error: String },
}

pub type CommandResult<T> = Result<T, anyhow::Error>;

/// What tawks pushes to a bot over the events socket. Bots act on server
/// action requests and chat messages; anything else arrives as `Other`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageEvent {
    WorkflowRequestServerAction {
        id: String,
        workflow: WorkflowResourceWithContext,
        action_id: String,
    },
    WorkflowCreated {
        workflow: WorkflowResource,
    },
    WorkflowUpdated {
        workflow: WorkflowResource,
    },
    ChatMessage {
        message: ChatMessage,
        chat_id: String,
    },
    Error {
        message: String,
    },
    #[serde(other)]
    Other,
}

/// A message posted in a chat the bot can see.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub user: UserResource,
    pub content: String,
    pub chat_id: String,
    #[serde(default)]
    pub parent_message_id: Option<String>,
    #[serde(default)]
    pub timestamp: String,
}

#[derive(Clone, Hash, PartialEq, Eq, Type, Deserialize, Serialize, Debug)]
pub struct UserResource {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: String,
    pub db_score: i32,
    #[serde(default)]
    pub is_bot: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SpeakInLobbyArgs {
    pub text: String,
    pub lobby_id: String,
}

#[cfg(test)]
mod tests {
    use super::MessageEvent;

    #[test]
    fn chat_messages_parse_and_unknown_events_fall_through() {
        let chat = r#"{"type":"chat_message","chat_id":"c1","message":{"id":"m1","chat_id":"c1","content":"!ping",
            "user":{"id":"u1","username":"ana","display_name":"Ana","avatar_url":"","db_score":0},
            "attachments":[],"embeddings":[],"timestamp":"2026-10-24T00:00:00Z","revision":0,"deleted":false}}"#;
        match serde_json::from_str::<MessageEvent>(chat).unwrap() {
            MessageEvent::ChatMessage { message, chat_id } => {
                assert_eq!(chat_id, "c1");
                assert_eq!(message.content, "!ping");
                assert!(!message.user.is_bot);
            }
            other => panic!("expected a chat message, got {other:?}"),
        }
        let typing = r#"{"type":"chat_typing","user_id":"u1","chat_id":"c1"}"#;
        assert!(matches!(serde_json::from_str::<MessageEvent>(typing).unwrap(), MessageEvent::Other));
    }
}
