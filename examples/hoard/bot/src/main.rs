//! Hoard: a 2-4 player card game as a tawks activity. Rules and hidden hands
//! live in this bot; the table is a web app in `../client`.

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
use tawks_bot::{Activity, TawksBot, TawksSdk, shutdown_token, with_jitter};
use tokio::sync::mpsc;
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

use game::{Action, Hoard, HoardArgs, Input, Standing};

struct Routes {
    view: String,
    input: String,
}

async fn register_routes(engine: Arc<GameEngine<Hoard>>) -> Result<Routes> {
    let view = engine
        .clone()
        .register_route("hoard.view", |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            let name = if user.display_name.trim().is_empty() { user.username } else { user.display_name };
            let place = ctx.get_input_as::<String>("lobby_id").or_else(|_| ctx.get_input_as::<String>("channel_id"))?;
            Ok(Action::View { name, place })
        })
        .await?;
    let input = engine
        .register_route("hoard.input", |ctx| {
            let input: Value = ctx.get_input_as("input")?;
            Ok(Action::Input(serde_json::from_value::<Input>(input)?))
        })
        .await?;
    Ok(Routes { view, input })
}

fn workflow(routes: &Routes) -> CreateWorkflowDefinition {
    let table = WorkflowNodeBuilder::new("Hoard")
        .layout("min-h-screen w-full relative")
        .visual(
            WorkflowVisualBuilder::new("table")
                .tile(vec![WorkflowDisplayBuilder::new("absolute inset-0").activity("game", "input").build()])
                .build(),
        )
        .build();

    WorkflowDefinitionBuilder::new("hoard", "Hoard")
        .description("Bank treasure, rob vaults, wake the dragon.")
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
        .add_node("table", table)
        .build()
}

async fn record_results(sdk: Arc<TawksSdk>, lobby_id: String, standings: Vec<Standing>) {
    let [home, away, ..] = &standings[..] else { return };
    let args = json!({
        "lobby_id": lobby_id,
        "game_id": "hoard",
        "title": "Hoard",
        "home_team": home.name,
        "away_team": away.name,
        "home_score": home.score,
        "away_score": away.score,
        "home_humans": 1,
        "away_humans": 1,
        "players": standings.iter().enumerate().map(|(i, s)| json!({
            "user_id": s.id,
            "seat": i,
            "side": if i == 0 { "home" } else { "away" },
        })).collect::<Vec<_>>(),
    });
    if let Err(error) = sdk.web_client.mutate::<_, Value>("game_result_record", &args).await {
        warn!(%error, "recording the result failed");
    }
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
    let client_url = std::env::var("CLIENT_URL").unwrap_or_else(|_| "http://localhost:5390/".to_string());
    let seed = std::env::var("HOARD_SEED").ok().and_then(|s| s.parse().ok());

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            let (results, mut inbox) = mpsc::unbounded_channel::<(String, Vec<Standing>)>();
            let sdk = bot.sdk.clone();
            tokio::spawn(async move {
                while let Some((lobby_id, standings)) = inbox.recv().await {
                    record_results(sdk.clone(), lobby_id, standings).await;
                }
            });
            let engine = Arc::new(GameEngine::<Hoard>::new(HoardArgs { results, seed }, bot.clone()));
            let routes = register_routes(engine).await?;
            let workflow_id = bot.register_workflow(workflow(&routes)).await?;
            bot.register_activity(
                &Activity::new(&workflow_id, "Hoard", &client_url)
                    .source("https://github.com/timfeid/tawks-dev/tree/main/examples/hoard")
                    .description("Bank treasure, rob vaults, wake the dragon")
                    .icon("gem")
                    .players(2, 4),
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
