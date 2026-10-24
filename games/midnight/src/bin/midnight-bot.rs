//! Midnight as a bot of its own, with the usual bot settings.

use std::sync::Arc;

use anyhow::Result;
use game_framework::engine::GameEngine;
use midnight::gamestate::{GameState, MidnightInitArgs};
use tawks_bot::types::BotOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .init();

    tawks_bot::run(BotOptions::from_env()?, |bot| async move {
        let engine = Arc::new(GameEngine::<GameState>::new(
            MidnightInitArgs::default(),
            bot.clone(),
        ));
        let actions = midnight::register_routes(engine.clone(), "games.midnight").await?;
        midnight::workflows::register(&bot, engine, actions).await
    })
    .await
}
