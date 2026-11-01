use serde_json::json;
use workflow_model::CreateWorkflowDefinition;
use workflow_model::builder::{
    Cond, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow_model::transform::TransformOp;
use workflow_model::ui::*;

fn set_mood(action: &str, mood: &str) -> workflow_model::WorkflowAction {
    WorkflowActionBuilder::new(action)
        .transform_responses(vec![TransformOp::Set { path: "mood".into(), value: json!(mood) }], true)
        .build()
}

/// No bot needed: buttons change responses with transforms, and conditions
/// pick what to show.
pub fn definition() -> CreateWorkflowDefinition {
    let page = WorkflowVisualBuilder::new("page")
        .page(
            PageWidth::Narrow,
            vec![
                heading("How is your day?"),
                row(Gap::Tight, vec![secondary_action("Good", act("good")), secondary_action("Meh", act("meh"))]),
                when(Cond::response_equals("mood", json!("good")), vec![body("Glad to hear it.").tone(Tone::Primary)]),
                when(Cond::response_equals("mood", json!("meh")), vec![body("Tomorrow is another one.")]),
            ],
        )
        .build();

    WorkflowDefinitionBuilder::new("mood", "Mood")
        .add_action(set_mood("good", "good"))
        .add_action(set_mood("meh", "meh"))
        .add_node("home", WorkflowNodeBuilder::new("Mood").visual(page).build())
        .build()
}
