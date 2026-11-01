use std::collections::HashMap;

use workflow::{
    DisplayVisual, NodeCondition, WorkflowNode, WorkflowVisual,
    builder::{WorkflowNodeBuilder, WorkflowVisualBuilder},
    ui::PageWidth,
};

use crate::MidnightPage;
use crate::visuals::{
    night_results::{new_game_action, night_results},
    votes_and_accusations::votes_and_accusations,
};

pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Game is finished")
        .description("finished")
        .visual(
            WorkflowVisualBuilder::new("results")
                .midnight_page(
                    PageWidth::Normal,
                    vec![votes_and_accusations(), night_results(), new_game_action()],
                )
                .build(),
        )
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: r#"[data-mark~="executed-true"]"#.into(),
                animation_type: "executed".into(),
                trigger: Some("on_appear".into()),
                delay_ms: None,
            },
        })
        .visual(WorkflowVisual {
            id: "animation".into(),
            display: DisplayVisual::Animation {
                meta: HashMap::new(),
                id: r#"[data-mark~="winner-true"]"#.into(),
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
