use anyhow::{Context, Result};
use tawks_bot::{Activity, TawksBot};

use crate::CardGameActionIds;

mod builder_workflow;
mod card_builder;
mod game_workflow;
mod helpers;

pub use builder_workflow::builder_definition;
pub use game_workflow::activity_definition;

pub async fn register(bot: &TawksBot, actions: &CardGameActionIds) -> Result<()> {
    bot.register_workflow(builder_definition(actions)).await?;
    let client_url = std::env::var("CARDS_ACTIVITY_URL").context("CARDS_ACTIVITY_URL is not set")?;
    let workflow_id = bot.register_workflow(activity_definition(actions)).await?;
    bot.register_activity(
        &Activity::new(workflow_id, "Cards", client_url)
            .source("https://github.com/timfeid/tawks-dev/tree/main/games/cards")
            .description("Mana Clash, the collectible card game")
            .icon("layers")
            .players(1, 4),
    )
    .await?;
    Ok(())
}
