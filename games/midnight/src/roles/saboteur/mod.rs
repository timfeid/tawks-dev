//! Refactor target: new registration style (no registrar).
//!
//! Pattern:
//! pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
//!   let sa_id = engine.register_state_handler(...).await?;
//!   bot.register_workflow(workflow_def(sa_id, ...)).await?;
//!   Ok(())
//! }
//!
//! Notes:
//! - RoleCard has `register: Option<RegisterHook>` but that hook signature is still
//!   `Fn(Arc<dyn RoleRegistrar>) -> ...` in your pasted reminder. Since you said you removed
//!   the registrar layer, this file exposes `register(bot, engine)` directly (like Lost).
//!   Wire it into your role registration bootstrap the same way you did for Lost.
//! - Saboteur uses `on_night_start` to inject its own workflow into the game tile.
//! - Workflow nodes are mutually exclusive and never depend on missing keys.
//! - Uses `role_id` (string like "witness") consistently, not card.id ULID.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde_json::{Value, json};
use tracing::warn;
use ulid::Ulid;

use tawks_bot::TawksBot;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{CreateWorkflowDefinition, DisplayVisual, NodeCondition};

use crate::gamestate::{GameState, Player, PlayerRole, canonical_workflow_id};
use crate::roles::{Alliance, RoleCard, WorkflowHook, WorkflowStart};
use crate::title_visual;
use game_framework::engine::GameEngine;

// =============================================================================
// Public constants (server action names)
// =============================================================================

pub const SABOTEUR_START_ACTION: &str = "games.midnight.roles.saboteur.start";
pub const SABOTEUR_RESULTS_ACTION: &str = "games.midnight.roles.saboteur.results";
pub const SABOTEUR_WORKFLOW_DEFINITION: &str = "saboteur_sabotage_workflow";
pub const SABOTEUR_WORKFLOW_DEFINITION_BOT: &str = "wf-saboteur_sabotage_workflow-bot";

mod keys {
    pub const NO_ROLES: &str = "no_roles"; // bool
    pub const ROLE_NAME: &str = "role_name"; // string
    pub const ROLE_ID: &str = "role_id"; // role_id string (e.g. "witness"), NOT card.id/ULID
    pub const CHILD_DONE: &str = "child_done"; // bool or non-null marker injected on completion
}

// =============================================================================
// Registration entrypoint (new style)
// =============================================================================

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    // 1) state handlers
    let results_action_target_id = engine
        .clone()
        .register_state_handler(SABOTEUR_RESULTS_ACTION, |game, ctx| {
            saboteur_results(game, ctx)
        })
        .await?;

    let start_action_target_id = engine
        .clone()
        .register_state_handler(SABOTEUR_START_ACTION, |game, ctx| saboteur_start(game, ctx))
        .await?;

    // 2) workflow definition
    bot.register_workflow(saboteur_sabotage_workflow(
        start_action_target_id,
        results_action_target_id,
    ))
    .await?;

    Ok(())
}

// =============================================================================
// Server action handlers (run inside lobby actor)
// =============================================================================

fn saboteur_results(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let no_roles = ctx.get_input_as::<bool>(keys::NO_ROLES).unwrap_or(false);
    if no_roles {
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses: HashMap::new(),
            message: "".into(),
        });
    }

    let role_id = ctx
        .get_input_as::<String>(keys::ROLE_ID)
        .ok()
        .unwrap_or_else(|| "unknown".into());

    let role_name = ctx
        .get_input_as::<String>(keys::ROLE_NAME)
        .ok()
        .unwrap_or_else(|| role_id.clone());

    let Some(actor) = game.players.get(&ctx.workflow.user_id) else {
        warn!("saboteur_results: actor not found");
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses: HashMap::new(),
            message: "".into(),
        });
    };

    let who = PlayerRole::new(actor);

    let targets = game
        .players
        .values()
        .filter(|p| p.is_alive && p.effective_role_card(None).role_id == role_id)
        .map(PlayerRole::new)
        .collect::<Vec<_>>();

    game.add_history(who, format!("sabotaged {role_name}").as_str(), targets);

    Ok(ServerActionResult::CompleteWorkflow {
        close_active_modal: false,
        responses: HashMap::new(),
        message: "".into(),
    })
}

