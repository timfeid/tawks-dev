use serde_json::json;
use workflow::builder::{Cond, WorkflowDisplayBuilder};
use workflow::{InputType, WorkflowInput};

pub(super) fn simple_input(r#type: InputType, required: bool) -> WorkflowInput {
    WorkflowInput {
        r#type,
        default_value: None,
        required,
        readonly: None,
        on_change_action: None,
    }
}

pub(super) fn effect_fields(title: &str, key_prefix: &str) -> Vec<workflow::WorkflowDisplay> {
    let kind_key = format!("{key_prefix}.kind");
    let mana_type_key = format!("{key_prefix}.mana_type");
    let amount_key = format!("{key_prefix}.amount");
    let count_key = format!("{key_prefix}.count");

    vec![
        WorkflowDisplayBuilder::new("text-xs uppercase tracking-widest text-slate-400")
            .text(title)
            .build(),
        WorkflowDisplayBuilder::new("space-y-2")
            .div(vec![
                WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                    .text("Effect")
                    .build(),
                WorkflowDisplayBuilder::new("")
                    .input(
                        kind_key,
                        simple_input(
                            InputType::Select {
                                filter: None,
                                items_key: "game.effect_kind_options".to_string(),
                                r#as: "effect_kind_option".to_string(),
                                value: "{effect_kind_option.value}".to_string(),
                                content: vec![
                                    WorkflowDisplayBuilder::new("")
                                        .text("{effect_kind_option.label}")
                                        .build(),
                                ],
                                selected_content: None,
                                unselected_content: None,
                                class: "px-4 py-2 hover:bg-slate-700".to_string(),
                            },
                            false,
                        ),
                    )
                    .build(),
            ])
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::response_equals(format!("{key_prefix}.kind"), json!("add_mana")),
                vec![
                    WorkflowDisplayBuilder::new("grid grid-cols-2 gap-3")
                        .div(vec![
                            WorkflowDisplayBuilder::new("space-y-2")
                                .div(vec![
                                    WorkflowDisplayBuilder::new(
                                        "block text-sm font-medium text-gray-400",
                                    )
                                    .text("Mana Type")
                                    .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            mana_type_key,
                                            simple_input(
                                                InputType::Select {
                                                    filter: None,
                                                    items_key: "game.mana_type_options".to_string(),
                                                    r#as: "mana_type_option".to_string(),
                                                    value: "{mana_type_option.value}".to_string(),
                                                    content: vec![
                                                        WorkflowDisplayBuilder::new("")
                                                            .text("{mana_type_option.label}")
                                                            .build(),
                                                    ],
                                                    selected_content: None,
                                                    unselected_content: None,
                                                    class: "px-4 py-2 hover:bg-slate-700"
                                                        .to_string(),
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-2")
                                .div(vec![
                                    WorkflowDisplayBuilder::new(
                                        "block text-sm font-medium text-gray-400",
                                    )
                                    .text("Amount")
                                    .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            amount_key,
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                        ])
                        .build(),
                ],
            )
            .build(),
        WorkflowDisplayBuilder::new("")
            .condition(
                Cond::response_equals(format!("{key_prefix}.kind"), json!("draw_cards")),
                vec![
                    WorkflowDisplayBuilder::new("space-y-2")
                        .div(vec![
                            WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                                .text("Cards to Draw")
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .input(
                                    count_key,
                                    simple_input(
                                        InputType::Number {
                                            min: Some(0),
                                            max: None,
                                        },
                                        false,
                                    ),
                                )
                                .build(),
                        ])
                        .build(),
                ],
            )
            .build(),
    ]
}

pub(super) fn effect_editor(title: &str, key_prefix: &str) -> workflow::WorkflowDisplay {
    WorkflowDisplayBuilder::new("rounded-md border border-white/10 bg-white/5 p-3 space-y-3")
        .div(effect_fields(title, key_prefix))
        .build()
}

pub(super) fn trigger_editor(title: &str, key_prefix: &str) -> workflow::WorkflowDisplay {
    let event_key = format!("{key_prefix}.event");
    WorkflowDisplayBuilder::new("rounded-md border border-white/10 bg-white/5 p-3 space-y-3")
        .div(vec![
            WorkflowDisplayBuilder::new("text-xs uppercase tracking-widest text-slate-400")
                .text(title)
                .build(),
            WorkflowDisplayBuilder::new("space-y-2")
                .div(vec![
                    WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                        .text("Trigger Event")
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .input(
                            event_key,
                            simple_input(
                                InputType::Select {
                                    filter: None,
                                    items_key: "game.trigger_event_options".to_string(),
                                    r#as: "trigger_event_option".to_string(),
                                    value: "{trigger_event_option.value}".to_string(),
                                    content: vec![
                                        WorkflowDisplayBuilder::new("")
                                            .text("{trigger_event_option.label}")
                                            .build(),
                                    ],
                                    selected_content: None,
                                    unselected_content: None,
                                    class: "px-4 py-2 hover:bg-slate-700".to_string(),
                                },
                                false,
                            ),
                        )
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new("space-y-3")
                .div(effect_fields("Effect", &format!("{key_prefix}.effect")))
                .build(),
        ])
        .build()
}

pub(super) fn ability_editor(title: &str, key_prefix: &str) -> workflow::WorkflowDisplay {
    let name_key = format!("{key_prefix}.name");
    let mana_prefix = format!("{key_prefix}.cost.mana");

    WorkflowDisplayBuilder::new("rounded-md border border-white/10 bg-white/5 p-3 space-y-3")
        .div(vec![
            WorkflowDisplayBuilder::new("text-xs uppercase tracking-widest text-slate-400")
                .text(title)
                .build(),
            WorkflowDisplayBuilder::new("space-y-2")
                .div(vec![
                    WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                        .text("Ability Name")
                        .build(),
                    WorkflowDisplayBuilder::new("")
                        .input(
                            name_key,
                            simple_input(InputType::Text { placeholder: None }, false),
                        )
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new("space-y-2")
                .div(vec![
                    WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                        .text("Mana Cost")
                        .build(),
                    WorkflowDisplayBuilder::new("grid grid-cols-3 gap-2")
                        .div(vec![
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Neon")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.neon"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Glitch")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.glitch"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Chrome")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.chrome"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Slag")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.slag"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Bio")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.bio"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Colorless")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{mana_prefix}.colorless"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new("space-y-2")
                .div(vec![
                    WorkflowDisplayBuilder::new("block text-sm font-medium text-gray-400")
                        .text("Additional Costs")
                        .build(),
                    WorkflowDisplayBuilder::new("grid grid-cols-2 gap-3")
                        .div(vec![
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Requires Tap")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{key_prefix}.cost.tap"),
                                            simple_input(
                                                InputType::Checkbox {
                                                    label: "Requires Tap".to_string(),
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Pay Life")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{key_prefix}.cost.life"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Tap Others")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{key_prefix}.cost.tap_other"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                            WorkflowDisplayBuilder::new("space-y-1")
                                .div(vec![
                                    WorkflowDisplayBuilder::new("text-xs text-gray-400")
                                        .text("Sacrifice")
                                        .build(),
                                    WorkflowDisplayBuilder::new("")
                                        .input(
                                            format!("{key_prefix}.cost.sacrifice"),
                                            simple_input(
                                                InputType::Number {
                                                    min: Some(0),
                                                    max: None,
                                                },
                                                false,
                                            ),
                                        )
                                        .build(),
                                ])
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
            WorkflowDisplayBuilder::new("space-y-3")
                .div(effect_fields("Effect", &format!("{key_prefix}.effect")))
                .build(),
        ])
        .build()
}
