use std::collections::HashMap;

use serde_json::json;
use workflow::InputType;
use workflow::builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder};

use super::helpers::{ability_editor, simple_input, trigger_editor};

// Helper function to create a section card with consistent styling
fn section_card(title: &str, content: Vec<workflow::WorkflowDisplay>) -> workflow::WorkflowDisplay {
    WorkflowDisplayBuilder::new("bg-slate-900/50 rounded-lg border border-slate-800 p-6 space-y-4")
        .div(vec![
            WorkflowDisplayBuilder::new(
                "text-lg font-semibold text-slate-200 border-b border-slate-800 pb-3 mb-4",
            )
            .text(title)
            .build(),
            WorkflowDisplayBuilder::new("space-y-4")
                .div(content)
                .build(),
        ])
        .build()
}

// Helper to create form field
fn form_field(label: &str, key: &str, input: workflow::WorkflowInput) -> workflow::WorkflowDisplay {
    WorkflowDisplayBuilder::new("space-y-2")
        .div(vec![
            WorkflowDisplayBuilder::new("block text-sm font-medium text-slate-400")
                .text(label)
                .build(),
            WorkflowDisplayBuilder::new("").input(key, input).build(),
        ])
        .build()
}

pub(super) fn card_builder_node() -> workflow::WorkflowNode {
    WorkflowNodeBuilder::new("Card Builder")
        .layout("h-full w-full bg-slate-950 text-slate-100 overflow-y-auto")
        .visual(
            WorkflowVisualBuilder::new("builder")
                .block("w-full max-w-7xl mx-auto px-4 py-8", vec![
                    // Header
                    WorkflowDisplayBuilder::new("mb-8")
                        .div(vec![
                            WorkflowDisplayBuilder::new("text-3xl font-bold text-white mb-2")
                                .text("Card Builder")
                                .build(),
                            WorkflowDisplayBuilder::new("text-sm text-slate-400")
                                .text("Create and customize your cards with powerful effects and abilities")
                                .build(),
                        ])
                        .build(),

                    // Main content grid
                    WorkflowDisplayBuilder::new("grid grid-cols-1 lg:grid-cols-3 gap-6")
                        .div(vec![
                            // Left column - Form (2/3 width on large screens)
                            WorkflowDisplayBuilder::new("lg:col-span-2 space-y-6")
                                .div(vec![
                                    // SECTION 1: Basic Information
                                    section_card("Basic Information", vec![
                                        WorkflowDisplayBuilder::new("grid grid-cols-1 md:grid-cols-2 gap-4")
                                            .div(vec![
                                                form_field("Card ID *", "card.id", simple_input(InputType::Text { placeholder: Some("unique-card-id".to_string()) }, true)),
                                                form_field("Card Name *", "card.name", simple_input(InputType::Text { placeholder: Some("Enter card name".to_string()) }, true)),
                                            ])
                                            .build(),
                                        form_field(
                                            "Description",
                                            "card.description",
                                            simple_input(InputType::TextArea { placeholder: Some("Describe the card's lore and mechanics...".to_string()), rows: Some(3) }, false),
                                        ),
                                        form_field("Image URL", "card.image_url", simple_input(InputType::Text { placeholder: Some("https://...".to_string()) }, false)),
                                    ]),

                                    // SECTION 2: Card Type
                                    section_card("Card Type", vec![
                                        WorkflowDisplayBuilder::new("grid grid-cols-1 md:grid-cols-2 gap-4")
                                            .div(vec![
                                                form_field(
                                                    "Supertype",
                                                    "card.supertype",
                                                    simple_input(
                                                        InputType::Select {
                                                            filter: None,
                                                            items_key: "game.supertype_options".to_string(),
                                                            r#as: "supertype_option".to_string(),
                                                            value: "{supertype_option.value}".to_string(),
                                                            content: vec![
                                                                WorkflowDisplayBuilder::new("")
                                                                    .text("{supertype_option.label}")
                                                                    .build(),
                                                            ],
                                                            selected_content: None,
                                                            unselected_content: None,
                                                            class: "px-4 py-2 hover:bg-slate-700".to_string(),
                                                        },
                                                        false,
                                                    ),
                                                ),
                                                form_field(
                                                    "Kind *",
                                                    "card.kind",
                                                    simple_input(
                                                        InputType::Select {
                                                            filter: None,
                                                            items_key: "game.kind_options".to_string(),
                                                            r#as: "kind_option".to_string(),
                                                            value: "{kind_option.value}".to_string(),
                                                            content: vec![
                                                                WorkflowDisplayBuilder::new("")
                                                                    .text("{kind_option.label}")
                                                                    .build(),
                                                            ],
                                                            selected_content: None,
                                                            unselected_content: None,
                                                            class: "px-4 py-2 hover:bg-slate-700".to_string(),
                                                        },
                                                        true,
                                                    ),
                                                ),
                                            ])
                                            .build(),
                                        // Conditional: Unit Type
                                        WorkflowDisplayBuilder::new("")
                                            .condition(
                                                Cond::response_equals("card.kind", json!("unit")),
                                                vec![form_field(
                                                    "Unit Type",
                                                    "card.unit_type",
                                                    simple_input(
                                                        InputType::Select {
                                                            filter: None,
                                                            items_key: "game.unit_type_options".to_string(),
                                                            r#as: "unit_type_option".to_string(),
                                                            value: "{unit_type_option.value}".to_string(),
                                                            content: vec![
                                                                WorkflowDisplayBuilder::new("")
                                                                    .text("{unit_type_option.label}")
                                                                    .build(),
                                                            ],
                                                            selected_content: None,
                                                            unselected_content: None,
                                                            class: "px-4 py-2 hover:bg-slate-700".to_string(),
                                                        },
                                                        false,
                                                    ),
                                                )],
                                            )
                                            .build(),
                                        // Conditional: Artifact Type
                                        WorkflowDisplayBuilder::new("")
                                            .condition(
                                                Cond::response_equals("card.kind", json!("relic")),
                                                vec![form_field(
                                                    "Artifact Type",
                                                    "card.artifact_type",
                                                    simple_input(
                                                        InputType::Select {
                                                            filter: None,
                                                            items_key: "game.artifact_type_options".to_string(),
                                                            r#as: "artifact_type_option".to_string(),
                                                            value: "{artifact_type_option.value}".to_string(),
                                                            content: vec![
                                                                WorkflowDisplayBuilder::new("")
                                                                    .text("{artifact_type_option.label}")
                                                                    .build(),
                                                            ],
                                                            selected_content: None,
                                                            unselected_content: None,
                                                            class: "px-4 py-2 hover:bg-slate-700".to_string(),
                                                        },
                                                        false,
                                                    ),
                                                )],
                                            )
                                            .build(),
                                    ]),

                                    // SECTION 3: Costs & Stats
                                    section_card("Costs & Stats", vec![
                                        WorkflowDisplayBuilder::new("space-y-3")
                                            .div(vec![
                                                WorkflowDisplayBuilder::new("text-sm font-medium text-slate-300 mb-2")
                                                    .text("Mana Cost")
                                                    .build(),
                                                WorkflowDisplayBuilder::new("grid grid-cols-3 md:grid-cols-6 gap-3")
                                                    .div(vec![
                                                        form_field("Neon", "card.cost.neon", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        form_field("Glitch", "card.cost.glitch", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        form_field("Chrome", "card.cost.chrome", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        form_field("Slag", "card.cost.slag", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        form_field("Bio", "card.cost.bio", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        form_field("Colorless", "card.cost.colorless", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                    ])
                                                    .build(),
                                            ])
                                            .build(),
                                        // Conditional: Unit Stats
                                        WorkflowDisplayBuilder::new("")
                                            .condition(
                                                Cond::response_equals("card.kind", json!("unit")),
                                                vec![
                                                    WorkflowDisplayBuilder::new("pt-4 border-t border-slate-800")
                                                        .div(vec![
                                                            WorkflowDisplayBuilder::new("text-sm font-medium text-slate-300 mb-3")
                                                                .text("Unit Stats")
                                                                .build(),
                                                            WorkflowDisplayBuilder::new("grid grid-cols-2 gap-4")
                                                                .div(vec![
                                                                    form_field("Power", "card.power", simple_input(InputType::Number { min: None, max: None }, false)),
                                                                    form_field("Toughness", "card.toughness", simple_input(InputType::Number { min: None, max: None }, false)),
                                                                ])
                                                                .build(),
                                                        ])
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                        // Conditional: Agent Loyalty
                                        WorkflowDisplayBuilder::new("")
                                            .condition(
                                                Cond::response_equals("card.kind", json!("agent")),
                                                vec![
                                                    WorkflowDisplayBuilder::new("pt-4 border-t border-slate-800")
                                                        .div(vec![
                                                            WorkflowDisplayBuilder::new("text-sm font-medium text-slate-300 mb-3")
                                                                .text("Agent")
                                                                .build(),
                                                            form_field("Starting Loyalty", "card.loyalty", simple_input(InputType::Number { min: Some(0), max: None }, false)),
                                                        ])
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                    ]),

                                    // SECTION 4: Triggered Abilities
                                    section_card("Triggered Abilities", vec![
                                        WorkflowDisplayBuilder::new("space-y-3")
                                            .div(vec![
                                                trigger_editor("Trigger 1", "card.trigger_one"),
                                                trigger_editor("Trigger 2", "card.trigger_two"),
                                                trigger_editor("Trigger 3", "card.trigger_three"),
                                            ])
                                            .build(),
                                    ]),

                                    // SECTION 5: Activated Abilities
                                    section_card("Activated Abilities", vec![
                                        WorkflowDisplayBuilder::new("space-y-3")
                                            .div(vec![
                                                ability_editor("Ability 1", "card.ability_one"),
                                                ability_editor("Ability 2", "card.ability_two"),
                                            ])
                                            .build(),
                                    ]),

                                    // Action Buttons
                                    WorkflowDisplayBuilder::new("flex gap-3 pt-4")
                                        .div(vec![
                                            WorkflowDisplayBuilder::new("flex-1")
                                                .button(
                                                    vec![
                                                        WorkflowDisplayBuilder::new("text-sm font-semibold")
                                                            .text("💾 Save Card")
                                                            .build(),
                                                    ],
                                                    None,
                                                    "games.cardgame.save_card",
                                                    "solid",
                                                    "md",
                                                )
                                                .build(),
                                            WorkflowDisplayBuilder::new("flex-1")
                                                .button(
                                                    vec![
                                                        WorkflowDisplayBuilder::new("text-sm font-semibold")
                                                            .text("📋 List All Cards")
                                                            .build(),
                                                    ],
                                                    Some(HashMap::from([("pagination".into(), json!({"first": 20}))])),
                                                    "games.cardgame.list_cards",
                                                    "outline",
                                                    "md",
                                                )
                                                .build(),
                                        ])
                                        .build(),
                                ])
                                .build(),

                            // Right column - Preview & Card List (1/3 width on large screens)
                            WorkflowDisplayBuilder::new("lg:col-span-1 space-y-6")
                                .div(vec![
                                    // Card Preview
                                    section_card("Preview", vec![
                                        WorkflowDisplayBuilder::new("w-full aspect-[2.5/3.5] rounded-lg overflow-hidden bg-gradient-to-br from-slate-800 to-slate-900 flex items-center justify-center border border-slate-700")
                                            .div(vec![
                                                WorkflowDisplayBuilder::new("text-center space-y-2")
                                                    .div(vec![
                                                        WorkflowDisplayBuilder::new("text-4xl")
                                                            .text("🎴")
                                                            .build(),
                                                        WorkflowDisplayBuilder::new("text-sm text-slate-400")
                                                            .text("Card Preview")
                                                            .build(),
                                                        WorkflowDisplayBuilder::new("text-xs text-slate-500")
                                                            .text("Coming soon...")
                                                            .build(),
                                                    ])
                                                    .build(),
                                            ])
                                            .build(),
                                    ]),

                                    // Existing Cards List
                                    section_card("Your Cards", vec![
                                        WorkflowDisplayBuilder::new("space-y-2 max-h-96 overflow-y-auto")
                                            .list(
                                                "game.cards",
                                                "card_item",
                                                vec![
                                                    WorkflowDisplayBuilder::new("p-3 rounded-md bg-slate-800/50 hover:bg-slate-700/50 transition-colors border border-slate-700/50 hover:border-slate-600")
                                                        .div(vec![
                                                            WorkflowDisplayBuilder::new("font-medium text-slate-200")
                                                                .text("{card_item.name}")
                                                                .build(),
                                                            WorkflowDisplayBuilder::new("text-xs text-slate-400 mt-1")
                                                                .text("ID: {card_item.id}")
                                                                .build(),
                                                        ])
                                                        .build(),
                                                ],
                                            )
                                            .build(),
                                    ]),
                                ])
                                .build(),
                        ])
                        .build(),
                ])
                .build(),
        )
        .build()
}
