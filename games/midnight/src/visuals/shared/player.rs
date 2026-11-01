use workflow::{WorkflowDisplay, ui::*};

pub fn player_display(var: &str) -> WorkflowDisplay {
    person(format!("{{{var}.avatar_url}}"), format!("{{{var}.name}}"))
        .mark(format!("player-{{{var}.id}}"))
}
