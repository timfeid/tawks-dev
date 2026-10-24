use workflow::{DisplayType, WorkflowDisplay};

pub fn role_select_display(var: &str) -> DisplayType {
    DisplayType::Tooltip {
        tooltip_html: format!("{{{var}.description}}"),
        class: "text-left font-serif".into(),
        view_display: vec![
            WorkflowDisplay {
                class: "size-12 rounded-full midnight-tint-{".to_owned() + var + ".alliance}",
                r#type: DisplayType::Image {
                    src: format!("{{{var}.image_url}}"),
                    alt_text: format!("{{{var}.name}}"),
                },
            },
            WorkflowDisplay {
                class: "font-bold text-xl".into(),
                r#type: DisplayType::Text {
                    text: format!("{{{var}.name}}"),
                },
            },
        ],
    }
}
