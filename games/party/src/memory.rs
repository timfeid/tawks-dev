//! Memory Tiles: a three by three grid lights a sequence, then everyone walks
//! it back in order, stomping each tile. One wrong stomp, or running out of
//! time, and you are out. The sequence grows by one tile each round until one
//! player is left.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Skill, Stick};

pub const KEY: &str = "memory";
pub const SIDE: usize = 3;
pub const TILES: usize = SIDE * SIDE;
/// Centre to centre distance between neighbouring tiles.
pub const SPACING: f32 = 5.0;
/// Half the width of a tile; the rest of the spacing is a gap that counts as
/// no tile at all, so a stomp between two tiles is forgiven.
pub const TILE_HALF: f32 = 2.0;
pub const BOARD_HALF: f32 = 9.5;
pub const FIRST_LENGTH: usize = 3;
pub const LIGHT_ON: f32 = 0.4;
pub const LIGHT_GAP: f32 = 0.12;
pub const LEAD_IN: f32 = 0.8;
pub const BREAK: f32 = 1.2;
/// A last-resort cap so a table of perfect players still ends; they share
/// first place.
pub const MAX_ROUNDS: u32 = 7;
const WALK: f32 = 11.0;

/// The time allowed to walk a sequence back. Generous on purpose: this bout
/// tests memory, not speed.
pub fn recall_seconds(length: usize) -> f32 {
    4.0 + length as f32 * 1.5
}

pub fn show_seconds(length: usize) -> f32 {
    LEAD_IN + length as f32 * (LIGHT_ON + LIGHT_GAP)
}

pub fn tile_centre(tile: usize) -> (f32, f32) {
    let col = (tile % SIDE) as f32 - 1.0;
    let row = (tile / SIDE) as f32 - 1.0;
    (col * SPACING, row * SPACING)
}

pub fn tile_at(x: f32, z: f32) -> Option<usize> {
    tile_at_for_build(x, z, 1.0)
}

/// A wider stance forgives a stomp a little farther from a tile's edge.
/// Even the widest stance leaves a gap between adjacent tiles.
fn tile_at_for_build(x: f32, z: f32, width: f32) -> Option<usize> {
    let half = TILE_HALF + (BodyProfile::new(width).radius - 1.0) * 1.5;
    let index = |v: f32| {
        let slot = (v / SPACING).round();
        if slot.abs() > 1.0 || (v - slot * SPACING).abs() > half {
            None
        } else {
            Some((slot + 1.0) as usize)
        }
    };
    Some(index(z)? * SIDE + index(x)?)
}

/// How likely a bot is to misremember the step at `index`. Everyone slips now
/// and then; past what a bot can hold in its head it slips a lot.
fn slip(skill: f32, index: usize) -> f64 {
    let level = Skill(skill).level();
    let span = 3.0 + level.powi(3) * 7.0;
    let base = 0.01 + (1.0 - level) as f64 * 0.04;
    if index as f32 >= span {
        base + 0.35
    } else {
        base
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Show,
    Recall,
    Break,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Show => "show",
            Phase::Recall => "recall",
            Phase::Break => "break",
        }
    }
}

#[derive(Clone)]
struct Walker {
    seat: u8,
    x: f32,
    z: f32,
    width: f32,
    skill: f32,
    /// Tiles walked correctly this round.
    progress: usize,
    done: bool,
    /// The round they went out in and how far into it they got.
    out: Option<(u32, usize)>,
    /// A bot's chosen tile for the step it is on.
    plan: Option<(usize, usize)>,
    /// How long a bot pauses before setting off for its next tile.
    wait: f32,
}

impl Walker {
    fn playing(&self) -> bool {
        self.out.is_none() && !self.done
    }
}

pub struct MemoryTiles {
    walkers: Vec<Walker>,
    sequence: Vec<usize>,
    round: u32,
    phase: Phase,
    /// Seconds into the current phase.
    clock: f32,
    /// How many tiles of the sequence have lit so far this showing.
    shown: usize,
    /// How much bigger than the board for four, for the size of the field.
    /// The grid stays three by three; its tiles and gaps grow.
    size: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

impl MemoryTiles {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut sequence = Vec::new();
        for _ in 0..FIRST_LENGTH {
            sequence.push(next_tile(&mut rng, sequence.last().copied()));
        }
        // The corners first, then the middle of each edge for a bigger field.
        let corners = [
            (-1.0, -1.0),
            (1.0, 1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
        ];
        let size = crate::bout::crowd_scale(contestants.len());
        let start = BOARD_HALF * size - 1.0;
        MemoryTiles {
            walkers: contestants
                .iter()
                .enumerate()
                .map(|(index, one)| {
                    let (cx, cz) = corners[index % corners.len()];
                    Walker {
                        seat: one.seat,
                        x: cx * start,
                        z: cz * start,
                        width: BodyProfile::new(one.body_width).width,
                        skill: one.skill,
                        progress: 0,
                        done: false,
                        out: None,
                        plan: None,
                        wait: 0.0,
                    }
                })
                .collect(),
            sequence,
            round: 1,
            phase: Phase::Show,
            clock: 0.0,
            shown: 0,
            size,
            rng,
            placed: None,
        }
    }

