//! What the playground starts from: a palette of displays and a few templates,
//! all built with the real builders so their JSON is exactly what bots send.

use serde::Serialize;
use serde_json::{Value, json};
use workflow_model::builder::{
    Cond, InputTypeBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder, table_header,
};
use workflow_model::{ui, CreateWorkflowDefinition, SelectOption, SubmitWhen, WorkflowDisplay};

use crate::templates;

#[derive(Serialize)]
pub struct PaletteItem {
    pub label: &'static str,
    pub hint: &'static str,
    pub display: WorkflowDisplay,
}

#[derive(Serialize)]
pub struct Template {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub definition: CreateWorkflowDefinition,
    /// What each server action answers in the playground, by action id.
    pub mocks: Value,
    pub source: &'static str,
}

#[derive(Serialize)]
pub struct Playground {
    pub palette: Vec<PaletteItem>,
    pub templates: Vec<Template>,
}

pub fn build() -> Playground {
    let palette = vec![
        PaletteItem {
            label: "Hero",
            hint: "The top of a page: title, lead and actions",
            display: ui::hero("Every game counts.")
                .eyebrow("Live results")
                .lead("Your rivalries. Your records. A place for every final horn.")
                .actions(vec![ui::link_primary("/", "Get started"), ui::link_secondary("/", "Learn more")])
                .build(),
        },
        PaletteItem {
            label: "Section",
            hint: "A part of the page with its own heading",
            display: ui::section("What's new")
                .lead("A line or two about this part of the page.")
                .content(vec![ui::grid(
                    2,
                    vec![
                        ui::feature("Fast", "Results land the moment a game ends.").with_icon("zap"),
                        ui::feature("Fair", "Everyone plays the same puzzle.").with_icon("scale"),
                    ],
                )])
                .build(),
        },
        PaletteItem {
            label: "Panel",
            hint: "A bordered box; tone Primary highlights it",
            display: ui::panel(vec![ui::subheading("Before you start"), ui::body("Each player needs their own copy.")]),
        },
        PaletteItem { label: "Heading", hint: "A section heading", display: ui::heading("A heading") },
        PaletteItem { label: "Text", hint: "Use {key} to show a response", display: ui::body("Hello, {name}") },
        PaletteItem {
            label: "Button",
            hint: "Runs a workflow action",
            display: ui::primary_action("Press me", ui::act("my_action")),
        },
        PaletteItem {
            label: "Form",
            hint: "Sends the inputs inside it to an action when its submit button is pressed",
            display: WorkflowDisplayBuilder::new()
                .form(
                    "my_action",
                    vec![ui::column(
                        ui::Gap::Tight,
                        vec![
                            WorkflowDisplayBuilder::new()
                                .input("name", WorkflowInputBuilder::new().r#type(InputTypeBuilder::text(Some("Type here".into()))).build())
                                .build(),
                            ui::primary_action("Send", ui::act("my_action")).submits(),
                        ],
                    )],
                    SubmitWhen::OnSubmit,
                )
                .build(),
        },
        PaletteItem {
            label: "Text input",
            hint: "Saved under its id. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new()
                .input("name", WorkflowInputBuilder::new().r#type(InputTypeBuilder::text(Some("Type here".into()))).build())
                .build(),
        },
        PaletteItem {
            label: "Number input",
            hint: "With a min and max. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new()
                .input("amount", WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(0), Some(100))).build())
                .build(),
        },
        PaletteItem {
            label: "Modal",
            hint: "A button that opens its contents over the page, for a short task",
            display: ui::dialog(
                "Start a wager",
                "New wager",
                vec![ui::body("Forms, buttons and text go here.")],
            ),
        },
        PaletteItem {
            label: "Drawer",
            hint: "The same from the side, for longer content or to keep the page in view",
            display: ui::dialog("Open details", "Details", vec![ui::body("Forms, buttons and text go here.")]).drawer(),
        },
        PaletteItem {
            label: "Dropdown",
            hint: "Pick from a list; options, or items_key to read rows. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new()
                .input(
                    "stake",
                    WorkflowInputBuilder::new()
                        .r#type(InputTypeBuilder::dropdown(
                            "Stake",
                            vec![
                                SelectOption::new("5", "5 points"),
                                SelectOption::new("10", "10 points"),
                                SelectOption::new("25", "25 points"),
                                SelectOption::new("50", "50 points"),
                            ],
                            Some("Pick a stake".into()),
                        ))
                        .build(),
                )
                .build(),
        },
        PaletteItem {
            label: "Switch",
            hint: "A boolean input. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new()
                .input("enabled", WorkflowInputBuilder::new().r#type(InputTypeBuilder::switch()).build())
                .build(),
        },
        PaletteItem {
            label: "Row",
            hint: "Children side by side, wrapping when they run out of room",
            display: ui::row(ui::Gap::Normal, vec![ui::body("Left"), ui::body("Right")]),
        },
        PaletteItem {
            label: "Row item",
            hint: "A title with a line under it, for lists and leaderboards",
            display: ui::list_row("Card title").meta("Some details"),
        },
        PaletteItem { label: "Badge", hint: "A small label", display: ui::badge("New", ui::Tone::Info) },
        PaletteItem {
            label: "Icon",
            hint: "A Lucide icon from the supported set",
            display: ui::icon("sparkles", ui::IconSize::Medium),
        },
        PaletteItem {
            label: "Image",
            hint: "Any https image",
            display: ui::media("https://cdn.tawks.app/avatars/default-1.png", "An avatar", ui::Aspect::Square, ui::Size::Medium),
        },
        PaletteItem {
            label: "List",
            hint: "Repeats its content for each item",
            display: ui::items("items", "item", ui::Layout::Stack { gap: ui::Gap::Tight }, vec![ui::body("{item.name}")]),
        },
        PaletteItem {
            label: "Table",
            hint: "Rows from a response list",
            display: WorkflowDisplayBuilder::new()
                .table(
                    "rows",
                    "row",
                    vec![table_header("Name", "name", None, None), table_header("Score", "score", Some("6rem".into()), None)],
                    vec![ui::body("{row.name}"), ui::body("{row.score}").numeric()],
                )
                .build(),
        },
        PaletteItem {
            label: "Show when",
            hint: "Content that appears when a condition holds",
            display: ui::when(Cond::response_exists("name"), vec![ui::body("You told us your name.")]),
        },
        PaletteItem {
            label: "Link",
            hint: "http(s) or an in-app path",
            display: ui::link_inline("https://tawks.app", "Open tawks"),
        },
    ];

    let templates = vec![
        Template {
            key: "hello",
            name: "Hello",
            description: "An input, a button and a server action answered by a mock.",
            definition: templates::hello::definition("hello.wave"),
            mocks: json!({ "hello.wave": { "UpdateResponses": { "reply": "The bot waves back." } } }),
            source: include_str!("templates/hello.rs"),
        },
        Template {
            key: "guess",
            name: "Guess",
            description: "A server action answers each guess, and the right answer switches the page.",
            definition: templates::guess::definition("guess.check"),
            mocks: json!({
                "guess.check": [
                    {
                        "if": { "ResponseEquals": { "field": "gn_guess", "value": 42 } },
                        "then": { "UpdateResponses": { "gn_done": true, "gn_message": "42 it is." } }
                    },
                    {
                        "if": { "Gte": { "field": "gn_guess", "value": 43 } },
                        "then": { "UpdateResponses": { "gn_message": "Too high. Try lower." } }
                    },
                    { "then": { "UpdateResponses": { "gn_message": "Too low. Try higher." } } }
                ]
            }),
            source: include_str!("templates/guess.rs"),
        },
        Template {
            key: "mood",
            name: "Mood",
            description: "Transforms and conditions, no bot needed.",
            definition: templates::mood::definition(),
            mocks: json!({}),
            source: include_str!("templates/mood.rs"),
        },
        Template {
            key: "steps",
            name: "Steps",
            description: "Forms carry what you type from page to page, and Done completes the instance.",
            definition: templates::steps::definition(),
            mocks: json!({}),
            source: include_str!("templates/steps.rs"),
        },
    ];

    Playground { palette, templates }
}
