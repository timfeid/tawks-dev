use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::CreateWorkflowDefinition;
use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{WorkflowVisual, builder::*, ui::*};

use crate::MidnightPage;
use crate::ability_header;
use crate::gamestate::{GameState, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const LOST_ROLE_ID: &str = "lost";
pub const LOST_WORKFLOW_ID: &str = "lost_ability_workflow";
pub const LOST_SWAP_ACTION: &str = "games.midnight.lost.swap_hidden";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn lost_selection_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("lost_selection")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                ability_header(
                    "The Void Beckons",
                    Tone::Purple,
                    "Abandon your identity. Seize another.",
                ),
                items(
                    "card_options",
                    "card",
                    Layout::Grid {
                        columns: 3,
                        gap: Gap::Normal,
                        fixed: false,
                    },
                    vec![choice(
                        vec![title("?").themed().tone(Tone::Purple), fine("{card.name}")],
                        act(LOST_SWAP_ACTION).input("hidden_index", "{card.value}"),
                    )],
                ),
            ],
        )
        .build()
}

fn lost_success_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("lost_success")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                column(
                    Gap::Tight,
                    vec![
                        icon("moon", IconSize::Large).tone(Tone::Purple),
                        ability_header(
                            "Identity Veiled",
                            Tone::Purple,
                            "The void has reshaped you, but your true form remains hidden in the shadows...",
                        ),
                    ],
                )
                .align(Align::Center),
                panel(vec![display_title("?").themed().centered().tone(Tone::Purple)])
                    .tone(Tone::Purple),
                fine("Trust no one. Not even yourself.").centered(),
            ],
        )
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn lost_ability_workflow(swap_action_target_id: String) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(LOST_WORKFLOW_ID, "The Lost Ability")
        .description("Choose a hidden card to blindly swap with.")
        // Node 1: Selection
        .add_node(
            "choose_hidden",
            WorkflowNodeBuilder::new("The Void")
                .condition(Cond::not(Cond::response_exists("swapped")))
                .visual(lost_selection_visual())
                .build(),
        )
        // Node 2: Success / Reveal
        .add_node(
            "reveal",
            WorkflowNodeBuilder::new("Reborn")
                .parent_id("choose_hidden")
                .condition(Cond::response_equals("swapped", json!(true)))
                .visual(lost_success_visual())
                .build(),
        )
        // Actions
        .add_action(
            WorkflowActionBuilder::new(LOST_SWAP_ACTION)
                .run_server_action()
                .target(swap_action_target_id)
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// Server action handler
// ----------------------------------------------------------------------------

pub fn lost_swap_hidden(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let index = ctx
        .get_input_as::<String>("hidden_index")
        .map_err(|_| anyhow!("Missing hidden_index"))?
        .parse::<usize>()
        .map_err(|_| anyhow!("Invalid hidden_index"))?;

    let user_id = &ctx.workflow.user_id;
    let player = game
        .players
        .get(user_id)
        .ok_or_else(|| anyhow!("No player"))?;
    let player_role = PlayerRole::new(player);

    // 1. Get current data (Immutable borrow logic)
    let (player_role_card, _player_name) =
        { (player.effective_role_card(None), player.name.clone()) };

    // 2. Perform Swap (Mutable borrow)
    // get_hidden_card clones, so we are safe
    let hidden_card = game.get_hidden_card(index)?;

    // Set the hidden card to what the player *was*
    game.set_hidden_card(index, player_role_card);

    // Set the player to what the hidden card *is*
    game.set_player_role(user_id, hidden_card.clone());

    game.add_history(
        player_role,
        "swapped with a hidden role",
        vec![PlayerRole::new_from_hidden(index, hidden_card.clone())],
    );

    // 4. Return result with the NEW role for display
    let mut r = HashMap::new();
    r.insert("swapped".into(), json!(true));
    r.insert("title".into(), json!("Identity Reformed"));

    Ok(ServerActionResult::CompleteWorkflow {
        responses: r,
        close_active_modal: false,
        message: "nice".into(),
    })
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    let swap_action_target_id = engine
        .clone()
        .register_state_handler(LOST_SWAP_ACTION, |game, ctx| lost_swap_hidden(game, ctx))
        .await?;

    bot.register_workflow(lost_ability_workflow(swap_action_target_id))
        .await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn lost_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: LOST_ROLE_ID.to_string(),
        name: "The Lost".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/lost.png".into(),
        priority: 10,
        alliance: Alliance::Unmarked,
        enabled,
        night_duration: 0,
        night_action: None,
        register: None,

        on_night_start: Some(Arc::new(|game: &GameState, _player| {
            // Center Cards (Hidden Roles)
            let options: Vec<Value> = (0..game.hidden_cards.len())
                .map(|i| {
                    json!({
                        // "Center 1", "Center 2" sounds better than "Hidden Role 1" for UI
                        "name": format!("Center {}", i + 1),
                        "value": i,
                    })
                })
                .collect();

            let mut inputs = HashMap::new();
            inputs.insert("card_options".into(), json!(options));

            Some(WorkflowStart {
                workflow_id: LOST_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
