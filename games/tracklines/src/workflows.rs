use anyhow::Result;
use serde_json::json;
use tawks_bot::TawksBot;
use workflow::CreateWorkflowDefinition;
use workflow::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

use crate::TracklinesActionIds;

pub const WORKFLOW_ID: &str = "tracklines_board";

pub fn definition(actions: &TracklinesActionIds) -> CreateWorkflowDefinition {
    let action = |id: &str, target: &str| {
        WorkflowActionBuilder::new(id).run_server_action().target(target).build()
    };
    let view_action = WorkflowActionBuilder::new("games.tracklines.view")
        .run_server_action()
        .allow_while_loading()
        .target(&actions.view)
        .build();
    let node = WorkflowNodeBuilder::new("Tracklines")
        .layout("min-h-screen w-full bg-slate-950 text-slate-100 relative")
        .condition(Cond::response_exists("lobby_id"))
        .visual(
            WorkflowVisualBuilder::new("board")
                .tile(vec![WorkflowDisplayBuilder::new("w-full h-full absolute inset-0")
                    .tracklines_canvas()
                    .build()])
                .build(),
        )
        .build();
    WorkflowDefinitionBuilder::new(WORKFLOW_ID, "Tracklines")
        .description("Claim rail routes across Verdance and finish your tickets.")
        .starting_server_action(&actions.view)
        .response("game", json!(null))
        .add_actions(vec![
            view_action,
            action("games.tracklines.join", &actions.join),
            action("games.tracklines.add_ai", &actions.add_ai),
            action("games.tracklines.start", &actions.start),
            action("games.tracklines.draw_card", &actions.draw_card),
            action("games.tracklines.claim_route", &actions.claim_route),
            action("games.tracklines.draw_tickets", &actions.draw_tickets),
            action("games.tracklines.keep_tickets", &actions.keep_tickets),
            action("games.tracklines.reset", &actions.reset),
        ])
        .add_node("board", node)
        .build()
}

pub async fn register(bot: &TawksBot, actions: &TracklinesActionIds) -> Result<()> {
    bot.register_workflow(definition(actions)).await?;
    Ok(())
}
