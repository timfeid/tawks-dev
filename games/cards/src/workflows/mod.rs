use anyhow::Result;
use tawks_bot::{Activity, TawksBot};

use crate::CardGameActionIds;

mod builder_workflow;
mod card_browser;
mod card_builder;
mod deck_builder;
mod game_workflow;
mod helpers;

pub use builder_workflow::builder_definition;
pub use game_workflow::{activity_definition, definition};

pub async fn register(bot: &TawksBot, actions: &CardGameActionIds) -> Result<()> {
    bot.register_workflow(definition(actions)).await?;
    bot.register_workflow(builder_definition(actions)).await?;
    if let Ok(client_url) = std::env::var("CARDS_ACTIVITY_URL") {
        let workflow_id = bot.register_workflow(activity_definition(actions)).await?;
        bot.register_activity(
            &Activity::new(workflow_id, "Cards (beta)", client_url)
                .source("https://github.com/timfeid/tawks-dev/tree/main/games/cards")
                .description("Mana Clash running as an activity")
                .icon("layers")
                .players(1, 4),
        )
        .await?;
    }
    Ok(())
}