fn saboteur_start(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let candidates = game.get_sabotage_candidates(["lost", "witness", "tangle", "spy"]);
    let selected = game.pick_random_role(&candidates).cloned();

    let Some(selected) = selected else {
        let mut responses = HashMap::<String, Value>::new();
        responses.insert(keys::NO_ROLES.into(), json!(true));
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses,
            message: "No role to sabotage.".into(),
        });
    };

    let Some(actor) = game.players.get(&ctx.workflow.user_id) else {
        let mut responses = HashMap::<String, Value>::new();
        responses.insert(keys::NO_ROLES.into(), json!(true));
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses,
            message: "You are not in the game.".into(),
        });
    };

    // Who are we sabotaging?
    let victim_ids: Vec<String> = game
        .players
        .values()
        .filter(|p| p.is_alive && p.effective_role_card(None).role_id == selected.role_id)
        .map(|p| p.id.clone())
        .collect();

    // If nobody has that role, nothing to do.
    if victim_ids.is_empty() {
        let mut responses = HashMap::<String, Value>::new();
        responses.insert(keys::NO_ROLES.into(), json!(false));
        responses.insert(keys::ROLE_NAME.into(), json!(selected.name));
        responses.insert(keys::ROLE_ID.into(), json!(selected.role_id));
        responses.insert(keys::CHILD_DONE.into(), json!(true));
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses,
            message: "Nobody has that role.".into(),
        });
    }

    // The hook we will sabotage (must exist)
    let Some(hook) = &selected.on_night_start else {
        let mut responses = HashMap::<String, Value>::new();
        responses.insert(keys::NO_ROLES.into(), json!(false));
        responses.insert(keys::ROLE_NAME.into(), json!(selected.name));
        responses.insert(keys::ROLE_ID.into(), json!(selected.role_id));
        responses.insert(keys::CHILD_DONE.into(), json!(true));
        return Ok(ServerActionResult::CompleteWorkflow {
            close_active_modal: false,
            responses,
            message: "Role has no workflow to sabotage.".into(),
        });
    };

    // Record history immediately
    let who = PlayerRole::new(actor);
    let targets = victim_ids
        .iter()
        .filter_map(|vid| game.players.get(vid))
        .map(PlayerRole::new)
        .collect::<Vec<_>>();
    game.add_history(
        who,
        format!("sabotaged {}", selected.name).as_str(),
        targets,
    );

    // For EACH victim, compute their workflow id + store sabotage plan for later injection.
    for vid in victim_ids {
        let Some(victim) = game.players.get(&vid) else {
            continue;
        };

        // IMPORTANT: call hook with the victim, not the saboteur
        let Some(WorkflowStart {
            workflow_id,
            mut inputs,
            context: _,
        }) = hook(game, victim)
        else {
            continue;
        };

        // Decide what sabotage means:
        // - easiest: mark a flag the victim role handlers understand
        // - or force a specific selection key
        //
        // Keep it generic:
        // inputs.insert("$sabotaged".into(), json!(true));

        // Example: force victim’s ability to choose some target deterministically
        // (replace these keys with your real role input keys)
        inputs.insert("$sabotaged".into(), json!(true));
        inputs.insert("$sabotage_seed".into(), json!(Ulid::new().to_string()));

        // Store override inputs for (victim_user_id, workflow_id)
        let key = canonical_workflow_id(&workflow_id).to_string();
        game.set_sabotage_inputs(&vid, &key, (PlayerRole::new(victim), inputs));
    }

    // Saboteur workflow done; the actual sabotage happens later at injection time.
    let mut responses = HashMap::<String, Value>::new();
    responses.insert(keys::NO_ROLES.into(), json!(false));
    responses.insert(keys::ROLE_NAME.into(), json!(selected.name));
    responses.insert(keys::ROLE_ID.into(), json!(selected.role_id));
    responses.insert(keys::CHILD_DONE.into(), json!(true));

    Ok(ServerActionResult::CompleteWorkflow {
        close_active_modal: false,
        responses,
        message: format!("You sabotaged {}", selected.name),
    })
}

