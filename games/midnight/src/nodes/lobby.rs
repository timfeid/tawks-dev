use std::collections::HashMap;
use workflow::builder::{Cond, WorkflowDisplayBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder};
use workflow::{NodeCondition, WorkflowDisplay, WorkflowNode, ui::*};

use crate::MidnightPage;
use crate::role_image;

fn is_owner() -> NodeCondition {
    Cond::response_equals("$user_id", serde_json::json!("{game.owner_user_id}"))
}

fn role_card_face(var: &str) -> Vec<WorkflowDisplay> {
    vec![
        role_image(var, Size::Large),
        subheading(format!("{{{var}.name}}"))
            .themed()
            .centered()
            .tone_from(format!("{{{var}.alliance_tone}}")),
        row(
            Gap::Tight,
            vec![
                badge(format!("{{{var}.alliance}}"), Tone::Default)
                    .tone_from(format!("{{{var}.alliance_tone}}")),
                WorkflowDisplayBuilder::new()
                    .tooltip(
                        format!("{{{var}.description}}"),
                        vec![icon("info", IconSize::Small).tone(Tone::Muted)],
                    )
                    .build(),
            ],
        )
        .justify(Justify::Center),
    ]
}

/// A role in the grimoire: the host turns it on or off, everyone else sees
/// which are in play.
fn role_choice(var: &str) -> WorkflowDisplay {
    let toggle = || {
        act("games.midnight.enable_role")
            .input("role_id", format!("{{{var}.id}}"))
            .disabled_when(Cond::not(is_owner()))
    };
    let enabled = |on: bool| Cond::response_equals(format!("{var}.enabled"), serde_json::json!(on));
    column(
        Gap::None,
        vec![
            when(
                enabled(true),
                vec![choice(role_card_face(var), toggle()).state(State::Selected)],
            ),
            when(
                enabled(false),
                vec![choice(role_card_face(var), toggle()).state(State::Muted)],
            ),
        ],
    )
}

pub fn create() -> WorkflowNode {
    let mut meta = HashMap::new();
    meta.insert("duration".into(), serde_json::json!(5000));

    let joined = Cond::some_in_list(
        "game.players",
        "player",
        Cond::response_equals("$user_id", serde_json::json!("{player.id}")),
    );
    let enough_players = Cond::gte(
        "game.players_count",
        serde_json::json!("{game.players_required}"),
    );

    WorkflowNodeBuilder::new("Lobby State")
        .condition(Cond::and(vec![
            Cond::response_exists("game"),
            Cond::response_equals("game.stage", serde_json::json!("Lobby")),
        ]))
        .visual(
            WorkflowVisualBuilder::new("fog_anim")
                .animation(".wf-shell", "fog", Some("on_appear".into()), None, meta)
                .build(),
        )
        .visual(
            WorkflowVisualBuilder::new("main_content")
                .midnight_page(
                    PageWidth::Wide,
                    vec![
                        hero("The Gathering")
                            .lead("{game.players_count} / {game.players_required} SOULS PRESENT")
                            .build(),
                        items(
                            "game.players",
                            "player",
                            Layout::Row { gap: Gap::Loose },
                            vec![person("{player.avatar_url}", "{player.name}").size(Size::Large)],
                        ),
                        section("The Grimoire")
                            .content(vec![items(
                                "game.all_roles",
                                "role",
                                Layout::Grid {
                                    columns: 4,
                                    gap: Gap::Normal,
                                    fixed: false,
                                },
                                vec![role_choice("role")],
                            )])
                            .build(),
                        action_bar(vec![
                            when(
                                Cond::not(joined),
                                vec![primary_action(
                                    "ENTER THE CIRCLE",
                                    act("games.midnight.join"),
                                )],
                            ),
                            when(
                                Cond::and(vec![is_owner(), enough_players.clone()]),
                                vec![primary_action("BEGIN RITUAL", act("games.midnight.start"))],
                            ),
                            when(
                                Cond::and(vec![is_owner(), Cond::not(enough_players)]),
                                vec![fine("Waiting for souls...")],
                            ),
                        ]),
                    ],
                )
                .build(),
        )
        .build()
}
