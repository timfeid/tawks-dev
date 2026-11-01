use crate::MidnightPage;
use workflow::{
    NodeCondition, WorkflowNode,
    builder::{WorkflowNodeBuilder, WorkflowVisualBuilder},
    ui::*,
};

pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Waiting to start")
        .description("No game has started yet or no players are in the lobby")
        .visual(
            WorkflowVisualBuilder::new("not_started_message")
                .midnight_page(
                    PageWidth::Normal,
                    vec![
                        media(
                            "https://cdn.tawks.app/midnight/midnight.webp",
                            "No one is here!",
                            Aspect::Square,
                            Size::Fill,
                        ),
                        action_bar(vec![primary_action(
                            "Join Lobby",
                            act("games.midnight.join"),
                        )]),
                    ],
                )
                .build(),
        )
        .condition(NodeCondition::Or(vec![
            NodeCondition::ResponseEquals {
                field: "game.stage".into(),
                value: "NotStarted".into(),
            },
            NodeCondition::Not(Box::new(NodeCondition::ResponseExists("game".into()))),
        ]))
        .build()
}
