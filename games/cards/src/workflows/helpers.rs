use serde_json::json;
use workflow::builder::{Cond, WorkflowDisplayBuilder};
use workflow::ui::*;
use workflow::{InputType, WorkflowDisplay, WorkflowInput};

pub(super) fn simple_input(r#type: InputType, required: bool) -> WorkflowInput {
    WorkflowInput {
        r#type,
        default_value: None,
        required,
        readonly: None,
        on_change_action: None,
    }
}

pub(super) fn select(items_key: &str, item: &str) -> InputType {
    InputType::Select {
        filter: None,
        items_key: items_key.to_string(),
        r#as: item.to_string(),
        value: format!("{{{item}.value}}"),
        content: vec![body(format!("{{{item}.label}}"))],
        selected_content: None,
        unselected_content: None,
        options: Vec::new(),
        option_label: None,
        label: None,
        placeholder: None,
    }
}

pub(super) fn number(min: Option<u32>) -> InputType {
    InputType::Number { min, max: None, placeholder: None }
}

fn input(key: impl Into<String>, input: WorkflowInput) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new().input(key, input).build()
}

pub(super) fn field(label: &str, key: impl Into<String>, value: WorkflowInput) -> WorkflowDisplay {
    column(Gap::Tight, vec![subheading(label), input(key, value)])
}

fn small_field(label: &str, key: impl Into<String>, value: WorkflowInput) -> WorkflowDisplay {
    column(Gap::Tight, vec![fine(label), input(key, value)])
}

fn effect_fields(title: &str, key_prefix: &str) -> Vec<WorkflowDisplay> {
    vec![
        eyebrow(title),
        field(
            "Effect",
            format!("{key_prefix}.kind"),
            simple_input(
                select("game.effect_kind_options", "effect_kind_option"),
                false,
            ),
        ),
        when(
            Cond::response_equals(format!("{key_prefix}.kind"), json!("add_mana")),
            vec![
                grid(
                    2,
                    vec![
                        field(
                            "Mana Type",
                            format!("{key_prefix}.mana_type"),
                            simple_input(
                                select("game.mana_type_options", "mana_type_option"),
                                false,
                            ),
                        ),
                        field(
                            "Amount",
                            format!("{key_prefix}.amount"),
                            simple_input(number(Some(0)), false),
                        ),
                    ],
                )
                .gap(Gap::Normal),
            ],
        ),
        when(
            Cond::response_equals(format!("{key_prefix}.kind"), json!("draw_cards")),
            vec![field(
                "Cards to Draw",
                format!("{key_prefix}.count"),
                simple_input(number(Some(0)), false),
            )],
        ),
    ]
}

pub(super) fn trigger_editor(title: &str, key_prefix: &str) -> WorkflowDisplay {
    panel(vec![
        eyebrow(title),
        field(
            "Trigger Event",
            format!("{key_prefix}.event"),
            simple_input(
                select("game.trigger_event_options", "trigger_event_option"),
                false,
            ),
        ),
        column(
            Gap::Normal,
            effect_fields("Effect", &format!("{key_prefix}.effect")),
        ),
    ])
}

pub(super) fn ability_editor(title: &str, key_prefix: &str) -> WorkflowDisplay {
    let mana_prefix = format!("{key_prefix}.cost.mana");
    let mana = |label: &str, color: &str| {
        small_field(
            label,
            format!("{mana_prefix}.{color}"),
            simple_input(number(Some(0)), false),
        )
    };
    let cost = |label: &str, key: &str, kind: InputType| {
        small_field(
            label,
            format!("{key_prefix}.cost.{key}"),
            simple_input(kind, false),
        )
    };

    panel(vec![
        eyebrow(title),
        field(
            "Ability Name",
            format!("{key_prefix}.name"),
            simple_input(InputType::Text { placeholder: None }, false),
        ),
        column(
            Gap::Tight,
            vec![
                subheading("Mana Cost"),
                grid(
                    3,
                    vec![
                        mana("Neon", "neon"),
                        mana("Glitch", "glitch"),
                        mana("Chrome", "chrome"),
                        mana("Slag", "slag"),
                        mana("Bio", "bio"),
                        mana("Colorless", "colorless"),
                    ],
                )
                .gap(Gap::Normal),
            ],
        ),
        column(
            Gap::Tight,
            vec![
                subheading("Additional Costs"),
                grid(
                    2,
                    vec![
                        cost(
                            "Requires Tap",
                            "tap",
                            InputType::Checkbox {
                                label: "Requires Tap".to_string(),
                            },
                        ),
                        cost("Pay Life", "life", number(Some(0))),
                        cost("Tap Others", "tap_other", number(Some(0))),
                        cost("Sacrifice", "sacrifice", number(Some(0))),
                    ],
                )
                .gap(Gap::Normal),
            ],
        ),
        column(
            Gap::Normal,
            effect_fields("Effect", &format!("{key_prefix}.effect")),
        ),
    ])
}
