use anyhow::Result;
use pucklab_bot::{boards::BoardCache, client::PuckLab, club, hut, jersey, store};
use std::{sync::Arc, time::Duration};
use tawks_bot::{
    TawksBot, shutdown_token,
    types::{BotOptions, Event},
    with_jitter,
};
use tokio::{select, time::sleep};
use tracing::{info, warn};

fn env(name: &str) -> Result<String> {
    std::env::var(name).map_err(|_| anyhow::anyhow!("{name} is required"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let service = env("PUCKLAB_URL")?;
    let api = Arc::new(
        PuckLab::new(
            &service,
            env("PUCKLAB_BOT_SECRET")?,
            env("PUCKLAB_BUILD")?.parse()?,
            env("PUCKLAB_PROTOCOL")?.parse()?,
        )?
        .with_site(&std::env::var("PUCKLAB_SITE_URL").unwrap_or(service))?,
    );
    let roster = env("PUCKLAB_ROSTER")?;
    let hut_capabilities = hut::HutCapabilities::from_env(&api.origin());
    let opts = BotOptions {
        rspc_url: env("RSPC_URL")?,
        events_ws_url: env("EVENTS_WS_URL")?,
        token: env("BOT_TOKEN")?,
        token_type: Some("Bot".into()),
    };
    let board_cache = Arc::new(BoardCache::default());
    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            let id = club::register(&bot, api.clone(), board_cache.clone()).await?;
            info!(workflow = %id,"PuckLab club workflow registered");
            if let Err(error) = store::register(&bot).await {
                warn!(%error, "PuckLab store products were not registered");
            }
            if let Some(capabilities) = &hut_capabilities {
                let id = hut::register_capabilities(&bot, capabilities).await?;
                info!(workflow = %id,"PuckLab HUT capability status registered; game provider disabled");
            }
            let events = bot.start(|event, _| async move {
                match event {
                    Event::Connect => info!("connected"),
                    Event::Disconnect => warn!("disconnected"),
                    Event::Error { .. } => warn!("bot event stream error"),
                    Event::Message { .. } => {}
                }
            });
            let clubs = async {
                if hut_capabilities.is_none() {
                    return std::future::pending().await;
                }
                loop {
                    if let Err(error) = hut::sync_clubs(&bot, &api, &roster).await {
                        warn!(%error, "PuckLab HUT club sync failed");
                    }
                    if let Err(error) = hut::sync_matches(&bot, &api, &roster).await {
                        warn!(%error, "PuckLab HUT match sync failed");
                    }
                    sleep(Duration::from_secs(3)).await;
                }
            };
            let entitlements = async {
                let mut reported = store::Reported::new();
                loop {
                    if let Err(error) = store::sync_entitlements(&bot, &api, &mut reported).await {
                        warn!(%error, "PuckLab entitlement sync failed");
                    }
                    sleep(Duration::from_secs(30)).await;
                }
            };
            let jerseys = async {
                loop {
                    let pause = match jersey::sync(&bot, &api, &roster).await {
                        Ok(_) => Duration::from_secs(5),
                        Err(error) => {
                            warn!(%error, "PuckLab jersey sync failed");
                            Duration::from_secs(60)
                        }
                    };
                    sleep(pause).await;
                }
            };
            let leaderboards = async {
                loop {
                    if let Err(error) = board_cache.sync(&bot, &api).await {
                        warn!(%error, "PuckLab leaderboard sync failed");
                    }
                    sleep(Duration::from_secs(60)).await;
                }
            };
            select! { result = events => result, never = clubs => never, never = entitlements => never, never = jerseys => never, never = leaderboards => never }
        };
        select! { _ = shutdown.cancelled() => return Ok(()), result = session => { if result.is_err() { warn!("PuckLab bot session stopped"); } } }
        select! { _ = shutdown.cancelled() => return Ok(()), _ = sleep(with_jitter(backoff)) => {} }
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}
