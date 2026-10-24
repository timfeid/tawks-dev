pub mod gamestate;
mod nodes;
pub mod registrar;
pub mod roles;
mod visuals;
pub mod workflows;

#[cfg(test)]
mod tests;

use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::{full_avatar_url, types::UserResource};
use workflow::{builder::*, *};

use crate::gamestate::{GameState, MidnightAction};
use game_framework::engine::GameEngine;

// ==================================================================================
// 1. ACTION REGISTRY
// ==================================================================================

#[derive(Clone, Debug)]
pub struct MidnightActionIds {
    pub join: String,
    pub start: String,
    pub vote: String,
    pub accuse: String,
    pub toggle_lock: String,
    pub view: String,
    pub reset: String,
    pub sus: String,
    pub enable_role: String,
    // Role specific
    pub show_bound: String,
}

pub async fn register_routes(
    engine: Arc<GameEngine<GameState>>,
    prefix: &str,
) -> Result<MidnightActionIds> {
    // --- LOBBY ACTIONS ---

    let join = engine
        .clone()
        .register_route(&format!("{}.join", prefix), |ctx| {
            let user: UserResource = ctx.ctx_required_as("$user")?;
            Ok(MidnightAction::Join {
                name: user.display_name.clone(),
                avatar: full_avatar_url(&user),
                instance_id: ctx.workflow.instance_id,
            })
        })
        .await?;

    let start = engine
        .clone()
        .register_route(&format!("{}.start", prefix), |_| {
            Ok(MidnightAction::StartGame)
        })
        .await?;

    let toggle_lock = engine
        .clone()
        .register_route(&format!("{}.toggle_lock", prefix), |_| {
            Ok(MidnightAction::ToggleLock)
        })
        .await?;

    let enable_role = engine
        .clone()
        .register_route(&format!("{}.enable_role", prefix), |ctx| {
            let role_id = ctx.get_input_as::<String>("role_id").or_else(|_| {
                ctx.get_input_as::<String>("role_id")
                    .map_err(|_| anyhow!("Missing role_id"))
            })?;
            Ok(MidnightAction::EnableRole { role_id })
        })
        .await?;

    let reset = engine
        .clone()
        .register_route(&format!("{}.reset", prefix), |_| Ok(MidnightAction::Reset))
        .await?;

    // --- GAMEPLAY ACTIONS ---

    let vote = engine
        .clone()
        .register_route(&format!("{}.vote", prefix), |ctx| {
            let target_id = ctx
                .get_input_as::<String>("target_id")
                .map_err(|_| anyhow!("Missing target_id"))?;
            Ok(MidnightAction::Vote { target_id })
        })
        .await?;

    let accuse = engine
        .clone()
        .register_route(&format!("{}.accuse", prefix), |ctx| {
            let role_id = ctx
                .get_input_as::<String>("role_id")
                .map_err(|_| anyhow!("Missing role_id"))?;
            let target_id = ctx
                .get_input_as::<String>("target_id")
                .map_err(|_| anyhow!("Missing target_id"))?;
            Ok(MidnightAction::Accuse { target_id, role_id })
        })
        .await?;

    let sus = engine
        .clone()
        .register_route(&format!("{}.sus", prefix), |ctx| {
            let target_id = ctx
                .get_input_as::<String>("target_id")
                .map_err(|_| anyhow!("Missing target_id"))?;
            Ok(MidnightAction::Sus { target_id })
        })
        .await?;

    // --- SYSTEM ACTIONS ---

    let view = engine
        .clone()
        .register_route(&format!("{}.view", prefix), |_| Ok(MidnightAction::Tick))
        .await?;

    // --- STATE HANDLERS (Custom UI Logic) ---

    let show_bound = engine
        .clone()
        .register_route_with_result(&format!("{}.show_bound", prefix), |_| {
            Ok(MidnightAction::ShowBound)
        })
        .await?;

    Ok(MidnightActionIds {
        join,
        start,
        vote,
        accuse,
        toggle_lock,
        view,
        reset,
        sus,
        enable_role,
        show_bound,
    })
}

// ==================================================================================
// 2. VIEW HELPERS
// ==================================================================================

pub fn single_response<E: Serialize>(key: &str, e: E) -> HashMap<String, Value> {
    let mut responses = HashMap::new();
    responses.insert(key.into(), json!(e));
    responses
}

pub fn title_visual(arg: &str) -> WorkflowVisual {
    WorkflowVisual {
        id: "title".into(),
        display: DisplayVisual::Tile {
            content: vec![WorkflowDisplay {
                class: "text-2xl font-halloween text-center my-4".into(),
                r#type: DisplayType::Text { text: arg.into() },
            }],
        },
    }
}