    fn half(&self) -> f32 {
        BOARD_HALF * self.size
    }

    /// Where a tile's middle is on this board.
    fn centre(&self, tile: usize) -> (f32, f32) {
        let (x, z) = tile_centre(tile);
        (x * self.size, z * self.size)
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn sequence(&self) -> &[usize] {
        &self.sequence
    }

    /// The tile lit right now, while the sequence is being shown.
    pub fn lit(&self) -> Option<usize> {
        if self.phase != Phase::Show || self.clock < LEAD_IN {
            return None;
        }
        let into = self.clock - LEAD_IN;
        let slot = (into / (LIGHT_ON + LIGHT_GAP)) as usize;
        let within = into - slot as f32 * (LIGHT_ON + LIGHT_GAP);
        if within < LIGHT_ON {
            self.sequence.get(slot).copied()
        } else {
            None
        }
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.clock = 0.0;
    }

    fn end_recall(&mut self, events: &mut Vec<Value>) {
        let round = self.round;
        for walker in self.walkers.iter_mut().filter(|w| w.playing()) {
            walker.out = Some((round, walker.progress));
            events.push(json!({ "kind": "timeout", "seat": walker.seat }));
        }
        let left = self.walkers.iter().filter(|w| w.out.is_none()).count();
        if left <= 1 || self.round >= MAX_ROUNDS {
            self.settle();
        } else {
            events.push(json!({ "kind": "survived", "round": round, "left": left }));
            self.enter(Phase::Break);
        }
    }

    fn next_round(&mut self) {
        self.round += 1;
        let last = self.sequence.last().copied();
        let tile = next_tile(&mut self.rng, last);
        self.sequence.push(tile);
        self.shown = 0;
        for walker in self.walkers.iter_mut() {
            walker.progress = 0;
            walker.done = false;
            walker.plan = None;
        }
        self.enter(Phase::Show);
    }

    /// Survivors first, then whoever went out latest, then whoever got
    /// furthest into the round they went out in.
    fn settle(&mut self) {
        let length = self.sequence.len();
        let mut order: Vec<((u32, usize), u8)> = self
            .walkers
            .iter()
            .map(|w| (w.out.unwrap_or((u32::MAX, length)), w.seat))
            .collect();
        order.sort_by(|a, b| b.0.cmp(&a.0));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (key, seat) in order {
            match groups.last_mut() {
                Some(group) if last == Some(key) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(key);
        }
        self.placed = Some(groups);
    }
}

fn next_tile(rng: &mut StdRng, last: Option<usize>) -> usize {
    loop {
        let tile = rng.random_range(0..TILES);
        if Some(tile) != last {
            return tile;
        }
    }
}

impl Bout for MemoryTiles {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "side": SIDE,
            "spacing": SPACING * self.size,
            "tile": TILE_HALF * self.size,
            "half": self.half(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        let half = self.half();
        let size = self.size;

        for walker in self.walkers.iter_mut().filter(|w| w.out.is_none()) {
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == walker.seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let stick = input.stick.clamped();
            let pace = WALK * BodyProfile::new(walker.width).speed;
            walker.x = (walker.x + stick.x * pace * dt).clamp(-half, half);
            walker.z = (walker.z + stick.z * pace * dt).clamp(-half, half);

            if self.phase != Phase::Recall || walker.done || !input.fire {
                continue;
            }
            // The grid is laid out for four, so measure against that.
            let Some(tile) = tile_at_for_build(walker.x / size, walker.z / size, walker.width) else {
                continue;
            };
            let wanted = self.sequence[walker.progress];
            if tile == wanted {
                walker.progress += 1;
                events.push(json!({
                    "kind": "stomp",
                    "seat": walker.seat,
                    "tile": tile,
                    "step": walker.progress,
                }));
                if walker.progress == self.sequence.len() {
                    walker.done = true;
                    events.push(json!({ "kind": "done", "seat": walker.seat }));
                }
            } else {
                walker.out = Some((self.round, walker.progress));
                events.push(json!({
                    "kind": "wrong",
                    "seat": walker.seat,
                    "tile": tile,
                    "wanted": wanted,
                }));
            }
        }

        match self.phase {
            Phase::Show => {
                while self.shown < self.sequence.len()
                    && self.clock >= LEAD_IN + self.shown as f32 * (LIGHT_ON + LIGHT_GAP)
                {
                    events.push(json!({
                        "kind": "light",
                        "tile": self.sequence[self.shown],
                        "step": self.shown,
                    }));
                    self.shown += 1;
                }
                if self.clock >= show_seconds(self.sequence.len()) {
                    self.enter(Phase::Recall);
                    events.push(json!({ "kind": "go", "round": self.round }));
                }
            }
            Phase::Recall => {
                let waiting = self.walkers.iter().any(|w| w.playing());
                if !waiting || self.clock >= recall_seconds(self.sequence.len()) {
                    self.end_recall(events);
                }
            }
            Phase::Break => {
                if self.clock >= BREAK {
                    self.next_round();
                }
            }
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(index) = self.walkers.iter().position(|w| w.seat == seat) else {
            return Input::default();
        };
        if self.phase != Phase::Recall || !self.walkers[index].playing() {
            return Input::default();
        }
        let progress = self.walkers[index].progress;
        let skill = self.walkers[index].skill;
        let tile = match self.walkers[index].plan {
            Some((step, tile)) if step == progress => tile,
            _ => {
                let right = self.sequence[progress];
                let tile = if self.rng.random_bool(slip(skill, progress)) {
                    next_tile(&mut self.rng, Some(right))
                } else {
                    right
                };
                let me = &mut self.walkers[index];
                me.plan = Some((progress, tile));
                me.wait = 0.1 + (1.0 - Skill(skill).level()) * 0.3;
                tile
            }
        };
        let (tx, tz) = self.centre(tile);
        let me = &mut self.walkers[index];
        if me.wait > 0.0 {
            me.wait -= dt;
            return Input::default();
        }
        let (dx, dz) = (tx - me.x, tz - me.z);
        let far = dx.hypot(dz);
        if far < 0.6 {
            return Input {
                fire: true,
                ..Default::default()
            };
        }
        let pace = Skill(skill).pace() * far.min(1.0);
        Input::from_stick(Stick {
            x: dx / far * pace,
            z: dz / far * pace,
        })
    }

    fn snapshot(&self) -> Value {
        let mut snap = json!({
            "game": KEY,
            "time": self.clock,
            "phase": self.phase.name(),
            "round": self.round,
            "length": self.sequence.len(),
            "lit": self.lit(),
            "players": self
                .walkers
                .iter()
                .map(|w| json!({
                    "seat": w.seat,
                    "x": w.x,
                    "z": w.z,
                    "body_width": w.width,
                    "progress": w.progress,
                    "done": w.done,
                    "out": w.out.is_some(),
                    "cleared": w.out.map(|(round, _)| round - 1).unwrap_or(self.round - 1),
                }))
                .collect::<Vec<_>>(),
        });
        // Only the walk back is against the clock, so the countdown shows then.
        if self.phase == Phase::Recall {
            snap["limit"] = json!(recall_seconds(self.sequence.len()));
        }
        snap
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_bigger_tiles_and_its_own_starting_spots() {
        let four = MemoryTiles::new(&field(4), 1);
        assert_eq!(four.half(), BOARD_HALF, "four play it as built");
        assert_eq!(four.centre(2), tile_centre(2));
        assert_eq!(four.walkers[0].x, -(BOARD_HALF - 1.0));
        let eight = MemoryTiles::new(&field(8), 1);
        assert!((eight.half() / BOARD_HALF - 2f32.sqrt()).abs() < 1e-4, "eight get the room of two fours");
        for tile in 0..TILES {
            let (x, z) = eight.centre(tile);
            assert_eq!(tile_at(x / eight.size, z / eight.size), Some(tile), "every tile is still where it is drawn");
        }
        let mut spots: Vec<(i32, i32)> = eight
            .walkers
            .iter()
            .map(|w| ((w.x * 10.0) as i32, (w.z * 10.0) as i32))
            .collect();
        spots.sort();
        spots.dedup();
        assert_eq!(spots.len(), 8, "nobody starts on top of anyone else");
        let setup = eight.setup();
        assert_eq!(setup["half"], json!(eight.half()), "and the client is told");
        assert_eq!(setup["spacing"], json!(SPACING * eight.size));
        assert_eq!(setup["tile"], json!(TILE_HALF * eight.size));
    }
    #[test]
    fn small_walkers_move_faster_and_broad_walkers_stomp_farther_from_a_tile() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = MemoryTiles::new(&[small, broad], 3);
        for walker in &mut game.walkers {
            walker.x = 0.0;
            walker.z = 0.0;
        }
        let right = Input::from_stick(Stick { x: 1.0, z: 0.0 });
        game.step(&[(0, right), (1, right)], 0.1, &mut Vec::new());
        assert!(game.walkers[0].x > game.walkers[1].x);
        assert_eq!(tile_at_for_build(2.08, 0.0, 0.7), None);
        assert_eq!(tile_at_for_build(2.08, 0.0, 1.4), Some(4));
        assert_eq!(tile_at_for_build(2.5, 0.0, 1.4), None);
        game.phase = Phase::Recall;
        game.sequence = vec![4];
        for walker in &mut game.walkers {
            walker.x = 2.08 * game.size;
            walker.z = 0.0;
        }
        game.step(&[(0, Input { fire: true, ..Default::default() }),
            (1, Input { fire: true, ..Default::default() })], 0.0, &mut Vec::new());
        assert_eq!(game.walkers[0].progress, 0);
        assert_eq!(game.walkers[1].progress, 1);
        assert!((game.snapshot()["players"][1]["body_width"].as_f64().unwrap() - 1.4).abs() < 0.001);
    }
}
