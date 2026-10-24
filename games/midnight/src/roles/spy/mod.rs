use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{
    CreateWorkflowDefinition, DisplayType, WorkflowDisplay, WorkflowPredicate,
};
use workflow::{WorkflowVisual, builder::*};

use crate::gamestate::{GameState, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::role_card_display_for;
use game_framework::engine::GameEngine;

// ----------------------------------------------------------------------------
// Constants
// ----------------------------------------------------------------------------

pub const SPY_ROLE_ID: &str = "spy";
pub const SPY_WORKFLOW_ID: &str = "spy_observe_workflow";

// Actions
pub const SPY_START_OBSERVE_ACTION: &str = "games.midnight.spy.start_observe";

// Sabotage stash key
pub const SPY_SABOTAGE_KEY: &str = "spy";

// ----------------------------------------------------------------------------
// Visuals
// ----------------------------------------------------------------------------

fn spy_target_acquisition_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("spy_target_acquisition")
        .tile(vec![
            // 1. Header
            WorkflowDisplayBuilder::new("w-full flex flex-col items-center justify-center mb-6")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-2xl font-halloween text-green-500 tracking-widest uppercase opacity-90 drop-shadow-sm")
                        .text("Target Acquisition")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xs font-mono text-green-400/50 uppercase tracking-widest")
                        .text("Select a signature to intercept")
                        .build(),
                ])
                .build(),

            // 2. The Grid
            WorkflowDisplayBuilder::new("w-full grid grid-cols-2 gap-3")
                .list(
                    "available_roles",
                    "role",
                    vec![
                        // BUTTON WRAPPER
                        // !p-0: Removes default button padding so the card touches the edges.
                        // !h-auto: Stops the button from forcing a specific height.
                        // !bg-transparent: Ensures the button's own hover effect doesn't show up behind your card.
                        WorkflowDisplayBuilder::new("w-full h-auto p-0 bg-transparent hover:bg-transparent border-0 focus:outline-none")
                        .submit(
                                        vec![
                                            // INNER CARD CONTAINER
                                            WorkflowDisplayBuilder::new("w-full h-full bg-neutral-900/80 border border-white/10 rounded-md flex flex-col items-center justify-between overflow-hidden transition-all duration-200 group hover:border-green-500/50 hover:shadow-[0_0_15px_rgba(34,197,94,0.1)] active:scale-95")
                                                // A. CONTENT AREA
                                                // We create a nested div for the content logic
                                                .div(vec![
                                                    WorkflowDisplayBuilder::new("w-full p-4 flex flex-col items-center justify-center grow")
                                                        .div(vec![
                                                            WorkflowDisplay {
                                                                class: "pointer-events-none".into(),
                                                                r#type: role_card_display_for("role"),
                                                            },
                                                        ])
                                                        .build(),

                                                    // B. FOOTER BAR
                                                    WorkflowDisplayBuilder::new("w-full bg-white/5 py-2 flex items-center justify-center border-t border-white/5 group-hover:bg-green-500/10 group-hover:border-green-500/30 transition-colors")
                                                        .div(vec![
                                                            WorkflowDisplayBuilder::new("text-[10px] font-mono font-bold uppercase tracking-widest text-green-500/60 transition-colors group-hover:text-green-400")
                                                                .text("Initialize Link")
                                                                .build()
                                                        ])
                                                        .build(),
                                                ])
                                                .build(),
                                        ],
                                        Some({
                                            let mut m = HashMap::new();
                                            m.insert("chosen_role".into(), json!("{role.role_id}"));
                                            m
                                        }),
                                        SPY_START_OBSERVE_ACTION,
                                        "ghost",
                                        "md",
                                    )
                            .build(),
                    ],
                )
                .build(),
        ])
        .build()
}

fn spy_intercepting_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("spy_intercepting")
        .tile(vec![
            WorkflowDisplayBuilder::new(
                "w-full flex flex-col items-center justify-center py-12 space-y-4 animate-pulse",
            )
            .div(vec![
                // Icon or visual indicator
                WorkflowDisplayBuilder::new("text-4xl text-green-500 mb-2")
                    .text("📡")
                    .build(),
                WorkflowDisplayBuilder::new("text-xl font-halloween text-center text-green-400")
                    .text("Intercepting Feed...")
                    .build(),
                WorkflowDisplayBuilder::new("text-sm font-mono text-center text-white/50")
                    .text("Decryption in progress: {role_name}")
                    .build(),
            ])
            .build(),
        ])
        .build()
}

