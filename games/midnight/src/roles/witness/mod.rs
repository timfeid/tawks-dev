use anyhow::{Result, anyhow};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{
    CreateWorkflowDefinition, NodeCondition,
    WorkflowDisplay,
};
use workflow::{WorkflowVisual, builder::*};

use crate::gamestate::{GameState, PlayerResource, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::role_card_display_for;
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const WITNESS_ROLE_ID: &str = "witness";
pub const WITNESS_WORKFLOW_ID: &str = "witness_inspect_workflow";

// Actions
pub const WITNESS_ANALYZE_ACTION: &str = "games.midnight.witness.analyze";
pub const WITNESS_REVEAL_ACTION: &str = "games.midnight.witness.reveal";

// Sabotage key
pub const WITNESS_SABOTAGE_KEY: &str = "witness";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn witness_selection_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("witness_selection")
        .tile(vec![
            // 1. Header
            WorkflowDisplayBuilder::new("w-full flex flex-col items-center justify-center mb-6")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-2xl font-halloween text-green-500 tracking-widest uppercase opacity-90 drop-shadow-sm")
                        .text("Witness Ability")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xs font-mono text-green-400/50 uppercase tracking-widest")
                        .text("Choose 1 player or 2 hidden roles (middle only) to observe")
                        .build(),
                ])
                .build(),

            // 2. Multi-Select Grid
            WorkflowDisplayBuilder::new("w-full grid grid-cols-2 gap-2")
                .input(
                    "select_cards",
                    WorkflowInputBuilder::new()
                        .r#type(InputTypeBuilder::multi_select_from_list(
                            "card_options", // items_key
                            "player_or_hidden", // as
                            "{player_or_hidden.id}",  // value
                            vec![
                                        // Avatar or Hidden Role display
                                        WorkflowDisplayBuilder::new("size-8 rounded-full border border-white/20 bg-black/50 overflow-hidden")
                                            .image("{player_or_hidden.avatar_url}", "{player_or_hidden.name}")
                                            .build(),
                                        // Name or Hidden Role text
                                        WorkflowDisplayBuilder::new("text-sm font-halloween text-white/90 truncate")
                                            .text("{player_or_hidden.name}")
                                            .build()
                            ],
                            "flex gap-3 items-center", // grid layout
                            Some(1), // min
                            Some(2)  // max, allow 2 hidden roles or 1 player
                        ))
                        .required(true)
                        .on_change_action(WITNESS_ANALYZE_ACTION, HashMap::new())
                        .build()
                )
                .build(),

            // 3. Conditional Feedback (Dynamic Instructions)
            WorkflowDisplayBuilder::new("w-full text-center text-xs font-mono text-green-400/60")
                .condition(
                    NodeCondition::ResponseEquals {
                        field: "show_reveal_action".into(),
                        value: json!(false),
                    },
                    vec![
                        WorkflowDisplayBuilder::new("text-center font-mono text-yellow-500")
                            .text("Select 2 hidden cards or 1 player to observe")
                            .build()
                    ]
                )
                .condition(
                    NodeCondition::ResponseEquals {
                        field: "show_reveal_action".into(),
                        value: json!(true),
                    },
                    vec![
                        WorkflowDisplayBuilder::new("text-center font-mono text-green-500")
                            .text("Confirm the selection?")
                            .build()
                    ]
                )
                .build(),

            // 4. Conditional "Reveal" Button
            WorkflowDisplayBuilder::new("w-full mt-6")
                .condition(
                    NodeCondition::ResponseEquals {
                        field: "show_reveal_action".into(),
                        value: json!(true),
                    },
                    vec![
                         WorkflowDisplayBuilder::new("w-full")
                            .button(
                                vec![
                                    WorkflowDisplayBuilder::new("w-full text-center font-halloween text-green-400 uppercase tracking-widest")
                                        .text("Reveal Card")
                                        .build()
                                ],
                                None,
                                WITNESS_REVEAL_ACTION,
                                "outline",
                                "lg"
                            )
                            .build()
                    ]
                )
                .build()
        ])
        .build()
}

