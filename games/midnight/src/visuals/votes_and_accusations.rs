use workflow::{WorkflowDisplay, ui::*};

use crate::role_image;
use crate::visuals::vote_tile::{accusations_display, picture_action, votes_display};

/// Every player's revealed role with the votes and accusations they drew;
/// the row's state marks who won and who was voted out.
pub fn votes_and_accusations() -> WorkflowDisplay {
    section("Votes & Accusations")
        .content(vec![items(
            "game.players",
            "player",
            Layout::Stack { gap: Gap::Tight },
            vec![
                list_row("{player.name}")
                    .meta("{player.role.name}")
                    .state_from("{player.state}")
                    .lead_with(vec![role_image("player.role", Size::Medium).badge(vec![
                        avatar("{player.avatar_url}", "{player.name}", Size::ExtraSmall)
                            .mark("avatar player-{player.id}"),
                    ])])
                    .trail(vec![
                        votes_display(),
                        accusations_display(),
                        picture_action(
                            "https://cdn.tawks.app/midnight/sus.png",
                            "Sus",
                            Tone::Default,
                            act("games.midnight.sus").input("target_id", "{player.id}"),
                        ),
                    ])
                    .mark("winner-{player.is_winner} executed-{player.executed}"),
            ],
        )])
        .build()
}