fn spy_intel_report_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("spy_intel_report")
        .tile(vec![
            // Report Header
            WorkflowDisplayBuilder::new("w-full border-b border-dashed border-green-500/30 pb-2 mb-4 flex justify-between items-end")
                .div(vec![
                    WorkflowDisplayBuilder::new("text-lg font-halloween text-green-400")
                        .text("Intel Report")
                        .build(),
                    WorkflowDisplayBuilder::new("text-xs font-mono text-green-500/60 uppercase")
                        .text("Subject: {role_name}")
                        .build(),
                ])
                .build(),

        ])
        .tile(vec![
            WorkflowDisplay {
                                class: "text-lg font-halloween text-center mt-4".into(),
                                r#type: DisplayType::Tile {
                                    workflow: "observed_results".into(),
                                },
                            }

        ])
        .build()
}

fn spy_mission_failed_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("spy_mission_failed")
        .tile(vec![
            WorkflowDisplayBuilder::new(
                "flex flex-col items-center justify-center py-8 opacity-70",
            )
            .div(vec![
                WorkflowDisplayBuilder::new("text-3xl mb-2")
                    .text("🚫")
                    .build(),
                WorkflowDisplayBuilder::new("text-lg font-halloween text-center text-red-400")
                    .text("Signal Lost")
                    .build(),
                WorkflowDisplayBuilder::new("text-sm text-center text-white/60 mt-2 max-w-xs")
                    .text("The target could not be located or has no active night signature.")
                    .build(),
            ])
            .build(),
        ])
        .build()
}

// ----------------------------------------------------------------------------
// Workflow definition
// ----------------------------------------------------------------------------

pub fn spy_observe_workflow(start_action_target_id: String) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(SPY_WORKFLOW_ID, "Spy Ability")
        .description("Intercept the night actions of another role.")
        .add_node(
            "select_role",
            WorkflowNodeBuilder::new("Target Acquisition")
                // We remove title_visual here to implement a custom header inside the tile for better control
                .visual(spy_target_acquisition_visual())
                .build(),
        )
        .add_node(
            "waiting",
            WorkflowNodeBuilder::new("Intercepting...")
                .parent_id("select_role")
                .condition(Cond::response_equals("$waiting", json!(true)))
                .visual(spy_intercepting_visual())
                .build(),
        )
        .add_node(
            "results",
            WorkflowNodeBuilder::new("Intel Report")
                .parent_id("select_role")
                .condition(Cond::response_exists("observed_results"))
                .visual(spy_intel_report_visual())
                .build(),
        )
        .add_node(
            "no_results",
            WorkflowNodeBuilder::new("Mission Failed")
                .parent_id("select_role")
                .condition(Cond::response_equals("no_results", json!(true)))
                .visual(spy_mission_failed_visual())
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(SPY_START_OBSERVE_ACTION)
                .run_server_action()
                .target(start_action_target_id)
                .build(),
        )
        .build()
}

// ----------------------------------------------------------------------------
// State handler (Logic remains mostly the same, just updated for safety)
// ----------------------------------------------------------------------------