// =============================================================================
// Workflow definition (mutually exclusive nodes)
// =============================================================================

pub fn saboteur_sabotage_workflow(
    start_action_target_id: String,
    results_action_target_id: String,
) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(SABOTEUR_WORKFLOW_DEFINITION, "Saboteur Sabotage")
        .description("Sabotage a random role and trigger its workflow")
        .starting_server_action(start_action_target_id.clone())
        // No roles (exclusive)
        .add_node(
            "no_roles",
            WorkflowNodeBuilder::new("No valid roles to sabotage")
                .condition(Cond::response_equals(keys::NO_ROLES, json!(true)))
                .description("No roles")
                .visual(title_visual("Better luck next time"))
                .build(),
        )
        // In progress (exclusive)
        .add_node(
            "start_node",
            WorkflowNodeBuilder::new("Sabotaging…")
                .condition(NodeCondition::And(vec![
                    NodeCondition::Not(Box::new(Cond::response_equals(
                        keys::NO_ROLES,
                        json!(true),
                    ))),
                    NodeCondition::Not(Box::new(Cond::response_equals(
                        keys::CHILD_DONE,
                        json!(true),
                    ))),
                ]))
                .description("You are sabotaging a random role.")
                .visual(title_visual("Sabotaging…"))
                .visual(
                    WorkflowVisualBuilder::new("sabotage")
                        .display(DisplayVisual::Tile { content: vec![] })
                        .build(),
                )
                .build(),
        )
        // Finished (exclusive)
        .add_node(
            "finished",
            WorkflowNodeBuilder::new("Done")
                .condition(Cond::response_equals(keys::CHILD_DONE, json!(true)))
                .description("You sabotaged a random role.")
                .visual(title_visual("You sabotaged {role_name}"))
                .visual(
                    WorkflowVisualBuilder::new("sabotage")
                        .display(DisplayVisual::Tile { content: vec![] })
                        .build(),
                )
                .build(),
        )
        // Actions (optional; useful for debugging/manual triggers)
        .add_action(
            WorkflowActionBuilder::new(SABOTEUR_START_ACTION)
                .run_server_action()
                .target(start_action_target_id)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(SABOTEUR_RESULTS_ACTION)
                .run_server_action()
                .target(results_action_target_id)
                .build(),
        )
        .build()
}

// =============================================================================
// Role card + night hook
// =============================================================================

pub fn saboteur_on_night_start() -> WorkflowHook {
    Arc::new(|_game: &GameState, _player: &Player| {
        Some(WorkflowStart {
            workflow_id: SABOTEUR_WORKFLOW_DEFINITION_BOT.to_string(),
            inputs: HashMap::new(),
            context: HashMap::new(),
        })
    })
}

pub fn saboteur_card() -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        id: Ulid::new().to_string(),
        role_id: "saboteur".to_string(),
        name: "The Saboteur".to_string(),
        description: include_str!("./description.html").to_string(),
        alliance: Alliance::Hollow,
        image_url: "https://cdn.tawks.app/midnight/roles/saboteur.png".into(),

        // If you want it to have its own turn in the night queue, set priority > 0.
        // Otherwise it can still be triggered via other means.
        priority: 0,
        night_duration: 15,
        enabled: false,

        night_action: None,
        winner_check: None,

        // You said you removed registrar — so leave register None here and call `register(bot, engine)`
        // from your central role registration bootstrap (like Lost).
        register: None,

        on_night_start: Some(saboteur_on_night_start()),
    })
}
