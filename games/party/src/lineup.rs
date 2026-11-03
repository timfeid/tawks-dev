//! Lineup: a parade of dressed-up characters walks onto the stage and stands
//! in a row. The lights drop, and when they come back up something about one
//! or more of them is different: a hat, a coat, a pet, an accessory, or two of
//! them have traded places. Click who changed. Faster finds score more; a
//! wrong click locks you out for a moment. Later rounds bring more people,
//! subtler changes, two changes at once, and a second question: what changed.
//!
//! A pick arrives as a press of fire with the slot in `yaw`; the answer to
//! "what changed" is a press with `WHAT_BASE` plus the category.

use rand::{
    Rng, SeedableRng,
    rngs::StdRng,
    seq::{IndexedRandom, SliceRandom},
};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "lineup";
pub const SLOT_GAP: f32 = 1.4;
pub const LOCKOUT_SECONDS: f32 = 2.0;
pub const WALK_SECONDS: f32 = 2.6;
pub const DARK_SECONDS: f32 = 1.2;
pub const REVEAL_SECONDS: f32 = 2.4;
pub const FINALE_SECONDS: f32 = 3.5;
pub const FIND_POINTS: u32 = 100;
pub const SPEED_POINTS: f32 = 100.0;
pub const FIRST_POINTS: u32 = 25;
pub const WHAT_POINTS: u32 = 60;
pub const WHAT_BASE: f32 = 100.0;
/// The players watch from a gallery raised behind the stage, facing the
/// camera, so they never stand in front of the lineup.
pub const GALLERY_Y: f32 = 3.0;
pub const GALLERY_Z: f32 = -3.6;

pub const BODIES: [&str; 9] = [
    "sunny", "clover", "blip", "rosie", "bruno", "vex", "frost", "bolt", "onyx",
];
pub const HATS: [&str; 9] = [
    "none",
    "hat-cap",
    "hat-beanie",
    "hat-tophat",
    "hat-crown",
    "hat-wizard",
    "hat-visor",
    "hat-horns",
    "hat-ears",
];
pub const EXTRAS: [&str; 6] = [
    "none",
    "extra-glasses",
    "extra-bowtie",
    "extra-headphones",
    "extra-medal",
    "extra-scarf",
];
pub const PETS: [&str; 6] = [
    "none",
    "pet-ember",
    "pet-puddle",
    "pet-sprout",
    "pet-fizz",
    "pet-mochi",
];
/// Coat and hat colours, far enough apart to tell at a glance.
pub const PALETTE: [&str; 9] = [
    "#e63946", "#f4a261", "#ffd23f", "#2a9d8f", "#3a86ff", "#8338ec", "#ff70a6", "#f1f1f1",
    "#222831",
];
const COLOUR_NAMES: [&str; 9] = [
    "red", "orange", "yellow", "teal", "blue", "purple", "pink", "white", "black",
];

/// What changed, in the order the "what changed" options are shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hat,
    Coat,
    Pet,
    Extra,
    Places,
}

impl Kind {
    pub const ALL: [Kind; 5] = [Kind::Hat, Kind::Coat, Kind::Pet, Kind::Extra, Kind::Places];

    pub fn index(self) -> u8 {
        Kind::ALL.iter().position(|k| *k == self).unwrap_or(0) as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Hat => "hat",
            Kind::Coat => "coat",
            Kind::Pet => "pet",
            Kind::Extra => "accessory",
            Kind::Places => "places",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    pub id: u8,
    pub body: u8,
    pub hat: u8,
    pub hat_tint: u8,
    pub coat: u8,
    pub extra: u8,
    pub pet: u8,
}

impl Look {
    fn json(&self, slot: usize) -> Value {
        let named = |list: &[&'static str], index: u8| {
            (index != 0).then(|| list[index as usize % list.len()])
        };
        json!({
            "id": self.id,
            "slot": slot,
            "body": BODIES[self.body as usize % BODIES.len()],
            "hat": named(&HATS, self.hat),
            "hat_tint": PALETTE[self.hat_tint as usize % PALETTE.len()],
            "coat": PALETTE[self.coat as usize % PALETTE.len()],
            "extra": named(&EXTRAS, self.extra),
            "pet": named(&PETS, self.pet),
        })
    }

