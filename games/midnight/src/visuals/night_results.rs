use workflow::{WorkflowDisplay, builder::Cond, ui::*};

use crate::role_image;

fn player_role_chip(var: &str) -> WorkflowDisplay {
    row(
        Gap::Tight,
        vec![
            role_image(&format!("{var}.role"), Size::Small),
            person(
                format!("{{{var}.player.avatar_url}}"),
                format!("{{{var}.player.name}}"),
            )
            .size(Size::Small)
            .meta(format!("{{{var}.role.name}}")),
        ],
    )
    .nowrap()
}

pub fn night_results() -> WorkflowDisplay {
    section("What happened last night")
        .content(vec![
            when(
                Cond::response_exists("game.result.actions"),
                vec![items(
                    "game.result.actions",
                    "act",
                    Layout::Stack { gap: Gap::Normal },
                    vec![panel(vec![
                        grid(
                            3,
                            vec![
                                player_role_chip("act.who"),
                                body("{act.action}").centered(),
                                items(
                                    "act.what",
                                    "t",
                                    Layout::Row { gap: Gap::Normal },
                                    vec![player_role_chip("t")],
                                ),
                            ],
                        )
                        .gap(Gap::Normal),
                    ])],
                )],
            ),
            when(
                Cond::not(Cond::response_exists("game.result.actions")),
                vec![empty("No night actions were recorded.")],
            ),
        ])
        .build()
}

pub fn new_game_action() -> WorkflowDisplay {
    row(
        Gap::Normal,
        vec![primary_action(
            "Create New Game",
            act("games.midnight.reset"),
        )],
    )
    .justify(Justify::Center)
}
