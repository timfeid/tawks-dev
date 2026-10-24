use anyhow::{Result, anyhow};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use ulid::Ulid;

use workflow::builder::*;
use workflow::server_action::ServerActionResult;
use workflow::{
    CreateWorkflowDefinition, DisplayType, DisplayVisual, NodeCondition, WorkflowDisplay,
    WorkflowVisual,
};

use crate::gamestate::{GameState, Player, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::title_visual;

/// Shared cell holding the workflow definition id returned by Tawks when we register.
/// Every Bound card clone must point at the SAME cell.
pub type WorkflowIdCell = Arc<std::sync::RwLock<Option<String>>>;

pub fn make_workflow_id_cell() -> WorkflowIdCell {
    Arc::new(std::sync::RwLock::new(None))
}

pub fn get_bound_workflow(action_id: String) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new("bound_workflow", "Bound Role Peek")
        .description("Reveal other Bound players.")
        .add_node(
            "peek",
            WorkflowNodeBuilder::new("Bound Night Info")
                // A. Dynamic Title
                .visual(title_visual("{title}"))
                // B. The Main Visual Block
                .visual(
                    WorkflowVisualBuilder::new("bound_reveal")
                        .display(DisplayVisual::Tile {
                            content: vec![WorkflowDisplay {
                                class: "flex flex-wrap gap-4 justify-center".into(),
                                r#type: DisplayType::Condition {
                                    // Only show the list if peers exist in the response
                                    condition: NodeCondition::ResponseExists("bound_peers".into()),
                                    content: vec![WorkflowDisplay {
                                        r#type: DisplayType::List {
                                            items_key: "bound_peers".into(),
                                            r#as: "player".into(),
                                            content: vec![WorkflowDisplay {
                                                r#type: DisplayType::Div {
                                                    content: vec![
                                                        WorkflowDisplay {
                                                            r#type: DisplayType::Image {
                                                                src: "{player.avatar_url}".into(),
                                                                alt_text: "{player.name}".into(),
                                                            },
                                                            class: "w-16 h-16 rounded-full border-2 border-white/20 mb-2"
                                                                .into(),
                                                        },
                                                        WorkflowDisplay {
                                                            r#type: DisplayType::Text {
                                                                text: "{player.name}".into(),
                                                            },
                                                            class: "text-center text-lg font-bold font-halloween".into(),
                                                        },
                                                    ],
                                                },
                                                class: "flex player flex-col items-center min-w-[120px] bg-white/5 p-4 rounded-lg"
                                                    .into(),
                                            }],
                                        },
                                        class: "flex flex-wrap gap-4 justify-center items-center w-full".into(),
                                    }],
                                },
                            }],
                        })
                        .build(),
                )
                // C. Entrance Animation
                .visual(WorkflowVisual {
                    id: "animation".into(),
                    display: DisplayVisual::Animation {
                        meta: HashMap::new(),
                        id: ".player".into(),
                        animation_type: "link".into(),
                        trigger: Some("on_appear".into()),
                        delay_ms: Some(250),
                    },
                })
                .build(),
        )
        // Trigger the logic immediately when this workflow loads
        .starting_server_action(action_id)
        .build()
}

/// Create a Bound card that shares a workflow id cell with all other Bound cards.
pub fn bound_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,
        id: Ulid::new().to_string(),
        role_id: "bound".to_string(),
        name: "The Bound".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/bound.png".into(),
        priority: 4,
        alliance: Alliance::Unmarked,
        enabled,
        night_duration: 0,

        // Logic fields
        night_action: None,

        // The engine calls this during NightTurn stage entry.
        // We return the registered workflow id once it exists.
        on_night_start: Some(Arc::new(move |_game: &GameState, _player: &Player| {
            Some(WorkflowStart {
                workflow_id: "bound_workflow".into(),
                inputs: HashMap::new(),
                context: HashMap::new(),
            })
        })),

        // if you still have other fields like night_ability/register, keep them None here
        register: None,
    })
}

/// Called by your GameState action (ShowBound) to populate responses and complete the workflow.
pub fn resolve_bound_info(game: &mut GameState, user_id: &str) -> Result<ServerActionResult> {
    // 1. Find all players currently holding "The Bound"
    let bound_players_infos: Vec<(String, String, String)> = game
        .players
        .values()
        .filter(|p| p.effective_role_card(None).name == "The Bound")
        .map(|p| (p.name.clone(), p.avatar_url.clone(), p.id.clone()))
        .collect();

    // 2. Response list for UI
    let json_players = bound_players_infos
        .iter()
        .map(|(name, avatar_url, _)| {
            json!({
                "name": name,
                "avatar_url": avatar_url,
            })
        })
        .collect::<Vec<_>>();

    // 3. Construct response payload
    let mut response = HashMap::new();

    if json_players.len() > 1 {
        response.insert("bound_peers".into(), json!(json_players));
        response.insert("title".into(), json!("You are Bound together…"));
    } else {
        response.insert("title".into(), json!("You are the only Bound tonight."));
    }

    // 4. Record history
    let actor_player = game
        .players
        .get(user_id)
        .ok_or_else(|| anyhow!("User not found in game"))?;
    let actor_snapshot = PlayerRole::new(actor_player);

    let target_snapshots: Vec<PlayerRole> = bound_players_infos
        .iter()
        .filter(|(_, _, id)| id != user_id)
        .filter_map(|(_, _, id)| game.players.get(id))
        .map(|p| PlayerRole::new(p))
        .collect();

    let action_verb = if !target_snapshots.is_empty() {
        "locked eyes with"
    } else {
        "woke up alone"
    };

    game.add_history(actor_snapshot, action_verb, target_snapshots);

    // 5. Complete the workflow
    Ok(ServerActionResult::CompleteWorkflow {
        message: "Bound info revealed.".into(),
        responses: response,
        close_active_modal: false,
    })
}
