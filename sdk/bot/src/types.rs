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
/// action requests; anything else arrives as `Other`.
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
    Error {
        message: String,
    },
    #[serde(other)]
    Other,
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
