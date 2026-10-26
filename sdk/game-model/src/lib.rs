//! Contracts for bot-owned, installed games. A launch request describes a game
//! session, never a shell command, executable path, or arbitrary environment.
use serde::{Deserialize, Serialize};
use specta::Type;

pub const LAUNCH_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct GameMode {
    pub id: String,
    pub name: String,
    pub min_players: u16,
    pub max_players: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RegisterGame {
    pub key: String,
    pub name: String,
    pub niche_id: String,
    pub app_id: String,
    pub modes: Vec<GameMode>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct RegisteredGame {
    pub id: String,
    pub bot_user_id: String,
    #[serde(flatten)]
    #[specta(flatten)]
    pub game: RegisterGame,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct GameInvitePerson {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct CreateGameInvite {
    pub game_id: String,
    pub mode_id: String,
    pub user_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameInviteStatus {
    Pending,
    Accepted,
    Declined,
    Cancelled,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct GameParticipant {
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub status: GameInviteStatus,
    pub invite_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct GameSession {
    pub id: String,
    pub game_id: String,
    pub bot_user_id: String,
    pub game_name: String,
    pub app_id: String,
    pub mode: GameMode,
    pub host_user_id: String,
    pub participants: Vec<GameParticipant>,
    pub cancelled: bool,
    pub expired: bool,
    pub expires_at: String,
}

/// Returned only to a participating person, after their own launch request.
/// Tickets must never be persisted in shared workflow responses or chat cards.
#[derive(Clone, Serialize, Deserialize, Type)]
pub struct GameLaunch {
    pub protocol: u32,
    pub game_id: String,
    pub app_id: String,
    pub session_id: String,
    pub mode_id: String,
    pub ticket: String,
}

impl std::fmt::Debug for GameLaunch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameLaunch")
            .field("game_id", &self.game_id)
            .field("app_id", &self.app_id)
            .field("session_id", &self.session_id)
            .field("mode_id", &self.mode_id)
            .field("ticket", &"<redacted>")
            .finish()
    }
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

impl RegisterGame {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !identifier(&self.key)
            || !identifier(&self.app_id)
            || self.niche_id.is_empty()
            || self.niche_id.len() > 200
        {
            return Err(
                "Game and app identifiers must contain lowercase letters, numbers, '-' or '_'",
            );
        }
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.modes.is_empty()
            || self.modes.len() > 16
        {
            return Err("A game needs a name and 1 to 16 modes");
        }
        let mut ids = std::collections::HashSet::new();
        for mode in &self.modes {
            if !identifier(&mode.id)
                || mode.name.trim().is_empty()
                || mode.name.len() > 80
                || mode.min_players < 2
                || mode.max_players < mode.min_players
                || mode.max_players > 32
                || !ids.insert(&mode.id)
            {
                return Err(
                    "Modes need unique identifiers and a valid player count between 2 and 32",
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> RegisterGame {
        RegisterGame {
            key: "nhl-legacy".into(),
            name: "NHL Legacy".into(),
            niche_id: "hockey".into(),
            app_id: "nhl-legacy".into(),
            modes: vec![
                GameMode {
                    id: "ranked".into(),
                    name: "Ranked".into(),
                    min_players: 2,
                    max_players: 2,
                },
                GameMode {
                    id: "quick".into(),
                    name: "Quick".into(),
                    min_players: 2,
                    max_players: 2,
                },
                GameMode {
                    id: "online_play".into(),
                    name: "Online Team Play".into(),
                    min_players: 2,
                    max_players: 4,
                },
            ],
        }
    }

    #[test]
    fn modes_are_typed_and_cannot_expand_into_commands_or_duplicate_ids() {
        assert!(game().validate().is_ok());
        let mut bad = game();
        bad.app_id = "powershell.exe -Command something".into();
        assert!(bad.validate().is_err());
        let mut bad = game();
        bad.modes.push(bad.modes[0].clone());
        assert!(bad.validate().is_err());
    }

    #[test]
    fn launch_shape_contains_only_the_typed_handoff_and_redacts_the_ticket() {
        let launch = GameLaunch {
            protocol: LAUNCH_PROTOCOL,
            game_id: "hockey:nhl-legacy".into(),
            app_id: "nhl-legacy".into(),
            session_id: "GS-123".into(),
            mode_id: "ranked".into(),
            ticket: "secret".into(),
        };
        assert!(!format!("{launch:?}").contains("secret"));
        let value = serde_json::to_value(launch).unwrap();
        assert_eq!(value["mode_id"], "ranked");
        assert!(value.get("command").is_none() && value.get("path").is_none());
    }
}
