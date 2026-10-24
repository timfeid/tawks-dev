use workflow::{
    ButtonType, DisplayType, DisplayVisual, NodeCondition, StickyPosition, WorkflowDisplay,
    WorkflowNode,
    builder::{WorkflowNodeBuilder, WorkflowVisualBuilder},
};

pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Waiting to start")
        .description("No game has started yet or no players are in the lobby")
        .visual(
            WorkflowVisualBuilder::new("not_started_message")
                .display(DisplayVisual::Tile {
                    content: vec![WorkflowDisplay {
                        r#type: DisplayType::Image {
                            src: "https://cdn.tawks.app/midnight/midnight.webp".to_string(),
                            alt_text: "No one is here!".to_string(),
                        },
                        class: "w-full bg-blend-overlay mx-auto max-w-3xl aspect-square".into(),
                    }],
                })
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("join_lobby_tile")
                .display(DisplayVisual::Sticky {
                    position: StickyPosition::Bottom,
                    content: vec![WorkflowDisplay {
                        class: "flex h-12 items-center justify-center w-full".into(),
                        r#type: DisplayType::Div {
                            content: vec![WorkflowDisplay {
                                r#type: DisplayType::Button {
                                    r#type: ButtonType::Submit,
                                    input_map: None,
                                    size: "default".into(),
                                    content: vec![WorkflowDisplay {
                                        r#type: DisplayType::Text {
                                            text: "Join Lobby".into(),
                                        },
                                        class: "".into(),
                                    }],
                                    action_id: "games.midnight.join".into(),
                                    variant: "ghost".into(),
                                },
                                class: "font-halloween text-2xl text-black bg-[#BCAA87]".into(),
                            }],
                        },
                    }],
                })
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
