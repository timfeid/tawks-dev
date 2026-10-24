use workflow_model::{CreateWorkflowDefinition, SubmitWhen, WorkflowDisplay};
use workflow_model::builder::{
    InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

fn button(label: &str, action: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new("")
        .button(vec![WorkflowDisplayBuilder::new("").text(label).build()], None, action, "default", "default")
        .build()
}

fn submit(label: &str, action: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new("")
        .submit(vec![WorkflowDisplayBuilder::new("").text(label).build()], None, action, "default", "default")
        .build()
}

/// Typed values only reach an action from inside a form.
fn form(action: &str, content: Vec<WorkflowDisplay>) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new("flex flex-col gap-4").form(action, content, SubmitWhen::OnSubmit).build()
}

/// Three pages: Next and Back move between nodes, and Done completes the instance.
pub fn definition() -> CreateWorkflowDefinition {
    let first = WorkflowVisualBuilder::new("first")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Step 1: who are you?").build(),
                form(
                    "next",
                    vec![
                        WorkflowDisplayBuilder::new("")
                            .input(
                                "name",
                                WorkflowInputBuilder::new()
                                    .r#type(InputTypeBuilder::text(Some("Name".into())))
                                    .required(true)
                                    .build(),
                            )
                            .build(),
                        submit("Next", "next"),
                    ],
                ),
            ],
        )
        .build();
    let second = WorkflowVisualBuilder::new("second")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Step 2: pick a number, {name}").build(),
                form(
                    "next",
                    vec![
                        WorkflowDisplayBuilder::new("")
                            .input(
                                "favorite",
                                WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(1), Some(10))).build(),
                            )
                            .build(),
                        WorkflowDisplayBuilder::new("flex gap-2")
                            .div(vec![button("Back", "back"), submit("Next", "next")])
                            .build(),
                    ],
                ),
            ],
        )
        .build();
    let summary = WorkflowVisualBuilder::new("summary")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Thanks, {name}").build(),
                WorkflowDisplayBuilder::new("text-muted-foreground").text("You picked {favorite}.").build(),
                WorkflowDisplayBuilder::new("flex gap-2")
                    .div(vec![button("Back", "back"), button("Done", "finish")])
                    .build(),
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
