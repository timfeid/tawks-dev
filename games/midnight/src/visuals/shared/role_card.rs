use workflow::{WorkflowDisplay, builder::WorkflowDisplayBuilder, ui::*};

use crate::role_image;

/// A role's picture and name, its description on hover.
pub fn role_select_display(var: &str) -> WorkflowDisplay {
    WorkflowDisplayBuilder::new()
        .tooltip(
            format!("{{{var}.description}}"),
            vec![
                column(
                    Gap::Tight,
                    vec![
                        role_image(var, Size::Medium),
                        subheading(format!("{{{var}.name}}"))
                            .themed()
                            .centered()
                            .tone_from(format!("{{{var}.alliance_tone}}")),
                    ],
                )
                .align(Align::Center),
            ],
        )
        .build()
}
