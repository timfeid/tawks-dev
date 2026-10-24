//! The Tawks Dev niche bot. Every doc in `docs/` is a page channel in the
//! niche; on start the bot renders each one and publishes it to its channel,
//! then stays online. `tawks-dev publish` publishes and exits.

mod docs;

use std::time::Duration;

use anyhow::Result;
use serde::Serialize;
use tawks_bot::types::{BotOptions, Event};
use tawks_bot::{TawksBot, shutdown_token, with_jitter};
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

#[derive(Serialize, Debug)]
struct PublishArgs<'a> {
    channel_id: String,
    content: &'a str,
}

async fn publish(bot: &TawksBot) -> Result<()> {
    for doc in docs::all() {
        let channel_id = format!("dev-{}", doc.slug);
        let html = docs::render(doc.markdown);
        let changed: bool = bot
            .sdk()
            .web_client
            .mutate("channel_post_publish", &PublishArgs { channel_id: channel_id.clone(), content: &html })
            .await?;
        info!(%channel_id, changed, "published");
    }
    Ok(())
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("{key} is required"))
}

#[tokio::main]
async fn main() -> Result<()> {
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

    if std::env::args().nth(1).as_deref() == Some("publish") {
        let bot = TawksBot::new(opts).await?;
        return publish(&bot).await;
    }

    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(60);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            publish(&bot).await?;
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
