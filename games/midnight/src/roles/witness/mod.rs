use anyhow::{Result, anyhow};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{CreateWorkflowDefinition, NodeCondition};
use workflow::{WorkflowVisual, builder::*, ui::*};

use crate::MidnightPage;
use crate::gamestate::{GameState, PlayerResource, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::{ability_header, role_card};
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
    let reveal_ready = |ready: bool| NodeCondition::ResponseEquals {
        field: "show_reveal_action".into(),
        value: json!(ready),
    };
    WorkflowVisualBuilder::new("witness_selection")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                ability_header(
                    "Witness Ability",
                    Tone::Green,
                    "Choose 1 player or 2 hidden roles (middle only) to observe",
                ),
                WorkflowDisplayBuilder::new()
                    .input(
                        "select_cards",
                        WorkflowInputBuilder::new()
                            .r#type(InputTypeBuilder::multi_select_from_list(
                                "card_options",
                                "player_or_hidden",
                                "{player_or_hidden.id}",
                                vec![
                                    row(
                                        Gap::Normal,
                                        vec![
                                            avatar(
                                                "{player_or_hidden.avatar_url}",
                                                "{player_or_hidden.name}",
                                                Size::Small,
                                            ),
                                            body("{player_or_hidden.name}").themed(),
                                        ],
                                    )
                                    .nowrap(),
                                ],
                                2,
                                Some(1),
                                // Two hidden roles or one player.
                                Some(2),
                            ))
                            .required(true)
                            .on_change_action(WITNESS_ANALYZE_ACTION, HashMap::new())
                            .build(),
                    )
                    .build(),
                when(
                    reveal_ready(false),
                    vec![
                        fine("Select 2 hidden cards or 1 player to observe")
                            .centered()
                            .tone(Tone::Warning),
                    ],
                ),
                when(
                    reveal_ready(true),
                    vec![
                        fine("Confirm the selection?")
                            .centered()
                            .tone(Tone::Success),
                    ],
                ),
                when(
                    reveal_ready(true),
                    vec![primary_action("Reveal Card", act(WITNESS_REVEAL_ACTION)).full_width()],
                ),
            ],
        )
        .build()
}

fn witness_reveal_success_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("witness_reveal_success")
        .midnight_page(
            PageWidth::Narrow,
            vec![
                column(
                    Gap::Tight,
                    vec![
                        icon("search", IconSize::Large).tone(Tone::Green),
                        title("Insight Unveiled")
                            .themed()
                            .centered()
                            .tone(Tone::Green),
                    ],
                )
                .align(Align::Center),
                when(
                    NodeCondition::ResponseExists("revealed_player".into()),
                    vec![column(
                        Gap::Normal,
                        vec![
                            list_row("{revealed_player.name}")
                                .meta("Observed Player")
                                .lead_with(vec![avatar(
                                    "{revealed_player.avatar_url}",
                                    "{revealed_player.name}",
                                    Size::Medium,
                                )]),
                            role_card("revealed_role"),
                        ],
                    )],
                ),
                when(
                    NodeCondition::ResponseExists("revealed_hidden".into()),
                    vec![column(
                        Gap::Normal,
                        vec![
                            eyebrow("Hidden Roles Revealed").centered(),
                            // Each item wraps its card: { selected, card }.
                            items(
                                "revealed_hidden",
                                "item",
                                Layout::Grid {
                                    columns: 2,
                                    gap: Gap::Normal,
                                    fixed: false,
                                },
                                vec![role_card("item.card")],
                            ),
                        ],
                    )],
                ),
                fine("The role is now revealed in the shadows.").centered(),
            ],
        )
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
