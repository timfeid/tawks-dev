//! Simple character sheets: the numbers a table looks at every turn.

use serde::{Deserialize, Serialize};

use crate::srd::CONDITIONS;

pub const ABILITIES: [&str; 6] = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
pub const SLOT_LEVELS: usize = 9;
pub const MAX_NAME_CHARS: usize = 60;
pub const MAX_NOTES_CHARS: usize = 4000;
pub const MAX_EXHAUSTION: u8 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slots {
    pub max: u8,
    pub used: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sheet {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub class_level: String,
    pub hp: i32,
    pub hp_max: i32,
    pub ac: i32,
    pub abilities: [i32; 6],
    pub conditions: Vec<String>,
    #[serde(default)]
    pub exhaustion: u8,
    pub slots: [Slots; SLOT_LEVELS],
    pub notes: String,
}

/// The fields a sheet edit may change; absent fields stay as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetPatch {
    pub name: Option<String>,
    pub class_level: Option<String>,
    pub hp: Option<i32>,
    pub hp_max: Option<i32>,
    pub ac: Option<i32>,
    pub abilities: Option<[i32; 6]>,
    pub conditions: Option<Vec<String>>,
    pub exhaustion: Option<u8>,
    pub slots: Option<[Slots; SLOT_LEVELS]>,
    pub notes: Option<String>,
}

/// What other players see of a sheet that is not theirs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PartyStrip<'a> {
    pub id: &'a str,
    pub owner: &'a str,
    pub name: &'a str,
    pub class_level: &'a str,
    pub hp: i32,
    pub hp_max: i32,
    pub ac: i32,
    pub conditions: &'a [String],
    pub exhaustion: u8,
}

pub fn modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

impl Sheet {
    pub fn new(id: String, owner: String, name: &str) -> Sheet {
        Sheet {
            id,
            owner,
            name: clean_line(name).unwrap_or_else(|| "New hero".into()),
            class_level: String::new(),
            hp: 10,
            hp_max: 10,
            ac: 10,
            abilities: [10; 6],
            conditions: Vec::new(),
            exhaustion: 0,
            slots: [Slots::default(); SLOT_LEVELS],
            notes: String::new(),
        }
    }

    pub fn initiative_bonus(&self) -> i32 {
        modifier(self.abilities[1])
    }

    pub fn strip(&self) -> PartyStrip<'_> {
        PartyStrip {
            id: &self.id,
            owner: &self.owner,
            name: &self.name,
            class_level: &self.class_level,
            hp: self.hp,
            hp_max: self.hp_max,
            ac: self.ac,
            conditions: &self.conditions,
            exhaustion: self.exhaustion,
        }
    }

    /// Checks every field first, so a bad edit changes nothing.
    pub fn apply(&mut self, patch: SheetPatch) -> Result<(), String> {
        let mut next = self.clone();
        if let Some(name) = patch.name {
            next.name = clean_line(&name).ok_or("Give the character a name")?;
        }
        if let Some(class_level) = patch.class_level {
            next.class_level = clean_line(&class_level).unwrap_or_default();
        }
        if let Some(hp_max) = patch.hp_max {
            in_range(hp_max, 1, 9999, "Max HP")?;
            next.hp_max = hp_max;
        }
        if let Some(hp) = patch.hp {
            in_range(hp, -9999, 9999, "HP")?;
            next.hp = hp;
        }
        if let Some(ac) = patch.ac {
            in_range(ac, 0, 50, "AC")?;
            next.ac = ac;
        }
        if let Some(abilities) = patch.abilities {
            for score in abilities {
                in_range(score, 1, 30, "Ability scores")?;
            }
            next.abilities = abilities;
        }
        if let Some(conditions) = patch.conditions {
            let mut known = Vec::new();
            for condition in conditions {
                let Some(name) = CONDITIONS
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(condition.trim()))
                else {
                    return Err(format!("{condition} is not a condition"));
                };
                if !known.iter().any(|k: &String| k == name.name) {
                    known.push(name.name.to_string());
                }
            }
            next.conditions = known;
        }
        if let Some(exhaustion) = patch.exhaustion {
            if exhaustion > MAX_EXHAUSTION {
                return Err(format!("Exhaustion goes from 0 to {MAX_EXHAUSTION}"));
            }
            next.exhaustion = exhaustion;
        }
        if let Some(slots) = patch.slots {
            for slot in slots {
                if slot.max > 9 || slot.used > slot.max {
                    return Err("Spell slots go up to 9 a level, and you can't use more than you have".into());
                }
            }
            next.slots = slots;
        }
        if let Some(notes) = patch.notes {
            if notes.chars().count() > MAX_NOTES_CHARS {
                return Err(format!("Keep notes under {MAX_NOTES_CHARS} characters"));
            }
            next.notes = notes;
        }
        *self = next;
        Ok(())
    }
}

