use workflow::CreateWorkflowDefinition;
use workflow::builder::{WorkflowActionBuilder, WorkflowDefinitionBuilder};

use crate::CardGameActionIds;

use super::card_builder::card_builder_node;

pub fn builder_definition(actions: &CardGameActionIds) -> CreateWorkflowDefinition {
    let save_card_action = WorkflowActionBuilder::new("games.cardgame.save_card")
        .run_server_action()
        .target(&actions.save_card)
        .build();

    let list_cards_action = WorkflowActionBuilder::new("games.cardgame.list_cards")
        .run_server_action()
        .target(&actions.list_cards)
        .build();

    WorkflowDefinitionBuilder::new("cardgame_builder", "Card Builder")
        .description("Create and edit cardgame cards")
        .starting_server_action(&actions.view)
        .add_actions(vec![save_card_action, list_cards_action])
        .add_node("builder", card_builder_node())
        .build()
}
