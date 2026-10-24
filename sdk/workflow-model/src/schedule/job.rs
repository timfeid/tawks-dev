use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use crate::NodeCondition;

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub enum ScheduledAction {
    RunServerAction {
        action_id: String,
        payload: Value,
    },

    UpdateResponses {
        updates: std::collections::HashMap<String, Value>,
        reconcile_node: bool,
    },

    TransformResponses {
        ops: Vec<crate::TransformOp>,
        reconcile_node: bool,
    },
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub idempotency_key: String,
    pub after_ms: u32,
    pub guard: Option<NodeCondition>,
    pub action: ScheduledAction,
}

impl ScheduledJob {
    pub fn new(idempotency_key: impl Into<String>, after_ms: u32, action: ScheduledAction) -> Self {
        Self {
            idempotency_key: idempotency_key.into(),
            after_ms,
            guard: None,
            action,
        }
    }

    pub fn guard(mut self, guard: NodeCondition) -> Self {
        self.guard = Some(guard);
        self
    }
}
