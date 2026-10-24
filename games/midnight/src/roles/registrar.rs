use anyhow::Result;
use workflow::CreateWorkflowDefinition;
use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;

use crate::gamestate::GameState;

/// Boxed state handler so RoleRegistrar is object-safe
pub type BoxedStateHandler = Box<
    dyn Fn(&mut GameState, WorkflowResourceWithContext) -> Result<ServerActionResult> + Send + Sync,
>;

/// Object-safe interface roles use to register engine integrations
pub trait RoleRegistrar: Send + Sync {
    fn register_state_handler_boxed(&self, action_name: &str, handler: BoxedStateHandler);
    fn register_workflow_definition(&self, def: CreateWorkflowDefinition);
}
