use workflow::{ButtonType, DisplayVisual, NodeCondition, builder::*};
use workflow::{DisplayType, WorkflowDisplay, WorkflowVisual};

pub fn reveal_button_visual() -> WorkflowVisual {
    WorkflowVisualBuilder::new("reveal_button")
        .display(DisplayVisual::Tile {
            content: vec![WorkflowDisplay {
                r#type: DisplayType::Condition {
                    condition: NodeCondition::ResponseEquals {
                        field: "show_reveal_action".into(),
                        value: true.into(),
                    },
                    content: vec![WorkflowDisplay {
                        class: "flex justify-center w-full".into(),
                        r#type: DisplayType::Button {
                            r#type: ButtonType::Submit,
                            input_map: None,
                            content: vec![WorkflowDisplay {
                                class: "".into(),
                                r#type: DisplayType::Text {
                                    text: "Reveal Cards".into(),
                                },
                            }],
                            size: "lg".into(),
                            variant: "ghost".into(),
                            action_id: "games.midnight.reveal_cards".into(),
                        },
                    }],
                },
                class: "".into(),
            }],
        })
        .build()
}
