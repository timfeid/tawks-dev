use std::collections::HashMap;

use workflow::{
    ButtonType, DisplayType, DisplayVisual, WorkflowDisplay, WorkflowVisual, builder::WorkflowVisualBuilder,
};

pub fn votes_and_accusations_tile() -> WorkflowVisual {
    WorkflowVisualBuilder::new("votes_and_accusations_tile")
        .display(DisplayVisual::Tile {
            content: vec![
                // Title
                WorkflowDisplay {
                    class: "text-xl text-center font-semibold font-halloween mb-4 mt-16".into(),
                    r#type: DisplayType::Text { text: "Votes & Accusations".into() },
                },
                WorkflowDisplay {
                    class: "flex flex-wrap mx-auto max-w-3xl gap-2 w-full divide-y rounded mb-4".into(),
                    r#type: DisplayType::List {
                        items_key: "game.players".into(),
                        r#as: "player".into(),
                        content: vec![WorkflowDisplay {
                            class: "w-full relative {player.class} flex flex-col items-center justify-center rounded-lg border border-white/10 bg-white/5 group".into(),
                            r#type: DisplayType::Div {
                                content: vec![
                                    // Top: 3-column row (Votes | Player | Accusations)
                                    WorkflowDisplay {
                                        class: "w-full grid grid-cols-3".into(),
                                        r#type: DisplayType::Div {
                                            content: vec![
                                                // LEFT: Votes (avatars of people who voted for this player)
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
                                                                                content: vec![WorkflowDisplay {
                                                                                    class: "p-1".into(),
                                                                                    r#type: DisplayType::Div {
                                                                                        content: vec![WorkflowDisplay {
                                                                                            class: "size-5 rounded-md border".into(),
                                                                                            r#type: DisplayType::Image {
                                                                                                src: "{vote.avatar_url}".into(),
                                                                                                alt_text: "{vote.name}".into(),
                                                                                            },
                                                                                        }],
                                                                                    },
                                                                                }],
                                                                            },
                                                                        },
                                                                    ],
                                                                },
                                                            },
                                                        ],
                                                    },
                                                },

                                                // CENTER: Player (with eliminated styling, etc.)
                                                WorkflowDisplay {
                                                    class: "flex flex-col justify-center items-center font-halloween gap-1 p-2 w-full".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![
                                                            WorkflowDisplay {
                                                                class: "text-xl".into(),
                                                                r#type: DisplayType::Text {
                                                                    text: "{player.name}".into(),
                                                                }
                                                            },
                                                            WorkflowDisplay {
                                                                class: "relative rounded-full pointer-events-none".into(),
                                                                r#type: DisplayType::Div {
                                                                    content: vec![
                                                                        WorkflowDisplay {
                                                                            class: "midnight-tint-{player.role.alliance} size-16".into(),
                                                                            r#type: DisplayType::Image {
                                                                                src: "{player.role.image_url}".into(),
                                                                                alt_text: "{player.role.name}".into(),
                                                                            },
                                                                        },
                                                                        WorkflowDisplay {
                                                                            class: "player-{player.id} absolute bottom-0 right-0 -translate-x-[80%] translate-y-[15%] size-6 rounded-md".into(),
                                                                            r#type: DisplayType::Image {
                                                                                src: "{player.avatar_url}".into(),
                                                                                alt_text: "{player.name}".into(),
                                                                            },
                                                                        },
                                                                    ],
                                                                },
                                                            },
                                                            // player_display_elim("player"),
                                                            WorkflowDisplay {
                                                                class: "text-md".into(),
                                                                r#type: DisplayType::Text {
                                                                    text: "{player.role.name}".into(),
                                                                }
                                                            },
                                                        ]
                                                    }
                                                },

                                                // RIGHT: Accusations (role chip tinted by alliance + tiny accuser avatar)
                                                WorkflowDisplay {
                                                    class: "flex self-baseline".into(),
                                                    r#type: DisplayType::Div {
                                                        content: vec![WorkflowDisplay {
                                                            class: "flex items-end flex-col p-2 gap-2 w-full".into(),
                                                            r#type: DisplayType::Div {
                                                                content: vec![
                                                                    WorkflowDisplay {
                                                                        class: "flex flex-wrap -space-x-2 px-2".into(),
                                                                        r#type: DisplayType::List {
                                                                            items_key: "player.accusations".into(),
                                                                            r#as: "accuser".into(),
                                                                            content: vec![WorkflowDisplay {
                                                                                class: "relative rounded-full pointer-events-none".into(),
                                                                                r#type: DisplayType::Div {
                                                                                    content: vec![
                                                                                        WorkflowDisplay {
                                                                                            class: "midnight-tint-{accuser.voted_role.alliance} size-6".into(),
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
                                                                            }],
                                                                        },
                                                                    },
                                                                ],
                                                            },
                                                        }],
                                                    },
                                                },
                                            ],
                                        },
                                    },

                                    // Bottom: Sus button
                                    WorkflowDisplay {
                                        class: "items-between flex w-full justify-end gap-2 border-white/10".into(),
                                        r#type: DisplayType::Div {
                                            content: vec![WorkflowDisplay {
                                                class: "inline-flex size-12 items-center justify-center rounded-none grayscale opacity-50 md:opacity-5 group-hover:opacity-100 transition-all hover:grayscale-0".into(),
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
                                            }],
                                        },
                                    },
                                ],
                            },
                        }],
                    },
                },
            ],
        })
        .build()
}
