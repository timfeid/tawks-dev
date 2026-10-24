use workflow::{
    DisplayType, WorkflowDisplay, WorkflowVisual,
    builder::{Cond, WorkflowDisplayBuilder, WorkflowVisualBuilder},
};

fn player_role_chip(var: &str, extra: &str) -> WorkflowDisplay {
    WorkflowDisplay {
            class:
                "flex flex-col items-center gap-2 rounded-lg border border-white/10 bg-white/5 p-3 max-w-3xl mx-auto w-full"
                    .into(),
            r#type: DisplayType::Div {
                content: vec![
                    WorkflowDisplay {
                        class: "size-12 rounded-md midnight-tint-{".to_string()
                            + var
                            + ".role.alliance}",
                        r#type: DisplayType::Image {
                            src: format!("{{{var}.role.image_url}}"),
                            alt_text: format!("{{{var}.role.name}}"),
                        },
                    },
                    WorkflowDisplay {
                        class: "".into(),
                        r#type: DisplayType::Div {
                            content: vec![

    WorkflowDisplay {
        class: format!("{} inline-flex items-center gap-2 px-2 py-1 rounded-md", extra),
        r#type: DisplayType::Div {
            content: vec![

                WorkflowDisplay {
                    class: "size-6 rounded-md bg-background".into(),
                    r#type: DisplayType::Image {
                        src: format!("{{{var}.player.avatar_url}}"),
                        alt_text: format!("{{{var}.player.name}}"),
                    },
                },

                WorkflowDisplay {
                    class: "text-sm font-medium".into(),
                    r#type: DisplayType::Text {
                        text: format!("{{{var}.player.name}}"),
                    },
                },
            ],
        },
    }
                            ]
                        }
                    }
                ],
            },
        }
}

pub fn night_results_tile() -> WorkflowVisual {
    WorkflowVisualBuilder::new("night_results_tile")
        .tile(vec![
            // Background watermark image (grayscale, very low opacity)
            WorkflowDisplayBuilder::new(
                "fixed left-0 md:left-[var(--sidebar-width)] top-[25rem] flex right-0 grayscale opacity-5 pointer-events-none",
            )
            .div(vec![WorkflowDisplayBuilder::new("w-full max-w-3xl aspect-square mx-auto")
                .image(
                    "https://cdn.tawks.app/midnight/midnight.webp",
                    "midnight",
                )
                .build()])
            .build(),

            // Title
            WorkflowDisplayBuilder::new("text-xl text-center font-semibold font-halloween mt-6 mb-2")
                .text("What happened last night")
                .build(),

            // If actions exist, render the timeline + list
            WorkflowDisplayBuilder::new("")
                .condition(
                    Cond::response_exists("game.result.actions"),
                    vec![WorkflowDisplayBuilder::new("relative max-w-3xl w-full mx-auto")
                        .div(vec![
                            // vertical line
                            WorkflowDisplayBuilder::new("absolute left-4 top-0 bottom-0 w-px bg-white/10")
                                .div(vec![])
                                .build(),

                            // actions list
                            WorkflowDisplayBuilder::new("flex flex-col gap-3 max-w-3xl w-full mx-auto")
                                .list(
                                    "game.result.actions",
                                    "act",
                                    vec![
                                        WorkflowDisplayBuilder::new("relative pl-10 pr-3 py-3")
                                            .div(vec![
                                                // dot
                                                WorkflowDisplayBuilder::new("absolute left-3 top-16 size-2.5 rounded-full bg-[#B19B70] shadow")
                                                    .div(vec![])
                                                    .build(),

                                                // row content
                                                WorkflowDisplayBuilder::new("flex md:grid grid-cols-3 items-center flex-col")
                                                    .div(vec![
                                                        // who
                                                        player_role_chip("act.who", ""),

                                                        // action text
                                                        WorkflowDisplayBuilder::new("text-sm w-full text-gray-300 text-center")
                                                            .text("{act.action}")
                                                            .build(),

                                                        // targets
                                                        WorkflowDisplayBuilder::new("flex flex-wrap w-full items-center gap-2")
                                                            .list(
                                                                "act.what",
                                                                "t",
                                                                vec![player_role_chip("t", "")],
                                                            )
                                                            .build(),
                                                    ])
                                                    .build(),
                                            ])
                                            .build(),
                                    ],
                                )
                                .build(),
                        ])
                        .build()],
                )
                .build(),

            // Else: no actions
            WorkflowDisplayBuilder::new("")
                .condition(
                    Cond::not(Cond::response_exists("game.result.actions")),
                    vec![WorkflowDisplayBuilder::new("text-center text-gray-400 italic")
                        .text("No night actions were recorded.")
                        .build()],
                )
                .build(),
                // Create New Game button
            WorkflowDisplayBuilder::new("mt-10 flex justify-center")
                .button(
                    vec![
                        WorkflowDisplayBuilder::new("flex items-center gap-2")
                            .text("Create New Game")
                            .build(),
                    ],
                    None,
                    "games.midnight.reset",
                    "primary",
                    "lg",
                )
                .build(),
                    ])
        .build()
}
