use workflow_model::CreateWorkflowDefinition;
use workflow_model::builder::{
    InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

fn button(label: &str, action: &str) -> workflow_model::WorkflowDisplay {
    WorkflowDisplayBuilder::new("")
        .button(vec![WorkflowDisplayBuilder::new("").text(label).build()], None, action, "default", "default")
        .build()
}

/// Two pages and a finish: Next and Back move between nodes, Submit completes.
pub fn definition() -> CreateWorkflowDefinition {
    let first = WorkflowVisualBuilder::new("first")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Step 1: who are you?").build(),
                WorkflowDisplayBuilder::new("")
                    .input(
                        "name",
                        WorkflowInputBuilder::new()
                            .r#type(InputTypeBuilder::text(Some("Name".into())))
                            .required(true)
                            .build(),
                    )
                    .build(),
                button("Next", "next"),
            ],
        )
        .build();
    let second = WorkflowVisualBuilder::new("second")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Step 2: pick a number, {name}").build(),
                WorkflowDisplayBuilder::new("")
                    .input(
                        "favorite",
                        WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(1), Some(10))).build(),
                    )
                    .build(),
                WorkflowDisplayBuilder::new("flex gap-2")
                    .div(vec![button("Back", "back"), button("Finish", "finish")])
                    .build(),
            ],
        )
        .build();

    WorkflowDefinitionBuilder::new("steps", "Two steps")
        .add_action(WorkflowActionBuilder::new("next").next_node().build())
        .add_action(WorkflowActionBuilder::new("back").previous_node().build())
        .add_action(WorkflowActionBuilder::new("finish").submit().build())
        .add_node("a_start", WorkflowNodeBuilder::new("Step 1").visual(first).build())
        .add_node("b_details", WorkflowNodeBuilder::new("Step 2").parent_id("a_start").visual(second).build())
        .build()
}
