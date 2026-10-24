use anyhow::{Result, anyhow};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{
    CreateWorkflowDefinition, DisplayType, DisplayVisual, NodeCondition,
    WorkflowDisplay,
};
use workflow::{WorkflowVisual, builder::*};

use crate::gamestate::{GameState, PlayerResource, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::{create_reveal_cards_node, player_display, title_visual};
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const TAKER_ROLE_ID: &str = "taker";
pub const TAKER_WORKFLOW_ID: &str = "taker_ability_workflow";

pub const TAKER_STEAL_ACTION: &str = "games.midnight.taker.steal_card";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

// (you had this helper already; keeping it local like the Lost module style)
fn select_card_lists_visual(content: Vec<WorkflowDisplay>) -> WorkflowVisual {
    WorkflowVisualBuilder::new("taker_select_card_lists")
        .display(DisplayVisual::Tile { content })
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn taker_ability_workflow(steal_action_target_id: String) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(TAKER_WORKFLOW_ID, "The Taker Ability")
        .description("Choose another player to steal their role, then see your new role.")
        .add_node(
            "select_cards",
            WorkflowNodeBuilder::new("Select a Player")
                .condition(NodeCondition::Not(Box::new(NodeCondition::ResponseExists(
                    "reveal_cards".into(),
                ))))
                .description("Pick a player to steal from")
                .visual(title_visual("{title}"))
                .visual(select_card_lists_visual(vec![
                    WorkflowDisplayBuilder::new("space-y-2")
                        .list(
                            "players",
                            "player",
                            vec![
                                WorkflowDisplayBuilder::new("w-full")
                                    .submit(
                                        vec![player_display("player")],
                                        Some(HashMap::from([(
                                            "selected_player".into(),
                                            "{player.id}".into(),
                                        )])),
                                        TAKER_STEAL_ACTION,
                                        "outline",
                                        "default",
                                    )
                                    .build(),
                            ],
                        )
                        .build(),
                ]))
                .build(),
        )
        .add_node(
            "reveal_new_role",
            create_reveal_cards_node("{display_text}")
                .condition(NodeCondition::ResponseExists("reveal_cards".into()))
                .visual(
                    WorkflowVisualBuilder::new("your-role")
                        .display(DisplayVisual::Tile {
                            content: vec![WorkflowDisplay {
                                class: "font-halloween text-center italic".into(),
                                r#type: DisplayType::Text {
                                    text: "This is now your role".into(),
                                },
                            }],
                        })
                        .build(),
                )
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(TAKER_STEAL_ACTION)
                .run_server_action()
                .target(steal_action_target_id)
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// State handler
// ----------------------------------------------------------------------------

pub fn taker_steal_card(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    // Selection must exist for *either* phase
    let target_id = ctx
        .get_input_as::<String>("selected_player")
        .map_err(|_| anyhow!("Missing selected_player"))?;

    // Phase 2: execute swap
    let taker_id = &ctx.workflow.user_id;

    let taker_player = game
        .players
        .get(taker_id)
        .ok_or_else(|| anyhow!("No player"))?
        .clone();

    let target_player = game
        .players
        .get(&target_id)
        .ok_or_else(|| anyhow!("No target player"))?
        .clone();

    // Pre-swap history context
    let who = PlayerRole::new(&taker_player);
    let what = PlayerRole::new(&target_player);

    // Do the “rob” swap (taker sees taken card; target gets taker’s prior card hidden)
    game.rob_player(taker_id, &target_id)?;

    // After swap, build reveal response: taker’s *new* effective card
    let my_new_card = game
        .players
        .get(taker_id)
        .ok_or_else(|| anyhow!("No player after swap"))?
        .effective_role_card(None);

    // Display text: “X was Y” where Y is target’s post-swap effective role
    // (this matches your original behavior, even if it’s a bit mind-bendy)
    let target_after = game
        .players
        .get(&target_id)
        .ok_or_else(|| anyhow!("No target after swap"))?
        .clone();

    // History
    game.add_history(who, "swapped roles with", vec![what]);

    // Reveal cards node expects [{ name, card }]
    let reveal_cards = json!([{ "name": my_new_card.name, "card": my_new_card }]);

    let mut r = HashMap::new();
    r.insert(
        "display_text".into(),
        json!(format!(
            "{} was {}",
            target_after.name,
            target_after.effective_role_card(None).name
        )),
    );
    r.insert("reveal_cards".into(), reveal_cards);
    r.insert("show_reveal_action".into(), json!(false));

    Ok(ServerActionResult::CompleteWorkflow {
        responses: r,
        close_active_modal: false,
        message: "".into(),
    })
}

// ----------------------------------------------------------------------------
// Registration (new pattern)
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    // 1) state handler
    let steal_action_target_id = engine
        .clone()
        .register_state_handler(TAKER_STEAL_ACTION, |game, ctx| taker_steal_card(game, ctx))
        .await?;

    // 2) workflow definition
    bot.register_workflow(taker_ability_workflow(steal_action_target_id))
        .await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn taker_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,

        id: Ulid::new().to_string(),
        role_id: TAKER_ROLE_ID.to_string(),

        name: "The Taker".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/taker.png".into(),

        priority: 7,
        alliance: Alliance::Unmarked,

        enabled,
        night_duration: 10,

        night_action: None,
        register: None,

        on_night_start: Some(Arc::new(|game: &GameState, player| {
            let user_id = player.id.clone();

            let players: Vec<PlayerResource> = game
                .players
                .values()
                .filter(|p| p.is_alive && p.id != user_id)
                .map(|p| p.into())
                .collect();

            let mut inputs = HashMap::new();
            inputs.insert("title".into(), json!("Select a player to take from"));
            inputs.insert("players".into(), json!(players));
            inputs.insert("show_reveal_action".into(), json!(false));

            Some(WorkflowStart {
                workflow_id: TAKER_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
