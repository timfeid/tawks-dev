use std::sync::Arc;

use anyhow::Result;
use game_framework::engine::GameEngine;
use tawks_bot::TawksBot;

use crate::logic::{TracklinesInitArgs, TracklinesState};
use crate::TracklinesActionIds;

pub async fn register_runtime(bot: &TawksBot) -> Result<TracklinesActionIds> {
    let engine = Arc::new(GameEngine::<TracklinesState>::new(TracklinesInitArgs::default(), bot.clone()));
    let action_ids = crate::register_routes(engine, "games.tracklines").await?;
    crate::workflows::register(bot, &action_ids).await?;
    Ok(action_ids)
}
