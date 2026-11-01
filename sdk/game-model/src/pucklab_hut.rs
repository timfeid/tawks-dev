//! Exact provider identity for account-backed PuckLab HUT. These values do not
//! grant launch capability or choose a local executable/URL.
use crate::{GameMode, RegisterGame};
use serde::{Deserialize, Serialize};
use specta::Type;

pub const PUCKLAB_HUT_KEY: &str = "legacy-hut";
pub const PUCKLAB_HUT_MODE: &str = "lab_rivals";
pub const PUCKLAB_HUT_APP: &str = "nhl-legacy";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PuckLabHutProvider {
    pub bot_user_id: String,
    pub niche_id: String,
}
impl PuckLabHutProvider {
    pub fn new(bot_user_id: String, niche_id: String) -> Result<Self, &'static str> {
        if [&bot_user_id, &niche_id].iter().any(|value| {
            value.is_empty() || value.len() > 200 || value.chars().any(char::is_control)
        }) {
            return Err("Configure the PuckLab bot and Hockey niche IDs");
        }
        Ok(Self {
            bot_user_id,
            niche_id,
        })
    }
    pub fn id(&self) -> String {
        format!("{}:{PUCKLAB_HUT_KEY}", self.bot_user_id)
    }
    pub fn matches(&self, game_id: &str, app_id: &str, mode_id: &str) -> bool {
        game_id == self.id() && app_id == PUCKLAB_HUT_APP && mode_id == PUCKLAB_HUT_MODE
    }
    pub fn metadata(&self) -> RegisterGame {
        RegisterGame {
            key: PUCKLAB_HUT_KEY.into(),
            name: "PuckLab".into(),
            niche_id: self.niche_id.clone(),
            app_id: PUCKLAB_HUT_APP.into(),
            modes: vec![GameMode {
                id: PUCKLAB_HUT_MODE.into(),
                name: "Lab Rivals".into(),
                min_players: 2,
                max_players: 2,
            }],
        }
    }
}

/// Fresh validator facts only. Never copied from a signed ticket's old fields.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PuckLabHutEligibility {
    pub user_id: String,
    pub niche_id: String,
    pub member: bool,
    pub installed: bool,
}

/// Reported by the configured HUT bot after reading the linked PuckLab
/// account. PuckLab still decides at fixture preparation; this drives prompts.
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
pub struct PuckLabHutClubReport {
    pub user_id: String,
    pub ready: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
pub struct PuckLabHutClubStatus {
    pub hut: bool,
    pub ready: Option<bool>,
}

/// An accepted HUT invitation whose PuckLab match Tawks wants to follow. The
/// bot reads it as the host, who is always one of the two participants.
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
pub struct PuckLabHutMatchCheck {
    pub session_id: String,
    pub host_user_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PuckLabHutMatchState {
    Open,
    Cancelled,
    Disputed,
    Final,
}

/// PuckLab's state of one invitation's match. `reason` is PuckLab's
/// cancellation code, shown as text by the invite card.
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
pub struct PuckLabHutMatchReport {
    pub session_id: String,
    pub state: PuckLabHutMatchState,
    pub reason: Option<String>,
}

impl PuckLabHutMatchReport {
    pub fn validate(&self) -> Result<(), &'static str> {
        let code = |value: &str| {
            !value.is_empty()
                && value.len() <= 64
                && value.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        };
        if self.session_id.is_empty()
            || self.session_id.len() > 64
            || self.session_id.chars().any(char::is_control)
        {
            return Err("Invalid HUT invitation");
        }
        match (&self.state, &self.reason) {
            (PuckLabHutMatchState::Cancelled, Some(reason)) if code(reason) => Ok(()),
            (PuckLabHutMatchState::Cancelled, _) => {
                Err("A cancelled HUT match needs a reason code")
            }
            (_, None) => Ok(()),
            _ => Err("Only a cancelled HUT match has a reason"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_account_provider_cannot_alias_hockey_or_other_modes() {
        let provider = PuckLabHutProvider::new("pucklab-bot".into(), "hockey".into()).unwrap();
        assert_eq!(provider.id(), "pucklab-bot:legacy-hut");
        assert!(provider.metadata().validate().is_ok());
        assert!(provider.matches(&provider.id(), "nhl-legacy", "lab_rivals"));
        for (game, app, mode) in [
            ("hockey:nhl-legacy", "nhl-legacy", "lab_rivals"),
            ("other-bot:legacy-hut", "nhl-legacy", "lab_rivals"),
            ("pucklab-bot:legacy-hut", "nhl-legacy", "quick"),
            ("pucklab-bot:legacy-hut", "pucklab", "lab_rivals"),
        ] {
            assert!(!provider.matches(game, app, mode));
        }
        assert!(PuckLabHutProvider::new("".into(), "hockey".into()).is_err());
        assert!(PuckLabHutProvider::new("bot".into(), "hockey\n".into()).is_err());
    }
    #[test]
    fn participants_without_hut_state_keep_their_previous_wire_shape() {
        let person = crate::GameParticipant {
            user_id: "alice".into(),
            username: "alice".into(),
            display_name: None,
            status: crate::GameInviteStatus::Accepted,
            invite_id: None,
            hut_club_ready: None,
        };
        let wire = serde_json::to_value(&person).unwrap();
        assert!(wire.get("hut_club_ready").is_none());
        let back: crate::GameParticipant = serde_json::from_value(wire).unwrap();
        assert_eq!(back.hut_club_ready, None);
        let ready = crate::GameParticipant {
            hut_club_ready: Some(false),
            ..person
        };
        assert_eq!(
            serde_json::to_value(&ready).unwrap()["hut_club_ready"],
            false
        );
    }
}