    /// Enough to tell two people apart from the back of the room.
    fn silhouette(&self) -> (u8, u8, u8) {
        (self.body, self.hat, self.coat)
    }
}

#[derive(Clone, Debug)]
pub struct Change {
    pub kind: Kind,
    /// Slots in the lineup after the lights come up. A swap has two; clicking
    /// either one finds it.
    pub slots: Vec<usize>,
    pub detail: String,
    /// Harder to spot: a hat recoloured rather than replaced, an accessory, a
    /// swap. Bots miss these more.
    pub subtle: bool,
}

/// How a round is pitched: more people, less time to look, more and subtler
/// changes, and later the second question.
#[derive(Clone, Copy, Debug)]
pub struct RoundPlan {
    pub cast: usize,
    pub stand: f32,
    pub answer: f32,
    pub changes: usize,
    pub ask_what: bool,
    pub subtle: bool,
}

pub const PLAN: [RoundPlan; 5] = [
    RoundPlan {
        cast: 8,
        stand: 2.8,
        answer: 6.0,
        changes: 1,
        ask_what: false,
        subtle: false,
    },
    RoundPlan {
        cast: 10,
        stand: 2.6,
        answer: 6.5,
        changes: 1,
        ask_what: false,
        subtle: false,
    },
    RoundPlan {
        cast: 12,
        stand: 2.6,
        answer: 8.0,
        changes: 2,
        ask_what: false,
        subtle: true,
    },
    RoundPlan {
        cast: 12,
        stand: 2.4,
        answer: 7.5,
        changes: 1,
        ask_what: true,
        subtle: true,
    },
    RoundPlan {
        cast: 12,
        stand: 2.4,
        answer: 9.0,
        changes: 2,
        ask_what: true,
        subtle: true,
    },
];

pub fn parade_seconds(plan: &RoundPlan) -> f32 {
    WALK_SECONDS + plan.stand
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Parade,
    Dark,
    Answer,
    Reveal,
    Finale,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Parade => "parade",
            Stage::Dark => "dark",
            Stage::Answer => "answer",
            Stage::Reveal => "reveal",
            Stage::Finale => "finale",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Act {
    Pick(usize),
    What(u8),
}

struct Player {
    seat: u8,
    skill: f32,
    points: u32,
    gained: u32,
    locked: f32,
    found: Vec<bool>,
    asked: Option<usize>,
    whats: Vec<Option<bool>>,
    finds: u32,
    misses: u32,
    plan: Vec<(f32, Act)>,
    planned: Option<usize>,
    cheer: f32,
}

impl Player {
    fn done(&self) -> bool {
        self.asked.is_none() && self.found.iter().all(|f| *f)
    }
}

pub struct Lineup {
    rng: StdRng,
    round: usize,
    stage: Stage,
    clock: f32,
    before: Vec<Look>,
    after: Vec<Look>,
    changes: Vec<Change>,
    finders: Vec<Vec<u8>>,
    players: Vec<Player>,
    next_id: u8,
    placed: Option<Vec<Vec<u8>>>,
}

fn decode(input: &Input) -> Option<Act> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let index = input.yaw.round();
    if (input.yaw - index).abs() >= 0.25 || index < 0.0 {
        return None;
    }
    if index >= WHAT_BASE {
        let what = index - WHAT_BASE;
        (what < Kind::ALL.len() as f32).then_some(Act::What(what as u8))
    } else {
        (index < 64.0).then_some(Act::Pick(index as usize))
    }
}

pub fn pick(slot: usize) -> Input {
    Input {
        fire: true,
        yaw: slot as f32,
        ..Default::default()
    }
}

pub fn answer_what(kind: Kind) -> Input {
    Input {
        fire: true,
        yaw: WHAT_BASE + kind.index() as f32,
        ..Default::default()
    }
}

impl Lineup {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut lineup = Lineup {
            rng: StdRng::seed_from_u64(seed ^ 0x11e0_u64),
            round: 0,
            stage: Stage::Parade,
            clock: 0.0,
            before: Vec::new(),
            after: Vec::new(),
            changes: Vec::new(),
            finders: Vec::new(),
            players: contestants
                .iter()
                .map(|one| Player {
                    seat: one.seat,
                    skill: if one.is_bot {
                        one.skill.clamp(0.0, 1.0)
                    } else {
                        1.0
                    },
                    points: 0,
                    gained: 0,
                    locked: 0.0,
                    found: Vec::new(),
                    asked: None,
                    whats: Vec::new(),
                    finds: 0,
                    misses: 0,
                    plan: Vec::new(),
                    planned: None,
                    cheer: 0.0,
                })
                .collect(),
            next_id: 0,
            placed: None,
        };
        lineup.deal();
        lineup
    }

