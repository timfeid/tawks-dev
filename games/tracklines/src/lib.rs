//! Tracklines: a rail-route board game for two to five players. Claim routes
//! between cities with matching train cards, complete destination tickets,
//! and score the longest network. Rules and the AI live in `game`, `map` and
//! `ai`; the engine and 3D board wiring live in `logic`, `view`, `workflows`
//! and `runtime`.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use game_framework::engine::GameEngine;
use tawks_bot::{full_avatar_url, types::UserResource};

pub mod ai;
pub mod game;
pub mod logic;
pub mod map;
pub mod runtime;
pub mod view;
pub mod workflows;

#[cfg(test)]
mod tests;

pub use logic::{TracklinesAction, TracklinesInitArgs, TracklinesState};

/// Canonical server action ids handed back by the engine, one per route.
#[derive(Clone, Debug)]
pub struct TracklinesActionIds {
    pub view: String,
    pub join: String,
    pub add_ai: String,
    pub start: String,
    pub draw_card: String,
    pub claim_route: String,
    pub draw_tickets: String,
    pub keep_tickets: String,
    pub reset: String,
}

pub async fn register_routes(
    engine: Arc<GameEngine<TracklinesState>>,
    prefix: &str,
) -> Result<TracklinesActionIds> {
    let view = engine
        .clone()
        .register_route(&format!("{prefix}.view"), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(TracklinesAction::View {
                name: display_name(&user),
                avatar_url: full_avatar_url(&user),
                instance_id: ctx.workflow.instance_id.clone(),
            })
        })
        .await?;
    let join = engine
        .clone()
        .register_route(&format!("{prefix}.join"), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(TracklinesAction::Join {
                name: display_name(&user),
                avatar_url: full_avatar_url(&user),
                instance_id: ctx.workflow.instance_id.clone(),
            })
        })
        .await?;
    let add_ai = engine
        .clone()
        .register_route(&format!("{prefix}.add_ai"), |_| Ok(TracklinesAction::AddAi))
        .await?;
    let start = engine
        .clone()
        .register_route(&format!("{prefix}.start"), |_| Ok(TracklinesAction::Start))
        .await?;
    let draw_card = engine
        .clone()
        .register_route(&format!("{prefix}.draw_card"), |ctx| {
            let face_up = ctx.get_input_as::<Option<usize>>("face_up").ok().flatten();
            Ok(TracklinesAction::DrawCard { face_up })
        })
        .await?;
    let claim_route = engine
        .clone()
        .register_route(&format!("{prefix}.claim_route"), |ctx| {
            let route = ctx
                .get_input_as::<u16>("route")
                .map_err(|_| anyhow!("Missing route"))?;
            let color = ctx.get_input_as::<Option<game::Card>>("color").ok().flatten();
            Ok(TracklinesAction::ClaimRoute { route, color })
        })
        .await?;
    let draw_tickets = engine
        .clone()
        .register_route(&format!("{prefix}.draw_tickets"), |_| Ok(TracklinesAction::DrawTickets))
        .await?;
    let keep_tickets = engine
        .clone()
        .register_route(&format!("{prefix}.keep_tickets"), |ctx| {
            let keep = ctx.get_input_as::<Vec<u16>>("keep").unwrap_or_default();
            Ok(TracklinesAction::KeepTickets { keep })
        })
        .await?;
    let reset = engine
        .clone()
        .register_route(&format!("{prefix}.reset"), |_| Ok(TracklinesAction::Reset))
        .await?;
    Ok(TracklinesActionIds {
        view,
        join,
        add_ai,
        start,
        draw_card,
        claim_route,
        draw_tickets,
        keep_tickets,
        reset,
    })
}

fn display_name(user: &UserResource) -> String {
    if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.clone()
    }
}
