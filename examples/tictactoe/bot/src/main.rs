//! Tic-tac-toe as a tawks activity: the rules live in this bot, the board is
//! a web app in `../client` that tawks runs in a sandboxed frame.

mod game;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use game_framework::engine::GameEngine;
use serde_json::{Value, json};
use tawks_bot::types::{BotOptions, Event, UserResource};
use tawks_bot::workflow::CreateWorkflowDefinition;
use tawks_bot::workflow::builder::{
    WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use tawks_bot::{Activity, TawksBot, shutdown_token, with_jitter};
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

use game::{Action, Input, TicTacToe};

struct Routes {
    view: String,
    input: String,
}

async fn register_routes(engine: Arc<GameEngine<TicTacToe>>) -> Result<Routes> {
    let view = engine
        .clone()
        .register_route("tictactoe.view", |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            let name = if user.display_name.trim().is_empty() { user.username } else { user.display_name };
            Ok(Action::View { name })
        })
        .await?;
    let input = engine
        .register_route("tictactoe.input", |ctx| {
            let input: Value = ctx.get_input_as("input")?;
            Ok(Action::Input(serde_json::from_value::<Input>(input)?))
        })
        .await?;
    Ok(Routes { view, input })
}

fn workflow(routes: &Routes) -> CreateWorkflowDefinition {
    let board = WorkflowNodeBuilder::new("Tic-tac-toe")
        .layout("min-h-screen w-full relative")
        .visual(
            WorkflowVisualBuilder::new("board")
                .tile(vec![WorkflowDisplayBuilder::new("absolute inset-0").activity("game", "input").build()])
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new("tictactoe", "Tic-tac-toe")
        .description("Three in a row, as a tawks activity.")
        .starting_server_action(&routes.view)
        .response("game", json!(null))
        .add_action(
            WorkflowActionBuilder::new("view")
                .run_server_action()
                .allow_while_loading()
                .target(&routes.view)
                .build(),
        )
        .add_action(WorkflowActionBuilder::new("input").run_server_action().target(&routes.input).build())
        .add_node("board", board)
        .build()
}

async fn on_event(event: Event) {
    match event {
        Event::Connect => info!("connected"),
        Event::Disconnect => warn!("disconnected"),
        Event::Error { error } => error!(%error, "event stream error"),
        Event::Message { .. } => {}
    }
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    let opts = BotOptions {
        rspc_url: env("RSPC_URL")?,
        events_ws_url: env("EVENTS_WS_URL")?,
        token: env("BOT_TOKEN")?,
        token_type: Some(std::env::var("BOT_TOKEN_TYPE").unwrap_or_else(|_| "Bot".to_string())),
    };
    let client_url = std::env::var("CLIENT_URL").unwrap_or_else(|_| "http://localhost:5190/".to_string());

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            let engine = Arc::new(GameEngine::<TicTacToe>::new((), bot.clone()));
            let routes = register_routes(engine).await?;
            let workflow_id = bot.register_workflow(workflow(&routes)).await?;
            bot.register_activity(
                &Activity::new(&workflow_id, "Tic-tac-toe", &client_url)
                    .source("https://github.com/timfeid/tawks-dev/tree/main/examples/tictactoe")
                    .description("Three in a row")
                    .icon("game")
                    .players(2, 2),
            )
            .await?;
            info!(%workflow_id, %client_url, "registered");
            bot.start(|event, _| on_event(event)).await
        };

        select! {
            _ = shutdown.cancelled() => return Ok(()),
            result = session => {
                if let Err(error) = result {
                    error!(%error, "bot stopped");
                }
            }
        }

        let wait = with_jitter(backoff);
        warn!(?wait, "reconnecting");
        select! {
            _ = shutdown.cancelled() => return Ok(()),
            _ = sleep(wait) => {}
        }
        backoff = (backoff * 2).min(backoff_max);
    }
}
