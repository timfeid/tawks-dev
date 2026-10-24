//! Trivia as a bot of its own. It needs the usual bot settings plus
//! TRIVIA_DATABASE_URL. With OPENAI_API_KEY set, quizzes are written by
//! OpenAI (OPENAI_MODEL picks the model); without it every quiz is a built-in
//! sample, which is enough to play and develop against.

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use game_framework::engine::GameEngine;
use sqlx::PgPool;
use tawks_bot::types::{BotOptions, Event};
use tawks_bot::{TawksBot, shutdown_token, with_jitter};
use tawks_data::database::create_connection;
use tokio::{select, time::sleep};
use tracing::{error, info, warn};
use trivia::{
    Generator,
    generator::{local::LocalMockGenerator, openai::OpenAiGenerator},
    multiplayer::{TriviaGameState, TriviaInitArgs},
    state::TriviaState,
};

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
}

fn generator() -> Result<Arc<dyn Generator + Send + Sync>> {
    if std::env::var("OPENAI_API_KEY").is_ok_and(|key| !key.is_empty()) {
        Ok(Arc::new(OpenAiGenerator::from_env()?))
    } else {
        warn!("OPENAI_API_KEY is not set; every quiz will be the built-in sample");
        Ok(Arc::new(LocalMockGenerator::devils_fixture()))
    }
}

async fn register(
    bot: &TawksBot,
    pool: Arc<PgPool>,
    generator: Arc<dyn Generator + Send + Sync>,
) -> Result<()> {
    let engine = Arc::new(GameEngine::<TriviaGameState>::new(
        TriviaInitArgs::default(),
        bot.clone(),
    ));
    let mut trivia = TriviaState::new(pool, generator, Some(bot.clone()));
    trivia.set_multiplayer_engine(engine.clone());
    trivia.register().await?;
    let actions = trivia::multiplayer::register_routes(engine, "games.trivia").await?;
    if let Some(server_actions) = trivia.server_actions.get().cloned() {
        trivia::workflows::register_multiplayer_workflow(bot, &server_actions, &actions).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    let opts = BotOptions {
        rspc_url: env("RSPC_URL")?,
        events_ws_url: env("EVENTS_WS_URL")?,
        token: env("BOT_TOKEN")?,
        token_type: Some(std::env::var("BOT_TOKEN_TYPE").unwrap_or_else(|_| "Bot".to_string())),
    };
    let pool = create_connection(&env("TRIVIA_DATABASE_URL")?).await;
    sqlx::migrate!().run(pool.as_ref()).await?;
    let generator = generator()?;

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            register(&bot, pool.clone(), generator.clone()).await?;
            info!("trivia registered");
            bot.start(|event, _| async move {
                match event {
                    Event::Connect => info!("connected"),
                    Event::Disconnect => warn!("disconnected"),
                    Event::Error { error } => error!(%error, "event stream error"),
                    Event::Message { .. } => {}
                }
            })
            .await
        };

        select! {
            _ = shutdown.cancelled() => return Ok(()),
            result = session => {
                match result {
                    Ok(()) => backoff = Duration::from_secs(1),
                    Err(error) => error!(%error, "bot stopped"),
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
