use serde_json::json;
use workflow_model::{CreateWorkflowDefinition, SubmitWhen};
use workflow_model::builder::{
    InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};

/// A greeting page: type a name, press Wave, and the bot answers.
pub fn definition(wave_action: &str) -> CreateWorkflowDefinition {
    let page = WorkflowVisualBuilder::new("page")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-2xl font-bold").text("Hello, {name}!").build(),
                WorkflowDisplayBuilder::new("flex gap-2")
                    .form(
                        "wave",
                        vec![
                            WorkflowDisplayBuilder::new("flex-1")
                                .input(
                                    "name",
                                    WorkflowInputBuilder::new()
                                        .r#type(InputTypeBuilder::text(Some("Your name".into())))
                                        .build(),
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .submit(
                                    vec![WorkflowDisplayBuilder::new("").text("Wave").build()],
                                    None,
                                    "wave",
                                    "default",
                                    "default",
                                )
                                .build(),
                        ],
                        SubmitWhen::OnSubmit,
                    )
                    .build(),
                WorkflowDisplayBuilder::new("text-muted-foreground").text("{reply}").build(),
            ],
        )
        .build();

    WorkflowDefinitionBuilder::new("hello", "Hello")
        .response("name", json!("there"))
        .add_action(WorkflowActionBuilder::new("wave").run_server_action().target(wave_action).build())
        .add_node("home", WorkflowNodeBuilder::new("Hello").visual(page).build())
        .build()
}
