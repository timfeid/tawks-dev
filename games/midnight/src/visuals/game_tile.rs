use serde_json::json;
use workflow::{
    DisplayType, DisplayVisual, NodeCondition, WorkflowDisplay, WorkflowVisual,
    builder::WorkflowVisualBuilder,
};

use crate::visuals::shared::role_card::role_select_display;

pub fn game_tile() -> WorkflowVisual {
    WorkflowVisualBuilder::new("game_tile")
    .display(DisplayVisual::Tile {
        content: vec![
            WorkflowDisplay {
                class: "fixed left-0 md:right-[var(--userlist-width)] md:left-[var(--sidebar-width)] top-[25rem] flex right-0 grayscale opacity-5 pointer-events-none".into(),
                r#type: DisplayType::Div {
                    content: vec![
                        WorkflowDisplay {
                            r#type: DisplayType::Image {
                                src: "https://cdn.tawks.app/midnight/midnight.webp".into(),
                                alt_text: "midnight".into(),
                            },
                            class: "midnightbg w-full max-w-3xl aspect-square mx-auto".into(),
                        }
                    ],
                },
            },

            WorkflowDisplay {
                class: "grid grid-cols-3 justify-between py-4 max-w-3xl w-full mx-auto".into(),
                r#type: DisplayType::Div {
                    content: vec![
                        WorkflowDisplay {
                            r#type: DisplayType::Div {
                                content: vec![
                                WorkflowDisplay {
                                    r#type: DisplayType::Text {
                                        text: "You are".into(),
                                    },
                                    class: "text-sm text-gray-300 ".into(),
                                },
                                WorkflowDisplay {
                                    r#type:
                                    role_select_display("game.me.role_details"),
                                    class: "flex items-center flex-col".into(),
                                }
                                ],
                            },
                            class: "flex flex-col gap-2 font-halloween justify-center items-center w-fit".into(),
                        },
                        WorkflowDisplay {
                            class: "".into(),
                            r#type: DisplayType::Condition {
                                condition: NodeCondition::Not(Box::new(NodeCondition::ResponseEquals {
                                    field: "game.voting_results".into(),
                                    value: json!(null).into(),
                                })),
                                content: vec![
                                    WorkflowDisplay {
                                        class: "font-halloween items-center justify-center flex flex-col".into(),
                                        r#type: DisplayType::Div {content: vec![

                                    WorkflowDisplay {
                                        class: "text-lg".into(),

                                        r#type: DisplayType::Text { text: "Voting time!".into() }
                                    },
                                    WorkflowDisplay {
                                        class: "".into(),
                                        r#type: DisplayType::Div { content: vec![

                                    WorkflowDisplay {

                                    class: "relative flex justify-center items-center gap-2".into(),
                                    r#type: DisplayType::Div { content: vec![

                                   WorkflowDisplay {
                            class: "size-24 rounded-full flex-shrink-0".into(),
                            r#type: DisplayType::Image {
                                src: format!("https://cdn.tawks.app/midnight/voting.png"),
                                alt_text: format!("Voting time"),
                            },
                        },
                                    WorkflowDisplay {
                                        r#type: DisplayType::Countdown {
                                            until: "{game.voting_results.ends_at}".into(),
                                            shake_under_ms_left: Some(10_000),
                                            r#as: "timeleft".into(),
                                            text: "{timeleft.total_seconds}s".into(),
                                        },
                                        class: "text-shadow-lg/30 text-lg absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 font-bold text-white transition-all duration-300 shake:text-red-500 shake:animate-pulse shake:scale-110".into(),
                                    }
                                    ] }
                                    }

                                        ] }
                                    }
                                        ]}
                                    },
                        ]
                            },
                        },
                        WorkflowDisplay {
                            class: "".into(),
                            r#type: DisplayType::Condition {
                                condition: NodeCondition::Not(Box::new(NodeCondition::ResponseEquals {
                                    field: "game.turn.role".into(),
                                    value: json!(null).into(),
                                })),
                                content: vec![
                                    WorkflowDisplay {
                                        class: "font-halloween items-center justify-center flex flex-col".into(),
                                        r#type: DisplayType::Div {content: vec![

                                    WorkflowDisplay {
                                        class: "text-lg".into(),

                                        r#type: DisplayType::Text { text: "{game.turn.role.name}".into() }
                                    },
                                    WorkflowDisplay {
                                        class: "".into(),
                                        r#type: DisplayType::Div { content: vec![

                                    WorkflowDisplay {

                                    class: "relative flex justify-center items-center".into(),
                                    r#type: DisplayType::Div { content: vec![

                                   WorkflowDisplay {
                            class: "size-24 midnight-tint-{game.turn.role.alliance}  flex-shrink-0 ".into(),
                            r#type: DisplayType::Image {
                                src: format!("{{game.turn.role.image_url}}"),
                                alt_text: format!("{{game.turn.role.name}}"),
                            },
                        },
                                    WorkflowDisplay {
                                        r#type: DisplayType::Countdown {
                                            until: "{game.turn.ends_at}".into(),
                                            shake_under_ms_left: Some(5_000),
                                            r#as: "timeleft".into(),
                                            text: "{timeleft.total_seconds}s".into(),
                                        },
                                        class: "text-shadow-lg/30 text-xl absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 font-bold text-white transition-all duration-300 shake:text-red-500 shake:animate-pulse shake:scale-110".into(),
                                    }
                                    ] }
                                    }

                                        ] }
                                    }
                                        ]}
                                    },
                        ]
                            },
                        },

                        WorkflowDisplay {
                            class: "".into(),
                            r#type: DisplayType::Condition {
                                condition: NodeCondition::Not(Box::new(NodeCondition::ResponseEquals {
                                    field: "game.next_role".into(),
                                    value: json!(null).into(),
                                })),
                                content: vec![WorkflowDisplay {
                                    r#type: DisplayType::Div {
                                        content: vec![
                                            WorkflowDisplay {
                                                r#type: DisplayType::Text {
                                                    text: "Up next".into(),
                                                },
                                                class: "text-sm text-gray-300 ".into(),
                                            },
                                            WorkflowDisplay {
                                                r#type:
                                                role_select_display("game.next_role"),
                                                class: "flex items-center flex-col".into(),
                                            },

                                        ],
                                    },
                                    class: "flex flex-col font-halloween gap-2 justify-center w-fit items-center justify-self-end".into(),
                                }],
                            },
                        },
                    ],
                },
            },
        ],
    })
    .build()
}
