use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::CreateWorkflowDefinition;
use workflow::{WorkflowVisual, builder::*, ui::*};

use crate::MidnightPage;
use crate::gamestate::GameState;
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::{ability_header, role_card};
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const LUCID_ROLE_ID: &str = "lucid";
pub const LUCID_WORKFLOW_ID: &str = "lucid_reality_check_workflow";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn lucid_reality_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("lucid_reality")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                column(
                    Gap::Tight,
                    vec![
                        icon("sparkles", IconSize::Large).tone(Tone::Cyan),
                        ability_header(
                            "The Lucid",
                            Tone::Cyan,
                            "The dream shifts around you. What have you become?",
                        ),
                    ],
                )
                .align(Align::Center),
                // The role passed in as `current_role` when the night started.
                role_card("current_role"),
                column(
                    Gap::Tight,
                    vec![
                        body("This is your true form.").themed().centered(),
                        fine("If this is not The Lucid, you have been moved.").centered(),
                    ],
                ),
            ],
        )
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn lucid_workflow() -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(LUCID_WORKFLOW_ID, "Lucid Reality Check")
        .description("View your current role at the end of the night.")
        .add_node(
            "check",
            WorkflowNodeBuilder::new("Reality Check")
                .visual(lucid_reality_visual())
                .build(),
        )
        // No actions; this is a read-only workflow.
        .build()
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, _engine: Arc<GameEngine<GameState>>) -> Result<()> {
    bot.register_workflow(lucid_workflow()).await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn lucid_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec!["
While the village dreams in ignorance, your eyes remain wide open. At the very end of the night, you wake up and examine your own role. If you were swapped by another role during the darkness, you will see your new identity before the morning breaks. The Lucid aligns with the village. To know oneself is the only defense against the madness.
".into()],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: LUCID_ROLE_ID.to_string(),
        name: "The Lucid".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/lucid.png".into(),

        // Priority should be LAST (very high number) so they see the final result of all swaps.
        priority: 100,

        alliance: Alliance::Unmarked,
        enabled,
        night_duration: 10,
        night_action: None,
        register: None,

        on_night_start: Some(Arc::new(|_game: &GameState, player| {
            // Get the player's CURRENT role (accounting for swaps by Tangle, Taker, Lost, etc.)
            let current_role = player.effective_role_card(None);

            let mut inputs = HashMap::new();
            // We pass the full role object so 'role_card' can render it
            inputs.insert("current_role".into(), json!(current_role));

            Some(WorkflowStart {
                workflow_id: LUCID_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
