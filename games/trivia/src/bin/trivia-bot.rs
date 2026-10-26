//! Trivia as a bot of its own. It needs the usual bot settings plus
//! TRIVIA_DATABASE_URL. With OPENAI_API_KEY set, quizzes are written by
//! OpenAI (OPENAI_MODEL picks the model); without it every quiz is a built-in
//! sample, which is enough to play and develop against.

use std::sync::Arc;

use anyhow::Result;
use game_framework::engine::GameEngine;
use sqlx::PgPool;
use tawks_bot::{Activity, TawksBot};
use tawks_bot::types::BotOptions;
use tawks_data::database::create_connection;
use tracing::warn;
use trivia::{
    Generator,
    generator::{local::LocalMockGenerator, openai::OpenAiGenerator},
    multiplayer::{TriviaGameState, TriviaInitArgs},
    state::TriviaState,
};

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
        let workflow_id = trivia::workflows::register_multiplayer_workflow(bot, &server_actions, &actions).await?;
        bot.register_activity(
            &Activity::workflow(workflow_id, "Trivia")
                .source("https://github.com/timfeid/tawks-dev/tree/main/games/trivia")
                .description("Race your lobby to the right answer")
                .icon("brain")
                .players(1, 16),
        )
        .await?;
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

    let opts = BotOptions::from_env()?;
    let url = std::env::var("TRIVIA_DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("TRIVIA_DATABASE_URL is required"))?;
    let pool = create_connection(&url).await;
    sqlx::migrate!().run(pool.as_ref()).await?;
    let generator = generator()?;

    tawks_bot::run(opts, |bot| {
        let (pool, generator) = (pool.clone(), generator.clone());
        async move { register(&bot, pool, generator).await }
    })
    .await
}
