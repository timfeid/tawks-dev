use serde_json::json;
use workflow_model::builder::{
    InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder, WorkflowInputBuilder,
    WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow_model::ui::*;
use workflow_model::{CreateWorkflowDefinition, SubmitWhen};

/// A greeting page: type a name, press Wave, and the bot answers.
pub fn definition(wave_action: &str) -> CreateWorkflowDefinition {
    let name = WorkflowDisplayBuilder::new()
        .input("name", WorkflowInputBuilder::new().r#type(InputTypeBuilder::text(Some("Your name".into()))).build())
        .build();
    let wave = WorkflowDisplayBuilder::new()
        .form(
            "wave",
            vec![column(Gap::Tight, vec![name, primary_action("Wave", act("wave")).submits()])],
            SubmitWhen::OnSubmit,
        )
        .build();
    let page = WorkflowVisualBuilder::new("page")
        .page(PageWidth::Narrow, vec![title("Hello, {name}!"), wave, body("{reply}")])
        .build();

    WorkflowDefinitionBuilder::new("hello", "Hello")
        .response("name", json!("there"))
        .response("reply", json!(""))
        .add_action(WorkflowActionBuilder::new("wave").run_server_action().target(wave_action).build())
        .add_node("home", WorkflowNodeBuilder::new("Hello").visual(page).build())
        .build()
}
