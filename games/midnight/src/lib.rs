pub mod award;
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
use workflow::{builder::*, ui::*, *};

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

/// Builds a Midnight page: every one is set in the game's spooky typeface.
pub trait MidnightPage {
    fn midnight_page(self, width: PageWidth, content: Vec<WorkflowDisplay>) -> Self;
}

impl MidnightPage for WorkflowVisualBuilder {
    fn midnight_page(self, width: PageWidth, content: Vec<WorkflowDisplay>) -> Self {
        self.page(width, content).typeface(Typeface::Spooky)
    }
}

pub fn title_visual(arg: &str) -> WorkflowVisual {
    WorkflowVisualBuilder::new("title")
        .midnight_page(PageWidth::Narrow, vec![game_title(arg)])
        .build()
}

/// A heading in the game's own typeface.
pub fn game_title(text: &str) -> WorkflowDisplay {
    title(text).themed().centered()
}

/// The heading over a role's night ability: its name in the role's colour
/// and a line on what to do.
pub fn ability_header(name: &str, tone: Tone, lead: &str) -> WorkflowDisplay {
    column(
        Gap::Tight,
        vec![
            title(name).themed().centered().tone(tone),
            fine(lead).centered(),
        ],
    )
    .align(Align::Center)
}

pub fn role_image(var: &str, size: Size) -> WorkflowDisplay {
    media(
        format!("{{{var}.image_url}}"),
        format!("{{{var}.name}}"),
        Aspect::Square,
        size,
    )
    .fit(Fit::Contain)
    .tone_from(format!("{{{var}.alliance_tone}}"))
}

/// A role's picture over its name and side, coloured by the side.
pub fn role_summary(var: &str) -> WorkflowDisplay {
    column(
        Gap::Tight,
        vec![
            role_image(var, Size::Large),
            subheading(format!("{{{var}.name}}"))
                .themed()
                .centered()
                .tone_from(format!("{{{var}.alliance_tone}}")),
            badge(format!("{{{var}.alliance}}"), Tone::Default)
                .tone_from(format!("{{{var}.alliance_tone}}")),
        ],
    )
    .align(Align::Center)
}

/// A role card whose description shows on hover.
pub fn role_card(var: &str) -> WorkflowDisplay {
    panel(vec![
        WorkflowDisplayBuilder::new()
            .tooltip(format!("{{{var}.description}}"), vec![role_summary(var)])
            .build(),
    ])
}

pub fn player_display(variable: &str) -> WorkflowDisplay {
    person(
        format!("{{{variable}.avatar_url}}"),
        format!("{{{variable}.name}}"),
    )
    .size(Size::Small)
}

pub fn create_reveal_cards_node(display_text: &str, footer: Option<&str>) -> WorkflowNodeBuilder {
    let mut content = vec![
        game_title("{title}"),
        items(
            "reveal_cards",
            "card",
            Layout::Stack { gap: Gap::Loose },
            vec![
                column(
                    Gap::Normal,
                    vec![
                        subheading(display_text).themed().centered(),
                        role_card("card.card"),
                    ],
                )
                .mark("reveal-card"),
            ],
        ),
    ];
    if let Some(footer) = footer {
        content.push(body(footer).themed().centered());
    }

    WorkflowNodeBuilder::new("What You Saw")
        .parent_id("select_cards")
        .visual(
            WorkflowVisualBuilder::new("reveal_cards_display")
                .midnight_page(PageWidth::Narrow, content)
                .build(),
        )
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: r#"[data-mark~="reveal-card"]"#.into(),
                animation_type: "reveal".into(),
                trigger: Some("on_appear".into()),
                delay_ms: None,
            },
        })
}

pub fn hidden_display_for_card_list() -> WorkflowDisplay {
    body("{card.name}").themed()
}

pub fn player_display_for_card_list(player_prefix: &str) -> WorkflowDisplay {
    when(
        NodeCondition::ResponseEquals {
            field: "card.value.type".into(),
            value: "player".into(),
        },
        vec![player_display(player_prefix)],
    )
}
