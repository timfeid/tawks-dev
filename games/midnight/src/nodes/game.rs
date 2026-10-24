use std::collections::HashMap;

use workflow::{
    DisplayVisual, NodeCondition, WorkflowNode,
    builder::{WorkflowNodeBuilder, WorkflowVisualBuilder},
};

use crate::visuals::{game_tile::game_tile, vote_tile::vote_tile};

pub fn create() -> WorkflowNode {
    let mut meta = HashMap::new();
    meta.insert("duration".into(), 5000.into());
    WorkflowNodeBuilder::new("Game in progress")
        .description("The game is underway")
        .visual(game_tile())
        .visual(
            WorkflowVisualBuilder::new("games.midnight_tile")
                .display(DisplayVisual::Block {
                    class: "max-w-3xl mx-auto".into(),
                    content: vec![],
                })
                .build(),
        )
        .visual(vote_tile())
        .visual(
            WorkflowVisualBuilder::new("board_fog")
                .animation(".wf-shell", "fog", Some("on_appear".into()), None, meta)
                .build(),
        )
        .condition(NodeCondition::And(vec![
            NodeCondition::ResponseEquals {
                field: "game.is_role_reveal".into(),
                value: false.into(),
            },
            NodeCondition::ResponseEquals {
                field: "game.is_live".into(),
                value: true.into(),
            },
            NodeCondition::ResponseEquals {
                field: "game.is_complete".into(),
                value: false.into(),
            },
        ]))
        .build()
}