fn clean_line(text: &str) -> Option<String> {
    let line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let line: String = line.chars().take(MAX_NAME_CHARS).collect();
    (!line.is_empty()).then_some(line)
}

fn in_range(value: i32, low: i32, high: i32, what: &str) -> Result<(), String> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(format!("{what} must be between {low} and {high}"))
    }
}

/// Who may change a sheet: its owner, or the GM.
pub fn may_edit(sheet: &Sheet, actor: &str, gm: Option<&str>) -> bool {
    sheet.owner == actor || gm == Some(actor)
}

/// Who sees all of a sheet rather than its party strip.
pub fn may_read(sheet: &Sheet, viewer: &str, gm: Option<&str>) -> bool {
    may_edit(sheet, viewer, gm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> Sheet {
        Sheet::new("sh-1".into(), "player".into(), "  Mira   Vale ")
    }

    #[test]
    fn modifiers_round_down() {
        assert_eq!(modifier(10), 0);
        assert_eq!(modifier(11), 0);
        assert_eq!(modifier(12), 1);
        assert_eq!(modifier(9), -1);
        assert_eq!(modifier(8), -1);
        assert_eq!(modifier(1), -5);
        assert_eq!(modifier(30), 10);
    }

    #[test]
    fn owner_and_gm_edit_others_do_not() {
        let sheet = sheet();
        assert!(may_edit(&sheet, "player", Some("gm")));
        assert!(may_edit(&sheet, "gm", Some("gm")));
        assert!(!may_edit(&sheet, "other", Some("gm")));
        assert!(!may_edit(&sheet, "gm", None));
        assert!(may_read(&sheet, "gm", Some("gm")));
        assert!(!may_read(&sheet, "other", Some("gm")));
    }

    #[test]
    fn applies_a_good_patch_and_names_conditions_the_srd_way() {
        let mut sheet = sheet();
        assert_eq!(sheet.name, "Mira Vale");
        sheet
            .apply(SheetPatch {
                hp: Some(7),
                hp_max: Some(24),
                conditions: Some(vec!["prone".into(), " Poisoned ".into(), "PRONE".into()]),
                abilities: Some([8, 16, 14, 10, 12, 13]),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(sheet.hp, 7);
        assert_eq!(sheet.conditions, vec!["Prone", "Poisoned"]);
        assert_eq!(sheet.initiative_bonus(), 3);
    }

    #[test]
    fn a_bad_field_changes_nothing() {
        let mut sheet = sheet();
        let before = sheet.clone();
        for patch in [
            SheetPatch {
                hp: Some(5),
                ac: Some(99),
                ..Default::default()
            },
            SheetPatch {
                name: Some("   ".into()),
                ..Default::default()
            },
            SheetPatch {
                conditions: Some(vec!["Sleepy".into()]),
                ..Default::default()
            },
            SheetPatch {
                abilities: Some([10, 10, 10, 10, 10, 31]),
                ..Default::default()
            },
            SheetPatch {
                exhaustion: Some(7),
                ..Default::default()
            },
            SheetPatch {
                hp_max: Some(0),
                ..Default::default()
            },
            SheetPatch {
                notes: Some("x".repeat(MAX_NOTES_CHARS + 1)),
                ..Default::default()
            },
            SheetPatch {
                slots: Some({
                    let mut slots = [Slots::default(); SLOT_LEVELS];
                    slots[0] = Slots { max: 2, used: 3 };
                    slots
                }),
                ..Default::default()
            },
        ] {
            assert!(sheet.apply(patch.clone()).is_err(), "{patch:?}");
            assert_eq!(sheet, before);
        }
    }
}
