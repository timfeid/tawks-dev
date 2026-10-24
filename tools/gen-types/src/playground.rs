//! What the playground starts from: a palette of displays and a few templates,
//! all built with the real builders so their JSON is exactly what bots send.

use serde::Serialize;
use serde_json::{Value, json};
use workflow_model::builder::{
    Cond, InputTypeBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder, table_header,
};
use workflow_model::{CardLayout, CreateWorkflowDefinition, SubmitWhen, WorkflowDisplay};

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

fn text(class: &str, value: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new(class).text(value).build()
}

pub fn build() -> Playground {
    let palette = vec![
        PaletteItem { label: "Heading", hint: "Text with a larger class", display: text("text-xl font-semibold", "A heading") },
        PaletteItem { label: "Text", hint: "Use {key} to show a response", display: text("", "Hello, {name}") },
        PaletteItem {
            label: "Button",
            hint: "Runs a workflow action",
            display: WorkflowDisplayBuilder::new("").button(vec![text("", "Press me")], None, "my_action", "default", "default").build(),
        },
        PaletteItem {
            label: "Form",
            hint: "Sends the inputs inside it to an action when its submit button is pressed",
            display: WorkflowDisplayBuilder::new("flex flex-col gap-2")
                .form(
                    "my_action",
                    vec![
                        WorkflowDisplayBuilder::new("")
                            .input("name", WorkflowInputBuilder::new().r#type(InputTypeBuilder::text(Some("Type here".into()))).build())
                            .build(),
                        WorkflowDisplayBuilder::new("").submit(vec![text("", "Send")], None, "my_action", "default", "default").build(),
                    ],
                    SubmitWhen::OnSubmit,
                )
                .build(),
        },
        PaletteItem {
            label: "Text input",
            hint: "Saved under its id. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new("")
                .input("name", WorkflowInputBuilder::new().r#type(InputTypeBuilder::text(Some("Type here".into()))).build())
                .build(),
        },
        PaletteItem {
            label: "Number input",
            hint: "With a min and max. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new("")
                .input("amount", WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(0), Some(100))).build())
                .build(),
        },
        PaletteItem {
            label: "Switch",
            hint: "A boolean input. Put it inside a Form so actions receive it",
            display: WorkflowDisplayBuilder::new("")
                .input("enabled", WorkflowInputBuilder::new().r#type(InputTypeBuilder::switch()).build())
                .build(),
        },
        PaletteItem {
            label: "Row",
            hint: "A div that lays its children out",
            display: WorkflowDisplayBuilder::new("flex items-center gap-2").div(vec![text("", "Left"), text("", "Right")]).build(),
        },
        PaletteItem {
            label: "Card",
            hint: "A bordered panel",
            display: WorkflowDisplayBuilder::new("")
                .card(vec![text("font-semibold", "Card title"), text("text-muted-foreground", "Some details")], CardLayout::Vertical)
                .build(),
        },
        PaletteItem { label: "Badge", hint: "A small label", display: WorkflowDisplayBuilder::new("").badge("secondary", "New").build() },
        PaletteItem {
            label: "Icon",
            hint: "A Lucide icon from the supported set",
            display: WorkflowDisplayBuilder::new("size-5").lucide_icon("sparkles", Some("Sparkles")).build(),
        },
        PaletteItem {
            label: "Image",
            hint: "Any https image",
            display: WorkflowDisplayBuilder::new("w-48 rounded-lg")
                .image("https://cdn.tawks.app/avatars/default-1.png", "An avatar")
                .build(),
        },
        PaletteItem {
            label: "List",
            hint: "Repeats its content for each item",
            display: WorkflowDisplayBuilder::new("flex flex-col gap-1")
                .list("items", "item", vec![text("", "{item.name}")])
                .build(),
        },
        PaletteItem {
            label: "Table",
            hint: "Rows from a response list",
            display: WorkflowDisplayBuilder::new("")
                .table(
                    "rows",
                    "row",
                    vec![table_header("Name", "name", None, None), table_header("Score", "score", Some("6rem".into()), None)],
                    vec![text("", "{row.name}"), text("", "{row.score}")],
                )
                .build(),
        },
        PaletteItem {
            label: "Show when",
            hint: "Content that appears when a condition holds",
            display: WorkflowDisplayBuilder::new("")
                .condition(Cond::response_exists("name"), vec![text("", "You told us your name.")])
                .build(),
        },
        PaletteItem {
            label: "Link",
            hint: "http(s) or an in-app path",
            display: WorkflowDisplayBuilder::new("link").link("https://tawks.app", vec![text("", "Open tawks")]).build(),
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
