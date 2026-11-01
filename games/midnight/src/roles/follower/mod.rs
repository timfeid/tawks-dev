use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::CreateWorkflowDefinition;
use workflow::{WorkflowVisual, builder::*, ui::*};

use crate::MidnightPage;
use crate::ability_header;
use crate::gamestate::{GameState, PlayerResource};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const FOLLOWER_ROLE_ID: &str = "follower";
pub const FOLLOWER_WORKFLOW_ID: &str = "follower_awareness_workflow";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn follower_awareness_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("follower_awareness")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                column(
                    Gap::Tight,
                    vec![
                        icon("ghost", IconSize::Large).tone(Tone::Yellow),
                        ability_header(
                            "The Follower",
                            Tone::Yellow,
                            "You serve the Hollow alliance. You want to die.",
                        ),
                    ],
                )
                .align(Align::Center),
                items(
                    "hollows",
                    "hollow",
                    Layout::Grid {
                        columns: 2,
                        gap: Gap::Normal,
                        fixed: false,
                    },
                    vec![
                        list_row("{hollow.name}")
                            .meta("The Hollow")
                            .lead_with(vec![avatar(
                                "{hollow.avatar_url}",
                                "{hollow.name}",
                                Size::Medium,
                            )])
                            .state(State::Highlight),
                    ],
                ),
                notice(
                    Tone::Warning,
                    "They do NOT know who you are. Do not reveal yourself until the time is right.",
                ),
            ],
        )
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn follower_awareness_workflow() -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(FOLLOWER_WORKFLOW_ID, "Follower Awareness")
        .description("Reveal the Hollow(s) to the Follower.")
        .add_node(
            "awareness",
            WorkflowNodeBuilder::new("Awareness")
                .visual(follower_awareness_visual())
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, _engine: Arc<GameEngine<GameState>>) -> Result<()> {
    bot.register_workflow(follower_awareness_workflow()).await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn follower_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        register: None,
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: FOLLOWER_ROLE_ID.to_string(),
        name: "The Follower".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/follower.png".into(),
        priority: 3,
        alliance: Alliance::Hollow,
        enabled,

        night_duration: 0,
        night_action: None,

        on_night_start: Some(Arc::new(|game: &GameState, player| {
            let follower_id = player.id.clone();

            // Find ALL alive Hollows
            let hollows: Vec<PlayerResource> = game
                .players
                .values()
                .filter(|p| {
                    p.is_alive
                        && p.id != follower_id
                        && p.effective_role_card(None).role_id == "hollow"
                })
                .map(|p| p.into())
                .collect();

            let mut inputs = HashMap::new();
            // Pass the list of players as "hollows"
            inputs.insert("hollows".into(), json!(hollows));

            Some(WorkflowStart {
                workflow_id: FOLLOWER_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
