//! Cards as a bot of its own. It needs the usual bot settings plus
//! CARDGAME_DATABASE_URL, and CARDS_ACTIVITY_URL to offer Cards in lobbies'
//! game menus.

use std::time::Duration;

use anyhow::Result;
use tawks_bot::types::{BotOptions, Event};
use tawks_bot::{TawksBot, shutdown_token, with_jitter};
use tawks_data::database::create_connection;
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
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
    let pool = create_connection(&env("CARDGAME_DATABASE_URL")?).await;

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            cardgame::runtime::register_runtime(&bot, pool.clone()).await?;
            info!("cards registered");
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
