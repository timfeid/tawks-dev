use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::{CreateWorkflowDefinition, WorkflowDisplay};
use workflow::{WorkflowVisual, builder::*};

use crate::gamestate::GameState;
use crate::role_card_display_for;
use crate::roles::{Alliance, RoleCard, WorkflowStart};
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
        .tile(vec![
            // 1. Header
            WorkflowDisplayBuilder::new("w-full flex flex-col items-center justify-center mb-8")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-4xl animate-pulse mb-2 text-cyan-400")
                        .text("🧿")
                        .build(),
                    WorkflowDisplayBuilder::new("text-2xl font-halloween text-cyan-400 tracking-widest uppercase opacity-90 drop-shadow-[0_0_10px_rgba(34,211,238,0.5)]")
                        .text("The Lucid")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xs font-mono text-cyan-200/50 uppercase tracking-widest text-center mt-2")
                        .text("The dream shifts around you. What have you become?")
                        .build(),
                ])
                .build(),

            // 2. The Reveal Container
            WorkflowDisplayBuilder::new("w-full flex justify-center py-4 relative group")
                .div(vec![
                    // A glowing "Portal" effect behind the card
                    WorkflowDisplayBuilder::new("absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-64 h-80 bg-cyan-500/10 blur-3xl rounded-full animate-pulse pointer-events-none")
                        .build(),

                    // The Current Role Card
                    // We simply ask the frontend to render "my_role", which we passed in the workflow inputs
                    WorkflowDisplay {
                        class: "w-full max-w-xs relative z-10 transition-transform duration-500 hover:scale-105".into(),
                        r#type: role_card_display_for("current_role"),
                    }
                ])
                .build(),

            // 3. Status Footer
            WorkflowDisplayBuilder::new("w-full text-center mt-8")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-sm font-halloween text-white/90")
                        .text("This is your true form.")
                        .build(),
                    WorkflowDisplayBuilder::new("text-[10px] font-mono text-white/40 uppercase tracking-widest mt-1")
                        .text("If this is not The Lucid, you have been moved.")
                        .build(),
                ])
                .build(),
        ])
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
            // We pass the full role object so 'role_card_display_for' can render it
            inputs.insert("current_role".into(), json!(current_role));

            Some(WorkflowStart {
                workflow_id: LUCID_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
