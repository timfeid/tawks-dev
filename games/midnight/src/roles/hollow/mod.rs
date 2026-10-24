use anyhow::{Result, anyhow};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tawks_bot::TawksBot;
use ulid::Ulid;

use workflow::builder::*;
use workflow::server_action::ServerActionResult;
use workflow::service::WorkflowResourceWithContext;
use workflow::{
    CreateWorkflowDefinition, DisplayType, DisplayVisual, NodeCondition, WorkflowDisplay,
    WorkflowInput,
};

use crate::gamestate::{GameState, Player, PlayerRole};
use crate::roles::{Alliance, RoleCard, WorkflowStart};
use crate::workflows::CardSelect;
use crate::workflows::visuals::reveal_button_visual;
use crate::{create_reveal_cards_node, hidden_display_for_card_list, title_visual};
use game_framework::engine::GameEngine;

pub const HOLLOW_ROLE_ID: &str = "hollow";
pub const HOLLOW_WORKFLOW_ID: &str = "hollow_ability_workflow";
pub const HOLLOW_ANALYZE_ACTION: &str = "games.midnight.hollow.analyze_cards";

// ---- Workflow definition ----

fn select_card_lists_visual(content: Vec<WorkflowDisplay>) -> workflow::WorkflowVisual {
    WorkflowVisualBuilder::new("hollow_select_card_lists")
        .display(DisplayVisual::Tile { content })
        .build()
}

pub fn hollow_ability_workflow(analyze_action_id: String) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new(HOLLOW_WORKFLOW_ID, "Hollow Ability")
        .description(
            "If the Hollow wakes up and sees no other Hollows, they can inspect one center card",
        )
        .add_node(
            "select_cards",
            WorkflowNodeBuilder::new("Choose a Hidden Card")
                .description("Select a card to inspect from the center")
                .visual(title_visual("{title}"))
                .visual(select_card_lists_visual(vec![WorkflowDisplay {
                    class: "w-full space-y-2 flex items-center flex-col".into(),
                    r#type: DisplayType::Input {
                        id: "select_cards".into(),
                        input: WorkflowInput {
                            r#type: workflow::InputType::MultiSelectFromList {
                                items_key: "card_options".into(),
                                r#as: "card".into(),
                                value: "{card.value}".into(),
                                class: "h-auto w-full".into(),
                                content: vec![hidden_display_for_card_list()],
                                min: Some(1),
                                max: Some(1),
                            },
                            default_value: None,
                            required: true,
                            readonly: None,
                            on_change_action: Some((HOLLOW_ANALYZE_ACTION.into(), HashMap::new())),
                        },
                    },
                }]))
                .visual(reveal_button_visual())
                .build(),
        )
        .add_node(
            "reveal_hidden_cards",
            create_reveal_cards_node("{card.name} is currently")
                .condition(NodeCondition::ResponseExists("reveal_cards".into()))
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new(HOLLOW_ANALYZE_ACTION)
                .run_server_action()
                .target(analyze_action_id)
                .build(),
        )
        .build()
}

// ---- Role card ----

pub fn hollow_analyze_cards(
    game: &mut GameState,
    ctx: WorkflowResourceWithContext,
) -> Result<ServerActionResult> {
    let _effective_inputs = ctx.workflow.responses.clone();

    let selected_cards: Vec<CardSelect> = ctx
        .get_path_any("select_cards")
        .ok_or_else(|| anyhow!("no selected cards found"))?
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    let mut response = HashMap::new();

    if selected_cards.is_empty() {
        response.insert("title".into(), json!("Choose a card to inspect"));
        response.insert("show_reveal_action".into(), json!(false));
        return Ok(ServerActionResult::UpdateResponses(response));
    }

    let hidden_id = match selected_cards[0] {
        CardSelect::Hidden { id } => id,
        _ => return Err(anyhow!("expected hidden card selection")),
    };

    let hidden = game.get_hidden_card(hidden_id).map_err(|e| anyhow!(e))?;

    let name = format!("Hidden Role {}", hidden_id + 1);
    response.insert(
        "reveal_cards".to_string(),
        json!([{
            "name": name,
            "card": &*hidden,
        }]),
    );
    response.insert("title".into(), json!("Here is what you've revealed"));

    let player = game
        .players
        .get(&ctx.workflow.user_id)
        .ok_or_else(|| anyhow!("No player"))?
        .clone();
    game.add_history(
        PlayerRole::new(&player),
        format!("viewed {}", name).as_ref(),
        vec![PlayerRole::new_from_hidden(hidden_id, hidden)],
    );

    Ok(ServerActionResult::UpdateResponses(response))
}

pub fn hollow_card(enabled: bool) -> Arc<RoleCard> {
    Arc::new(RoleCard {
        enabled_texts: vec![],
        winner_check: None,

        id: Ulid::new().to_string(),
        role_id: HOLLOW_ROLE_ID.to_string(),

        name: "The Hollow".to_string(),
        description: include_str!("./description.html").to_string(),
        image_url: "https://cdn.tawks.app/midnight/roles/the-hollow.png".into(),

        // Run before Bound if Bound priority is 4; adjust as you want.
        priority: 2,
        alliance: Alliance::Hollow,

        enabled,
        night_duration: 0, // up to you; engine will max(10) unless you change that

        night_action: None,
        register: None,

        // NEW ENGINE: decide whether to inject workflow at night.
        // If condition is met, return workflow definition id.
        on_night_start: Some(Arc::new(|game: &GameState, _player: &Player| {
            let hollows = game
                .players
                .values()
                .filter(|p| p.is_alive && p.effective_role_card(None).alliance == Alliance::Hollow)
                .count();

            if hollows != 1 {
                return None;
            }

            let hidden_len = game.hidden_cards.len();
            let card_options = (1..=hidden_len)
                .map(|i| {
                    json!({
                        "name": format!("Hidden Role {}", i),
                        "value": crate::workflows::CardSelect::Hidden { id: i - 1 },
                    })
                })
                .collect::<Vec<_>>();

            let mut inputs = HashMap::new();
            inputs.insert("title".into(), json!("Choose a card to inspect"));
            inputs.insert("card_options".into(), json!(card_options));
            inputs.insert("show_reveal_action".into(), json!(false));

            Some(WorkflowStart {
                workflow_id: HOLLOW_WORKFLOW_ID.to_string(),
                inputs,
                context: HashMap::new(),
            })
        })),
    })
}

pub async fn register(bot: &TawksBot, engine: Arc<GameEngine<GameState>>) -> Result<()> {
    // 1) register state handler (server action name -> runs inside lobby actor)
    let analyze_action_id = engine
        .clone()
        .register_state_handler(HOLLOW_ANALYZE_ACTION, |game, ctx| {
            hollow_analyze_cards(game, ctx)
        })
        .await?;

    // 2) register workflow definition that references that action id
    bot.register_workflow(hollow_ability_workflow(analyze_action_id))
        .await?;

    Ok(())
}
