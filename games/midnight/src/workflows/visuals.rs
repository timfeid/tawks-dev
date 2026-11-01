use workflow::{NodeCondition, WorkflowDisplay, ui::*};

pub fn reveal_button() -> WorkflowDisplay {
    when(
        NodeCondition::ResponseEquals {
            field: "show_reveal_action".into(),
            value: true.into(),
        },
        vec![
            secondary_action("Reveal Cards", act("games.midnight.reveal_cards"))
                .submits()
                .full_width(),
        ],
    )
}
