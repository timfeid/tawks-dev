//! Tracklines as a bot of its own, with the usual bot settings.

use anyhow::Result;
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
        tracklines::runtime::register_runtime(&bot).await?;
        Ok(())
    })
    .await
}
