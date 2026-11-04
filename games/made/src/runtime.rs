//! The `made_join` action every made game's workflow starts with. The API
//! registers those workflows; this side only answers the action.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use made_protocol::MadeJoin;
use serde::Deserialize;
use serde_json::json;
use tawks_bot::types::UserResource;
use tawks_bot::{ServerActionHandler, TawksBot};
use workflow::server_action::ServerActionResult;

use crate::server::MadeServer;
use crate::token::Claims;

pub const JOIN_ACTION: &str = "games.made.join";

#[derive(Deserialize)]
struct MayJoin {
    activity_id: String,
    version: i32,
    seat: Option<u8>,
    owner: bool,
}

pub async fn register(bot: &TawksBot, server: &MadeServer) -> Result<()> {
    server.set_bot(bot.sdk.web_client.clone());
    bot.register_server_action(
        JOIN_ACTION,
        join_handler(server, bot.sdk.web_client.clone()),
    )
    .await?;
    Ok(())
}

fn join_handler(
    server: &MadeServer,
    web: Arc<tawks_bot::web_client::WebClient>,
) -> ServerActionHandler {
    let server = server.clone();
    Arc::new(Box::new(move |ctx| {
        let server = server.clone();
        let web = web.clone();
        Box::pin(async move {
            if ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false) {
                return Err(anyhow!("Sign in to play."));
            }
            let user: UserResource = ctx
                .ctx_required_as("$user")
                .map_err(|e| anyhow!(e.to_string()))?;
            let lobby_id = ctx
                .get_input_as::<String>("lobby_id")
                .map_err(|_| anyhow!("Open this game from a lobby."))?;
            // The lobby id comes from the client, so the API checks the caller
            // is in that lobby and that the lobby plays this very game.
            let allowed: Option<MayJoin> = web
                .query(
                    "made_may_join",
                    &json!({ "lobby_id": lobby_id, "user_id": ctx.workflow.user_id, "workflow_id": ctx.workflow.workflow_id }),
                )
                .await
                .map_err(|e| anyhow!("Could not check the lobby: {e}"))?;
            let Some(allowed) = allowed else {
                return Err(anyhow!(
                    "This game is not on in that lobby, or you are not in it."
                ));
            };
            let name = if user.display_name.trim().is_empty() {
                user.username.clone()
            } else {
                user.display_name.clone()
            };
            let token = server.issue(Claims {
                lobby_id: lobby_id.clone(),
                activity_id: allowed.activity_id.clone(),
                version: allowed.version,
                seat: allowed.seat,
                user_id: ctx.workflow.user_id.clone(),
                name,
                owner: allowed.owner,
                exp: 0,
            });
            let made = MadeJoin {
                ws_url: server.ws_url().to_string(),
                token,
                lobby_id: lobby_id.clone(),
                activity_id: allowed.activity_id,
                version: allowed.version,
                seat: allowed.seat,
            };
            let mut responses = HashMap::new();
            responses.insert("made".to_string(), serde_json::to_value(made)?);
            // The made canvas asks for a fresh pass with this when its pass stops working.
            responses.insert("lobby_id".to_string(), json!(lobby_id));
            Ok(ServerActionResult::UpdateResponses(responses))
        })
    }))
}
