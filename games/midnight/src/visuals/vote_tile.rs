use std::collections::HashMap;

use serde_json::json;
use workflow::{
    InputType, NodeCondition, WorkflowDisplay, WorkflowInput, WorkflowVisual,
    builder::{WorkflowDisplayBuilder, WorkflowVisualBuilder},
    ui::*,
};

use crate::MidnightPage;
use crate::role_image;
use crate::visuals::shared::player::player_display;

/// A small picture button, for the game's own vote and sus art.
pub fn picture_action(src: &str, label: &str, tint: Tone, act: Act) -> WorkflowDisplay {
    choice(
        vec![
            media(src, label, Aspect::Square, Size::Small)
                .fit(Fit::Contain)
                .tone(tint),
            fine(label),
        ],
        act,
    )
    .across()
}

pub fn votes_display() -> WorkflowDisplay {
    avatar_stack("player.votes", "vote", "{vote.avatar_url}", "{vote.name}")
}

/// The roles others have pinned on this player, each with who pinned it.
pub fn accusations_display() -> WorkflowDisplay {
    items(
        "player.accusations",
        "accuser",
        Layout::Row { gap: Gap::Tight },
        vec![
            row(
                Gap::None,
                vec![
                    role_image("accuser.voted_role", Size::Small),
                    avatar(
                        "{accuser.player.avatar_url}",
                        "{accuser.player.name}",
                        Size::ExtraSmall,
                    ),
                ],
            )
            .nowrap(),
        ],
    )
}

fn accuse_input() -> WorkflowDisplay {
    WorkflowDisplayBuilder::new()
        .input(
            "role_id",
            WorkflowInput {
                r#type: InputType::Select {
                    filter: Some(NodeCondition::ResponseEquals {
                        field: "role.enabled".into(),
                        value: json!(true),
                    }),
                    items_key: "game.all_roles".into(),
                    r#as: "role".into(),
                    value: "{role.role_id}".into(),
                    content: vec![
                        row(
                            Gap::Normal,
                            vec![
                                role_image("role", Size::Small),
                                body("{role.name}")
                                    .themed()
                                    .tone_from("{role.alliance_tone}"),
                            ],
                        )
                        .nowrap(),
                    ],
                    selected_content: Some(vec![role_image("role", Size::Small)]),
                    unselected_content: Some(vec![
                        media(
                            "https://cdn.tawks.app/midnight/roles/unknown.png",
                            "Unknown",
                            Aspect::Square,
                            Size::Small,
                        )
                        .fit(Fit::Contain),
                    ]),
                    options: Vec::new(),
                    option_label: None,
                    label: None,
                    placeholder: None,
                },
                default_value: None,
                required: false,
                readonly: None,
                on_change_action: Some((
                    "games.midnight.accuse".into(),
                    HashMap::from([("target_id".into(), "{player.id}".into())]),
                )),
            },
        )
        .build()
}

fn player_panel() -> WorkflowDisplay {
    panel(vec![
        row(
            Gap::Normal,
            vec![
                player_display("player"),
                votes_display(),
                accusations_display(),
            ],
        )
        .between(),
        row(
            Gap::Tight,
            vec![
                picture_action(
                    "https://cdn.tawks.app/midnight/accuse.png",
                    "Vote",
                    Tone::Rose,
                    act("games.midnight.vote").input("target_id", "{player.id}"),
                ),
                picture_action(
                    "https://cdn.tawks.app/midnight/sus.png",
                    "Sus",
                    Tone::Default,
                    act("games.midnight.sus").input("target_id", "{player.id}"),
                ),
                accuse_input(),
            ],
        ),
    ])
}

fn lock_vote() -> WorkflowDisplay {
    let toggle = || act("games.midnight.toggle_lock");
    when(
        NodeCondition::And(vec![
            NodeCondition::ResponseEquals {
                field: "game.stage".into(),
                value: "Voting".into(),
            },
            NodeCondition::ResponseEquals {
                field: "game.me.has_voted".into(),
                value: true.into(),
            },
        ]),
        vec![action_bar(vec![
            when(
                NodeCondition::Or(vec![
                    NodeCondition::ResponseEquals {
                        field: "game.me.vote_locked".into(),
                        value: false.into(),
                    },
                    NodeCondition::Not(
                        NodeCondition::ResponseExists("game.me.vote_locked".into()).into(),
                    ),
                ]),
                vec![primary_action("Lock vote", toggle())],
            ),
            when(
                NodeCondition::ResponseEquals {
                    field: "game.me.vote_locked".into(),
                    value: true.into(),
                },
                vec![secondary_action("Unlock vote", toggle())],
            ),
        ])],
    )
}

pub fn vote_tile() -> WorkflowVisual {
    WorkflowVisualBuilder::new("vote_tile")
        .midnight_page(
            PageWidth::Normal,
            vec![
                section("Who do you think is who?")
                    .content(vec![items(
                        "game.players",
                        "player",
                        Layout::Stack { gap: Gap::Normal },
                        vec![player_panel()],
                    )])
                    .build(),
                lock_vote(),
            ],
        )
        .build()
}
