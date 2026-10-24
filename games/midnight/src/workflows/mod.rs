use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tawks_bot::TawksBot;

use crate::MidnightActionIds;
use crate::gamestate::GameState;
use game_framework::engine::GameEngine;

pub mod sus_animation;
pub mod view_lobby;
pub mod visuals;

// Role workflows live under roles/...
use crate::roles;

pub async fn register(
    bot: &TawksBot,
    engine: Arc<GameEngine<GameState>>,
    ids: MidnightActionIds,
) -> Result<()> {
    // base workflows
    sus_animation::register(bot).await?;
    view_lobby::register(bot, &ids).await?;

    // Bound workflow definition (needs ids.show_bound)
    bot.register_workflow(roles::bound::get_bound_workflow(ids.show_bound.clone()))
        .await?;

    // Hollow: register handler + workflow definition
    roles::hollow::register(bot, engine.clone()).await?;
    roles::lost::register(bot, engine.clone()).await?;
    roles::saboteur::register(bot, engine.clone()).await?;
    roles::tangle::register(bot, engine.clone()).await?;
    roles::taker::register(bot, engine.clone()).await?;
    roles::spy::register(bot, engine.clone()).await?;
    roles::witness::register(bot, engine.clone()).await?;
    roles::follower::register(bot, engine.clone()).await?;

    Ok(())
}

#[derive(Clone, Debug, Default)]
pub struct MidnightWorkflowIds {
    pub bound: String,
    // add more as you make them:
    // pub seer: String,
    // pub witch: String,
}

// pub type SharedMidnightWorkflowIds = Arc<MidnightWorkflowIds>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
#[serde(rename_all = "lowercase")]
pub enum CardSelect {
    Hidden { id: usize },
    Player { id: String },
}
