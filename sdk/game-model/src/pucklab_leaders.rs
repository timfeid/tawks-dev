//! The PuckLab bot reports who leads PuckLab's Online Ranked ladder, as Tawks
//! users, so Tawks can move its traveling PuckLab titles.
use serde::{Deserialize, Serialize};
use specta::Type;

pub const HOLDERS_MAX: usize = 50;

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct PuckLabLeadersReport {
    /// First place on the main Online Ranked ladder, when that account is
    /// linked to Tawks. None means nobody linked holds first place.
    pub leader: Option<String>,
    /// Everyone sharing the longest current positive win streak there.
    pub hot_hand: Vec<String>,
}

fn user_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 200 && !id.chars().any(char::is_control)
}

impl PuckLabLeadersReport {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.hot_hand.len() > HOLDERS_MAX {
            return Err("Too many holders");
        }
        if self
            .leader
            .iter()
            .chain(&self.hot_hand)
            .all(|id| user_id(id))
        {
            Ok(())
        } else {
            Err("Invalid user")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_name_plain_user_ids() {
        assert!(PuckLabLeadersReport::default().validate().is_ok());
        let ok = PuckLabLeadersReport {
            leader: Some("USR-1".into()),
            hot_hand: vec!["USR-2".into()],
        };
        assert!(ok.validate().is_ok());
        let bad = PuckLabLeadersReport {
            leader: Some(String::new()),
            hot_hand: vec![],
        };
        assert!(bad.validate().is_err());
        let many = PuckLabLeadersReport {
            leader: None,
            hot_hand: vec!["u".into(); HOLDERS_MAX + 1],
        };
        assert!(many.validate().is_err());
    }
}
