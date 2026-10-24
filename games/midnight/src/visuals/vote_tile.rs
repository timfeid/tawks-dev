use std::collections::HashMap;

use serde_json::json;
use workflow::{
    ButtonType, DisplayType, DisplayVisual, InputType, NodeCondition, WorkflowDisplay,
    WorkflowInput, WorkflowVisual, builder::WorkflowVisualBuilder,
};

use crate::visuals::shared::player::player_display;
pub fn role_card_display_for_without_tooltip(var: &str, class: &str) -> DisplayType {
    let is_small = class.contains("small");

    let size_class = if is_small {
        "size-8 object-cover"
    } else {
        "size-16 rounded-full"
    };
    let name_class = if is_small { "text-lg" } else { "text-2xl" };
    let alliance_class = if is_small { "hidden" } else { "text-lg" };

    DisplayType::Div {
        content: vec![WorkflowDisplay {
            class: format!(
                "{class} font-halloween relative rounded-lg w-full max-w-64 min-w-48 flex justify-center items-center {}",
                if is_small { "gap-3" } else { "gap-4 p-4" }
            ),
            r#type: DisplayType::Div {
                content: vec![
                    WorkflowDisplay {
                        class: format!(
                            "{size_class} flex-shrink-0 midnight-tint-{{{var}.alliance}}"
                        ),
                        r#type: DisplayType::Image {
                            src: format!("{{{var}.image_url}}"),
                            alt_text: format!("{{{var}.name}}"),
                        },
                    },
                    WorkflowDisplay {
                        class: "flex-1".into(),
                        r#type: DisplayType::Div {
                            content: vec![
                                WorkflowDisplay {
                                    class: format!("font-bold {name_class}"),
                                    r#type: DisplayType::Text {
                                        text: format!("{{{var}.name}}"),
                                    },
                                },
                                WorkflowDisplay {
                                    class: format!(
                                        "italic midnight-text-{{{var}.aliance}} {alliance_class}"
                                    ),
                                    r#type: DisplayType::Text {
                                        text: format!("{{{var}.alliance}}"),
                                    },
                                },
                            ],
                        },
                    },
                ],
            },
        }],
    }
}

