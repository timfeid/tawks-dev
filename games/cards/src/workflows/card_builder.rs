use serde_json::json;
use workflow::builder::{Cond, WorkflowNodeBuilder, WorkflowVisualBuilder};
use workflow::ui::*;
use workflow::{InputType, WorkflowDisplay, WorkflowNode};

use super::helpers::{ability_editor, field, number, select, simple_input, trigger_editor};

fn text_input(placeholder: &str) -> InputType {
    InputType::Text {
        placeholder: Some(placeholder.to_string()),
    }
}

fn when_kind(kind: &str, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    when(Cond::response_equals("card.kind", json!(kind)), content)
}

fn basic_information() -> WorkflowDisplay {
    section("Basic Information")
        .content(vec![
            grid(
                2,
                vec![
                    field(
                        "Card ID *",
                        "card.id",
                        simple_input(text_input("unique-card-id"), true),
                    ),
                    field(
                        "Card Name *",
                        "card.name",
                        simple_input(text_input("Enter card name"), true),
                    ),
                ],
            )
            .gap(Gap::Normal),
            field(
                "Description",
                "card.description",
                simple_input(
                    InputType::TextArea {
                        placeholder: Some("Describe the card's lore and mechanics...".to_string()),
                        rows: Some(3),
                    },
                    false,
                ),
            ),
            field(
                "Image URL",
                "card.image_url",
                simple_input(text_input("https://..."), false),
            ),
        ])
        .build()
}

fn card_type() -> WorkflowDisplay {
    section("Card Type")
        .content(vec![
            grid(
                2,
                vec![
                    field(
                        "Supertype",
                        "card.supertype",
                        simple_input(select("game.supertype_options", "supertype_option"), false),
                    ),
                    field(
                        "Kind *",
                        "card.kind",
                        simple_input(select("game.kind_options", "kind_option"), true),
                    ),
                ],
            )
            .gap(Gap::Normal),
            when_kind(
                "unit",
                vec![field(
                    "Unit Type",
                    "card.unit_type",
                    simple_input(select("game.unit_type_options", "unit_type_option"), false),
                )],
            ),
            when_kind(
                "relic",
                vec![field(
                    "Artifact Type",
                    "card.artifact_type",
                    simple_input(
                        select("game.artifact_type_options", "artifact_type_option"),
                        false,
                    ),
                )],
            ),
        ])
        .build()
}

fn costs_and_stats() -> WorkflowDisplay {
    let mana = |label: &str, color: &str| {
        field(
            label,
            format!("card.cost.{color}"),
            simple_input(number(Some(0)), false),
        )
    };
    let stat = |label: &str, key: &str| {
        field(
            label,
            format!("card.{key}"),
            simple_input(number(None), false),
        )
    };

    section("Costs & Stats")
        .content(vec![
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
            when_kind(
                "unit",
                vec![panel(vec![
                    subheading("Unit Stats"),
                    grid(
                        2,
                        vec![stat("Power", "power"), stat("Toughness", "toughness")],
                    )
                    .gap(Gap::Normal),
                ])],
            ),
            when_kind(
                "agent",
                vec![panel(vec![
                    subheading("Agent"),
                    field(
                        "Starting Loyalty",
                        "card.loyalty",
                        simple_input(number(Some(0)), false),
                    ),
                ])],
            ),
        ])
        .build()
}

fn your_cards() -> WorkflowDisplay {
    section("Your Cards")
        .content(vec![items(
            "game.cards",
            "card_item",
            Layout::Scroll {
                gap: Gap::Tight,
                rows: 8,
            },
            vec![list_row("{card_item.name}").meta("ID: {card_item.id}")],
        )])
        .build()
}

pub(super) fn card_builder_node() -> WorkflowNode {
    let page = vec![
        hero("Card Builder")
            .lead("Create and customize your cards with powerful effects and abilities")
            .build(),
        basic_information(),
        card_type(),
        costs_and_stats(),
        section("Triggered Abilities")
            .content(vec![
                trigger_editor("Trigger 1", "card.trigger_one"),
                trigger_editor("Trigger 2", "card.trigger_two"),
                trigger_editor("Trigger 3", "card.trigger_three"),
            ])
            .build(),
        section("Activated Abilities")
            .content(vec![
                ability_editor("Ability 1", "card.ability_one"),
                ability_editor("Ability 2", "card.ability_two"),
            ])
            .build(),
        section("Preview")
            .content(vec![
                empty("Card Preview")
                    .titled("Coming soon...")
                    .with_icon("image"),
            ])
            .build(),
        your_cards(),
        action_bar(vec![
            secondary_action(
                "List All Cards",
                act("games.cardgame.list_cards").input("pagination", json!({"first": 20})),
            ),
            primary_action("Save Card", act("games.cardgame.save_card")),
        ]),
    ];

    WorkflowNodeBuilder::new("Card Builder")
        .visual(
            WorkflowVisualBuilder::new("builder")
                .page(PageWidth::Wide, page)
                .build(),
        )
        .build()
}
