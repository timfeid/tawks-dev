//! One lobby's table: who runs it, the sheets, the initiative order and the
//! roll log, with the rules for who may change what and who sees what.

use std::collections::{BTreeMap, VecDeque};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::dice::{self, Roll};
use crate::initiative::{Combatant, Tracker, TrackerView};
use crate::sheet::{self, PartyStrip, Sheet, SheetPatch};

pub const MAX_ROLLS_KEPT: usize = 200;
pub const ROLLS_SHOWN: usize = 60;
pub const MAX_SHEETS: usize = 24;
pub const MAX_SHEETS_EACH: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub avatar_url: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Public,
    /// The roller and the GM; when the GM rolls it, only the GM.
    Gm,
    /// Only the roller.
    Me,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollEntry {
    pub id: u64,
    pub at_ms: i64,
    pub by: String,
    pub by_name: String,
    pub visibility: Visibility,
    pub roll: Roll,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub gm: Option<String>,
    pub people: BTreeMap<String, Person>,
    pub sheets: Vec<Sheet>,
    pub initiative: Tracker,
    pub rolls: VecDeque<RollEntry>,
    pub next_roll: u64,
    pub next_sheet: u64,
}

pub struct Actor<'a> {
    pub user_id: &'a str,
    pub name: &'a str,
    /// Owns the lobby, and so may take the GM seat back.
    pub lobby_owner: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Roll {
        formula: String,
        #[serde(default)]
        visibility: Visibility,
    },
    SheetCreate {
        #[serde(default)]
        name: String,
    },
    SheetUpdate {
        id: String,
        patch: SheetPatch,
    },
    SheetDelete {
        id: String,
    },
    InitAddSheet {
        sheet_id: String,
        init: Option<i32>,
    },
    InitAddNpc {
        name: String,
        init: Option<i32>,
        #[serde(default)]
        bonus: i32,
        #[serde(default)]
        hidden: bool,
    },
    InitRemove {
        id: String,
    },
    InitSet {
        id: String,
        init: i32,
    },
    InitHide {
        id: String,
        hidden: bool,
    },
    InitSort,
    InitNext,
    InitPrev,
    InitClear,
    SetGm {
        user_id: String,
    },
    TakeGm,
}

/// What a change produced besides the new state, for the client to animate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub rolls: Vec<u64>,
}

#[derive(Serialize)]
#[serde(tag = "access", rename_all = "snake_case")]
pub enum SheetView<'a> {
    Full(&'a Sheet),
    Strip(PartyStrip<'a>),
}

#[derive(Serialize)]
#[serde(tag = "shown", rename_all = "snake_case")]
pub enum RollView<'a> {
    Open(&'a RollEntry),
    /// Someone rolled where this viewer may not look.
    Secret {
        id: u64,
        at_ms: i64,
        by: &'a str,
        by_name: &'a str,
        by_gm: bool,
    },
}

#[derive(Serialize)]
pub struct PersonView<'a> {
    pub user_id: &'a str,
    pub name: &'a str,
    pub avatar_url: &'a str,
    pub online: bool,
}

#[derive(Serialize)]
pub struct TableView<'a> {
    pub me: &'a str,
    pub gm: Option<&'a str>,
    pub people: Vec<PersonView<'a>>,
    pub sheets: Vec<SheetView<'a>>,
    pub initiative: TrackerView<'a>,
    pub rolls: Vec<RollView<'a>>,
}

/// Whose turn the lobby should show: the people to light up and a label.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turn {
    pub user_ids: Vec<String>,
    pub label: Option<String>,
}

impl Table {
    pub fn is_gm(&self, user_id: &str) -> bool {
        self.gm.as_deref() == Some(user_id)
    }

    /// Seats someone. The first person at an empty table runs it.
    pub fn arrive(&mut self, user_id: &str, name: &str, avatar_url: &str) {
        self.people.insert(
            user_id.to_string(),
            Person {
                name: name.to_string(),
                avatar_url: avatar_url.to_string(),
            },
        );
        if self.gm.is_none() {
            self.gm = Some(user_id.to_string());
        }
    }

