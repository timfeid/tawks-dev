use workflow::{
    WorkflowDisplay, WorkflowNode,
    builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder},
    ui::*,
};

use std::collections::HashMap;

use crate::role_image;

pub fn reveal_card_display(var: &str) -> WorkflowDisplay {
    panel(vec![
        column(
            Gap::Normal,
            vec![
                role_image(var, Size::ExtraLarge),
                display_title(format!("{{{var}.name}}"))
                    .themed()
                    .centered()
                    .tone_from(format!("{{{var}.alliance_tone}}")),
                divider(),
                // Descriptions are HTML, so they render rather than show as text.
                WorkflowDisplayBuilder::new()
                    .wysiyg(format!("{var}.description"))
                    .build(),
                badge(format!("{{{var}.alliance}}"), Tone::Default)
                    .tone_from(format!("{{{var}.alliance_tone}}")),
            ],
        )
        .align(Align::Center),
    ])
}

pub fn create() -> WorkflowNode {
    WorkflowNodeBuilder::new("Role reveal")
        .description("The moment of truth")
        .visual(
            WorkflowVisualBuilder::new("waiting")
                .page(
                    PageWidth::Narrow,
                    vec![
                        lead("The fog lifts. You are...").centered(),
                        reveal_card_display("game.role").mark("card"),
                        column(
                            Gap::Tight,
                            vec![
                                eyebrow("NIGHT FALLS IN").centered(),
                                WorkflowDisplayBuilder::new()
                                    .countdown(
                                        "{game.role_reveal_until}",
                                        "timeleft",
                                        "{timeleft.total_seconds}",
                                        Some(10_000),
                                    )
                                    .build()
                                    .role(TextRole::Display),
                            ],
                        ),
                    ],
                )
                .centered()
                .typeface(Typeface::Spooky)
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("animation")
                .animation(
                    r#"[data-mark~="card"]"#,
                    "reveal",
                    Some("on_appear".into()),
                    None,
                    HashMap::new(),
                )
                .build(),
        )
        .condition(Cond::and(vec![
            Cond::response_equals("game.is_role_reveal", true.into()),
            Cond::response_equals("game.is_live", true.into()),
            Cond::response_equals("game.is_complete", false.into()),
        ]))
        .build()
}
