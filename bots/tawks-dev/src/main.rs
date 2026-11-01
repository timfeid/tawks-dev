//! The Tawks Dev niche bot. The niche's landing page links to the docs site,
//! so the bot only keeps the niche's bot member online.

use std::time::Duration;

use anyhow::Result;
use tawks_bot::types::{BotOptions, Event};
use tawks_bot::{TawksBot, shutdown_token, with_jitter};
use tokio::{select, time::sleep};
use tracing::{info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let _glitchtip = tawks_telemetry::init_from_env("tawks-dev");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .finish()
        .with(tawks_telemetry::layer())
        .init();

    let opts = BotOptions {
        rspc_url: env("RSPC_URL")?,
        events_ws_url: env("EVENTS_WS_URL")?,
        token: env("BOT_TOKEN")?,
        token_type: Some(std::env::var("BOT_TOKEN_TYPE").unwrap_or_else(|_| "Bot".to_string())),
    };

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(60);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            bot.start(|event, _| async move {
                match event {
                    Event::Connect => info!("connected"),
                    Event::Disconnect => warn!("disconnected"),
                    Event::Error { error } => warn!(%error, "event stream error"),
                    Event::Message { .. } => {}
                }
            })
            .await
        };

        select! {
            _ = shutdown.cancelled() => return Ok(()),
            result = session => {
                if let Err(error) = result {
                    tawks_bot::connection_failed(&error, "bot stopped");
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
