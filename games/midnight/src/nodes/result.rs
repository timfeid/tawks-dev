use std::collections::HashMap;

use workflow::{
    DisplayVisual, NodeCondition, WorkflowNode, WorkflowVisual,
    builder::WorkflowNodeBuilder,
};

use crate::visuals::{
    night_results::night_results_tile,
    votes_and_accusations::votes_and_accusations_tile,
};

pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Game is finished")
        .description("finished")
        .visual(votes_and_accusations_tile())
        .visual(night_results_tile())
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: ".executed".into(),
                animation_type: "executed".into(),
                trigger: Some("on_appear".into()),
                delay_ms: None,
            },
        })
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: ".winner".into(),
                animation_type: "winner".into(),
                trigger: Some("on_appear".into()),
                delay_ms: None,
            },
        })
        .condition(NodeCondition::ResponseEquals {
            field: "game.is_complete".into(),
            value: true.into(),
        })
        .build()
}