pub fn spy_start_observe(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let user_id = ctx.workflow.user_id.clone();

    let chosen_role_id = ctx
        .get_input_as::<String>("chosen_role")
        .map_err(|_| anyhow!("Missing role selection"))?;

    let role = game
        .all_roles
        .clone()
        .into_iter()
        .find(|r| r.role_id == chosen_role_id)
        .ok_or_else(|| anyhow!("Invalid role selected {}", chosen_role_id))?;

    let user_player = game
        .players
        .get(&user_id)
        .ok_or_else(|| anyhow!("No player"))?
        .clone();

    let who = PlayerRole::new(&user_player);
    let my_role = user_player.effective_role_card(None);

    // --- Sabotage Logic ---
    if my_role.name == "The Saboteur" {
        game.set_sabotage_inputs(
            SPY_SABOTAGE_KEY,
            &ctx.workflow.workflow_id,
            (who.clone(), ctx.workflow.responses.clone()),
        );

        let mut r = HashMap::new();
        r.insert("role_name".into(), json!(role.name));
        r.insert(
            "observed_results".into(),
            json!({
                "type": "text",
                "text": "Fake sabotage data or generic confirmation."
            }),
        );
        // We trigger the result node immediately for Saboteur
        return Ok(ServerActionResult::UpdateResponses(r));
    }

    // --- Standard Logic ---

    // 1. Find the player holding the role
    let target_player = match game.get_player_by_role(&role.role_id, None) {
        Ok(p) => p,
        Err(_) => {
            let mut r = HashMap::new();
            r.insert("no_results".into(), json!(true));
            return Ok(ServerActionResult::UpdateResponses(r));
        }
    };

    // 2. Determine Workflow to wait for
    let _start = role
        .on_night_start
        .as_ref()
        .and_then(|hook| hook(game, &target_player))
        .ok_or_else(|| anyhow!("Selected role has no observable workflow"))?;

    // 3. Handle Sabotage consumption (if Spy was sabotaged previously)
    let sabotage_inputs = game.get_sabotage_inputs(SPY_SABOTAGE_KEY, &ctx.workflow.workflow_id);

    let effective_responses = if let Some((_, inputs)) = sabotage_inputs.clone() {
        game.add_history(
            who.clone(),
            "expected to spy on",
            vec![PlayerRole::new(&target_player)],
        );
        game.clear_sabotage_inputs(SPY_SABOTAGE_KEY, &ctx.workflow.workflow_id);
        inputs
    } else {
        ctx.workflow.responses.clone()
    };

    // 4. Resolve Effective Target (post-sabotage)
    let effective_chosen_role_id = effective_responses
        .get("chosen_role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing role selection"))?
        .to_string();

    let effective_role = game
        .all_roles
        .clone()
        .into_iter()
        .find(|r| r.role_id == effective_chosen_role_id)
        .ok_or_else(|| anyhow!("Invalid role selected"))?;

    let effective_target = match game.get_player_by_role(&effective_role.role_id, None) {
        Ok(p) => p,
        Err(_) => {
            let mut r = HashMap::new();
            r.insert("no_results".into(), json!(true));
            return Ok(ServerActionResult::UpdateResponses(r));
        }
    };

    let effective_start = effective_role
        .on_night_start
        .as_ref()
        .and_then(|hook| hook(game, &effective_target))
        .ok_or_else(|| anyhow!("Selected role has no observable workflow"))?;

    game.add_history(
        PlayerRole::new(&user_player),
        format!("spied on the {}", effective_role.name).as_str(),
        vec![PlayerRole::new(&effective_target)],
    );

    // 5. Wait
    Ok(ServerActionResult::WaitForPredicate {
        predicate: WorkflowPredicate::ByWorkflowDefinitionIdAndUserId(
            effective_start.workflow_id,
            effective_target.id.clone(),
        ),
        responses: {
            let mut r = HashMap::new();
            r.insert("role_name".into(), json!(effective_role.name));
            // Trigger the "waiting" node
            r
        },
        inject_response_as: Some("observed_results".to_string()),
        on_complete: vec![],
        reconcile_node: true,
    })
}

// ----------------------------------------------------------------------------
// Registration
// ----------------------------------------------------------------------------

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    let start_target_id = engine
        .clone()
        .register_state_handler(SPY_START_OBSERVE_ACTION, |game, ctx| {
            spy_start_observe(game, ctx)
        })
        .await?;

    bot.register_workflow(spy_observe_workflow(start_target_id))
        .await?;

    Ok(())
}

// ----------------------------------------------------------------------------
// Role card
// ----------------------------------------------------------------------------

pub fn spy_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: SPY_ROLE_ID.to_string(),
        name: "The Spy".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/spy.png".into(),
        priority: 0,
        alliance: Alliance::Unmarked,
        enabled,
        night_duration: 10,
        night_action: None,
        register: None,

        on_night_start: Some(Arc::new(|game: &GameState, _player| {
            use std::collections::HashSet;

            // Only pass 1 role card per role_id (the first one encountered)
            let mut seen = HashSet::new();
            let options: Vec<Value> = game
                .all_roles
                .iter()
                .filter(|c| {
                    c.enabled
                        && c.on_night_start.is_some()
                        && c.role_id != SPY_ROLE_ID
                        && c.alliance != Alliance::Hollow
                        && seen.insert(c.role_id.clone())
                })
                .map(|c| json!(c))
                .collect();

            let mut inputs = HashMap::new();
            inputs.insert("available_roles".into(), json!(options));

            Some(WorkflowStart {
                workflow_id: SPY_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}
