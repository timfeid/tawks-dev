use std::{future::Future, time::Duration};

use tokio::{select, time::sleep};
use tracing::{error, info, warn};

use crate::{
    TawksBot, shutdown_token,
    types::{BotOptions, Event},
    with_jitter,
};

/// Keeps a bot connected until the process is told to stop. Each time it
/// connects, `register` gets the new bot to register its workflows and server
/// actions; when the connection drops, it reconnects with backoff.
pub async fn run<F, Fut>(opts: BotOptions, mut register: F) -> anyhow::Result<()>
where
    F: FnMut(TawksBot) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let shutdown = shutdown_token();
    let mut backoff = Duration::from_secs(1);
    let backoff_max = Duration::from_secs(30);

    loop {
        let session = async {
            let bot = TawksBot::new(opts.clone()).await?;
            register(bot.clone()).await?;
            info!("registered");
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
            result = session => match result {
                Ok(()) => backoff = Duration::from_secs(1),
                Err(error) => error!(%error, "bot stopped"),
            },
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
