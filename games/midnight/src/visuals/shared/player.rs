use workflow::{DisplayType, WorkflowDisplay};

pub fn player_display(var: &str) -> WorkflowDisplay {
    WorkflowDisplay {
        class: format!("player-{{{var}.id}} flex flex-col gap-1 items-center"),
        r#type: DisplayType::Div {
            content: vec![
                WorkflowDisplay {
                    class: "rounded-md size-8".into(),
                    r#type: DisplayType::Image {
                        src: format!("{{{var}.avatar_url}}"),
                        alt_text: format!("{{{var}.name}}"),
                    },
                },
                WorkflowDisplay {
                    class: "font-medium font-halloween truncate".into(),
                    r#type: DisplayType::Text {
                        text: format!("{{{var}.name}}"),
                    },
                },
            ],
        },
    }
}
