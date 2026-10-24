use serde_json::json;
use workflow_model::builder::{
    Cond, InputTypeBuilder, WorkflowActionBuilder, WorkflowDefinitionBuilder, WorkflowDisplayBuilder,
    WorkflowInputBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder,
};
use workflow_model::{CreateWorkflowDefinition, SubmitWhen};

/// Guess a number the bot is thinking of. The secret stays on the bot: each
/// guess goes to a server action that answers higher, lower or correct, and a
/// correct answer moves the page to the win screen.
pub fn definition(check_action: &str) -> CreateWorkflowDefinition {
    let play = WorkflowVisualBuilder::new("play")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("Guess the number").build(),
                WorkflowDisplayBuilder::new("text-muted-foreground").text("{gn_message}").build(),
                WorkflowDisplayBuilder::new("flex gap-2")
                    .form(
                        "guess",
                        vec![
                            WorkflowDisplayBuilder::new("flex-1")
                                .input(
                                    "gn_guess",
                                    WorkflowInputBuilder::new()
                                        .r#type(InputTypeBuilder::number(Some(1), Some(100)))
                                        .required(true)
                                        .build(),
                                )
                                .build(),
                            WorkflowDisplayBuilder::new("")
                                .submit(
                                    vec![WorkflowDisplayBuilder::new("").text("Guess").build()],
                                    None,
                                    "guess",
                                    "default",
                                    "default",
                                )
                                .build(),
                        ],
                        SubmitWhen::OnSubmit,
                    )
                    .build(),
            ],
        )
        .build();
    let won = WorkflowVisualBuilder::new("won")
        .block(
            "mx-auto flex max-w-md flex-col gap-4 p-8",
            vec![
                WorkflowDisplayBuilder::new("text-xl font-semibold").text("You got it!").build(),
                WorkflowDisplayBuilder::new("text-muted-foreground").text("{gn_message}").build(),
            ],
        )
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