fn witness_reveal_success_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("witness_reveal_success")
        .tile(vec![
            // Header
            WorkflowDisplayBuilder::new("w-full flex flex-col items-center justify-center py-8 space-y-6")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-4xl animate-pulse mb-2 text-center")
                        .text("🔎")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xl font-halloween text-center text-green-400")
                        .text("Insight Unveiled")
                        .build(),
                ])
                .build(),

            // ------------------------------------------------------------
            // CASE A: Player reveal (Avatar + Role Card)
            // ------------------------------------------------------------
            WorkflowDisplayBuilder::new("w-full")
                .condition(
                    NodeCondition::ResponseExists("revealed_player".into()),
                    vec![
                        WorkflowDisplayBuilder::new("w-full max-w-md mx-auto space-y-6")
                            .div(vec![
                                // 1. The Player Identity
                                WorkflowDisplayBuilder::new("bg-neutral-900/80 border border-white/10 rounded-md p-3 flex items-center gap-3")
                                    .div(vec![
                                        WorkflowDisplayBuilder::new("size-10 rounded-full border border-green-500/30 bg-black/50 overflow-hidden shadow-[0_0_10px_rgba(34,197,94,0.2)]")
                                            .image("{revealed_player.avatar_url}", "{revealed_player.name}")
                                            .build(),
                                        WorkflowDisplayBuilder::new("flex-1 min-w-0")
                                            .div(vec![
                                                WorkflowDisplayBuilder::new("text-xs font-mono text-white/50 uppercase tracking-widest")
                                                    .text("Observed Player")
                                                    .build(),
                                                WorkflowDisplayBuilder::new("text-sm font-halloween text-white/90 truncate")
                                                    .text("{revealed_player.name}")
                                                    .build(),
                                            ])
                                            .build(),
                                    ])
                                    .build(),

                                // 2. The Revealed Role Card
                                WorkflowDisplayBuilder::new("w-full flex justify-center")
                                    .div(vec![
                                        WorkflowDisplay {
                                            class: "pointer-events-none".into(),
                                            r#type: role_card_display_for("revealed_role"),
                                        }
                                    ])
                                    .build(),
                            ])
                            .build(),
                    ],
                )
                .build(),

            // ------------------------------------------------------------
            // CASE B: Two hidden reveals (Side-by-side Cards)
            // ------------------------------------------------------------
            WorkflowDisplayBuilder::new("w-full")
                .condition(
                    NodeCondition::ResponseExists("revealed_hidden".into()),
                    vec![
                        WorkflowDisplayBuilder::new("w-full max-w-2xl mx-auto")
                            .div(vec![
                                WorkflowDisplayBuilder::new("text-center text-xs font-mono text-white/50 uppercase tracking-widest mb-4")
                                    .text("Hidden Roles Revealed")
                                    .build(),

                                // Grid of 2 Cards
                                WorkflowDisplayBuilder::new("w-full grid grid-cols-2 gap-4")
                                    .list(
                                        "revealed_hidden",
                                        "item",
                                        vec![
                                            // Each item is just the full card display
                                            WorkflowDisplayBuilder::new("w-full flex justify-center")
                                                .div(vec![
                                                    WorkflowDisplay {
                                                        class: "pointer-events-none mx-auto".into(),
                                                        // Note: 'item' is the wrapper { selected:..., card:... }
                                                        // so we point to 'item.card'
                                                        r#type: role_card_display_for("item.card"),
                                                    }
                                                ])
                                                .build()
                                        ],
                                    )
                                    .build(),
                            ])
                            .build(),
                    ],
                )
                .build(),

            // Footer
            WorkflowDisplayBuilder::new("text-sm font-mono text-center text-white/50 mx-auto mt-8")
                .text("The role is now revealed in the shadows.")
                .build(),
        ])
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn witness_inspect_workflow(
    analyze_action_target_id: String,
    reveal_action_target_id: String,
) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(WITNESS_WORKFLOW_ID, "Witness Inspect")
        .description("Select a player and reveal their role.")
        // Node 1: Selection
        .add_node(
            "select_cards",
            WorkflowNodeBuilder::new("Choose Player")
                .condition(Cond::not(Cond::response_exists("done")))
                .visual(witness_selection_visual())
                .build(),
        )
        // Node 2: Reveal Success
        .add_node(
            "reveal_success",
            WorkflowNodeBuilder::new("Reveal Success")
                .parent_id("select_cards")
                .condition(Cond::response_exists("done"))
                .visual(witness_reveal_success_visual())
                .build(),
        )
        // Actions
        .add_action(
            WorkflowActionBuilder::new(WITNESS_ANALYZE_ACTION)
                .run_server_action()
                .target(analyze_action_target_id)
                .allow_while_loading() // Smoother UI updates
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(WITNESS_REVEAL_ACTION)
                .run_server_action()
                .target(reveal_action_target_id)
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// State handlers
// ----------------------------------------------------------------------------

pub fn witness_analyze(
    _game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let selected: Vec<String> = ctx.get_input_as("select_cards")?;
    let card_options: Vec<PlayerResource> = ctx.get_input_as("card_options")?;

    let mut r = HashMap::new();

    // Nothing selected
    let Some(last_id) = selected.last().cloned() else {
        r.insert("show_reveal_action".into(), json!(false));
        r.insert("title".into(), json!("Select 1 player or 2 hidden roles"));
        r.insert("select_cards".into(), json!(Vec::<String>::new()));
        return Ok(ServerActionResult::UpdateResponses(r));
    };

    let last_card = card_options
        .iter()
        .find(|c| c.id == last_id)
        .ok_or_else(|| anyhow!("Last selected id not found in card_options"))?;

    let any_player_selected = selected.iter().any(|id| {
        card_options
            .iter()
            .find(|c| c.id == *id)
            .map(|c| !c.hidden.is_some())
            .unwrap_or(false)
    });

    // Build final selection based on "latest click" rules
    let final_ids: Vec<String> = if !last_card.hidden.is_some() {
        // If they select a player, reset to JUST that latest player
        vec![last_id.clone()]
    } else {
        // Latest is hidden
        if any_player_selected {
            // If latest is hidden and a player was present, reset to JUST that hidden
            vec![last_id.clone()]
        } else {
            // Only hidden selections exist; keep at most 2, dropping oldest
            let hidden_ids_in_order: Vec<String> = selected
                .iter()
                .filter_map(|id| {
                    card_options.iter().find(|c| c.id == *id).and_then(|c| {
                        if c.hidden.is_some() {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect();

            match hidden_ids_in_order.len() {
                0 => vec![last_id.clone()],
                1 => hidden_ids_in_order,
                2 => hidden_ids_in_order, // show confirm
                _ => hidden_ids_in_order[hidden_ids_in_order.len() - 2..].to_vec(), // drop oldest
            }
        }
    };

    // Confirm rules:
    // - show confirm if exactly 1 player (handled by player branch)
    // - OR exactly 2 hidden
    let show_confirm = if !last_card.hidden.is_some() {
        true
    } else {
        final_ids.len() == 2
    };

    r.insert("select_cards".into(), json!(final_ids));
    r.insert("show_reveal_action".into(), json!(show_confirm));
    r.insert(
        "title".into(),
        json!(if show_confirm {
            if !last_card.hidden.is_some() {
                "Confirm selection to reveal this player's role"
            } else {
                "Confirm selection to reveal these hidden roles"
            }
        } else {
            "Select 2 hidden cards or 1 player to observe"
        }),
    );

    Ok(ServerActionResult::UpdateResponses(r))
}

pub fn witness_reveal(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let user_id = ctx.workflow.user_id.clone();

    // Same loader approach as analyze
    let selected: Vec<String> = ctx.get_input_as("select_cards")?;
    let card_options: Vec<PlayerResource> = ctx.get_input_as("card_options")?;

    // Resolve selected -> PlayerResource(s), preserving selection order
    let selected_cards: Vec<PlayerResource> = selected
        .iter()
        .filter_map(|id| card_options.iter().find(|c| c.id == *id).cloned())
        .collect();

    // Split into player vs hidden
    let mut players: Vec<PlayerResource> = Vec::new();
    let mut hiddens: Vec<PlayerResource> = Vec::new();

    for c in &selected_cards {
        if c.hidden.is_some() {
            hiddens.push(c.clone());
        } else {
            players.push(c.clone());
        }
    }

    // Must match analyze confirm logic:
    // - exactly 1 player OR exactly 2 hidden
    if players.len() == 1 && hiddens.is_empty() && selected_cards.len() == 1 {
        // --- reveal player role ---
        let player_id = &players[0].id;
        let player = game
            .players
            .get(player_id)
            .ok_or_else(|| anyhow!("Player not found"))?;

        let player_role = player.effective_role_card(None);

        let mut response = HashMap::new();
        response.insert("revealed_player".into(), json!(player));
        response.insert("revealed_role".into(), json!(player_role));
        response.insert("done".into(), true.into());

        // History
        let who = PlayerRole::new(&game.players[&user_id]);
        game.add_history(who, "observed role of", vec![PlayerRole::new(player)]);

        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            message: "Role revealed successfully.".to_string(),
            responses: response,
        });
    }

    if players.is_empty() && hiddens.len() == 2 && selected_cards.len() == 2 {
        // --- reveal 2 hidden roles ---
        // Your game stores hidden cards; PlayerResource for hidden should contain the index.
        // Use whatever accessor you already have.
        //
        // Common patterns:
        //  - PlayerResource::hidden_index: Option<usize>
        //  - method like `as_hidden_index()`
        //
        // I’ll assume you have `hidden_index: Option<usize>`; if not, replace the 2 lines
        // that compute ia/ib with your existing way.

        let ia = hiddens[0]
            .hidden
            .ok_or_else(|| anyhow!("Hidden selection missing hidden"))?;
        let ib = hiddens[1]
            .hidden
            .ok_or_else(|| anyhow!("Hidden selection missing hidden"))?;

        let ca = game
            .hidden_cards
            .get(ia)
            .ok_or_else(|| anyhow!("Hidden card not found at index {}", ia))?;
        let cb = game
            .hidden_cards
            .get(ib)
            .ok_or_else(|| anyhow!("Hidden card not found at index {}", ib))?;

        // Shape this however your reveal UI expects.
        // Since analyze sets confirm for 2 hidden, return both here.
        let mut response = HashMap::new();
        response.insert(
            "revealed_hidden".into(),
            json!([
                { "selected": hiddens[0], "card": ca },
                { "selected": hiddens[1], "card": cb },
            ]),
        );
        response.insert("done".into(), true.into());

        // History (optional wording)
        let who = PlayerRole::new(&game.players[&user_id]);
        game.add_history(who, "observed hidden roles", vec![]);

        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            message: "Hidden roles revealed successfully.".to_string(),
            responses: response,
        });
    }

    Err(anyhow!(
        "Invalid selection: must be exactly 1 player or exactly 2 hidden roles"
    ))
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    let analyze_target_id = engine
        .clone()
        .register_state_handler(WITNESS_ANALYZE_ACTION, |game, ctx| {
            witness_analyze(game, ctx)
        })
        .await?;

    let reveal_target_id = engine
        .clone()
        .register_state_handler(WITNESS_REVEAL_ACTION, |game, ctx| witness_reveal(game, ctx))
        .await?;

    bot.register_workflow(witness_inspect_workflow(
        analyze_target_id,
        reveal_target_id,
    ))
    .await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn witness_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: WITNESS_ROLE_ID.to_string(),
        name: "The Witness".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/witness.png".into(),
        priority: 5,
        alliance: Alliance::Unmarked,
        enabled,
        night_duration: 0, // The Witness doesn't act at night directly, but uses workflows.
        night_action: None, // No immediate action, handled via the workflow.
        register: None,

        // Trigger workflow when night starts for the Witness role
        on_night_start: Some(Arc::new(|game: &GameState, player| {
            let user_id = player.id.clone();

            // Collect all players (excluding the current one) that are alive
            let players: Vec<PlayerResource> = game
                .players
                .values()
                .filter(|p| p.is_alive && p.id != user_id)
                .map(|p| p.into())
                .collect();

            // Collect all hidden cards available in the game
            let hidden_roles_len = game.hidden_cards.len();
            let hidden_roles: Vec<PlayerResource> = (0..hidden_roles_len)
                .map(|i| PlayerResource::new_from_hidden(i))
                .collect();

            let mut inputs = HashMap::new();
            // Combine player and hidden roles
            let mut combined_roles = players.clone();
            combined_roles.extend(hidden_roles);

            // Pass combined options to the workflow
            inputs.insert("card_options".into(), json!(combined_roles));
            inputs.insert("show_reveal_action".into(), json!(false));

            Some(WorkflowStart {
                workflow_id: WITNESS_WORKFLOW_ID.to_string(), // Trigger the Witness workflow
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
