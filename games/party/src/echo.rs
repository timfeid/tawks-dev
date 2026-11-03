//! Echo Chamber: a ring of glowing chimes plays a pattern, the room turns,
//! and everyone plays the pattern back on the chimes where they now stand.
//! The pattern is the chimes themselves (their colour and tone), never the
//! spots they stood on, so the turn is the trap. Later rounds also mirror the
//! ring and swap two chimes. A wrong chime or running out of time costs one of
//! two lives. Last one standing wins; after the final round the most rounds
//! cleared wins.
//!
//! A pick arrives as a press of fire with `PICK_BASE + slot` in `yaw`. The
//! slot is the fixed spot in the room the player hit, numbered clockwise from
//! the far side; the server maps it to whichever chime stands there now.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill};

pub const KEY: &str = "echo";
pub const CHIMES: usize = 6;
pub const LIVES: u8 = 2;
pub const FIRST_LENGTH: usize = 3;
pub const MAX_ROUNDS: u32 = 7;
pub const RING_RADIUS: f32 = 6.0;
pub const GALLERY_RADIUS: f32 = 11.5;
pub const CHIME_HEIGHT: f32 = 1.2;
/// Added to the slot in a pick's `yaw`, far outside any steering angle, so a
/// stray fire from another control can never read as a chime.
pub const PICK_BASE: f32 = 50.0;
pub const LEAD_IN: f32 = 1.0;
pub const TURN_LEAD: f32 = 0.35;
pub const SPIN_SECONDS: f32 = 1.1;
pub const FLIP_SECONDS: f32 = 0.9;
pub const SWAP_SECONDS: f32 = 0.9;
pub const TURN_SETTLE: f32 = 0.3;
pub const VERDICT_SECONDS: f32 = 1.8;
const HOP_SECONDS: f32 = 0.28;
/// A seat missing from the inputs this long has left the room.
const GONE_SECONDS: f32 = 0.5;

pub const NAMES: [&str; CHIMES] = ["Red", "Amber", "Lime", "Teal", "Blue", "Violet"];

/// Seconds each chime glows while the pattern is shown, and the gap after it.
/// The pattern plays a little faster every round.
pub fn note_seconds(round: u32) -> (f32, f32) {
    let quick = 0.96f32.powi(round.saturating_sub(1) as i32);
    (0.46 * quick, 0.16 * quick)
}

pub fn show_seconds(round: u32, length: usize) -> f32 {
    let (on, gap) = note_seconds(round);
    LEAD_IN + length as f32 * (on + gap)
}

/// Time allowed to play a pattern back.
pub fn replay_seconds(length: usize) -> f32 {
    2.5 + length as f32 * 0.75
}

pub fn slot_angle(slot: f32) -> f32 {
    slot * std::f32::consts::TAU / CHIMES as f32
}

/// Where a slot sits on the floor. Slot 0 is the far side (negative z) and
/// the numbers run clockwise as seen from above with +x to the right.
pub fn slot_position(slot: f32, radius: f32) -> (f32, f32) {
    let angle = slot_angle(slot);
    (angle.sin() * radius, -angle.cos() * radius)
}

/// The slot nearest a direction on the floor, for a stick pointing at a chime.
pub fn slot_toward(x: f32, z: f32) -> usize {
    let angle = x.atan2(-z).rem_euclid(std::f32::consts::TAU);
    ((angle / slot_angle(1.0)).round() as usize) % CHIMES
}

/// How the room changes between showing the pattern and playing it back,
/// applied in this order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Twist {
    /// Slots every chime moves clockwise; negative turns the other way.
    pub spin: i8,
    /// The ring flips left to right across the line through slot 0.
    pub mirror: bool,
    /// Two slots trade chimes after the spin and the flip.
    pub swap: Option<(u8, u8)>,
}

impl Twist {
    pub fn is_none(&self) -> bool {
        self.spin == 0 && !self.mirror && self.swap.is_none()
    }

