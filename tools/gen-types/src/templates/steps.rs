use workflow_model::builder::{
    InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow_model::ui::*;
use workflow_model::{CreateWorkflowDefinition, SubmitWhen, WorkflowDisplay, WorkflowInput};

fn field(id: &str, input: WorkflowInput) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new().input(id, input).build()
}

/// Typed values only reach an action from inside a form.
fn form(action: &str, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new().form(action, vec![column(Gap::Normal, content)], SubmitWhen::OnSubmit).build()
}

/// Three pages: Next and Back move between nodes, and Done completes the instance.
pub fn definition() -> CreateWorkflowDefinition {
    let first = WorkflowVisualBuilder::new("first")
        .page(
            PageWidth::Narrow,
            vec![
                heading("Step 1: who are you?"),
                form(
                    "next",
                    vec![
                        field(
                            "name",
                            WorkflowInputBuilder::new()
                                .r#type(InputTypeBuilder::text(Some("Name".into())))
                                .required(true)
                                .build(),
                        ),
                        row(Gap::Tight, vec![primary_action("Next", act("next")).submits()]),
                    ],
                ),
            ],
        )
        .build();
    let second = WorkflowVisualBuilder::new("second")
        .page(
            PageWidth::Narrow,
            vec![
                heading("Step 2: pick a number, {name}"),
                form(
                    "next",
                    vec![
                        field(
                            "favorite",
                            WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(1), Some(10))).build(),
                        ),
                        row(
                            Gap::Tight,
                            vec![secondary_action("Back", act("back")), primary_action("Next", act("next")).submits()],
                        ),
                    ],
                ),
            ],
        )
        .build();
    let summary = WorkflowVisualBuilder::new("summary")
        .page(
            PageWidth::Narrow,
            vec![
                heading("Thanks, {name}"),
                body("You picked {favorite}."),
                row(Gap::Tight, vec![secondary_action("Back", act("back")), primary_action("Done", act("finish"))]),
            ],
        )
        .build();

    WorkflowDefinitionBuilder::new("steps", "Steps")
        .add_action(WorkflowActionBuilder::new("next").next_node().build())
        .add_action(WorkflowActionBuilder::new("back").previous_node().build())
        .add_action(WorkflowActionBuilder::new("finish").submit().build())
        .add_node("a_start", WorkflowNodeBuilder::new("Step 1").visual(first).build())
        .add_node("b_details", WorkflowNodeBuilder::new("Step 2").parent_id("a_start").visual(second).build())
        .add_node("c_summary", WorkflowNodeBuilder::new("Summary").parent_id("b_details").visual(summary).build())
        .build()
}
