use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found")]
    WorkflowNotFound,

    #[error("Workflow instance not found")]
    WorkflowInstanceNotFound,

    #[error("Node not found")]
    NodeNotFound,

    #[error("Action not found")]
    ActionNotFound,

    #[error("Server action not found: {0}")]
    ServerActionNotFound(String),

    #[error("{0}")]
    WorkflowUnavailable(String),

    #[error("{0}")]
    ServerActionFailed(String),

    #[error("Workflow already completed")]
    WorkflowAlreadyCompleted,

    #[error("Invalid state")]
    InvalidState,

    /// Another process changed the instance after this one read it.
    #[error("Workflow instance {0} was changed by another request; try again")]
    ConcurrentModification(String),
}

pub type AppResult<T> = Result<T, WorkflowError>;