    pub fn plan(&self) -> &RoundPlan {
        &PLAN[self.round.min(PLAN.len() - 1)]
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn round(&self) -> usize {
        self.round
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn before(&self) -> &[Look] {
        &self.before
    }

    pub fn after(&self) -> &[Look] {
        &self.after
    }

    pub fn points(&self, seat: u8) -> Option<u32> {
        self.players
            .iter()
            .find(|p| p.seat == seat)
            .map(|p| p.points)
    }

    pub fn locked(&self, seat: u8) -> bool {
        self.players
            .iter()
            .any(|p| p.seat == seat && p.locked > 0.0)
    }

    pub fn asking(&self, seat: u8) -> bool {
        self.players
            .iter()
            .any(|p| p.seat == seat && p.asked.is_some())
    }

    /// A slot nobody changed, for tests and for bots that guess.
    pub fn unchanged_slot(&self) -> Option<usize> {
        (0..self.after.len()).find(|slot| !self.changes.iter().any(|c| c.slots.contains(slot)))
    }

    fn stage_limit(&self) -> f32 {
        match self.stage {
            Stage::Parade => parade_seconds(self.plan()),
            Stage::Dark => DARK_SECONDS,
            Stage::Answer => self.plan().answer,
            Stage::Reveal => REVEAL_SECONDS,
            Stage::Finale => FINALE_SECONDS,
        }
    }

    fn fresh_look(&mut self) -> Look {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        Look {
            id,
            body: self.rng.random_range(0..BODIES.len() as u8),
            hat: if self.rng.random_bool(0.8) {
                self.rng.random_range(1..HATS.len() as u8)
            } else {
                0
            },
            hat_tint: self.rng.random_range(0..PALETTE.len() as u8),
            coat: self.rng.random_range(0..PALETTE.len() as u8),
            extra: if self.rng.random_bool(0.5) {
                self.rng.random_range(1..EXTRAS.len() as u8)
            } else {
                0
            },
            pet: if self.rng.random_bool(0.4) {
                self.rng.random_range(1..PETS.len() as u8)
            } else {
                0
            },
        }
    }

    /// A new cast for the round and the changes the dark will hide.
    fn deal(&mut self) {
        let plan = *self.plan();
        let mut cast: Vec<Look> = Vec::with_capacity(plan.cast);
        while cast.len() < plan.cast {
            let look = self.fresh_look();
            if cast
                .iter()
                .all(|other| other.silhouette() != look.silhouette())
            {
                cast.push(look);
            }
        }
        self.before = cast.clone();
        self.changes.clear();
        let mut touched = vec![false; cast.len()];
        let mut tries = 0;
        while self.changes.len() < plan.changes && tries < 200 {
            tries += 1;
            if let Some(change) = self.make_change(&mut cast, &mut touched, plan.subtle) {
                self.changes.push(change);
            }
        }
        self.after = cast;
        self.finders = vec![Vec::new(); self.changes.len()];
        for player in self.players.iter_mut() {
            player.found = vec![false; self.changes.len()];
            player.whats = vec![None; self.changes.len()];
            player.asked = None;
            player.locked = 0.0;
            player.gained = 0;
            player.plan.clear();
        }
    }

    fn make_change(
        &mut self,
        cast: &mut [Look],
        touched: &mut [bool],
        subtle: bool,
    ) -> Option<Change> {
        let kinds: &[Kind] = if subtle {
            &Kind::ALL
        } else {
            &[Kind::Hat, Kind::Coat, Kind::Pet]
        };
        let kind = *kinds.choose(&mut self.rng)?;
        let free: Vec<usize> = (0..cast.len()).filter(|s| !touched[*s]).collect();
        let slot = *free.choose(&mut self.rng)?;
        let look = cast[slot];
        let change = match kind {
            Kind::Hat => {
                let recolour = subtle && look.hat != 0 && self.rng.random_bool(0.4);
                if recolour {
                    let tint = self.other(look.hat_tint, PALETTE.len());
                    cast[slot].hat_tint = tint;
                    Change {
                        kind,
                        slots: vec![slot],
                        detail: format!(
                            "{} {} to {}",
                            hat_name(look.hat),
                            COLOUR_NAMES[look.hat_tint as usize],
                            COLOUR_NAMES[tint as usize]
                        ),
                        subtle: true,
                    }
                } else {
                    let hat = self.other(look.hat, HATS.len());
                    cast[slot].hat = hat;
                    Change {
                        kind,
                        slots: vec![slot],
                        detail: format!("{} to {}", hat_name(look.hat), hat_name(hat)),
                        subtle: false,
                    }
                }
            }
            Kind::Coat => {
                let coat = self.other(look.coat, PALETTE.len());
                cast[slot].coat = coat;
                Change {
                    kind,
                    slots: vec![slot],
                    detail: format!(
                        "{} coat to {}",
                        COLOUR_NAMES[look.coat as usize], COLOUR_NAMES[coat as usize]
                    ),
                    subtle: false,
                }
            }
            Kind::Pet => {
                let pet = if look.pet == 0 {
                    self.rng.random_range(1..PETS.len() as u8)
                } else if self.rng.random_bool(0.6) {
                    0
                } else {
                    let mut pet = self.other(look.pet, PETS.len());
                    if pet == 0 {
                        pet = (look.pet % (PETS.len() as u8 - 1)) + 1;
                    }
                    pet
                };
                cast[slot].pet = pet;
                Change {
                    kind,
                    slots: vec![slot],
                    detail: match (look.pet, pet) {
                        (0, _) => format!("a {} turned up", pet_name(pet)),
                        (_, 0) => format!("the {} went missing", pet_name(look.pet)),
                        _ => format!("{} to {}", pet_name(look.pet), pet_name(pet)),
                    },
                    subtle: false,
                }
            }
            Kind::Extra => {
                let extra = self.other(look.extra, EXTRAS.len());
                cast[slot].extra = extra;
                Change {
                    kind,
                    slots: vec![slot],
                    detail: match (look.extra, extra) {
                        (0, _) => format!("put on {}", extra_name(extra)),
                        (_, 0) => format!("lost the {}", extra_name(look.extra)),
                        _ => format!("{} to {}", extra_name(look.extra), extra_name(extra)),
                    },
                    subtle: true,
                }
            }
            Kind::Places => {
                let partners: Vec<usize> = free
                    .iter()
                    .copied()
                    .filter(|other| {
                        *other != slot
                            && other.abs_diff(slot) <= 3
                            && (cast[*other].body != look.body || cast[*other].coat != look.coat)
                    })
                    .collect();
                let other = *partners.choose(&mut self.rng)?;
                cast.swap(slot, other);
                touched[other] = true;
                Change {
                    kind,
                    slots: vec![slot.min(other), slot.max(other)],
                    detail: "two of them traded places".into(),
                    subtle: true,
                }
            }
        };
        touched[slot] = true;
        Some(change)
    }

    fn other(&mut self, current: u8, count: usize) -> u8 {
        let step = self.rng.random_range(1..count as u8);
        (current + step) % count as u8
    }

    fn change_at(&self, slot: usize) -> Option<usize> {
        self.changes.iter().position(|c| c.slots.contains(&slot))
    }

    fn act(&mut self, seat: u8, act: Act, events: &mut Vec<Value>) {
        let answer = self.plan().answer;
        let ask_what = self.plan().ask_what;
        let clock = self.clock;
        let cast = self.after.len();
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return;
        };
        match act {
            Act::Pick(slot) => {
                let player = &self.players[index];
                if player.locked > 0.0 || player.asked.is_some() || slot >= cast {
                    return;
                }
                match self.change_at(slot) {
                    Some(change) if self.players[index].found[change] => {}
                    Some(change) => {
                        let first = self.finders[change].is_empty();
                        let speed = (1.0 - clock / answer).clamp(0.0, 1.0);
                        let gain = FIND_POINTS
                            + (SPEED_POINTS * speed).round() as u32
                            + if first { FIRST_POINTS } else { 0 };
                        self.finders[change].push(seat);
                        let player = &mut self.players[index];
                        player.found[change] = true;
                        player.points += gain;
                        player.gained += gain;
                        player.finds += 1;
                        player.cheer = 0.6;
                        if ask_what {
                            player.asked = Some(change);
                        }
                        events.push(json!({
                            "kind": "found",
                            "seat": seat,
                            "points": gain,
                            "first": first,
                        }));
                    }
                    None => {
                        let player = &mut self.players[index];
                        player.locked = LOCKOUT_SECONDS;
                        player.misses += 1;
                        events.push(json!({ "kind": "miss", "seat": seat }));
                    }
                }
            }
            Act::What(what) => {
                let Some(change) = self.players[index].asked else {
                    return;
                };
                let right = self.changes[change].kind.index() == what;
                let player = &mut self.players[index];
                player.asked = None;
                player.whats[change] = Some(right);
                if right {
                    player.points += WHAT_POINTS;
                    player.gained += WHAT_POINTS;
                }
                events.push(json!({ "kind": "what", "seat": seat, "right": right }));
            }
        }
    }

    fn settle(&mut self) {
        let mut order: Vec<(u32, u8)> = self.players.iter().map(|p| (p.points, p.seat)).collect();
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (points, seat) in order {
            match groups.last_mut() {
                Some(group) if last == Some(points) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(points);
        }
        self.placed = Some(groups);
    }

    fn enter(&mut self, stage: Stage, events: &mut Vec<Value>) {
        self.stage = stage;
        self.clock = 0.0;
        match stage {
            Stage::Parade => events.push(json!({ "kind": "parade", "round": self.round })),
            Stage::Dark => events.push(json!({ "kind": "lights_out", "round": self.round })),
            Stage::Answer => events.push(json!({
                "kind": "lights_up",
                "round": self.round,
                "changes": self.changes.len(),
                "ask_what": self.plan().ask_what,
            })),
            Stage::Reveal => events.push(json!({ "kind": "reveal", "round": self.round })),
            Stage::Finale => events.push(json!({ "kind": "finale" })),
        }
    }

    fn make_plan(&mut self, index: usize) -> Vec<(f32, Act)> {
        let skill = self.players[index].skill;
        let plan = *self.plan();
        let round = self.round as f32;
        let mut order: Vec<usize> = (0..self.changes.len()).collect();
        order.shuffle(&mut self.rng);
        let mut acts = Vec::new();
        let mut t = 0.7 + (1.0 - skill) * 1.2 + self.rng.random_range(0.0..1.0);
        for change in order {
            let kind = self.changes[change].kind;
            let slots = self.changes[change].slots.clone();
            let difficulty = round * 0.04
                + if self.changes[change].subtle {
                    0.14
                } else {
                    0.0
                }
                + if kind == Kind::Coat { 0.06 } else { 0.0 };
            let remembers = (0.40 + 0.45 * skill - difficulty).clamp(0.08, 0.95);
            let right = *slots.choose(&mut self.rng).unwrap_or(&0);
            let mut found = false;
            if self.rng.random_bool(remembers as f64) {
                t += 0.6 + (1.0 - skill) * 1.6 + self.rng.random_range(0.0..1.6);
                acts.push((t, Act::Pick(right)));
                found = true;
            } else {
                if self.rng.random_bool((0.45 + 0.35 * (1.0 - skill)) as f64) {
                    t += 0.8 + (1.0 - skill) * 1.6 + self.rng.random_range(0.0..1.2);
                    if let Some(decoy) = self.decoy(right) {
                        acts.push((t, Act::Pick(decoy)));
                        t += LOCKOUT_SECONDS + 0.2;
                    }
                }
                if self.rng.random_bool((0.25 + 0.35 * skill) as f64) {
                    t += 0.8 + self.rng.random_range(0.0..2.0);
                    acts.push((t, Act::Pick(right)));
                    found = true;
                }
            }
            if found && plan.ask_what {
                t += 0.5 + (1.0 - skill) * 1.4 + self.rng.random_range(0.0..0.8);
                let knows = self.rng.random_bool((0.5 + 0.45 * skill) as f64);
                let what = if knows {
                    kind.index()
                } else {
                    (kind.index() + self.rng.random_range(1..Kind::ALL.len() as u8))
                        % Kind::ALL.len() as u8
                };
                acts.push((t, Act::What(what)));
            }
        }
        acts
    }

    /// A wrong slot a confused eye would land on: usually a neighbour.
    fn decoy(&mut self, near: usize) -> Option<usize> {
        let wrong: Vec<usize> = (0..self.after.len())
            .filter(|slot| self.change_at(*slot).is_none())
            .collect();
        let close: Vec<usize> = wrong
            .iter()
            .copied()
            .filter(|slot| slot.abs_diff(near) <= 2)
            .collect();
        if !close.is_empty() && self.rng.random_bool(0.7) {
            close.choose(&mut self.rng).copied()
        } else {
            wrong.choose(&mut self.rng).copied()
        }
    }

    fn shown(&self) -> &[Look] {
        let lit = match self.stage {
            Stage::Parade => false,
            Stage::Dark => self.clock >= DARK_SECONDS * 0.5,
            _ => true,
        };
        if lit { &self.after } else { &self.before }
    }

    fn standing_spot(&self, index: usize) -> (f32, f32) {
        let count = self.players.len().max(1) as f32;
        let gap = (14.0 / count).min(2.2);
        ((index as f32 - (count - 1.0) / 2.0) * gap, GALLERY_Z)
    }
}

fn hat_name(hat: u8) -> &'static str {
    match HATS[hat as usize % HATS.len()] {
        "hat-cap" => "cap",
        "hat-beanie" => "beanie",
        "hat-tophat" => "top hat",
        "hat-crown" => "crown",
        "hat-wizard" => "wizard hat",
        "hat-visor" => "visor",
        "hat-horns" => "horns",
        "hat-ears" => "bunny ears",
        _ => "no hat",
    }
}

fn pet_name(pet: u8) -> &'static str {
    match PETS[pet as usize % PETS.len()] {
        "pet-ember" => "ember",
        "pet-puddle" => "puddle",
        "pet-sprout" => "sprout",
        "pet-fizz" => "fizz",
        "pet-mochi" => "mochi",
        _ => "pet",
    }
}

