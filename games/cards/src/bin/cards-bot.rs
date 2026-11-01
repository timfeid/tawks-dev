//! Cards as a bot of its own. It needs the usual bot settings plus
//! CARDGAME_DATABASE_URL and CARDS_ACTIVITY_URL, where the Cards client that
//! lobbies load is served.

use anyhow::Result;
use tawks_bot::types::BotOptions;
use tawks_data::database::create_connection;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    let _glitchtip = tawks_telemetry::init_from_env("cards-bot");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .finish()
        .with(tawks_telemetry::layer())
        .init();

    let opts = BotOptions::from_env()?;
    let url = std::env::var("CARDGAME_DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("CARDGAME_DATABASE_URL is required"))?;
    let pool = create_connection(&url).await;

    tawks_bot::run(opts, |bot| {
        let pool = pool.clone();
        async move {
            cardgame::runtime::register_runtime(&bot, pool).await?;
            Ok(())
        }
    })
    .await
}
