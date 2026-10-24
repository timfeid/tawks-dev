use std::collections::HashMap;

use anyhow::Result;
use workflow::service::StartWorkflowFromInstanceArgs;

pub mod engine;

pub trait GameLogic: Sized + Send + Sync + 'static {
    type Action: serde::de::DeserializeOwned + Send;
    type View: serde::Serialize + Send + Clone;
    type Effect: std::fmt::Debug + Send + Sync;

    type InitArgs: Send + Sync + Clone + 'static;

    fn get_instance_id_for_player_id(&self, _player_id: &str) -> Option<String> {
        None
    }
    fn record_instance_id(&mut self, _player_id: &str, _instance_id: &str) {}
    fn should_fanout(&self, _action: &Self::Action) -> bool {
        true
    }
    fn update(&mut self, player_id: &str, action: Self::Action) -> Result<Vec<Self::Effect>>;
    fn new(args: Self::InitArgs) -> Self;
    /// Called once, right after `new`, with the lobby this game belongs to.
    fn lobby_started(&mut self, _lobby_id: &str) {}
    fn view_for(&self, player_id: &str) -> Self::View;
    fn tick(&mut self) -> Vec<Self::Action> {
        vec![]
    }

    /// Everyone the engine sends a view to after an action.
    fn get_player_ids(&self) -> Vec<String>;
}

#[derive(Debug, Clone)]
pub enum GameEffect {
    ClearWorkflow {
        user_id: String,
        workflow_id: String, // e.g. "bound_workflow"
        attach_as: String,
    },
    CompleteWorkflow {
        user_id: String,
        message: String,
        responses: HashMap<String, serde_json::Value>,
        close_active_modal: bool,
    },
    InjectWorkflow(StartWorkflowFromInstanceArgs),
    RunServerAction {
        instance_id: String,
        action_id: String,
        inputs: HashMap<String, serde_json::Value>,
    },
    EndGameCleanup {
        user_id: String,
    },
    Speak {
        text: String,
    },
    MilestoneEvent {
        game_id: String,
        event_key: String,
        user_id: String,
        lobby_id: String,
        meta: HashMap<String, serde_json::Value>,
    },
}
