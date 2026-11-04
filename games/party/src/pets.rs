//! Petting and poking a pet in a room. Cosmetic only: everyone sees it react,
//! and nothing about the game changes.
use serde::{Deserialize, Serialize};

use crate::room::TICK_HZ;

pub const SPECIES: [&str; 5] = ["pet-ember", "pet-puddle", "pet-sprout", "pet-fizz", "pet-mochi"];

/// The room, native renderer, and Tawks use the same account hatch order.
pub fn species_for(user: &str, chosen: Option<&str>) -> u8 {
    if let Some(index) = chosen.and_then(|id| SPECIES.iter().position(|pet| *pet == id)) {
        return index as u8;
    }
    let hash = "pet:"
        .bytes()
        .chain(user.bytes())
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    (hash as usize % SPECIES.len()) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PetAction {
    Pet,
    Poke,
}

/// A seat's latest reaction. `n` counts up with every one in the room, so a
/// client that sees a new `n` plays it once.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PetReact {
    pub seat: u8,
    pub action: PetAction,
    pub n: u32,
    #[serde(skip)]
    tick: u64,
}

/// A pet finishes reacting before it reacts again.
pub const PET_EVERY: u64 = (TICK_HZ as u64 * 4) / 5;

#[derive(Clone, Debug, Default)]
pub struct PetReacts {
    latest: Vec<PetReact>,
    n: u32,
}

impl PetReacts {
    /// False while the seat's last reaction is still playing.
    pub fn react(&mut self, seat: u8, action: PetAction, tick: u64) -> bool {
        if let Some(last) = self.latest.iter().find(|r| r.seat == seat) {
            if tick.saturating_sub(last.tick) < PET_EVERY {
                return false;
            }
        }
        self.n = self.n.wrapping_add(1);
        self.latest.retain(|r| r.seat != seat);
        self.latest.push(PetReact {
            seat,
            action,
            n: self.n,
            tick,
        });
        true
    }

    pub fn view(&self) -> Vec<PetReact> {
        self.latest.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::Room;

    #[test]
    fn chosen_species_overrides_the_account_hatch() {
        assert_eq!(species_for("alice", Some("pet-fizz")), 3);
        assert_eq!(species_for("alice", Some("unknown")), species_for("alice", None));
        assert!(species_for("alice", None) < SPECIES.len() as u8);
    }

    #[test]
    fn a_seat_reacts_once_its_last_reaction_has_played() {
        let mut reacts = PetReacts::default();
        assert!(reacts.react(0, PetAction::Pet, 10));
        assert!(!reacts.react(0, PetAction::Poke, 10 + PET_EVERY - 1));
        assert!(
            reacts.react(1, PetAction::Poke, 11),
            "another seat is not held up"
        );
        assert!(reacts.react(0, PetAction::Poke, 10 + PET_EVERY));
        let view = reacts.view();
        assert_eq!(view.len(), 2);
        let mine = view.iter().find(|r| r.seat == 0).unwrap();
        assert_eq!((mine.action, mine.n), (PetAction::Poke, 3));
    }

    #[test]
    fn everyone_in_the_room_sees_a_seated_players_pet_react() {
        let mut room = Room::new("lobby", 1);
        room.connect("alice", "Alice", "").unwrap();
        assert!(room.pet_react("alice", PetAction::Pet).unwrap());
        assert!(
            !room.pet_react("alice", PetAction::Pet).unwrap(),
            "spamming waits"
        );
        assert!(room.pet_react("nobody", PetAction::Pet).is_err());
        let view = serde_json::to_value(room.view()).unwrap();
        assert_eq!(
            view["pet_reacts"],
            serde_json::json!([{ "seat": 0, "action": "pet", "n": 1 }])
        );
    }
}