fn extra_name(extra: u8) -> &'static str {
    match EXTRAS[extra as usize % EXTRAS.len()] {
        "extra-glasses" => "glasses",
        "extra-bowtie" => "bow tie",
        "extra-headphones" => "headphones",
        "extra-medal" => "medal",
        "extra-scarf" => "scarf",
        _ => "nothing",
    }
}

impl Bout for Lineup {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rounds": PLAN.len(),
            "gap": SLOT_GAP,
            "lockout": LOCKOUT_SECONDS,
            "walk": WALK_SECONDS,
            "dark": DARK_SECONDS,
            "gallery": [GALLERY_Y, GALLERY_Z],
            "kinds": Kind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>(),
            // The shared arena disc is not used; the client builds the stage.
            "radius": 0.01,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        for player in self.players.iter_mut() {
            player.locked = (player.locked - dt).max(0.0);
            player.cheer = (player.cheer - dt).max(0.0);
        }
        match self.stage {
            Stage::Parade => {
                if self.clock >= parade_seconds(self.plan()) {
                    self.enter(Stage::Dark, events);
                }
            }
            Stage::Dark => {
                if self.clock >= DARK_SECONDS {
                    self.enter(Stage::Answer, events);
                }
            }
            Stage::Answer => {
                for (seat, input) in inputs {
                    if let Some(act) = decode(input) {
                        self.act(*seat, act, events);
                    }
                }
                let everyone = self.players.iter().all(Player::done);
                if everyone || self.clock >= self.plan().answer {
                    for player in self.players.iter_mut() {
                        player.asked = None;
                    }
                    self.enter(Stage::Reveal, events);
                }
            }
            Stage::Reveal => {
                if self.clock >= REVEAL_SECONDS {
                    if self.round + 1 >= PLAN.len() {
                        self.enter(Stage::Finale, events);
                    } else {
                        self.round += 1;
                        self.deal();
                        self.enter(Stage::Parade, events);
                    }
                }
            }
            Stage::Finale => {
                if self.clock >= FINALE_SECONDS {
                    self.settle();
                }
            }
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() || self.stage != Stage::Answer {
            return Input::default();
        }
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.players[index].planned != Some(self.round) {
            let plan = self.make_plan(index);
            let player = &mut self.players[index];
            player.plan = plan;
            player.planned = Some(self.round);
        }
        let player = &mut self.players[index];
        if player.locked > 0.0 {
            return Input::default();
        }
        let Some(&(at, act)) = player.plan.first() else {
            return Input::default();
        };
        if self.clock < at {
            return Input::default();
        }
        player.plan.remove(0);
        match act {
            Act::Pick(slot) => pick(slot),
            Act::What(_) if player.asked.is_none() => Input::default(),
            Act::What(what) => answer_what(Kind::ALL[what as usize % Kind::ALL.len()]),
        }
    }

