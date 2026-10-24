use serde_json::json;
use workflow_model::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowNodeBuilder,
    WorkflowVisualBuilder,
};
use workflow_model::transform::TransformOp;
use workflow_model::{CreateWorkflowDefinition, WorkflowDisplay};

fn button(label: &str, action: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new("")
        .button(vec![WorkflowDisplayBuilder::new("").text(label).build()], None, action, "default", "default")
        .build()
}

fn set_mood(action: &str, mood: &str) -> workflow_model::WorkflowAction {
    WorkflowActionBuilder::new(action)
        .transform_responses(vec![TransformOp::Set { path: "mood".into(), value: json!(mood) }], true)
        .build()
}

/// No bot needed: buttons change responses with transforms, and conditions
/// pick what to show.
pub fn definition() -> CreateWorkflowDefinition {
    let page = WorkflowVisualBuilder::new("page")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("How is your day?").build(),
                WorkflowDisplayBuilder::new("flex gap-2")
                    .div(vec![button("Good", "good"), button("Meh", "meh")])
                    .build(),
                WorkflowDisplayBuilder::new("")
                    .condition(
                        Cond::response_equals("mood", json!("good")),
                        vec![WorkflowDisplayBuilder::new("text-primary").text("Glad to hear it.").build()],
                    )
                    .build(),
                WorkflowDisplayBuilder::new("")
                    .condition(
                        Cond::response_equals("mood", json!("meh")),
                        vec![WorkflowDisplayBuilder::new("text-muted-foreground").text("Tomorrow is another one.").build()],
                    )
                    .build(),
            ],
        )
        .build();

    WorkflowDefinitionBuilder::new("mood", "Mood")
        .add_action(set_mood("good", "good"))
        .add_action(set_mood("meh", "meh"))
        .add_node("home", WorkflowNodeBuilder::new("Mood").visual(page).build())
        .build()
}
