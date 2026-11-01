use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{CreateWorkflowDefinition, NodeCondition, WorkflowDisplay};
use workflow::{WorkflowVisual, builder::*, ui::*};

use crate::MidnightPage;
use crate::gamestate::{GameState, PlayerResource, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::{ability_header, single_response};
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const TANGLE_ROLE_ID: &str = "tangle";
pub const TANGLE_WORKFLOW_ID: &str = "tangle_swap_workflow";

// Actions
pub const TANGLE_ANALYZE_ACTION: &str = "games.midnight.tangle.analyze";
pub const TANGLE_EXECUTE_ACTION: &str = "games.midnight.tangle.execute";

// Sabotage key
pub const TANGLE_SABOTAGE_KEY: &str = "tangle";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn tangle_selection_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("tangle_selection")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                ability_header("Entanglement", Tone::Green, "Select two souls to swap"),
                WorkflowDisplayBuilder::new()
                    .input(
                        "select_cards",
                        WorkflowInputBuilder::new()
                            .r#type(InputTypeBuilder::multi_select_from_list(
                                "card_options",
                                "player",
                                "{player.id}",
                                vec![
                                    row(
                                        Gap::Normal,
                                        vec![
                                            avatar(
                                                "{player.avatar_url}",
                                                "{player.name}",
                                                Size::Small,
                                            ),
                                            body("{player.name}").themed(),
                                        ],
                                    )
                                    .nowrap(),
                                ],
                                2,
                                Some(2),
                                Some(2),
                            ))
                            .required(true)
                            .on_change_action(TANGLE_ANALYZE_ACTION, HashMap::new())
                            .build(),
                    )
                    .build(),
                when(
                    NodeCondition::ResponseEquals {
                        field: "show_tangle_action".into(),
                        value: json!(true),
                    },
                    vec![primary_action("Execute Swap", act(TANGLE_EXECUTE_ACTION)).full_width()],
                ),
            ],
        )
        .build()
}

fn swapped_player(var: &str) -> WorkflowDisplay {
    panel(vec![person(
        format!("{{{var}.avatar_url}}"),
        format!("{{{var}.name}}"),
    )])
}

