//! The lobby entry: a join action that hands each player a pass for their
//! lobby's drive, and the activity in the game menu.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tawks_bot::types::UserResource;
use tawks_bot::web_client::WebClient;
use tawks_bot::{Activity, ServerActionHandler, TawksBot, full_avatar_url};
use workflow::CreateWorkflowDefinition;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use workflow::server_action::ServerActionResult;

use crate::server::OutroadsServer;

pub const WORKFLOW_ID: &str = "outroads";
pub const JOIN_ACTION: &str = "games.outroads.join";
const SOURCE_URL: &str = "https://github.com/timfeid/tawks-dev/tree/main/games/outroads";
/// The first-party lobby check: the lobby exists, its hub is open to the
/// person, they have an account and may join.
const MAY_JOIN_QUERY: &str = "encore_may_join";

pub fn definition(join_action_id: &str) -> CreateWorkflowDefinition {
    let join = WorkflowActionBuilder::new(JOIN_ACTION)
        .run_server_action()
        .allow_while_loading()
        .target(join_action_id)
        .build();
    let node = WorkflowNodeBuilder::new("Outroads")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("drive")
                .tile(vec![
                    WorkflowDisplayBuilder::new().activity("outroads", JOIN_ACTION).build(),
                ])
                .build(),
        )
        .build();
    WorkflowDefinitionBuilder::new(WORKFLOW_ID, "Outroads")
        .description("Drive real streets anywhere in the world with your lobby.")
        .starting_server_action(join_action_id)
        .response("outroads", json!(null))
        .add_actions(vec![join])
        .add_node("drive", node)
        .build()
}

/// Registers the drive when `OUTROADS_ACTIVITY_URL` says where its client is.
pub async fn register(bot: &TawksBot, server: &OutroadsServer) -> Result<()> {
    let Ok(client_url) = std::env::var("OUTROADS_ACTIVITY_URL") else {
        tracing::info!("OUTROADS_ACTIVITY_URL is not set; Outroads is not in the lobby menu");
        return Ok(());
    };
    let join_id = bot
        .register_server_action(JOIN_ACTION, join_handler(server, bot.sdk.web_client.clone()))
        .await?;
    let workflow_id = bot.register_workflow(definition(&join_id)).await?;
    bot.register_activity(
        &Activity::new(workflow_id, "Outroads", client_url)
            .source(SOURCE_URL)
            .description("Drive real streets anywhere in the world with your lobby")
            .icon("car")
            .players(1, 16),
    )
    .await?;
    Ok(())
}

fn display_name(user: &UserResource) -> String {
    if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.clone()
    }
}

fn join_handler(server: &OutroadsServer, web: Arc<WebClient>) -> ServerActionHandler {
    let server = server.clone();
    Arc::new(Box::new(move |ctx| {
        let server = server.clone();
        let web = web.clone();
        Box::pin(async move {
            if ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false) {
                return Err(anyhow!("Sign in to drive."));
            }
            let user: UserResource = ctx.ctx_required_as("$user").map_err(|e| anyhow!(e.to_string()))?;
            let lobby_id: String = ctx
                .get_input_as("lobby_id")
                .map_err(|_| anyhow!("Open Outroads from a lobby."))?;
            let user_id = ctx.workflow.user_id.clone();
            let allowed: bool = web
                .query(MAY_JOIN_QUERY, &json!({ "lobby_id": lobby_id, "user_id": user_id }))
                .await
                .unwrap_or(false);
            if !allowed {
                return Err(anyhow!("You are not in that lobby"));
            }
            let owner = web
                .query::<_, Value>("lobby_find", &lobby_id)
                .await
                .ok()
                .and_then(|lobby| lobby.get("owner_user_id")?.as_str().map(|owner| owner == user_id))
                .unwrap_or(false);
            let token = server.issue_pass(
                &lobby_id,
                &user_id,
                &display_name(&user),
                &full_avatar_url(&user),
                owner,
            );
            let mut responses = HashMap::new();
            responses.insert(
                "outroads".to_string(),
                json!({ "ws_url": server.ws_url(), "token": token, "lobby_id": lobby_id }),
            );
            Ok(ServerActionResult::UpdateResponses(responses))
        })
    }))
}
