use std::collections::HashMap;

use anyhow::Result;
use tawks_bot::TawksBot;
use workflow::{
    CreateWorkflowDefinition, DisplayVisual,
    builder::{WorkflowDefinitionBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder},
};

pub fn definition() -> CreateWorkflowDefinition {
    WorkflowDefinitionBuilder::new("sus_animation_workflow", "Sus Animation")
        .description("Play sus animation for a specific player")
        .add_node(
            "sus_anim_node",
            WorkflowNodeBuilder::new("Sus Anim Node")
                .visual(
                    WorkflowVisualBuilder::new("sus_anim_visual")
                        .display(DisplayVisual::Animation {
                            meta: {
                                let mut responses = HashMap::new();
                                responses
                                    .insert("message".into(), "{player.name} says SUS!".into());
                                responses.insert(
                                    "image".into(),
                                    "https://cdn.tawks.app/midnight/sus.png".into(),
                                );
                                responses
                            },
                            id: ".player-{target_id}".into(),
                            animation_type: "sus".into(),
                            trigger: Some("on_appear".into()),
                            delay_ms: None,
                        })
                        .build(),
                )
                .build(),
        )
        .build()
}

// Matches the signature expected by workflows/mod.rs
pub async fn register(bot: &TawksBot) -> Result<()> {
    bot.register_workflow(definition()).await?;
    Ok(())
}
