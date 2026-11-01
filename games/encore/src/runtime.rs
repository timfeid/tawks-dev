//! The lobby entry: a join action that hands each player a pass and the
//! addresses of their lobby's room, and the activity frame that runs the game.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde_json::json;
use tawks_bot::types::UserResource;
use tawks_bot::{Activity, ServerActionHandler, TawksBot, full_avatar_url};
use workflow::CreateWorkflowDefinition;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow::server_action::ServerActionResult;

use crate::server::EncoreServer;

pub const WORKFLOW_ID: &str = "encore";
pub const JOIN_ACTION: &str = "games.encore.join";

pub fn definition(join_action_id: &str) -> CreateWorkflowDefinition {
    let join = WorkflowActionBuilder::new(JOIN_ACTION)
        .run_server_action()
        .allow_while_loading()
        .target(join_action_id)
        .build();
    let node = WorkflowNodeBuilder::new("Encore")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("stage")
                .tile(vec![
                    WorkflowDisplayBuilder::new()
                        .activity("encore", JOIN_ACTION)
                        .build(),
                ])
                .build(),
        )
        .build();
    WorkflowDefinitionBuilder::new(WORKFLOW_ID, "Encore")
        .description("Plug in a guitar or drums and play songs together as a band.")
        .starting_server_action(join_action_id)
        .response("encore", json!(null))
        .add_actions(vec![join])
        .add_node("stage", node)
        .build()
}

/// Registers the game when `ENCORE_ACTIVITY_URL` says where its client is.
pub async fn register(bot: &TawksBot, server: &EncoreServer) -> Result<()> {
    server.set_bot(bot.sdk.clone());
    let Ok(client_url) = std::env::var("ENCORE_ACTIVITY_URL") else {
        tracing::info!("ENCORE_ACTIVITY_URL is not set; Encore is not in the lobby menu");
        return Ok(());
    };
    let join_id = bot
        .register_server_action(
            JOIN_ACTION,
            join_handler(server, bot.sdk.web_client.clone()),
        )
        .await?;
    let workflow_id = bot.register_workflow(definition(&join_id)).await?;
    bot.register_activity(
        &Activity::new(workflow_id, "Encore", client_url)
            .source("https://github.com/timfeid/tawks-dev/tree/main/games/encore")
            .description("Plug in a guitar or drums and play songs together")
            .icon("guitar")
            .players(1, 8),
    )
    .await?;
    Ok(())
}

fn join_handler(
    server: &EncoreServer,
    web: Arc<tawks_bot::web_client::WebClient>,
) -> ServerActionHandler {
    let server = server.clone();
    Arc::new(Box::new(move |ctx| {
        let server = server.clone();
        let web = web.clone();
        Box::pin(async move {
            let user: UserResource = ctx
                .ctx_required_as("$user")
                .map_err(|e| anyhow!(e.to_string()))?;
            if ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false) {
                return Err(anyhow!("Sign in to play Encore."));
            }
            let lobby_id = ctx
                .get_input_as::<String>("lobby_id")
                .unwrap_or_else(|_| ctx.workflow.instance_id.clone());
            // The lobby id comes from the frame, and a pass is what lets someone
            // hear the uploads of the people in that lobby's room.
            let allowed: bool = web
                .query(
                    "encore_may_join",
                    &json!({ "lobby_id": lobby_id, "user_id": ctx.workflow.user_id }),
                )
                .await
                .unwrap_or(false);
            if !allowed {
                return Err(anyhow!("You are not in that lobby"));
            }
            let name = if user.display_name.trim().is_empty() {
                user.username.clone()
            } else {
                user.display_name.clone()
            };
            let token = server.issue_token(
                &lobby_id,
                &ctx.workflow.user_id,
                &name,
                &full_avatar_url(&user),
                ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false),
            );
            let mut responses = HashMap::new();
            responses.insert(
                "encore".to_string(),
                json!({ "ws_url": server.ws_url(), "api_url": server.api_url(), "token": token, "lobby_id": lobby_id }),
            );
            Ok(ServerActionResult::UpdateResponses(responses))
        })
    }))
}
