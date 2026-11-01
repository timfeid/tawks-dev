use anyhow::Result;
use serde_json::json;
use tawks_bot::{Activity, TawksBot};
use workflow::CreateWorkflowDefinition;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

pub const WORKFLOW_ID: &str = "party";
pub const JOIN_ACTION: &str = "games.party.join";
/// The card game gets its own lobby entry; it opens the same room with the
/// table already chosen.
pub const COVEN_WORKFLOW_ID: &str = "coven";
pub const COVEN_JOIN_ACTION: &str = "games.coven.join";
pub const LANDLORD_WORKFLOW_ID: &str = "landlord";
pub const LANDLORD_JOIN_ACTION: &str = "games.landlord.join";
/// Speedball has its own entry too: the room opens on that bout.
pub const SPEEDBALL_WORKFLOW_ID: &str = "speedball";
pub const SPEEDBALL_JOIN_ACTION: &str = "games.speedball.join";

pub fn definition(join_action_id: &str) -> CreateWorkflowDefinition {
    table(WORKFLOW_ID, "Tawkathon", "A board game with a bout after every round, played in the lobby.", JOIN_ACTION, join_action_id)
}

pub fn landlord_definition(join_action_id: &str) -> CreateWorkflowDefinition {
    table(LANDLORD_WORKFLOW_ID, "Landlord", "Deal, rent and steal your way to three districts.", LANDLORD_JOIN_ACTION, join_action_id)
}

pub fn speedball_definition(join_action_id: &str) -> CreateWorkflowDefinition {
    table(SPEEDBALL_WORKFLOW_ID, "Speedball", "Two teams, a field of inflatable bunkers and one flag.", SPEEDBALL_JOIN_ACTION, join_action_id)
}

pub fn coven_definition(join_action_id: &str) -> CreateWorkflowDefinition {
    table(COVEN_WORKFLOW_ID, "Coven", "A hidden-role card game for four to twelve at one table.", COVEN_JOIN_ACTION, join_action_id)
}

fn table(id: &str, title: &str, description: &str, action: &str, join_action_id: &str) -> CreateWorkflowDefinition {
    let join = WorkflowActionBuilder::new(action)
        .run_server_action()
        .allow_while_loading()
        .target(join_action_id)
        .build();
    let node = WorkflowNodeBuilder::new(title)
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("stage")
                .tile(vec![
                    WorkflowDisplayBuilder::new().party_canvas().build(),
                ])
                .build(),
        )
        .build();
    WorkflowDefinitionBuilder::new(id, title)
        .description(description)
        .starting_server_action(join_action_id)
        .response("party", json!(null))
        .add_actions(vec![join])
        .add_node("stage", node)
        .build()
}

pub async fn register(
    bot: &TawksBot,
    join_action_id: &str,
    coven_join_action_id: &str,
    landlord_join_action_id: &str,
    speedball_join_action_id: &str,
) -> Result<()> {
    let games = [
        (definition(join_action_id), "party-popper", (2, crate::room::MAX_SEATS as i32)),
        (coven_definition(coven_join_action_id), "cat", (4, 12)),
        (landlord_definition(landlord_join_action_id), "building-2", (2, 5)),
        (speedball_definition(speedball_join_action_id), "flag", (1, 4)),
    ];
    for (workflow, icon, (min, max)) in games {
        let (name, description) = (workflow.name.clone(), workflow.description.clone());
        let workflow_id = bot.register_workflow(workflow).await?;
        if name == "Tawkathon" {
            for item in crate::roster::cosmetics().iter().filter(|item| item.for_sale()) {
                bot.register_product(&tawks_bot::store::Product::new(format!("tawkathon-{}",item.id),&item.name,item.price,&workflow_id)
                    .description(if item.id.starts_with("model-") { "A permanent Tawkathon character. Cosmetic only." } else { "A permanent accessory for your Tawkathon character. Cosmetic only." })
                    .product_type("tawkathon-cosmetic").icon(&item.icon)).await?;
            }
            for reaction in crate::roster::reactions() {
                bot.register_product(&tawks_bot::store::Product::new(format!("tawkathon-{}",reaction.id),&reaction.name,reaction.price,&workflow_id)
                    .description(&reaction.description).product_type("tawkathon-cosmetic").icon(&reaction.icon)).await?;
            }
        }

        let mut game = Activity::workflow(workflow_id, name).icon(icon).players(min, max);
        if let Some(description) = description {
            game = game.description(description.trim_end_matches('.'));
        }
        bot.register_activity(&game).await?;
    }
    Ok(())
}