    pub fn seconds(&self) -> f32 {
        if self.is_none() {
            return 0.6;
        }
        TURN_LEAD
            + if self.spin != 0 { SPIN_SECONDS } else { 0.0 }
            + if self.mirror { FLIP_SECONDS } else { 0.0 }
            + if self.swap.is_some() {
                SWAP_SECONDS
            } else {
                0.0
            }
            + TURN_SETTLE
    }

    pub fn apply(&self, layout: &[u8; CHIMES]) -> [u8; CHIMES] {
        let mut out = spun(layout, self.spin);
        if self.mirror {
            out = mirrored(&out);
        }
        if let Some((a, b)) = self.swap {
            out.swap(a as usize, b as usize);
        }
        out
    }

    pub fn to_json(&self) -> Value {
        json!({
            "spin": self.spin,
            "mirror": self.mirror,
            "swap": self.swap.map(|(a, b)| [a, b]),
        })
    }

    pub fn from_json(value: &Value) -> Twist {
        Twist {
            spin: value["spin"].as_i64().unwrap_or(0) as i8,
            mirror: value["mirror"].as_bool().unwrap_or(false),
            swap: value["swap"].as_array().and_then(|pair| {
                Some((pair.first()?.as_u64()? as u8, pair.get(1)?.as_u64()? as u8))
            }),
        }
    }
}

fn spun(layout: &[u8; CHIMES], spin: i8) -> [u8; CHIMES] {
    let mut out = [0; CHIMES];
    for (slot, chime) in layout.iter().enumerate() {
        let to = (slot as i32 + spin as i32).rem_euclid(CHIMES as i32) as usize;
        out[to] = *chime;
    }
    out
}