    fn snapshot(&self) -> Value {
        let revealing = matches!(self.stage, Stage::Reveal | Stage::Finale);
        let lit = matches!(self.stage, Stage::Answer | Stage::Reveal);
        json!({
            "game": KEY,
            "time": self.clock.min(self.stage_limit()),
            "limit": self.stage_limit(),
            "round": self.round,
            "rounds": PLAN.len(),
            "stage": self.stage.name(),
            "cast": self.after.len(),
            "changes": if lit { self.changes.len() } else { 0 },
            "ask_what": self.plan().ask_what,
            "lineup": self
                .shown()
                .iter()
                .enumerate()
                .map(|(slot, look)| look.json(slot))
                .collect::<Vec<_>>(),
            // Held back until the reveal so it is not on the wire while it
            // can still be picked.
            "reveal": if revealing && self.stage == Stage::Reveal {
                self.changes
                    .iter()
                    .zip(self.finders.iter())
                    .map(|(change, finders)| json!({
                        "kind": change.kind.name(),
                        "slots": change.slots,
                        "detail": change.detail,
                        "finders": finders,
                    }))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            },
            "players": self
                .players
                .iter()
                .enumerate()
                .map(|(index, player)| {
                    let (x, z) = self.standing_spot(index);
                    json!({
                        "seat": player.seat,
                        "x": x,
                        "z": z,
                        "y": GALLERY_Y + if player.cheer > 0.0 { (player.cheer / 0.6 * std::f32::consts::PI).sin() * 0.5 } else { 0.0 },
                        "yaw": 0.0,
                        "points": player.points,
                        "gained": player.gained,
                        "locked": player.locked,
                        "found": player.found.iter().filter(|f| **f).count(),
                        "asked": player.asked.is_some(),
                        "done": lit && player.done(),
                        "finds": player.finds,
                        "misses": player.misses,
                        "whats": if revealing { player.whats.iter().map(|w| *w).collect::<Vec<_>>() } else { Vec::new() },
                    })
                })
                .collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::{TICK_DT, TICK_HZ};

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn run_until(game: &mut Lineup, stage: Stage) {
        let mut events = Vec::new();
        for _ in 0..TICK_HZ * 60 {
            if game.stage == stage {
                return;
            }
            game.step(&[], TICK_DT, &mut events);
        }
        panic!("never reached {stage:?}");
    }

    fn tick(game: &mut Lineup, inputs: &[(u8, Input)]) -> Vec<Value> {
        let mut events = Vec::new();
        game.step(inputs, TICK_DT, &mut events);
        events
    }

    #[test]
    fn every_round_deals_distinct_people_and_the_planned_changes() {
        for seed in 0..200 {
            let mut game = Lineup::new(&people(2), seed);
            for round in 0..PLAN.len() {
                let plan = PLAN[round];
                assert_eq!(game.round, round);
                assert_eq!(game.before.len(), plan.cast);
                assert_eq!(
                    game.changes.len(),
                    plan.changes,
                    "seed {seed} round {round}"
                );
                let mut slots: Vec<usize> =
                    game.changes.iter().flat_map(|c| c.slots.clone()).collect();
                let count = slots.len();
                slots.sort();
                slots.dedup();
                assert_eq!(slots.len(), count, "a person changes at most once");
                for (slot, (was, now)) in game.before.iter().zip(game.after.iter()).enumerate() {
                    let changed = game.changes.iter().any(|c| c.slots.contains(&slot));
                    assert_eq!(was != now, changed, "seed {seed} round {round} slot {slot}");
                }
                for change in &game.changes {
                    if !plan.subtle {
                        assert!(matches!(change.kind, Kind::Hat | Kind::Coat | Kind::Pet));
                        assert!(!change.subtle);
                    }
                    if change.kind == Kind::Places {
                        let [a, b] = change.slots[..] else {
                            panic!("a swap has two")
                        };
                        assert_eq!(game.before[a], game.after[b]);
                        assert_eq!(game.before[b], game.after[a]);
                    }
                }
                run_until(&mut game, Stage::Reveal);
                run_until(
                    &mut game,
                    if round + 1 == PLAN.len() {
                        Stage::Finale
                    } else {
                        Stage::Parade
                    },
                );
            }
        }
    }

    #[test]
    fn the_changes_stay_off_the_wire_until_the_lights_drop_and_the_reveal() {
        let mut game = Lineup::new(&people(2), 7);
        let shown = |game: &Lineup| game.snapshot()["lineup"].clone();
        let before: Vec<Value> = game
            .before
            .iter()
            .enumerate()
            .map(|(s, l)| l.json(s))
            .collect();
        let after: Vec<Value> = game
            .after
            .iter()
            .enumerate()
            .map(|(s, l)| l.json(s))
            .collect();
        assert_eq!(shown(&game), json!(before));
        assert_eq!(game.snapshot()["changes"], 0);
        run_until(&mut game, Stage::Dark);
        assert_eq!(shown(&game), json!(before), "the swap happens in the dark");
        run_until(&mut game, Stage::Answer);
        assert_eq!(shown(&game), json!(after));
        assert_eq!(game.snapshot()["reveal"], json!([]));
        run_until(&mut game, Stage::Reveal);
        assert_eq!(game.snapshot()["reveal"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_find_scores_more_the_faster_it_comes_and_the_first_finder_gets_a_bonus() {
        let mut game = Lineup::new(&people(3), 3);
        run_until(&mut game, Stage::Answer);
        let right = game.changes[0].slots[0];
        let events = tick(&mut game, &[(0, pick(right))]);
        assert_eq!(events[0]["kind"], "found");
        assert_eq!(events[0]["first"], true);
        let quick = game.points(0).unwrap();
        assert!(quick > FIND_POINTS + FIRST_POINTS + 90, "{quick}");
        for _ in 0..TICK_HZ * 4 {
            tick(&mut game, &[]);
        }
        tick(&mut game, &[(1, pick(right))]);
        let slow = game.points(1).unwrap();
        assert!(
            slow < quick - FIRST_POINTS - 30,
            "slow {slow} quick {quick}"
        );
        assert!(slow > FIND_POINTS);
        tick(&mut game, &[(1, pick(right))]);
        assert_eq!(
            game.points(1).unwrap(),
            slow,
            "a second click on a find is ignored"
        );
    }

    #[test]
    fn a_wrong_click_locks_you_out_briefly_and_clicks_while_locked_do_nothing() {
        let mut game = Lineup::new(&people(2), 11);
        run_until(&mut game, Stage::Answer);
        let wrong = game.unchanged_slot().unwrap();
        let right = game.changes[0].slots[0];
        let events = tick(&mut game, &[(0, pick(wrong))]);
        assert_eq!(events[0]["kind"], "miss");
        assert!(game.locked(0));
        tick(&mut game, &[(0, pick(right))]);
        assert_eq!(game.points(0), Some(0), "locked out");
        for _ in 0..(LOCKOUT_SECONDS / TICK_DT) as usize + 2 {
            tick(&mut game, &[]);
        }
        assert!(!game.locked(0));
        tick(&mut game, &[(0, pick(right))]);
        assert!(game.points(0).unwrap() > 0);
    }

    #[test]
    fn picks_outside_the_answer_window_or_off_the_lineup_are_ignored() {
        let mut game = Lineup::new(&people(2), 5);
        let right = game.changes[0].slots[0];
        tick(&mut game, &[(0, pick(right))]);
        assert_eq!(game.points(0), Some(0), "no picks during the parade");
        run_until(&mut game, Stage::Answer);
        tick(
            &mut game,
            &[
                (0, pick(40)),
                (
                    1,
                    Input {
                        fire: true,
                        yaw: f32::NAN,
                        ..Default::default()
                    },
                ),
            ],
        );
        assert!(
            !game.locked(0) && !game.locked(1),
            "nonsense is not a wrong click"
        );
        tick(
            &mut game,
            &[(
                0,
                Input {
                    fire: true,
                    yaw: right as f32 + 0.4,
                    ..Default::default()
                },
            )],
        );
        assert!(!game.locked(0));
        assert_eq!(game.points(0), Some(0));
        tick(&mut game, &[(9, pick(right))]);
        assert_eq!(game.points(0), Some(0), "an unseated pick changes nothing");
    }

    #[test]
    fn what_rounds_ask_after_a_find_and_pay_only_the_right_answer() {
        let mut game = Lineup::new(&people(2), 21);
        while !game.plan().ask_what {
            run_until(&mut game, Stage::Reveal);
            run_until(&mut game, Stage::Parade);
        }
        run_until(&mut game, Stage::Answer);
        let change = game.changes[0].clone();
        tick(
            &mut game,
            &[(0, pick(change.slots[0])), (1, pick(change.slots[0]))],
        );
        assert!(game.asking(0) && game.asking(1));
        let found = game.points(0).unwrap();
        let other = Kind::ALL
            .iter()
            .copied()
            .find(|k| *k != change.kind)
            .unwrap();
        if let Some(second) = game.changes.get(1).map(|c| c.slots[0]) {
            tick(&mut game, &[(0, pick(second))]);
            assert_eq!(game.points(0).unwrap(), found, "answer what changed first");
        }
        tick(
            &mut game,
            &[(0, answer_what(change.kind)), (1, answer_what(other))],
        );
        assert_eq!(game.points(0).unwrap(), found + WHAT_POINTS);
        assert!(game.points(1).unwrap() <= found);
        assert!(!game.asking(0) && !game.asking(1));
        tick(&mut game, &[(1, answer_what(change.kind))]);
        assert!(game.points(1).unwrap() <= found, "one answer each");
    }

    #[test]
    fn the_round_ends_early_once_everyone_has_found_everything() {
        let mut game = Lineup::new(&people(2), 9);
        run_until(&mut game, Stage::Answer);
        let right = game.changes[0].slots[0];
        tick(&mut game, &[(0, pick(right))]);
        assert_eq!(game.stage, Stage::Answer);
        let events = tick(&mut game, &[(1, pick(right))]);
        assert!(events.iter().any(|e| e["kind"] == "reveal"));
        assert_eq!(game.stage, Stage::Reveal);
    }

    #[test]
    fn a_player_who_leaves_only_runs_the_clock_out_and_still_places() {
        let mut game = Lineup::new(&people(3), 13);
        let mut ticks = 0;
        while game.placements().is_none() {
            let mut inputs = Vec::new();
            if game.stage == Stage::Answer {
                let right = game.changes.iter().map(|c| c.slots[0]).collect::<Vec<_>>();
                for (seat, slot) in [(0u8, right[0]), (1, *right.last().unwrap())] {
                    inputs.push((seat, pick(slot)));
                }
                if game.asking(0) {
                    inputs[0].1 = answer_what(Kind::Hat);
                }
            }
            tick(&mut game, &inputs);
            ticks += 1;
            assert!(ticks < TICK_HZ * 120, "the bout ends");
        }
        let placed = game.placements().unwrap();
        assert_eq!(placed.last().unwrap(), &vec![2], "the leaver came last");
        assert_eq!(placed.iter().flatten().count(), 3);
    }

    #[test]
    fn equal_points_share_a_place() {
        let mut game = Lineup::new(&people(3), 1);
        for _ in 0..TICK_HZ * 200 {
            if game.placements().is_some() {
                break;
            }
            tick(&mut game, &[]);
        }
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }

    fn bots_play(skills: &[f32], seed: u64) -> (Lineup, f32) {
        let field: Vec<Contestant> = skills
            .iter()
            .enumerate()
            .map(|(seat, skill)| Contestant::bot(seat as u8, *skill))
            .collect();
        let mut game = Lineup::new(&field, seed);
        let mut seconds = 0.0;
        while game.placements().is_none() {
            let inputs: Vec<(u8, Input)> = field
                .iter()
                .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                .collect();
            tick(&mut game, &inputs);
            seconds += TICK_DT;
            assert!(seconds < 150.0, "bots finish");
        }
        (game, seconds)
    }

    #[test]
    fn bots_finish_inside_the_time_budget_and_sharper_bots_score_more() {
        let skills = [0.15, 0.4, 0.7, 1.0];
        let mut totals = [0u64; 4];
        let mut misses = [0u32; 4];
        let mut wins = [0u32; 4];
        let mut longest: f32 = 0.0;
        let mut shortest = f32::MAX;
        let mut spent = 0.0;
        let games = 300;
        for seed in 0..games {
            let (game, seconds) = bots_play(&skills, seed);
            longest = longest.max(seconds);
            shortest = shortest.min(seconds);
            spent += seconds;
            for (i, player) in game.players.iter().enumerate() {
                totals[i] += player.points as u64;
                misses[i] += player.misses;
            }
            for seat in &game.placements().unwrap()[0] {
                wins[*seat as usize] += 1;
            }
        }
        let averages: Vec<u64> = totals.iter().map(|t| t / games).collect();
        println!(
            "lineup sim: {games} bouts of four, {shortest:.0}-{longest:.0}s (mean {:.0}s), average points {averages:?}, misses {misses:?}, wins {wins:?}",
            spent / games as f32
        );
        assert!(
            longest < 90.0,
            "a bout runs about a minute and a half at most"
        );
        assert!(shortest > 45.0);
        assert!(averages.windows(2).all(|w| w[0] < w[1]), "{averages:?}");
        assert!(
            wins[1] > 0,
            "a steady bot takes a four-way now and then: {wins:?}"
        );
        assert!(
            wins[3] < games as u32 * 9 / 10,
            "the ruthless bot does not always win: {wins:?}"
        );
        assert!(
            misses[0] > misses[3] * 2,
            "clumsy bots click wrong more: {misses:?}"
        );
    }

    #[test]
    fn head_to_head_the_weaker_bot_still_takes_some_bouts() {
        for (weak, strong, floor) in [
            (0.15, 0.4, 0.15),
            (0.4, 0.7, 0.1),
            (0.7, 1.0, 0.1),
            (0.15, 1.0, 0.0),
        ] {
            let games = 200;
            let upsets = (0..games)
                .filter(|seed| {
                    let (game, _) = bots_play(&[weak, strong], *seed);
                    game.placements().unwrap()[0].contains(&0)
                })
                .count();
            let rate = upsets as f32 / games as f32;
            println!(
                "lineup head to head: {weak} beats {strong} in {:.0}%",
                rate * 100.0
            );
            assert!(rate >= floor && rate < 0.5, "{weak} vs {strong}: {rate}");
        }
    }

    #[test]
    fn eight_players_fit_the_audience_row() {
        let game = Lineup::new(&people(8), 2);
        let snapshot = game.snapshot();
        let xs: Vec<f64> = snapshot["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["x"].as_f64().unwrap())
            .collect();
        assert!(xs.windows(2).all(|w| w[1] - w[0] > 1.5));
        assert!(xs.iter().all(|x| x.abs() < 8.0));
    }
}