    pub fn apply(&mut self, actor: &Actor, action: Action, rng: &mut impl Rng, now_ms: i64) -> Result<Outcome, String> {
        let mut outcome = Outcome::default();
        match action {
            Action::Roll { formula, visibility } => {
                let roll = dice::roll(&formula, rng).map_err(|e| e.to_string())?;
                outcome
                    .rolls
                    .push(self.log(actor.user_id, actor.name, visibility, roll, now_ms));
            }
            Action::SheetCreate { name } => {
                if self.sheets.len() >= MAX_SHEETS {
                    return Err(format!("A table holds up to {MAX_SHEETS} sheets"));
                }
                if self.sheets.iter().filter(|s| s.owner == actor.user_id).count() >= MAX_SHEETS_EACH {
                    return Err(format!("You can keep up to {MAX_SHEETS_EACH} sheets here"));
                }
                self.next_sheet += 1;
                let name = if name.trim().is_empty() {
                    actor.name
                } else {
                    name.as_str()
                };
                let sheet = Sheet::new(format!("s{}", self.next_sheet), actor.user_id.to_string(), name);
                self.sheets.push(sheet);
            }
            Action::SheetUpdate { id, patch } => {
                let gm = self.gm.clone();
                let sheet = self.sheet_mut(&id)?;
                if !sheet::may_edit(sheet, actor.user_id, gm.as_deref()) {
                    return Err("Only its player or the GM can change that sheet".into());
                }
                sheet.apply(patch)?;
                let (name, bonus) = (sheet.name.clone(), sheet.initiative_bonus());
                for combatant in self
                    .initiative
                    .entries
                    .iter_mut()
                    .filter(|c| c.sheet_id.as_deref() == Some(&id))
                {
                    combatant.name = name.clone();
                    combatant.bonus = bonus;
                }
            }
            Action::SheetDelete { id } => {
                let gm = self.gm.clone();
                let sheet = self.sheet_mut(&id)?;
                if !sheet::may_edit(sheet, actor.user_id, gm.as_deref()) {
                    return Err("Only its player or the GM can remove that sheet".into());
                }
                self.sheets.retain(|s| s.id != id);
                let gone: Vec<String> = self
                    .initiative
                    .entries
                    .iter()
                    .filter(|c| c.sheet_id.as_deref() == Some(&id))
                    .map(|c| c.id.clone())
                    .collect();
                for combatant in gone {
                    self.initiative.remove(&combatant);
                }
            }
            Action::SetGm { user_id } => {
                self.require_gm(actor)?;
                if !self.people.contains_key(&user_id) {
                    return Err("Hand the GM seat to someone at the table".into());
                }
                self.gm = Some(user_id);
            }
            Action::TakeGm => {
                if !actor.lobby_owner && self.gm.is_some() {
                    return Err("Only the lobby's owner can take the GM seat".into());
                }
                self.gm = Some(actor.user_id.to_string());
            }
            action => {
                self.require_gm(actor)?;
                self.initiative_action(actor, action, rng, now_ms, &mut outcome)?;
            }
        }
        Ok(outcome)
    }

