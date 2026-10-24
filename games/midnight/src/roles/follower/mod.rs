use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::CreateWorkflowDefinition;
use workflow::{WorkflowVisual, builder::*};

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
        .tile(vec![
            // 1. Header
            WorkflowDisplayBuilder::new("w-full flex flex-col items-center justify-center mb-6")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-4xl animate-pulse mb-2 text-yellow-500")
                        .text("👁️")
                        .build(),
                    WorkflowDisplayBuilder::new("text-2xl font-halloween text-yellow-400 tracking-widest uppercase opacity-90 drop-shadow-sm")
                        .text("The Follower")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xs font-mono text-white/50 uppercase tracking-widest text-center max-w-xs mt-2")
                        .text("You serve the Hollow alliance. You want to die.")
                        .build(),
                ])
                .build(),

            // 2. The Grid of Hollows
            // We use a responsive grid: 1 column for single hollow, 2 for multiples
            WorkflowDisplayBuilder::new("w-full grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl mx-auto")
                .list(
                    "hollows", // Input key containing the array of players
                    "hollow",  // Loop variable name
                    vec![
                        // INNER CARD FOR EACH HOLLOW
                        WorkflowDisplayBuilder::new("bg-neutral-900/90 border border-yellow-500/30 rounded-lg p-4 flex items-center gap-4 shadow-[0_0_15px_rgba(234,179,8,0.1)] relative overflow-hidden")
                            .div(vec![
                                // Background glow
                                WorkflowDisplayBuilder::new("absolute inset-0 bg-[radial-gradient(circle_at_right,_var(--tw-gradient-stops))] from-yellow-900/10 to-transparent pointer-events-none")
                                    .build(),

                                // Avatar
                                WorkflowDisplayBuilder::new("size-12 rounded-full border-2 border-yellow-500/50 bg-black/50 overflow-hidden shadow-sm relative z-10")
                                    .image("{hollow.avatar_url}", "{hollow.name}")
                                    .build(),

                                // Text Details
                                WorkflowDisplayBuilder::new("flex-1 min-w-0 flex flex-col justify-center relative z-10")
                                    .div(vec![
                                        WorkflowDisplayBuilder::new("text-base font-halloween text-white/90 truncate")
                                            .text("{hollow.name}")
                                            .build(),
                                        WorkflowDisplayBuilder::new("text-[10px] font-mono text-yellow-400/80 uppercase tracking-widest")
                                            .text("The Hollow")
                                            .build(),
                                    ])
                                    .build(),
                            ])
                            .build()
                    ],
                )
                .build(),

            // 3. Status / Win Condition Footer
            WorkflowDisplayBuilder::new("w-full max-w-md mx-auto mt-8 border-t border-white/5 pt-4")
                .div(vec![
                    WorkflowDisplayBuilder::new("flex items-start gap-3 opacity-70")
                        .div(vec![
                            WorkflowDisplayBuilder::new("text-lg").text("⚠️").build(),
                            WorkflowDisplayBuilder::new("text-xs text-white/80 leading-relaxed")
                                .text("They do NOT know who you are. Do not reveal yourself until the time is right.")
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
        ])
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