fn mirrored(layout: &[u8; CHIMES]) -> [u8; CHIMES] {
    let mut out = [0; CHIMES];
    for (slot, chime) in layout.iter().enumerate() {
        out[(CHIMES - slot) % CHIMES] = *chime;
    }
    out
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How far through its spin the room is `t` seconds into the turn, 0 to 1.
pub fn spin_progress(twist: &Twist, t: f32) -> f32 {
    if twist.spin == 0 {
        return 1.0;
    }
    ease((t - TURN_LEAD) / SPIN_SECONDS)
}

/// Where every chime is `t` seconds into the turn, indexed by chime, as
/// (x, lift above its resting height, z). The ring spins, then flips over
/// the line through slot 0, then the swapped pair hop past each other. Kept
/// here beside the rules so the drawing can never disagree with them.
pub fn chime_positions(
    before: &[u8; CHIMES],
    twist: &Twist,
    t: f32,
    radius: f32,
) -> [[f32; 3]; CHIMES] {
    let mut out = [[0.0; 3]; CHIMES];
    let mut clock = t - TURN_LEAD;
    let spin_t = if twist.spin != 0 {
        let p = ease(clock / SPIN_SECONDS);
        clock -= SPIN_SECONDS;
        p
    } else {
        1.0
    };
    let flip_t = if twist.mirror {
        let p = ease(clock / FLIP_SECONDS);
        clock -= FLIP_SECONDS;
        p
    } else {
        0.0
    };
    let swap_t = if twist.swap.is_some() {
        ease(clock / SWAP_SECONDS)
    } else {
        0.0
    };
    let after_spin = spun(before, twist.spin);
    let after_flip = if twist.mirror {
        mirrored(&after_spin)
    } else {
        after_spin
    };
    for (slot, chime) in before.iter().enumerate() {
        let turned = slot as f32 + twist.spin as f32 * spin_t;
        let (x, z) = slot_position(turned, radius);
        let flip = std::f32::consts::PI * flip_t;
        let mut pos = [x * flip.cos(), x.abs() * flip.sin() * 0.55, z];
        if let Some((a, b)) = twist.swap {
            if swap_t > 0.0 {
                let here = after_flip.iter().position(|c| c == chime).unwrap_or(0);
                let other = if here == a as usize {
                    Some(b)
                } else if here == b as usize {
                    Some(a)
                } else {
                    None
                };
                if let Some(other) = other {
                    let (fx, fz) = slot_position(here as f32, radius);
                    let (tx, tz) = slot_position(other as f32, radius);
                    // The two hop on opposite sides of the straight line so
                    // they visibly pass each other instead of colliding.
                    let side = if here < other as usize { 1.0 } else { -1.0 };
                    let (dx, dz) = (tx - fx, tz - fz);
                    let len = dx.hypot(dz).max(0.001);
                    let bow = (std::f32::consts::PI * swap_t).sin();
                    pos = [
                        fx + dx * swap_t - dz / len * side * bow * 1.4,
                        bow * 2.2,
                        fz + dz * swap_t + dx / len * side * bow * 1.4,
                    ];
                }
            }
        }
        out[*chime as usize] = pos;
    }
    out
}

/// The twist for a round: none to start, then a spin, then a spin and a swap,
/// then a spin and a mirror, then all three.
fn twist_for(round: u32, rng: &mut StdRng) -> Twist {
    if round <= 1 {
        return Twist::default();
    }
    let mut spin = rng.random_range(1..=2) as i8;
    if rng.random_bool(0.5) {
        spin = -spin;
    }
    let mut swap = None;
    if round >= 4 && round != 6 {
        let mut slots: Vec<u8> = (0..CHIMES as u8).collect();
        slots.shuffle(rng);
        swap = Some((slots[0].min(slots[1]), slots[0].max(slots[1])));
    }
    Twist {
        spin,
        mirror: round >= 6,
        swap,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Show,
    Turn,
    Replay,
    Verdict,
}

impl Phase {
    fn name(self) -> &'static str {
        match self {
            Phase::Show => "show",
            Phase::Turn => "turn",
            Phase::Replay => "replay",
            Phase::Verdict => "verdict",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Playing,
    Cleared,
    Missed,
    TimedOut,
}

/// A bot's playback for one round: the slot it will hit for each step it
/// gets to, and when.
#[derive(Clone)]
struct Plan {
    round: u32,
    picks: Vec<(f32, usize)>,
    next: usize,
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    lives: u8,
    /// The round they went out in; 0 for a player who left.
    out: Option<u32>,
    status: Status,
    progress: usize,
    cleared: u32,
    steps: u32,
    /// Milliseconds spent playing back the rounds they cleared; the quicker
    /// player wins a tie.
    pace: u32,
    hop: f32,
    missing: f32,
    plan: Option<Plan>,
}

impl Player {
    fn in_game(&self) -> bool {
        self.out.is_none()
    }

    fn status_name(&self) -> &'static str {
        if self.out == Some(0) {
            return "left";
        }
        if self.out.is_some() {
            return "out";
        }
        match self.status {
            Status::Playing => "playing",
            Status::Cleared => "cleared",
            Status::Missed => "missed",
            Status::TimedOut => "timeout",
        }
    }
}

pub struct EchoChamber {
    players: Vec<Player>,
    sequence: Vec<u8>,
    /// Which chime stands at each slot right now.
    layout: [u8; CHIMES],
    /// The layout while this round's pattern was shown.
    before: [u8; CHIMES],
    twist: Twist,
    round: u32,
    phase: Phase,
    clock: f32,
    shown: usize,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

/// The slot a pick names, if the input is a pick at all.
fn pick(input: &Input) -> Option<usize> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let raw = input.yaw - PICK_BASE;
    let slot = raw.round();
    ((raw - slot).abs() < 0.25 && (0.0..CHIMES as f32).contains(&slot)).then_some(slot as usize)
}

pub fn pick_input(slot: usize) -> Input {
    Input {
        fire: true,
        yaw: PICK_BASE + slot as f32,
        ..Default::default()
    }
}

/// How likely a bot is to lose the step at `index` of a pattern: a small slip
/// anywhere, and a lot more once the pattern runs past what it can hold.
fn forget(skill: f32, index: usize) -> f64 {
    let level = Skill(skill).level();
    let span = 3.5 + level * 5.5;
    let base = 0.008 + (1.0 - level) as f64 * 0.03;
    if index as f32 >= span {
        base + 0.12 + (index as f32 - span) as f64 * 0.12
    } else {
        base
    }
}

/// How likely a bot is to reach for where a chime stood while it was shown
/// instead of where the twist put it.
fn fooled(skill: f32, twist: &Twist) -> f64 {
    if twist.is_none() {
        return 0.0;
    }
    let level = Skill(skill).level() as f64;
    let weight = 1.0 + twist.mirror as u8 as f64 * 0.8 + twist.swap.is_some() as u8 as f64 * 0.5;
    (0.012 + (1.0 - level) * 0.045) * weight
}

impl EchoChamber {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut layout = [0u8; CHIMES];
        for (slot, chime) in layout.iter_mut().enumerate() {
            *chime = slot as u8;
        }
        layout.shuffle(&mut rng);
        let mut sequence = Vec::new();
        for _ in 0..FIRST_LENGTH {
            sequence.push(next_chime(&mut rng, sequence.last().copied()));
        }
        let twist = twist_for(1, &mut rng);
        EchoChamber {
            players: contestants
                .iter()
                .map(|one| Player {
                    seat: one.seat,
                    skill: one.skill,
                    lives: LIVES,
                    out: None,
                    status: Status::Playing,
                    progress: 0,
                    cleared: 0,
                    steps: 0,
                    pace: 0,
                    hop: 0.0,
                    missing: 0.0,
                    plan: None,
                })
                .collect(),
            sequence,
            layout,
            before: layout,
            twist,
            round: 1,
            phase: Phase::Show,
            clock: 0.0,
            shown: 0,
            rng,
            placed: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn sequence(&self) -> &[u8] {
        &self.sequence
    }

    pub fn layout(&self) -> [u8; CHIMES] {
        self.layout
    }

    pub fn twist(&self) -> Twist {
        self.twist
    }

    pub fn lives(&self, seat: u8) -> Option<u8> {
        self.players
            .iter()
            .find(|p| p.seat == seat)
            .map(|p| p.lives)
    }

    /// The slot holding a chime in the current layout.
    pub fn slot_of(&self, chime: u8) -> usize {
        self.layout.iter().position(|c| *c == chime).unwrap_or(0)
    }

    /// The chime glowing right now while the pattern is shown.
    pub fn lit(&self) -> Option<u8> {
        if self.phase != Phase::Show || self.clock < LEAD_IN {
            return None;
        }
        let (on, gap) = note_seconds(self.round);
        let into = self.clock - LEAD_IN;
        let step = (into / (on + gap)) as usize;
        (into - step as f32 * (on + gap) < on)
            .then(|| self.sequence.get(step).copied())
            .flatten()
    }

    fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.clock = 0.0;
    }

    fn gallery_spot(&self, index: usize) -> (f32, f32, f32) {
        let count = self.players.len().max(1);
        let spread = (count as f32 - 1.0).max(1.0);
        let arc = (0.32 * (count as f32 - 1.0)).min(2.2);
        let angle = if count == 1 {
            0.0
        } else {
            -arc / 2.0 + arc * index as f32 / spread
        };
        let x = angle.sin() * GALLERY_RADIUS;
        let z = -angle.cos() * GALLERY_RADIUS;
        // Facing the room's middle.
        (x, z, (-x).atan2(-z))
    }

    fn ring(&mut self, index: usize, slot: usize, events: &mut Vec<Value>) {
        let chime = self.layout[slot];
        let player = &mut self.players[index];
        let wanted = self.sequence[player.progress];
        player.hop = HOP_SECONDS;
        if chime == wanted {
            player.progress += 1;
            player.steps += 1;
            // Which chime was hit stays off the wire while others are still
            // playing back; the player's own client lights it locally.
            events.push(json!({
                "kind": "ring",
                "seat": player.seat,
                "step": player.progress,
                "ok": true,
            }));
            if player.progress == self.sequence.len() {
                player.status = Status::Cleared;
                player.cleared += 1;
                player.pace += (self.clock * 1000.0) as u32;
                events.push(json!({ "kind": "clear", "seat": player.seat, "round": self.round }));
            }
        } else {
            player.status = Status::Missed;
            player.lives = player.lives.saturating_sub(1);
            events.push(json!({
                "kind": "ring",
                "seat": player.seat,
                "step": player.progress + 1,
                "ok": false,
                "lives": player.lives,
            }));
            if player.lives == 0 {
                player.out = Some(self.round);
                events.push(json!({ "kind": "out", "seat": player.seat, "round": self.round }));
            }
        }
    }

    fn end_replay(&mut self, events: &mut Vec<Value>) {
        let round = self.round;
        for player in self.players.iter_mut().filter(|p| p.in_game()) {
            if player.status == Status::Playing {
                player.status = Status::TimedOut;
                player.lives = player.lives.saturating_sub(1);
                events
                    .push(json!({ "kind": "timeout", "seat": player.seat, "lives": player.lives }));
                if player.lives == 0 {
                    player.out = Some(round);
                    events.push(json!({ "kind": "out", "seat": player.seat, "round": round }));
                }
            }
        }
        events.push(json!({
            "kind": "verdict",
            "round": round,
            "sequence": self.sequence,
            "standing": self.players.iter().filter(|p| p.in_game()).count(),
        }));
        self.enter(Phase::Verdict);
    }

    fn after_verdict(&mut self, events: &mut Vec<Value>) {
        let standing = self.players.iter().filter(|p| p.in_game()).count();
        if standing <= 1 || self.round >= MAX_ROUNDS {
            self.settle(events);
            return;
        }
        self.round += 1;
        let last = self.sequence.last().copied();
        self.sequence.push(next_chime(&mut self.rng, last));
        self.before = self.layout;
        self.twist = twist_for(self.round, &mut self.rng);
        self.shown = 0;
        for player in self.players.iter_mut() {
            player.progress = 0;
            player.status = Status::Playing;
            player.plan = None;
        }
        events.push(json!({ "kind": "round", "round": self.round, "length": self.sequence.len() }));
        self.enter(Phase::Show);
    }

    /// Still standing first, then whoever lasted longest, then most rounds
    /// cleared, lives left, chimes hit, and the quicker playback.
    fn settle(&mut self, events: &mut Vec<Value>) {
        let mut order: Vec<((bool, u32, u32, u8, u32, u32), u8)> = self
            .players
            .iter()
            .map(|p| {
                (
                    (
                        p.out.is_none(),
                        p.out.unwrap_or(u32::MAX),
                        p.cleared,
                        p.lives,
                        p.steps,
                        u32::MAX - p.pace,
                    ),
                    p.seat,
                )
            })
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
        events.push(
            json!({ "kind": "final", "winners": groups.first().cloned().unwrap_or_default() }),
        );
        self.placed = Some(groups);
    }

    fn plan_for(&mut self, index: usize) -> Plan {
        let skill = self.players[index].skill;
        let level = Skill(skill).level();
        let mut at = Skill(skill).reaction() * 0.8 + self.rng.random_range(0.1..0.5);
        let mut picks = Vec::new();
        for (step, chime) in self.sequence.iter().enumerate() {
            let right = self.slot_of(*chime);
            let was = self.before.iter().position(|c| c == chime).unwrap_or(right);
            let slot = if was != right && self.rng.random_bool(fooled(skill, &self.twist)) {
                was
            } else if self.rng.random_bool(forget(skill, step)) {
                let mut other = self.rng.random_range(0..CHIMES - 1);
                if other >= right {
                    other += 1;
                }
                other
            } else {
                right
            };
            picks.push((at, slot));
            if slot != right {
                break;
            }
            at += 0.26 + (1.0 - level) * 0.32 + self.rng.random_range(0.0..0.22);
        }
        Plan {
            round: self.round,
            picks,
            next: 0,
        }
    }
}

fn next_chime(rng: &mut StdRng, last: Option<u8>) -> u8 {
    loop {
        let chime = rng.random_range(0..CHIMES as u8);
        if Some(chime) != last {
            return chime;
        }
    }
}

impl Bout for EchoChamber {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "chimes": CHIMES,
            "ring": RING_RADIUS,
            "gallery": GALLERY_RADIUS,
            "lives": LIVES,
            "rounds": MAX_ROUNDS,
            "names": NAMES,
            // The default round floor the generic stage draws.
            "radius": GALLERY_RADIUS + 2.5,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        for player in self.players.iter_mut() {
            player.hop = (player.hop - dt).max(0.0);
            if inputs.iter().any(|(seat, _)| *seat == player.seat) {
                player.missing = 0.0;
            } else if player.in_game() {
                player.missing += dt;
                if player.missing >= GONE_SECONDS {
                    player.out = Some(0);
                    player.lives = 0;
                    events.push(json!({ "kind": "left", "seat": player.seat }));
                }
            }
        }

        if self.phase == Phase::Replay {
            for (seat, input) in inputs {
                let Some(slot) = pick(input) else { continue };
                let Some(index) = self
                    .players
                    .iter()
                    .position(|p| p.seat == *seat && p.in_game() && p.status == Status::Playing)
                else {
                    continue;
                };
                self.ring(index, slot, events);
            }
        }

        match self.phase {
            Phase::Show => {
                let (on, gap) = note_seconds(self.round);
                while self.shown < self.sequence.len()
                    && self.clock >= LEAD_IN + self.shown as f32 * (on + gap)
                {
                    events.push(json!({
                        "kind": "chime",
                        "chime": self.sequence[self.shown],
                        "step": self.shown,
                    }));
                    self.shown += 1;
                }
                if self.clock >= show_seconds(self.round, self.sequence.len()) {
                    self.layout = self.twist.apply(&self.before);
                    events.push(json!({
                        "kind": "twist",
                        "round": self.round,
                        "twist": self.twist.to_json(),
                    }));
                    self.enter(Phase::Turn);
                }
            }
            Phase::Turn => {
                if self.clock >= self.twist.seconds() {
                    events.push(json!({ "kind": "go", "round": self.round }));
                    self.enter(Phase::Replay);
                }
            }
            Phase::Replay => {
                let waiting = self
                    .players
                    .iter()
                    .any(|p| p.in_game() && p.status == Status::Playing);
                if !waiting || self.clock >= replay_seconds(self.sequence.len()) {
                    self.end_replay(events);
                }
            }
            Phase::Verdict => {
                if self.clock >= VERDICT_SECONDS {
                    self.after_verdict(events);
                }
            }
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() || self.phase != Phase::Replay {
            return Input::default();
        }
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let me = &self.players[index];
        if !me.in_game() || me.status != Status::Playing {
            return Input::default();
        }
        if me.plan.as_ref().is_none_or(|plan| plan.round != self.round) {
            let plan = self.plan_for(index);
            self.players[index].plan = Some(plan);
        }
        let clock = self.clock;
        let Some(plan) = self.players[index].plan.as_mut() else {
            return Input::default();
        };
        match plan.picks.get(plan.next) {
            Some((at, slot)) if clock >= *at => {
                plan.next += 1;
                pick_input(*slot)
            }
            _ => Input::default(),
        }
    }

    fn snapshot(&self) -> Value {
        let limit = match self.phase {
            Phase::Show => show_seconds(self.round, self.sequence.len()),
            Phase::Turn => self.twist.seconds(),
            Phase::Replay => replay_seconds(self.sequence.len()),
            Phase::Verdict => VERDICT_SECONDS,
        };
        json!({
            "game": KEY,
            "time": self.clock,
            "limit": limit,
            "phase": self.phase.name(),
            "round": self.round,
            "rounds": MAX_ROUNDS,
            "length": self.sequence.len(),
            "lit": self.lit(),
            "layout": self.layout,
            "before": self.before,
            "twist": self.twist.to_json(),
            // The pattern goes on the wire only once nobody can still use it.
            "sequence": (self.phase == Phase::Verdict || self.placed.is_some()).then_some(&self.sequence),
            "players": self
                .players
                .iter()
                .enumerate()
                .map(|(index, p)| {
                    let (x, z, yaw) = self.gallery_spot(index);
                    let hop = (p.hop / HOP_SECONDS * std::f32::consts::PI).sin().max(0.0) * 0.45;
                    json!({
                        "seat": p.seat,
                        "x": x,
                        "z": z,
                        "yaw": yaw,
                        "height": hop,
                        "lives": p.lives,
                        "progress": p.progress,
                        "cleared": p.cleared,
                        "status": p.status_name(),
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
#[path = "echo_tests.rs"]
mod tests;