pub fn role_card_display_for(var: &str) -> DisplayType {
    DisplayType::Div {
        content: vec![WorkflowDisplay {
            class: "font-halloween relative p-4 rounded-lg bg-white/10 w-full max-w-64 min-w-48 flex flex-col justify-center items-center w-full gap-4".into(),
            r#type: DisplayType::Tooltip {
                tooltip_html: format!("{{{var}.description}}"),
                class: "text-left".into(),
                view_display: vec![
                    WorkflowDisplay {
                        class: format!("size-16 flex-shrink-0 rounded-full border midnight-tint-{{{var}.alliance}}"),
                        r#type: DisplayType::Image {
                            src: format!("{{{var}.image_url}}"),
                            alt_text: format!("{{{var}.name}}"),
                        },
                    },
                    WorkflowDisplay {
                        class: "flex-1".into(),
                        r#type: DisplayType::Div {
                            content: vec![
                                WorkflowDisplay {
                                    class: "font-bold".into(),
                                    r#type: DisplayType::Text {
                                        text: format!("{{{var}.name}}"),
                                    },
                                },
                            ],
                        },
                    },
                ],
            },
        }],
    }
}

pub fn player_display(variable: &str) -> WorkflowDisplay {
    WorkflowDisplay {
        class: "flex gap-2 items-center justify-center".into(),
        r#type: DisplayType::Div {
            content: vec![
                WorkflowDisplay {
                    class: "rounded-md size-5".into(),
                    r#type: DisplayType::Image {
                        src: format!("{{{variable}.avatar_url}}"),
                        alt_text: format!("{{{variable}.name}}"),
                    },
                },
                WorkflowDisplay {
                    r#type: DisplayType::Text {
                        text: format!("{{{variable}.name}}"),
                    },
                    class: "text-lg font-medium font-halloween truncate".into(),
                },
            ],
        },
    }
}

pub fn create_reveal_cards_node(display_text: &str) -> WorkflowNodeBuilder {
    WorkflowNodeBuilder::new("What You Saw")
        .parent_id("select_cards")
        .visual(title_visual("{title}"))
        .visual(
            WorkflowVisualBuilder::new("reveal_cards_display")
                .display(DisplayVisual::Tile {
                    content: vec![WorkflowDisplay {
                        class: "flex flex-wrap gap-4 w-full justify-center items-center".into(),
                        r#type: DisplayType::List {
                            items_key: "reveal_cards".into(),
                            r#as: "card".into(),
                            content: vec![WorkflowDisplay {
                                class: "reveal-card opacity-0 transition-opacity duration-1000 ease-out".into(),
                                r#type: DisplayType::Div {
                                    content: vec![
                                        WorkflowDisplay {
                                            class: "text-center text-xl font-halloween".into(),
                                            r#type: DisplayType::Text {
                                                text: display_text.into(),
                                            },
                                        },
                                        WorkflowDisplay {
                                            class: "".into(),
                                            r#type: role_card_display_for("card.card"),
                                        },
                                    ],
                                },
                            }],
                        },
                    }],
                })
                .build(),
        )
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: ".reveal-card".into(),
                animation_type: "reveal".into(),
                trigger: Some("on_appear".into()),
                delay_ms: None,
            },
        })
}

pub fn hidden_display_for_card_list() -> WorkflowDisplay {
    WorkflowDisplay {
        class: "flex gap-2 items-center justify-center".into(),
        r#type: DisplayType::Div {
            content: vec![WorkflowDisplay {
                class: "text-lg font-medium font-halloween truncate".into(),
                r#type: DisplayType::Text {
                    text: "{card.name}".into(),
                },
            }],
        },
    }
}

pub fn player_display_for_card_list(player_prefix: &str) -> WorkflowDisplay {
    let avatar_url = format!("{{{player_prefix}.avatar_url}}");
    let name = format!("{{{player_prefix}.name}}");

    WorkflowDisplay {
        class: "flex gap-2 items-center justify-center".into(),
        r#type: DisplayType::Condition {
            condition: NodeCondition::ResponseEquals {
                field: "card.value.type".into(),
                value: "player".into(),
            },
            content: vec![WorkflowDisplay {
                class: "flex gap-2 items-center justify-center".into(),
                r#type: DisplayType::Div {
                    content: vec![
                        WorkflowDisplay {
                            class: "rounded-md size-5".into(),
                            r#type: DisplayType::Image {
                                src: avatar_url,
                                alt_text: name.clone(),
                            },
                        },
                        WorkflowDisplay {
                            r#type: DisplayType::Text { text: name },
                            class: "text-lg font-medium font-halloween truncate".into(),
                        },
                    ],
                },
            }],
        },
    }
}