    fn initiative_action(
        &mut self,
        actor: &Actor,
        action: Action,
        rng: &mut impl Rng,
        now_ms: i64,
        outcome: &mut Outcome,
    ) -> Result<(), String> {
        let missing = || "That combatant is gone".to_string();
        match action {
            Action::InitAddSheet { sheet_id, init } => {
                let sheet = self
                    .sheets
                    .iter()
                    .find(|s| s.id == sheet_id)
                    .ok_or("That sheet is gone")?;
                if self
                    .initiative
                    .entries
                    .iter()
                    .any(|c| c.sheet_id.as_deref() == Some(&sheet_id))
                {
                    return Err(format!("{} is already in the order", sheet.name));
                }
                let (name, owner, bonus) = (sheet.name.clone(), sheet.owner.clone(), sheet.initiative_bonus());
                let owner_name = self
                    .people
                    .get(&owner)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| name.clone());
                let init = match init {
                    Some(init) => init,
                    None => {
                        let roll = self.roll_initiative(bonus, &name, rng)?;
                        let total = roll.total as i32;
                        outcome
                            .rolls
                            .push(self.log(&owner, &owner_name, Visibility::Public, roll, now_ms));
                        total
                    }
                };
                let owner = (!self.is_gm(&owner)).then_some(owner);
                self.initiative.add(Combatant {
                    id: String::new(),
                    name,
                    init,
                    bonus,
                    sheet_id: Some(sheet_id),
                    owner,
                    hidden: false,
                })?;
            }
            Action::InitAddNpc {
                name,
                init,
                bonus,
                hidden,
            } => {
                let init = match init {
                    Some(init) => init,
                    None => {
                        let roll = self.roll_initiative(bonus, &name, rng)?;
                        let total = roll.total as i32;
                        let shown = if hidden { Visibility::Gm } else { Visibility::Public };
                        outcome
                            .rolls
                            .push(self.log(actor.user_id, actor.name, shown, roll, now_ms));
                        total
                    }
                };
                self.initiative.add(Combatant {
                    id: String::new(),
                    name,
                    init,
                    bonus,
                    sheet_id: None,
                    owner: None,
                    hidden,
                })?;
            }
            Action::InitRemove { id } => {
                if !self.initiative.remove(&id) {
                    return Err(missing());
                }
            }
            Action::InitSet { id, init } => {
                if !self.initiative.set_init(&id, init) {
                    return Err(missing());
                }
            }
            Action::InitHide { id, hidden } => {
                if !self.initiative.set_hidden(&id, hidden) {
                    return Err(missing());
                }
            }
            Action::InitSort => self.initiative.sort(),
            Action::InitNext => self.initiative.next(),
            Action::InitPrev => self.initiative.prev(),
            Action::InitClear => self.initiative.clear(),
            _ => unreachable!("handled in apply"),
        }
        Ok(())
    }

    fn roll_initiative(&self, bonus: i32, name: &str, rng: &mut impl Rng) -> Result<Roll, String> {
        let formula = match bonus {
            0 => format!("1d20 initiative {name}"),
            b if b > 0 => format!("1d20+{b} initiative {name}"),
            b => format!("1d20{b} initiative {name}"),
        };
        dice::roll(&formula, rng).map_err(|e| e.to_string())
    }

    fn require_gm(&self, actor: &Actor) -> Result<(), String> {
        if self.is_gm(actor.user_id) {
            Ok(())
        } else {
            Err("Only the GM can do that".into())
        }
    }

    fn sheet_mut(&mut self, id: &str) -> Result<&mut Sheet, String> {
        self.sheets
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| "That sheet is gone".into())
    }

    fn log(&mut self, by: &str, by_name: &str, visibility: Visibility, roll: Roll, now_ms: i64) -> u64 {
        self.next_roll += 1;
        let id = self.next_roll;
        self.rolls.push_back(RollEntry {
            id,
            at_ms: now_ms,
            by: by.to_string(),
            by_name: by_name.to_string(),
            visibility,
            roll,
        });
        while self.rolls.len() > MAX_ROLLS_KEPT {
            self.rolls.pop_front();
        }
        id
    }

    pub fn roll_for<'a>(&'a self, entry: &'a RollEntry, viewer: &str) -> Option<RollView<'a>> {
        let open = match entry.visibility {
            Visibility::Public => true,
            Visibility::Gm => entry.by == viewer || self.is_gm(viewer),
            Visibility::Me => entry.by == viewer,
        };
        if open {
            return Some(RollView::Open(entry));
        }
        (entry.visibility == Visibility::Gm).then(|| RollView::Secret {
            id: entry.id,
            at_ms: entry.at_ms,
            by: &entry.by,
            by_name: &entry.by_name,
            by_gm: self.is_gm(&entry.by),
        })
    }

    pub fn view_for<'a>(&'a self, viewer: &'a str, online: &dyn Fn(&str) -> bool) -> TableView<'a> {
        let gm = self.is_gm(viewer);
        let skip = self.rolls.len().saturating_sub(ROLLS_SHOWN);
        TableView {
            me: viewer,
            gm: self.gm.as_deref(),
            people: self
                .people
                .iter()
                .map(|(id, person)| PersonView {
                    user_id: id,
                    name: &person.name,
                    avatar_url: &person.avatar_url,
                    online: online(id),
                })
                .collect(),
            sheets: self
                .sheets
                .iter()
                .map(|s| {
                    if sheet::may_read(s, viewer, self.gm.as_deref()) {
                        SheetView::Full(s)
                    } else {
                        SheetView::Strip(s.strip())
                    }
                })
                .collect(),
            initiative: self.initiative.view(gm),
            rolls: self
                .rolls
                .iter()
                .skip(skip)
                .filter_map(|r| self.roll_for(r, viewer))
                .collect(),
        }
    }

    pub fn turn(&self) -> Turn {
        let Some(current) = self.initiative.current() else {
            return Turn::default();
        };
        let who = current.owner.clone().or_else(|| self.gm.clone());
        let label = if current.hidden {
            "GM".to_string()
        } else {
            current.name.clone()
        };
        Turn {
            user_ids: who.into_iter().collect(),
            label: Some(label),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn actor(user_id: &str) -> Actor<'_> {
        Actor {
            user_id,
            name: user_id,
            lobby_owner: false,
        }
    }

    fn table() -> (Table, StdRng) {
        let mut table = Table::default();
        table.arrive("gm", "Gem", "");
        table.arrive("pat", "Pat", "");
        table.arrive("rae", "Rae", "");
        (table, StdRng::seed_from_u64(7))
    }

    fn act(table: &mut Table, rng: &mut StdRng, who: &str, action: Action) -> Result<Outcome, String> {
        table.apply(&actor(who), action, rng, 1000)
    }

    fn json(view: &TableView) -> serde_json::Value {
        serde_json::to_value(view).unwrap()
    }

    #[test]
    fn the_first_to_arrive_runs_the_table_and_can_hand_it_on() {
        let (mut t, mut rng) = table();
        assert_eq!(t.gm.as_deref(), Some("gm"));
        assert!(act(&mut t, &mut rng, "pat", Action::SetGm { user_id: "pat".into() }).is_err());
        assert!(
            act(
                &mut t,
                &mut rng,
                "gm",
                Action::SetGm {
                    user_id: "stranger".into()
                }
            )
            .is_err()
        );
        act(&mut t, &mut rng, "gm", Action::SetGm { user_id: "pat".into() }).unwrap();
        assert_eq!(t.gm.as_deref(), Some("pat"));
        assert!(act(&mut t, &mut rng, "rae", Action::TakeGm).is_err());
        t.apply(
            &Actor {
                user_id: "rae",
                name: "Rae",
                lobby_owner: true,
            },
            Action::TakeGm,
            &mut rng,
            0,
        )
        .unwrap();
        assert_eq!(t.gm.as_deref(), Some("rae"));
    }

    #[test]
    fn hidden_gm_rolls_reach_only_the_gm() {
        let (mut t, mut rng) = table();
        act(
            &mut t,
            &mut rng,
            "gm",
            Action::Roll {
                formula: "1d20+4".into(),
                visibility: Visibility::Gm,
            },
        )
        .unwrap();
        act(
            &mut t,
            &mut rng,
            "pat",
            Action::Roll {
                formula: "1d6".into(),
                visibility: Visibility::Gm,
            },
        )
        .unwrap();
        act(
            &mut t,
            &mut rng,
            "rae",
            Action::Roll {
                formula: "2d6".into(),
                visibility: Visibility::Me,
            },
        )
        .unwrap();
        act(
            &mut t,
            &mut rng,
            "rae",
            Action::Roll {
                formula: "1d8".into(),
                visibility: Visibility::Public,
            },
        )
        .unwrap();
        let online = |_: &str| true;
        let shown = |viewer: &str| -> Vec<String> {
            json(&t.view_for(viewer, &online))["rolls"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| format!("{}:{}", r["shown"].as_str().unwrap(), r["by"].as_str().unwrap()))
                .collect()
        };
        assert_eq!(shown("gm"), ["open:gm", "open:pat", "open:rae"]);
        assert_eq!(shown("pat"), ["secret:gm", "open:pat", "open:rae"]);
        assert_eq!(shown("rae"), ["secret:gm", "secret:pat", "open:rae", "open:rae"]);
        let pat = json(&t.view_for("pat", &online));
        let secret = &pat["rolls"][0];
        assert_eq!(secret["by_gm"], true);
        assert!(secret.get("roll").is_none(), "a secret roll carries no dice: {secret}");
        assert!(!pat.to_string().contains("1d20+4"));
    }

    #[test]
    fn players_edit_their_own_sheet_and_the_gm_edits_any() {
        let (mut t, mut rng) = table();
        act(&mut t, &mut rng, "pat", Action::SheetCreate { name: "Mira".into() }).unwrap();
        let id = t.sheets[0].id.clone();
        let hurt = |hp| Action::SheetUpdate {
            id: id.clone(),
            patch: SheetPatch {
                hp: Some(hp),
                ..Default::default()
            },
        };
        act(&mut t, &mut rng, "pat", hurt(6)).unwrap();
        assert!(act(&mut t, &mut rng, "rae", hurt(1)).is_err());
        act(&mut t, &mut rng, "gm", hurt(3)).unwrap();
        assert_eq!(t.sheets[0].hp, 3);
        assert!(act(&mut t, &mut rng, "rae", Action::SheetDelete { id: id.clone() }).is_err());
        let online = |_: &str| true;
        assert_eq!(json(&t.view_for("pat", &online))["sheets"][0]["access"], "full");
        assert_eq!(json(&t.view_for("gm", &online))["sheets"][0]["access"], "full");
        let rae = json(&t.view_for("rae", &online));
        assert_eq!(rae["sheets"][0]["access"], "strip");
        assert!(rae["sheets"][0].get("notes").is_none() && rae["sheets"][0].get("abilities").is_none());
        act(&mut t, &mut rng, "pat", Action::SheetDelete { id }).unwrap();
        assert!(t.sheets.is_empty());
    }

    #[test]
    fn only_the_gm_runs_initiative_and_the_turn_follows_it() {
        let (mut t, mut rng) = table();
        act(&mut t, &mut rng, "pat", Action::SheetCreate { name: "Mira".into() }).unwrap();
        let sheet = t.sheets[0].id.clone();
        assert!(
            act(
                &mut t,
                &mut rng,
                "pat",
                Action::InitAddSheet {
                    sheet_id: sheet.clone(),
                    init: Some(15)
                }
            )
            .is_err()
        );
        act(
            &mut t,
            &mut rng,
            "gm",
            Action::InitAddSheet {
                sheet_id: sheet.clone(),
                init: Some(15),
            },
        )
        .unwrap();
        assert!(
            act(
                &mut t,
                &mut rng,
                "gm",
                Action::InitAddSheet {
                    sheet_id: sheet.clone(),
                    init: Some(3)
                }
            )
            .is_err()
        );
        let out = act(
            &mut t,
            &mut rng,
            "gm",
            Action::InitAddNpc {
                name: "Wolf".into(),
                init: None,
                bonus: 2,
                hidden: true,
            },
        )
        .unwrap();
        assert_eq!(out.rolls.len(), 1);
        let wolf_roll = t.rolls.back().unwrap();
        assert_eq!(wolf_roll.visibility, Visibility::Gm);
        let wolf = t.initiative.entries[1].id.clone();
        act(&mut t, &mut rng, "gm", Action::InitSet { id: wolf, init: 20 }).unwrap();
        act(&mut t, &mut rng, "gm", Action::InitSort).unwrap();
        assert!(act(&mut t, &mut rng, "pat", Action::InitNext).is_err());
        act(&mut t, &mut rng, "gm", Action::InitNext).unwrap();
        assert_eq!(
            t.turn(),
            Turn {
                user_ids: vec!["gm".into()],
                label: Some("GM".into())
            }
        );
        act(&mut t, &mut rng, "gm", Action::InitNext).unwrap();
        assert_eq!(
            t.turn(),
            Turn {
                user_ids: vec!["pat".into()],
                label: Some("Mira".into())
            }
        );
        act(&mut t, &mut rng, "gm", Action::InitNext).unwrap();
        assert_eq!(t.initiative.round, 2);
        act(
            &mut t,
            &mut rng,
            "pat",
            Action::SheetUpdate {
                id: sheet.clone(),
                patch: SheetPatch {
                    name: Some("Mira the Bold".into()),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        assert!(t.initiative.entries.iter().any(|c| c.name == "Mira the Bold"));
        act(&mut t, &mut rng, "gm", Action::InitClear).unwrap();
        assert_eq!(t.turn(), Turn::default());
    }

    #[test]
    fn a_table_survives_a_round_trip_through_json() {
        let (mut t, mut rng) = table();
        act(&mut t, &mut rng, "pat", Action::SheetCreate { name: String::new() }).unwrap();
        act(
            &mut t,
            &mut rng,
            "gm",
            Action::Roll {
                formula: "4d6dl1".into(),
                visibility: Visibility::Public,
            },
        )
        .unwrap();
        let back: Table = serde_json::from_value(serde_json::to_value(&t).unwrap()).unwrap();
        assert_eq!(back, t);
        assert_eq!(back.sheets[0].name, "pat");
    }

    #[test]
    fn actions_read_from_client_json() {
        let action: Action = serde_json::from_str(r#"{"type":"roll","formula":"1d20 adv"}"#).unwrap();
        assert_eq!(
            action,
            Action::Roll {
                formula: "1d20 adv".into(),
                visibility: Visibility::Public
            }
        );
        let action: Action = serde_json::from_str(r#"{"type":"sheet_update","id":"s1","patch":{"hp":4}}"#).unwrap();
        assert!(matches!(action, Action::SheetUpdate { .. }));
        assert!(serde_json::from_str::<Action>(r#"{"type":"init_next"}"#).is_ok());
    }
}
