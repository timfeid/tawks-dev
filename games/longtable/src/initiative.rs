//! The initiative order: who acts, in what order, and which round it is.

use serde::{Deserialize, Serialize};

pub const MAX_COMBATANTS: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    pub init: i32,
    /// Breaks ties: the higher bonus acts first.
    pub bonus: i32,
    pub sheet_id: Option<String>,
    /// The person who plays this combatant; None for the GM's own.
    pub owner: Option<String>,
    /// Kept from players until the GM reveals it.
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tracker {
    pub entries: Vec<Combatant>,
    pub current: Option<String>,
    pub round: u32,
    #[serde(default)]
    pub next_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TrackerView<'a> {
    pub entries: Vec<&'a Combatant>,
    pub current: Option<&'a str>,
    pub round: u32,
    /// A player's view hides whose turn it is when it is a hidden combatant's.
    pub hidden_turn: bool,
}

impl Tracker {
    pub fn add(&mut self, mut combatant: Combatant) -> Result<String, String> {
        if self.entries.len() >= MAX_COMBATANTS {
            return Err(format!("The order holds up to {MAX_COMBATANTS}"));
        }
        let name: String = combatant.name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            return Err("Give the combatant a name".into());
        }
        combatant.name = name.chars().take(40).collect();
        combatant.init = combatant.init.clamp(-99, 99);
        combatant.bonus = combatant.bonus.clamp(-30, 30);
        self.next_id += 1;
        combatant.id = format!("c{}", self.next_id);
        let id = combatant.id.clone();
        self.entries.push(combatant);
        Ok(id)
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let Some(index) = self.index_of(id) else {
            return false;
        };
        if self.current.as_deref() == Some(id) {
            self.current = if self.entries.len() > 1 {
                Some(self.entries[(index + 1) % self.entries.len()].id.clone())
            } else {
                None
            };
        }
        self.entries.remove(index);
        if self.entries.is_empty() {
            self.round = 0;
        }
        true
    }

    pub fn set_init(&mut self, id: &str, init: i32) -> bool {
        match self.entries.iter_mut().find(|c| c.id == id) {
            Some(combatant) => {
                combatant.init = init.clamp(-99, 99);
                true
            }
            None => false,
        }
    }

    pub fn set_hidden(&mut self, id: &str, hidden: bool) -> bool {
        match self.entries.iter_mut().find(|c| c.id == id) {
            Some(combatant) => {
                combatant.hidden = hidden;
                true
            }
            None => false,
        }
    }

    /// Highest initiative first, then highest bonus, then by name, so an
    /// order never shuffles between two sorts.
    pub fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            b.init
                .cmp(&a.init)
                .then(b.bonus.cmp(&a.bonus))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    pub fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        match self.current.as_deref().and_then(|id| self.index_of(id)) {
            None => {
                self.current = Some(self.entries[0].id.clone());
                self.round = self.round.max(1);
            }
            Some(index) => {
                let next = index + 1;
                if next >= self.entries.len() {
                    self.round += 1;
                }
                self.current = Some(self.entries[next % self.entries.len()].id.clone());
            }
        }
    }

    pub fn prev(&mut self) {
        let Some(index) = self.current.as_deref().and_then(|id| self.index_of(id)) else {
            return;
        };
        if index == 0 {
            if self.round <= 1 {
                return;
            }
            self.round -= 1;
            self.current = Some(self.entries[self.entries.len() - 1].id.clone());
        } else {
            self.current = Some(self.entries[index - 1].id.clone());
        }
    }

    pub fn clear(&mut self) {
        let next_id = self.next_id;
        *self = Tracker {
            next_id,
            ..Tracker::default()
        };
    }

    pub fn current(&self) -> Option<&Combatant> {
        let id = self.current.as_deref()?;
        self.entries.iter().find(|c| c.id == id)
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.entries.iter().position(|c| c.id == id)
    }

    pub fn view(&self, gm: bool) -> TrackerView<'_> {
        let current = self.current();
        let hidden_turn = !gm && current.is_some_and(|c| c.hidden);
        TrackerView {
            entries: self.entries.iter().filter(|c| gm || !c.hidden).collect(),
            current: if hidden_turn {
                None
            } else {
                current.map(|c| c.id.as_str())
            },
            round: self.round,
            hidden_turn,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combatant(name: &str, init: i32, bonus: i32) -> Combatant {
        Combatant {
            id: String::new(),
            name: name.into(),
            init,
            bonus,
            sheet_id: None,
            owner: None,
            hidden: false,
        }
    }

    fn names(tracker: &Tracker) -> Vec<&str> {
        tracker.entries.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn sorts_by_initiative_then_bonus_then_name() {
        let mut t = Tracker::default();
        t.add(combatant("Goblin", 12, 2)).unwrap();
        t.add(combatant("Mira", 17, 3)).unwrap();
        t.add(combatant("bandit", 12, 1)).unwrap();
        t.add(combatant("Archer", 12, 1)).unwrap();
        t.add(combatant("Ogre", 4, -1)).unwrap();
        t.sort();
        assert_eq!(names(&t), ["Mira", "Goblin", "Archer", "bandit", "Ogre"]);
        t.sort();
        assert_eq!(names(&t), ["Mira", "Goblin", "Archer", "bandit", "Ogre"]);
    }

    #[test]
    fn next_walks_the_order_and_counts_rounds() {
        let mut t = Tracker::default();
        let a = t.add(combatant("A", 20, 0)).unwrap();
        let b = t.add(combatant("B", 10, 0)).unwrap();
        let c = t.add(combatant("C", 5, 0)).unwrap();
        assert_eq!(t.round, 0);
        t.next();
        assert_eq!((t.current.as_deref(), t.round), (Some(a.as_str()), 1));
        t.next();
        t.next();
        assert_eq!((t.current.as_deref(), t.round), (Some(c.as_str()), 1));
        t.next();
        assert_eq!((t.current.as_deref(), t.round), (Some(a.as_str()), 2));
        t.prev();
        assert_eq!((t.current.as_deref(), t.round), (Some(c.as_str()), 1));
        t.prev();
        assert_eq!(t.current.as_deref(), Some(b.as_str()));
        t.prev();
        t.prev();
        assert_eq!((t.current.as_deref(), t.round), (Some(a.as_str()), 1));
    }

    #[test]
    fn removing_the_current_combatant_passes_the_turn_on() {
        let mut t = Tracker::default();
        t.add(combatant("A", 20, 0)).unwrap();
        let b = t.add(combatant("B", 10, 0)).unwrap();
        let c = t.add(combatant("C", 5, 0)).unwrap();
        t.next();
        t.next();
        assert!(t.remove(&b));
        assert_eq!(t.current.as_deref(), Some(c.as_str()));
        t.clear();
        assert!(t.entries.is_empty() && t.current.is_none() && t.round == 0);
        let d = t.add(combatant("D", 1, 0)).unwrap();
        assert_ne!(d, c, "ids are not reused after a clear");
    }

    #[test]
    fn players_never_see_hidden_combatants() {
        let mut t = Tracker::default();
        let hero = t.add(combatant("Hero", 15, 0)).unwrap();
        let mut lurker = combatant("Lurker", 18, 0);
        lurker.hidden = true;
        t.add(lurker).unwrap();
        t.sort();
        t.next();
        let player = t.view(false);
        assert_eq!(player.entries.len(), 1);
        assert_eq!(player.current, None);
        assert!(player.hidden_turn);
        let gm = t.view(true);
        assert_eq!(gm.entries.len(), 2);
        assert!(!gm.hidden_turn);
        t.next();
        assert_eq!(t.view(false).current, Some(hero.as_str()));
    }

    #[test]
    fn refuses_nameless_and_too_many() {
        let mut t = Tracker::default();
        assert!(t.add(combatant("   ", 1, 0)).is_err());
        for i in 0..MAX_COMBATANTS {
            t.add(combatant(&format!("N{i}"), 1, 0)).unwrap();
        }
        assert!(t.add(combatant("One more", 1, 0)).is_err());
    }
}
