
use anyhow::Result;
use tawks_bot::{Activity, TawksBot};
use workflow::{
    CreateWorkflowDefinition,
    builder::{WorkflowActionBuilder, WorkflowDefinitionBuilder},
};

use crate::{MidnightActionIds, nodes};

/// Builds the Lobby workflow definition.
///
/// * `action_prefix`: The namespace for your actions (e.g. "midnight").
///   Actions will be constructed as "{prefix}.join", "{prefix}.vote", etc.
pub fn definition(actions: &MidnightActionIds) -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new("view_midnight_lobby", "View Midnight Lobby")
        .description("Displays lobby and player information for the Midnight game")
        // Nodes
        .add_node("game_node", nodes::game::create())
        .add_node("results_node", nodes::result::create())
        .add_node("reveal_role_node", nodes::role_reveal::create())
        .add_node("lobby_node", nodes::lobby::create())
        .add_node("not_started_node", nodes::not_started::create())
        // Initial Action (The entry point)
        .starting_server_action(&actions.view) // e.g. "midnight.view"
        // --- Actions ---
        .add_action(
            WorkflowActionBuilder::new("games.midnight.reset")
                .run_server_action()
                .target(&actions.reset) // <--- Correct ID
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.toggle_lock")
                .run_server_action()
                .target(&actions.toggle_lock)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.accuse")
                .run_server_action()
                .target(&actions.accuse)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.enable_role")
                .run_server_action()
                .target(&actions.enable_role)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.join")
                .transform_then_run(
                    vec![],
                    true,
                    &actions.join, // <--- Correct ID
                    None,
                )
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.vote")
                .run_server_action()
                .target(&actions.vote)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.sus")
                .run_server_action()
                .target(&actions.sus)
                .build(),
        )
        .add_action(
            WorkflowActionBuilder::new("games.midnight.start")
                .run_server_action()
                .target(&actions.start)
                .build(),
        )
        .build()
}

pub async fn register(bot: &TawksBot, actions: &MidnightActionIds) -> Result<()> {
    let workflow_id = bot.register_workflow(definition(actions)).await?;
    bot.register_activity(
        &Activity::workflow(workflow_id, "Midnight")
            .source("https://github.com/timfeid/tawks-dev/tree/main/games/midnight")
            .description("A hidden-role game of night and day")
            .icon("moon")
            .players(4, 16),
    )
    .await?;
    Ok(())
}
