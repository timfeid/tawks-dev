use serde_json::json;
use workflow_model::builder::{
    Cond, InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowInputBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow_model::ui::*;
use workflow_model::{CreateWorkflowDefinition, SubmitWhen};

/// Guess a number the bot is thinking of. The secret stays on the bot: each
/// guess goes to a server action that answers higher, lower or correct, and a
/// correct answer moves the page to the win screen.
pub fn definition(check_action: &str) -> CreateWorkflowDefinition {
    let number = WorkflowDisplayBuilder::new()
        .input(
            "gn_guess",
            WorkflowInputBuilder::new().r#type(InputTypeBuilder::number(Some(1), Some(100))).required(true).build(),
        )
        .build();
    let guess = WorkflowDisplayBuilder::new()
        .form(
            "guess",
            vec![column(Gap::Tight, vec![number, primary_action("Guess", act("guess")).submits()])],
            SubmitWhen::OnSubmit,
        )
        .build();
    let play = WorkflowVisualBuilder::new("play")
        .page(PageWidth::Narrow, vec![heading("Guess the number"), body("{gn_message}"), guess])
        .build();
    let won = WorkflowVisualBuilder::new("won")
        .page(PageWidth::Narrow, vec![heading("You got it!").tone(Tone::Success), body("{gn_message}")])
        .build();

    WorkflowDefinitionBuilder::new("guess", "Guess the number")
        .response("gn_message", json!("I'm thinking of a number from 1 to 100."))
        .response("gn_done", json!(false))
        .add_action(WorkflowActionBuilder::new("guess").run_server_action().target(check_action).build())
        .add_node("play", WorkflowNodeBuilder::new("Play").visual(play).build())
        .add_node(
            "won",
            WorkflowNodeBuilder::new("Won")
                .parent_id("play")
                .condition(Cond::response_equals("gn_done", json!(true)))
                .visual(won)
                .build(),
        )
        .build()
}