pub fn vote_tile() -> WorkflowVisual {
    WorkflowVisualBuilder::new("vote_tile")
        .display(DisplayVisual::Tile {
            content: vec![
                WorkflowDisplay {
                    class: "text-xl text-center font-semibold font-halloween mb-4 mt-16".into(),
                    r#type: DisplayType::Text {
                        text: "Who do you think is who?".into(),
                    },
                },
                WorkflowDisplay {
                    class: "flex flex-wrap mx-auto max-w-3xl gap-2 w-full divide-y rounded mb-4".into(),
                    r#type: DisplayType::List {
                        items_key: "game.players".into(),
                        r#as: "player".into(),
                        content: vec![WorkflowDisplay {
                            class: "w-full relative flex flex-col items-center justify-center rounded-lg border border-white/10  bg-white/5 group".into(),
                            r#type: DisplayType::Div {
                                content: vec![

                                    WorkflowDisplay {
                                        class: "w-full grid grid-cols-3 ".into(),
                                        r#type: DisplayType::Div {
                                            content: vec![
                                                WorkflowDisplay {
                                                    class: "flex flex-col p-2 gap-2".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![








                                                            WorkflowDisplay {
                                                                class: "flex items-center".into(),
                                                                r#type: DisplayType::Div {
                                                                    content: vec![
                                                                        WorkflowDisplay {
                                                                            class: "flex items-center -space-x-4 flex-wrap".into(),
                                                                            r#type: DisplayType::List {
                                                                                items_key: "player.votes".into(),
                                                                                r#as: "vote".into(),
                                                                                content: vec![
                                                                                    WorkflowDisplay {
                                                                                        class: "p-1".into(),
                                                                                        r#type: DisplayType::Div { content: vec![
                                                                                    WorkflowDisplay {
                                                                                        class: "size-5 rounded-md border".into(),
                                                                                        r#type: DisplayType::Image {
                                                                                            src: "{vote.avatar_url}".into(),
                                                                                            alt_text: "{vote.name}".into(),
                                                                                        },
                                                                                    }]
 }
                                                                                    }
                                                                                ],
                                                                            },
                                                                        },
                                                                    ]
                                                                }
                                                            },
                                                        ]
                                                    }
                                                },

                                                WorkflowDisplay {
                                                    class: "p-2".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![
                                                            player_display("player"),
                                                        ]
                                                    }
                                                },

                                                WorkflowDisplay {
                                                    class: "flex  self-baseline".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![
                                                WorkflowDisplay {
                                                    class: "flex items-end flex-col p-2 gap-2 w-full".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![







                                                            WorkflowDisplay {
                                                                class: "flex flex-wrap -space-x-2 px-2".into(),
                                                                r#type: DisplayType::List {
                                                                    items_key: "player.accusations".into(),
                                                                    r#as: "accuser".into(),
                                                                    content: vec![
                                                                        WorkflowDisplay {
                                                                            class: "relative rounded-full pointer-events-none".into(),
                                                                            r#type: DisplayType::Div {
                                                                                content: vec![
                                                                                    WorkflowDisplay {
                                                                                        class: " midnight-tint-{accuser.voted_role.alliance} size-6".into(),
                                                                                        r#type: DisplayType::Image {
                                                                                            src: "{accuser.voted_role.image_url}".into(),
                                                                                            alt_text: "{accuser.voted_role.name}".into(),
                                                                                        },
                                                                                    },
                                                                                    WorkflowDisplay {
                                                                                        class: "absolute grayscale bottom-0 right-0 -translate-x-[50%] translate-y-[50%] size-2.5 rounded-md".into(),
                                                                                        r#type: DisplayType::Image {
                                                                                            src: "{accuser.player.avatar_url}".into(),
                                                                                            alt_text: "{accuser.player.name}".into(),
                                                                                        },
                                                                                    },
                                                                                ],
                                                                            },
                                                                        }
                                                                    ],
                                                                },
                                                            }
                                                        ]
                                                    }
                                                }
                                                        ],
                                                    },
                                                },

                                            ],
                                        },
                                    },

                                    WorkflowDisplay {
                                        class: "items-between grid grid-cols-3 w-full justify-between gap-2 border-white/10".into(),
                                        r#type: DisplayType::Div {
                                            content: vec![

                                                WorkflowDisplay {
                                                    class: "inline-flex size-12 items-center justify-center grayscale opacity-50 md:opacity-5 group-hover:opacity-100 transition-all rounded-none transition hover:grayscale-0 w-full".into(),
                                                    r#type: DisplayType::Button {
                                                    r#type: ButtonType::Submit,
                                                        content: vec![WorkflowDisplay {
                                                            class: "midnight-tint-Hollow size-8".into(),
                                                            r#type: DisplayType::Image {
                                                                src: "https://cdn.tawks.app/midnight/accuse.png".into(),
                                                                alt_text: "vote".into(),
                                                            },
                                                        }],
                                                        action_id: "games.midnight.vote".into(),
                                                        input_map: Some({
                                                            let mut map = HashMap::new();
                                                            map.insert("target_id".into(), "{player.id}".into());
                                                            map
                                                        }),
                                                        variant: "ghost".into(),
                                                        size: "icon".into(),
                                                    },
                                                },

                                                WorkflowDisplay {
                                                    class: "inline-flex size-12 items-center justify-center  rounded-none grayscale opacity-50 md:opacity-5 group-hover:opacity-100 transition-all transition hover:grayscale-0 w-full".into(),
                                                    r#type: DisplayType::Button {
                                                    r#type: ButtonType::Submit,
                                                        content: vec![WorkflowDisplay {
                                                            class: "size-6".into(),
                                                            r#type: DisplayType::Image {
                                                                src: "https://cdn.tawks.app/midnight/sus.png".into(),
                                                                alt_text: "sus".into(),
                                                            },
                                                        }],
                                                        action_id: "games.midnight.sus".into(),
                                                        input_map: Some({
                                                            let mut map = HashMap::new();
                                                            map.insert("target_id".into(), "{player.id}".into());
                                                            map
                                                        }),
                                                        variant: "ghost".into(),
                                                        size: "icon".into(),
                                                    },
                                                },

                                                WorkflowDisplay {
                                                    class: "size-12 w-full".into(),
                                                    r#type: DisplayType::Input {
                                                        id: "role_id".into(),
                                                        input: WorkflowInput {
                                                            r#type: InputType::Select {
                                                                filter: Some(NodeCondition::ResponseEquals { field: "role.enabled".into(), value: json!(true) }),
                                                                class: "flex size-12 w-full items-center justify-center rounded-none border-0  bg-transparent grayscale opacity-50 md:opacity-5 group-hover:opacity-100 hover:bg-accent transition-all transition hover:grayscale-0".into(),
                                                                items_key: "game.all_roles".into(),
                                                                r#as: "role".into(),
                                                                value: "{role.role_id}".into(),
                                                                content: vec![WorkflowDisplay {
                                                                    class: "".into(),
                                                                    r#type: role_card_display_for_without_tooltip("role", "flex-row small"),
                                                                }],
                                                                selected_content: Some(vec![WorkflowDisplay {
                                                                    class: "size-8 flex-shrink-0 midnight-tint-{role.alliance}".into(),
                                                                    r#type: DisplayType::Image {
                                                                        src: "{role.image_url}".into(),
                                                                        alt_text: "{role.name}".into(),
                                                                    },
                                                                }]),
                                                                unselected_content: Some(vec![WorkflowDisplay {
                                                                    class: "size-8 flex-shrink-0".into(),
                                                                    r#type: DisplayType::Image {
                                                                        src: "https://cdn.tawks.app/midnight/roles/unknown.png".into(),
                                                                        alt_text: "Unknown".into(),
                                                                    },
                                                                }]),
                                                            },
                                                            default_value: None,
                                                            required: false,
                                                            readonly: None,
                                                            on_change_action: Some((
                                                                "games.midnight.accuse".into(),
                                                                {
                                                                    let mut map = HashMap::new();
                                                                    map.insert("target_id".into(), "{player.id}".into());
                                                                    map
                                                                },
                                                            )),
                                                        },
                                                    },
                                                },
                                            ],
                                        },
                                    },
                                ],
                            },
                        }],
                    },
                },
                WorkflowDisplay {
                    class: "".into(),
                    r#type: DisplayType::Condition {
                        condition: NodeCondition::And(vec![NodeCondition::ResponseEquals {
                            field: "game.stage".into(),
                            value: "Voting".into(),
                        }, NodeCondition::ResponseEquals {
                            field: "game.me.has_voted".into(),
                            value: true.into(),
                        }]),
                        content: vec![
                WorkflowDisplay {
                    class: "mx-auto max-w-3xl w-full".into(),
                r#type:            DisplayType::Button {
                                                    r#type: ButtonType::Submit,
                               input_map: None, action_id: "games.midnight.toggle_lock".into(), variant: "default".into(), size: "default".into(),
                                content: vec![
                                    WorkflowDisplay {
    class: "".into(),
    r#type: DisplayType::Condition {
        condition: NodeCondition::Or(vec![
            NodeCondition::ResponseEquals {
                field: "game.me.vote_locked".into(),
                value: false.into(),
            },
            NodeCondition::Not(
                NodeCondition::ResponseExists("game.me.vote_locked".into()).into()
            ),
        ]),
        content: vec![WorkflowDisplay {
            class: "mx-auto max-w-3xl w-full".into(),
            r#type: DisplayType::Text { text: "Lock vote".into() },
        }],
    },
},
WorkflowDisplay {
    class: "".into(),
    r#type: DisplayType::Condition {
        condition: NodeCondition::ResponseEquals {
            field: "game.me.vote_locked".into(),
            value: true.into(),
        },
        content: vec![WorkflowDisplay {
            class: "mx-auto max-w-3xl w-full".into(),
            r#type: DisplayType::Text { text: "Unlock vote".into() },
        }],
    },
},

                ],
            }}
                    ] }
                },
            ],
        })
        .build()
}
