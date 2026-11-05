//! The lobby entry and the chat commands: a join action that hands each
//! player a pass for their lobby's table, the activity in the game menu, and
//! `/roll` and `/init`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Result, anyhow};
use dashmap::DashMap;
use serde_json::{Value, json};
use tawks_bot::types::UserResource;
use tawks_bot::web_client::WebClient;
use tawks_bot::{Activity, Command, ServerActionHandler, TawksBot, TawksSdk, full_avatar_url};
use tokio::sync::Mutex;
use workflow::CreateWorkflowDefinition;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use workflow::server_action::ServerActionResult;

use crate::chat;
use crate::initiative::Tracker;
use crate::server::{LongtableServer, TurnSink};
use crate::store::{self, Store};
use crate::table::Turn;

pub const WORKFLOW_ID: &str = "longtable";
pub const JOIN_ACTION: &str = "games.longtable.join";
const ROLL_ACTION: &str = "games.longtable.roll";
const INIT_ACTION: &str = "games.longtable.init";
const SOURCE_URL: &str = "https://github.com/timfeid/tawks-dev/tree/main/games/longtable";
/// The first-party lobby check: the lobby exists, its hub is open to the
/// person, they have an account and may join. It carries Encore's name
/// because Encore was the first game to need it.
const MAY_JOIN_QUERY: &str = "encore_may_join";

pub fn definition(join_action_id: &str) -> CreateWorkflowDefinition {
    let join = WorkflowActionBuilder::new(JOIN_ACTION)
        .run_server_action()
        .allow_while_loading()
        .target(join_action_id)
        .build();
    let node = WorkflowNodeBuilder::new("Longtable")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("table")
                .tile(vec![
                    WorkflowDisplayBuilder::new().activity("longtable", JOIN_ACTION).build(),
                ])
                .build(),
        )
        .build();
    WorkflowDefinitionBuilder::new(WORKFLOW_ID, "Longtable")
        .description("Dice, initiative and character sheets for a 5E compatible table.")
        .starting_server_action(join_action_id)
        .response("longtable", json!(null))
        .add_actions(vec![join])
        .add_node("table", node)
        .build()
}

fn command_definition(id: &str, name: &str, action_id: &str) -> CreateWorkflowDefinition {
    let node = WorkflowNodeBuilder::new(name)
        .visual(
            WorkflowVisualBuilder::new("done")
                .tile(vec![
                    WorkflowDisplayBuilder::new().text("Answered in the chat.").build(),
                ])
                .build(),
        )
        .build();
    // Each run needs its own instance, or a later run replays the first one's text.
    WorkflowDefinitionBuilder::new(id, name)
        .multi_instance_per_user()
        .starting_server_action(action_id)
        .add_node("done", node)
        .build()
}

/// Registers the table when `LONGTABLE_ACTIVITY_URL` says where its client
/// is, and the chat commands either way.
pub async fn register(bot: &TawksBot, server: &LongtableServer) -> Result<()> {
    server.set_turns(Arc::new(BotTurns {
        sdk: bot.sdk.clone(),
        warned: AtomicBool::new(false),
    }));
    if let Err(error) = register_commands(bot, server.store()).await {
        tracing::warn!(%error, "the Longtable chat commands did not register");
    }
    let Ok(client_url) = std::env::var("LONGTABLE_ACTIVITY_URL") else {
        tracing::info!("LONGTABLE_ACTIVITY_URL is not set; Longtable is not in the lobby menu");
        return Ok(());
    };
    let join_id = bot
        .register_server_action(JOIN_ACTION, join_handler(server, bot.sdk.web_client.clone()))
        .await?;
    let workflow_id = bot.register_workflow(definition(&join_id)).await?;
    bot.register_activity(
        &Activity::new(workflow_id, "Longtable", client_url)
            .source(SOURCE_URL)
            .description("Dice, initiative and character sheets for a 5E compatible table")
            .icon("dices")
            .players(1, 12),
    )
    .await?;
    Ok(())
}

async fn register_commands(bot: &TawksBot, store: Arc<dyn Store>) -> Result<()> {
    let roll_id = bot
        .register_server_action(ROLL_ACTION, roll_handler(bot.sdk.web_client.clone()))
        .await?;
    let roll_workflow = bot
        .register_workflow(command_definition("longtable_roll", "Roll dice", &roll_id))
        .await?;
    bot.register_command(
        &Command::new("roll", "Roll dice, like 2d6+3 or 1d20 adv", roll_workflow)
            .placeholder("formula")
            .icon("dices"),
    )
    .await?;
    let locks: Arc<DashMap<String, Arc<Mutex<()>>>> = Arc::new(DashMap::new());
    let init_id = bot
        .register_server_action(INIT_ACTION, init_handler(bot.sdk.web_client.clone(), store, locks))
        .await?;
    let init_workflow = bot
        .register_workflow(command_definition("longtable_init", "Initiative", &init_id))
        .await?;
    bot.register_command(
        &Command::new("init", "Keep an initiative order in this chat", init_workflow)
            .placeholder("add Goblin 12 | next | clear")
            .icon("swords"),
    )
    .await?;
    Ok(())
}