fn tangle_success_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("tangle_success")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                column(
                    Gap::Tight,
                    vec![
                        icon("shuffle", IconSize::Large).tone(Tone::Green),
                        title("Fates Entwined")
                            .themed()
                            .centered()
                            .tone(Tone::Green),
                    ],
                )
                .align(Align::Center),
                row(
                    Gap::Normal,
                    vec![
                        swapped_player("swapped_p1"),
                        icon("arrow-right", IconSize::Medium).tone(Tone::Green),
                        swapped_player("swapped_p2"),
                    ],
                )
                .justify(Justify::Center),
                fine("Their roles have been exchanged in the shadows.").centered(),
            ],
        )
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn tangle_swap_workflow(
    analyze_action_target_id: String,
    execute_action_target_id: String,
) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(TANGLE_WORKFLOW_ID, "Tangle Swap")
        .description("Choose two players to tangle (swap).")
        // Node 1: Selection
        .add_node(
            "select_cards",
            WorkflowNodeBuilder::new("Tangle two players")
                .condition(Cond::not(Cond::response_exists("done")))
                // Removed title_visual in favor of the custom header in tangle_selection_visual
                .visual(tangle_selection_visual())
                .build(),
        )
        // Node 2: Success
        .add_node(
            "success",
            WorkflowNodeBuilder::new("Tangle Complete")
                .parent_id("select_cards")
                .condition(Cond::response_exists("done"))
                .visual(tangle_success_visual())
                .build(),
        )
        // Actions
        .add_action(
            WorkflowActionBuilder::new(TANGLE_ANALYZE_ACTION)
                .run_server_action()
                .target(analyze_action_target_id)
                .allow_while_loading() // Smoother UI updates
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(TANGLE_EXECUTE_ACTION)
                .run_server_action()
                .target(execute_action_target_id)
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// State handlers
// ----------------------------------------------------------------------------

pub fn tangle_analyze(
    _game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let selected = ctx
        .workflow
        .responses
        .get("select_cards")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut r = HashMap::new();

    match selected.len() {
        2 => {
            r.insert("show_tangle_action".into(), json!(true));
        }
        _ => {
            // Keep trimmed logic if the UI allows over-selection, though 'max: 2' helps.
            if selected.len() > 2 {
                let trimmed: Vec<Value> = selected.iter().rev().take(2).rev().cloned().collect();
                r.insert("select_cards".into(), json!(trimmed));
                r.insert("show_tangle_action".into(), json!(true));
            } else {
                r.insert("show_tangle_action".into(), json!(false));
            }
        }
    }

    Ok(ServerActionResult::UpdateResponses(r))
}

pub fn tangle_execute(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let user_id = ctx.workflow.user_id.clone();

    // ------------------------------------------------------------------
    // FIX: Scope the immutable borrow
    // We extract 'who' and the sabotage check inside this block.
    // When the block ends, the reference to 'player' (and thus 'game') is dropped.
    // ------------------------------------------------------------------
    let (who, is_saboteur) = {
        let player = game
            .players
            .get(&user_id)
            .ok_or_else(|| anyhow!("No player"))?;

        let who = PlayerRole::new(player);
        let my_role = player.effective_role_card(None);

        (who, my_role.name == "The Saboteur")
    };
    // ------------------------------------------------------------------
    // Now 'game' is free to be borrowed mutably because 'player' is gone.
    // ------------------------------------------------------------------

    // Selection as submitted
    let submitted = ctx
        .workflow
        .responses
        .get("select_cards")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if submitted.len() != 2 {
        return Err(anyhow!("Must select exactly 2 cards"));
    }

    // --- Sabotage Logic ---
    if is_saboteur {
        game.set_sabotage_inputs(
            TANGLE_SABOTAGE_KEY,
            &ctx.workflow.workflow_id,
            (who.clone(), ctx.workflow.responses.clone()),
        );

        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            message: "Sabotage complete.".to_string(),
            responses: single_response("done", true),
        });
    }

    // --- Resolve Sabotage ---
    let (_sabotaged_by, sabotage_inputs) = game
        .get_sabotage_inputs(TANGLE_SABOTAGE_KEY, &ctx.workflow.workflow_id)
        .map(|(by, inputs)| (Some(by), Some(inputs)))
        .unwrap_or((None, None));

    let effective_inputs = if let Some(inputs) = sabotage_inputs {
        // Log "expected to tangle" history
        let mut reveals = vec![];
        for card in &submitted {
            let pid = card.as_str().ok_or_else(|| anyhow!("Invalid ID"))?;
            // Using game.players.get here is fine because it's a temporary borrow
            // that ends immediately after this loop, before we swap.
            if let Some(p) = game.players.get(pid) {
                reveals.push(PlayerRole::new(p));
            }
        }
        game.add_history(who.clone(), "expected to tangle", reveals);
        game.clear_sabotage_inputs(TANGLE_SABOTAGE_KEY, &ctx.workflow.workflow_id);
        inputs
    } else {
        ctx.workflow.responses.clone()
    };

    // Final selection
    let selected = effective_inputs
        .get("select_cards")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if selected.len() != 2 {
        return Err(anyhow!(
            "Internal error: effective selection was not exactly 2"
        ));
    }

    let first_id = selected[0].as_str().ok_or_else(|| anyhow!("Invalid ID"))?;
    let second_id = selected[1].as_str().ok_or_else(|| anyhow!("Invalid ID"))?;

    // Perform Swap (Mutable Borrow)
    game.swap_players(first_id, second_id)?;

    // Extract everything we need from p1/p2 into owned values
    let (p1_role, p2_role, p1_res, p2_res) = {
        let p1 = game
            .players
            .get(first_id)
            .ok_or_else(|| anyhow!("P1 not found"))?;
        let p2 = game
            .players
            .get(second_id)
            .ok_or_else(|| anyhow!("P2 not found"))?;

        let p1_role = PlayerRole::new(p1);
        let p2_role = PlayerRole::new(p2);

        // These should be owned resources (no references into `game`)
        let p1_res: PlayerResource = p1.into();
        let p2_res: PlayerResource = p2.into();

        (p1_role, p2_role, p1_res, p2_res)
    }; // <- p1/p2 borrows end here

    // Now it's legal to mutably borrow `game`
    game.add_history(who, "swapped cards", vec![p1_role, p2_role]);

    Ok(ServerActionResult::CompleteWorkflow {
        close_active_modal: false,
        responses: {
            let mut r = single_response("done", true);
            r.insert("show_tangle_action".into(), json!(false));
            r.insert("title".into(), json!("Fates Entwined"));
            r.insert("swapped_p1".into(), json!(p1_res));
            r.insert("swapped_p2".into(), json!(p2_res));
            r
        },
        message: "Tangled cards successfully.".into(),
    })
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    let analyze_target_id = engine
        .clone()
        .register_state_handler(TANGLE_ANALYZE_ACTION, |game, ctx| tangle_analyze(game, ctx))
        .await?;

    let execute_target_id = engine
        .clone()
        .register_state_handler(TANGLE_EXECUTE_ACTION, |game, ctx| tangle_execute(game, ctx))
        .await?;

    bot.register_workflow(tangle_swap_workflow(analyze_target_id, execute_target_id))
        .await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn tangle_card() -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: TANGLE_ROLE_ID.to_string(),
        name: "The Tangle".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/tangle.png".into(),
        priority: 7,
        alliance: Alliance::Unmarked,
        enabled: false,
        night_duration: 10,
        night_action: None,
        register: None,

        on_night_start: Some(Arc::new(|game: &GameState, player| {
            let user_id = player.id.clone();
            let options: Vec<PlayerResource> = game
                .players
                .values()
                .filter(|p| p.is_alive && p.id != user_id)
                .map(|p| p.into())
                .collect();

            let mut inputs = HashMap::new();
            inputs.insert("card_options".into(), json!(options));
            inputs.insert("show_tangle_action".into(), json!(false));

            Some(WorkflowStart {
                workflow_id: TANGLE_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
