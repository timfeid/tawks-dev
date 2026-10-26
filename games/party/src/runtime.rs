use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde_json::json;
use tawks_bot::types::UserResource;
use tawks_bot::{ServerActionHandler, TawksBot, full_avatar_url};
use workflow::server_action::ServerActionResult;

use crate::server::PartyServer;
use crate::workflows::{self, COVEN_JOIN_ACTION, JOIN_ACTION, LANDLORD_JOIN_ACTION};

/// Registers the join actions and the party and Coven workflows. Each action
/// hands a player the socket address and a signed pass for their lobby's room;
/// the Coven pass also names the game the room should be set to.
pub async fn register(bot: &TawksBot, server: &PartyServer) -> Result<()> {
    let join_id = bot.register_server_action(JOIN_ACTION, join_handler(server, None)).await?;
    let coven_id = bot
        .register_server_action(COVEN_JOIN_ACTION, join_handler(server, Some(crate::coven::KEY)))
        .await?;
    let landlord_id = bot
        .register_server_action(LANDLORD_JOIN_ACTION, join_handler(server, Some(crate::landlord::KEY)))
        .await?;
    workflows::register(bot, &join_id, &coven_id, &landlord_id).await
}

fn join_handler(server: &PartyServer, game: Option<&'static str>) -> ServerActionHandler {
    let server = server.clone();
    Arc::new(Box::new(move |ctx| {
        let server = server.clone();
        Box::pin(async move {
                let user: UserResource = ctx
                    .ctx_required_as("$user")
                    .map_err(|e| anyhow!(e.to_string()))?;
                let lobby_id = ctx
                    .get_input_as::<String>("lobby_id")
                    .unwrap_or_else(|_| ctx.workflow.instance_id.clone());
                let name = if user.display_name.trim().is_empty() {
                    user.username.clone()
                } else {
                    user.display_name.clone()
                };
                let token = server.issue_token_for(
                    &lobby_id,
                    &ctx.workflow.user_id,
                    &name,
                    &full_avatar_url(&user),
                    game,
                    ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false),
                );
                let mut responses = HashMap::new();
                responses.insert(
                    "party".to_string(),
                    json!({ "ws_url": server.public_url(), "token": token, "match_id": lobby_id }),
                );
                Ok(ServerActionResult::UpdateResponses(responses))
            })
        }))
}