struct BotTurns {
    sdk: Arc<TawksSdk>,
    warned: AtomicBool,
}

impl TurnSink for BotTurns {
    fn set_turn(&self, lobby_id: &str, turn: &Turn) {
        let sdk = self.sdk.clone();
        let lobby_id = lobby_id.to_string();
        let turn = turn.clone();
        let warned = !self.warned.swap(true, Ordering::Relaxed);
        tokio::spawn(async move {
            if let Err(error) = sdk
                .set_lobby_turn(&lobby_id, &turn.user_ids, turn.label.as_deref())
                .await
                && warned
            {
                tracing::warn!(%error, "the lobby turn did not send; stage tiles will not light up");
            }
        });
    }
}

fn display_name(user: &UserResource) -> String {
    if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.clone()
    }
}

fn join_handler(server: &LongtableServer, web: Arc<WebClient>) -> ServerActionHandler {
    let server = server.clone();
    Arc::new(Box::new(move |ctx| {
        let server = server.clone();
        let web = web.clone();
        Box::pin(async move {
            if ctx.ctx_required_as::<bool>("$anonymous").unwrap_or(false) {
                return Err(anyhow!("Sign in to sit at the table."));
            }
            let user: UserResource = ctx.ctx_required_as("$user").map_err(|e| anyhow!(e.to_string()))?;
            let lobby_id: String = ctx
                .get_input_as("lobby_id")
                .map_err(|_| anyhow!("Open Longtable from a lobby."))?;
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
                "longtable".to_string(),
                json!({ "ws_url": server.ws_url(), "token": token, "lobby_id": lobby_id }),
            );
            Ok(ServerActionResult::UpdateResponses(responses))
        })
    }))
}

struct CommandCall {
    who: String,
    chat_id: String,
    text: String,
}

fn command_call(ctx: &workflow::service::WorkflowResourceWithContext) -> Result<CommandCall> {
    let user: UserResource = ctx.ctx_required_as("$user").map_err(|e| anyhow!(e.to_string()))?;
    let chat_id = ctx
        .ctx_get("chat_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Run this in a chat."))?;
    let text = ctx
        .ctx_get("text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| ctx.get_input_as::<String>("text").ok())
        .unwrap_or_default();
    Ok(CommandCall {
        who: display_name(&user),
        chat_id,
        text,
    })
}

async fn post(web: &WebClient, chat_id: &str, text: &str) -> Result<()> {
    web.mutate::<_, Value>(
        "chat_create_message",
        &json!({ "chat_id": chat_id, "contents": { "text": text, "attachments": null } }),
    )
    .await
    .map_err(|_| anyhow!("Longtable can't answer in this chat. It answers in hubs it belongs to."))?;
    Ok(())
}

fn roll_handler(web: Arc<WebClient>) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let web = web.clone();
        Box::pin(async move {
            let call = command_call(&ctx)?;
            let reply = chat::roll_reply(&call.who, &call.text, &mut rand::rng()).map_err(|e| anyhow!(e))?;
            post(&web, &call.chat_id, &reply).await?;
            Ok(ServerActionResult::None)
        })
    }))
}

fn init_handler(
    web: Arc<WebClient>,
    store: Arc<dyn Store>,
    locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
) -> ServerActionHandler {
    Arc::new(Box::new(move |ctx| {
        let (web, store, locks) = (web.clone(), store.clone(), locks.clone());
        Box::pin(async move {
            let call = command_call(&ctx)?;
            let command = chat::parse_init(&call.text).map_err(|e| anyhow!(e))?;
            let lock = locks.entry(call.chat_id.clone()).or_default().clone();
            let _held = lock.lock().await;
            let key = store::chat_key(&call.chat_id);
            let mut tracker: Tracker = match store.load(&key).await? {
                Some(state) => serde_json::from_value(state).unwrap_or_default(),
                None => Tracker::default(),
            };
            let reply = chat::run_init(&mut tracker, command, &mut rand::rng()).map_err(|e| anyhow!(e))?;
            store.save(&key, &serde_json::to_value(&tracker)?).await?;
            post(&web, &call.chat_id, &reply).await?;
            Ok(ServerActionResult::None)
        })
    }))
}
