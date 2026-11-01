use serde_json::json;
use workflow::{
    NodeCondition, WorkflowDisplay, WorkflowVisual,
    builder::{WorkflowDisplayBuilder, WorkflowVisualBuilder},
    ui::*,
};

use crate::MidnightPage;
use crate::role_image;
use crate::visuals::shared::role_card::role_select_display;

fn not_null(field: &str) -> NodeCondition {
    NodeCondition::Not(Box::new(NodeCondition::ResponseEquals {
        field: field.into(),
        value: json!(null),
    }))
}

fn countdown(until: &str, shake_under_ms_left: u32) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new()
        .countdown(
            until,
            "timeleft",
            "{timeleft.total_seconds}s",
            Some(shake_under_ms_left),
        )
        .build()
        .role(TextRole::Stat)
}

fn timed(heading: &str, picture: WorkflowDisplay, timer: WorkflowDisplay) -> WorkflowDisplay {
    column(
        Gap::Tight,
        vec![
            subheading(heading).themed().centered(),
            picture.caption(vec![timer]),
        ],
    )
    .align(Align::Center)
}

fn labelled_role(label: &str, var: &str) -> WorkflowDisplay {
    column(
        Gap::Tight,
        vec![fine(label).centered(), role_select_display(var)],
    )
    .align(Align::Center)
}

pub fn game_tile() -> WorkflowVisual {
    let voting = when(
        not_null("game.voting_results"),
        vec![timed(
            "Voting time!",
            avatar(
                "https://cdn.tawks.app/midnight/voting.png",
                "Voting time",
                Size::ExtraLarge,
            ),
            countdown("{game.voting_results.ends_at}", 10_000),
        )],
    );

    // Between turns, as while voting, there is no turn at all.
    let turn = when(
        NodeCondition::And(vec![
            NodeCondition::ResponseExists("game.turn.role".into()),
            not_null("game.turn.role"),
        ]),
        vec![timed(
            "{game.turn.role.name}",
            role_image("game.turn.role", Size::ExtraLarge),
            countdown("{game.turn.ends_at}", 5_000),
        )],
    );

    let up_next = when(
        not_null("game.next_role"),
        vec![labelled_role("Up next", "game.next_role")],
    );

    WorkflowVisualBuilder::new("game_tile")
        .midnight_page(
            PageWidth::Normal,
            vec![
                grid(
                    3,
                    vec![
                        labelled_role("You are", "game.me.role_details"),
                        column(Gap::None, vec![voting, turn]).align(Align::Center),
                        up_next,
                    ],
                )
                .fixed(),
            ],
        )
        .build()
}
