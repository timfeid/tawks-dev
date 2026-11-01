//! Ledge Shuffle: everyone on one round ledge over a long drop, shoving. Press
//! the action to wind up a shove that sends whoever is in front of you
//! skidding. Stand still and you brace: a shove barely moves you, and the one
//! who threw it bounces off. The ledge crumbles from the rim inward, each
//! section cracking a moment before it goes, so the room keeps shrinking until
//! one is left on the last stone.

use std::collections::HashMap;
use std::f32::consts::TAU;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, Runner};

/// The ledge for four; a bigger field gets a wider one with more sections.
pub const ARENA_RADIUS: f32 = 10.0;
/// The middle stone, which never falls.
pub const CORE_RADIUS: f32 = 2.8;
/// The rings round the core, innermost first, as (outer edge, sections).
pub const RINGS: [(f32, usize); 3] = [(5.2, 6), (7.6, 9), (ARENA_RADIUS, 12)];
/// When each ring, innermost first, starts and stops cracking. The rim goes
/// first and the inner ring last.
pub const RING_SCHEDULE: [(f32, f32); 3] = [(50.0, 62.0), (31.0, 47.0), (10.0, 28.0)];
/// How long a section shakes before it drops. It still holds you meanwhile.
pub const CRACK_SECONDS: f32 = 2.2;
/// Once the rings are gone the core itself wears away, down to room for one.
pub const CORE_SHRINK: (f32, f32) = (66.0, 78.0);
pub const CORE_MIN_RADIUS: f32 = 0.7;
pub const FALL_SECONDS: f32 = 0.6;
pub const ROUND_SECONDS: f32 = 80.0;

pub const WINDUP_SECONDS: f32 = 0.22;
/// From the press, so a shove can be thrown about once a second.
pub const SHOVE_COOLDOWN: f32 = 0.85;
/// Centre to centre, so a little over an arm's length between two players.
pub const SHOVE_REACH: f32 = 2.4;
/// The cosine of how far either side of straight ahead a shove still lands.
pub const SHOVE_CONE: f32 = 0.5;
pub const SHOVE_SPEED: f32 = 9.5;
pub const SHOVE_STAGGER: f32 = 0.45;
/// How long you have to stand still before you are braced.
pub const BRACE_SETTLE: f32 = 0.3;
/// The share of a shove a braced player still takes.
pub const BRACED_TAKES: f32 = 0.3;
/// How hard a shover bounces off someone braced, and for how long they reel.
pub const RECOIL_SPEED: f32 = 7.0;
pub const RECOIL_STAGGER: f32 = 0.35;

const WINDUP_PACE: f32 = 0.35;
const LUNGE_SPEED: f32 = 4.0;
/// Skidding keeps far more of its speed than running does.
const SLIDE_DRAG: f32 = 2.0;
const GRAVITY: f32 = 22.0;
const OUT_DEPTH: f32 = -9.0;
const STILL_STICK: f32 = 0.15;
/// A fall this soon after a shove counts for whoever threw it.
const CREDIT_SECONDS: f32 = 2.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    Solid,
    Cracking,
    Falling,
    Gone,
}

impl Footing {
    pub fn holds(self) -> bool {
        matches!(self, Footing::Solid | Footing::Cracking)
    }

    fn code(self) -> u8 {
        match self {
            Footing::Solid => 0,
            Footing::Cracking => 1,
            Footing::Falling => 2,
            Footing::Gone => 3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ring {
    pub inner: f32,
    pub outer: f32,
    pub count: usize,
    /// Where the first section starts, so the seams of neighbouring rings
    /// never line up.
    pub turn: f32,
    /// The index of this ring's first section.
    pub first: usize,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub ring: usize,
    pub crack_at: f32,
    pub state: Footing,
    pub timer: f32,
}

#[derive(Clone, Debug)]
pub struct Shover {
    pub seat: u8,
    pub run: Runner,
    pub y: f32,
    pub vy: f32,
    pub facing: f32,
    pub windup: f32,
    pub cooldown: f32,
    pub stagger: f32,
    /// How long the stick has been let go.
    pub still: f32,
    pub fell_at: Option<f32>,
    pub hit_by: Option<(u8, f32)>,
    pub knockoffs: u32,
}

impl Shover {
    pub fn standing(&self) -> bool {
        self.fell_at.is_none()
    }

    pub fn winding(&self) -> bool {
        self.windup > 0.0
    }

    pub fn skidding(&self) -> bool {
        self.stagger > 0.0
    }

    pub fn braced(&self) -> bool {
        self.standing() && !self.winding() && !self.skidding() && self.still >= BRACE_SETTLE
    }

    fn ahead(&self) -> (f32, f32) {
        (self.facing.sin(), self.facing.cos())
    }

    /// Whether `other` stands in front of this player within `reach`.
    fn covers(&self, other: &Shover, reach: f32) -> bool {
        let (dx, dz) = (other.run.x - self.run.x, other.run.z - self.run.z);
        let gap = (dx * dx + dz * dz).sqrt();
        if gap > reach || gap < 1e-4 {
            return false;
        }
        let (fx, fz) = self.ahead();
        (dx * fx + dz * fz) / gap >= SHOVE_CONE
    }
}

#[derive(Clone, Debug, Default)]
struct Brain {
    target: Option<u8>,
    think_in: f32,
    wobble: f32,
    pause: f32,
    /// Counts down once a mark is in reach, for the bot's reaction time.
    aim: Option<f32>,
    /// The shover whose wind-up this bot has already decided about.
    saw: Option<u8>,
    dodge: bool,
    /// Holding still to brace while a rival comes close.
    guard: bool,
}

pub struct LedgeShuffle {
    allies: HashMap<u8, u8>,
    pub players: Vec<Shover>,
    pub rings: Vec<Ring>,
    pub sections: Vec<Section>,
    /// The core as it stands; it starts at `base_core` and wears away at the end.
    pub core: f32,
    base_core: f32,
    pub time: f32,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    brains: HashMap<u8, Brain>,
    placements: Option<Vec<Vec<u8>>>,
}

impl LedgeShuffle {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let size = crate::bout::crowd_scale(contestants.len());
        let core = CORE_RADIUS * size;
        let mut rings = Vec::new();
        let mut sections = Vec::new();
        let mut inner = core;
        for (k, (outer, count)) in RINGS.iter().enumerate() {
            let count = ((*count as f32) * size).round() as usize;
            let outer = outer * size;
            rings.push(Ring {
                inner,
                outer,
                count,
                turn: k as f32 * 0.37,
                first: sections.len(),
            });
            let (from, until) = RING_SCHEDULE[k];
            let mut order: Vec<usize> = (0..count).collect();
            order.shuffle(&mut rng);
            let gap = (until - from) / (count.max(2) - 1) as f32;
            let mut ring_sections: Vec<Section> = (0..count)
                .map(|_| Section {
                    ring: k,
                    crack_at: 0.0,
                    state: Footing::Solid,
                    timer: 0.0,
                })
                .collect();
            for (slot, &index) in order.iter().enumerate() {
                ring_sections[index].crack_at = from + gap * slot as f32;
            }
            sections.extend(ring_sections);
            inner = outer;
        }
        let spread = ground::ring(
            contestants.len().max(1),
            (rings[0].inner + rings[0].outer) * 0.5,
            0.35,
        );
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Shover {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                y: 0.0,
                vy: 0.0,
                facing: (-spread[i].0).atan2(-spread[i].1),
                windup: 0.0,
                cooldown: 0.0,
                stagger: 0.0,
                still: 0.0,
                fell_at: None,
                hit_by: None,
                knockoffs: 0,
            })
            .collect();
        LedgeShuffle {
            allies: contestants
                .iter()
                .filter_map(|c| c.ally.map(|ally| (c.seat, ally)))
                .collect(),
            players,
            rings,
            sections,
            core,
            base_core: core,
            time: 0.0,
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            brains: contestants
                .iter()
                .filter(|c| c.is_bot)
                .map(|c| (c.seat, Brain::default()))
                .collect(),
            placements: None,
        }
    }

    pub fn radius(&self) -> f32 {
        self.rings.last().map_or(self.core, |ring| ring.outer)
    }

    /// The section under a point: `Some(None)` on the core, `None` off the
    /// ledge altogether.
    pub fn section_at(&self, x: f32, z: f32) -> Option<Option<usize>> {
        let out = (x * x + z * z).sqrt();
        if out <= self.core {
            return Some(None);
        }
        let ring = self
            .rings
            .iter()
            .find(|ring| out > ring.inner && out <= ring.outer)?;
        let around = (z.atan2(x) - ring.turn).rem_euclid(TAU);
        let index = ((around / (TAU / ring.count as f32)) as usize).min(ring.count - 1);
        Some(Some(ring.first + index))
    }

    /// Whether the ledge holds someone standing here.
    pub fn holds(&self, x: f32, z: f32) -> bool {
        match self.section_at(x, z) {
            Some(None) => true,
            Some(Some(index)) => self.sections[index].state.holds(),
            None => false,
        }
    }

    /// Whether this ground will still be here in a moment.
    fn sound(&self, x: f32, z: f32) -> bool {
        match self.section_at(x, z) {
            Some(None) => true,
            Some(Some(index)) => self.sections[index].state == Footing::Solid,
            None => false,
        }
    }

    /// How far from a point the nearest unsound ground is, and which way.
    fn nearest_drop(&self, x: f32, z: f32) -> (f32, (f32, f32)) {
        const DIRECTIONS: usize = 12;
        for step in 1..=7 {
            let reach = step as f32 * 0.5;
            let mut found: Option<(f32, f32)> = None;
            for d in 0..DIRECTIONS {
                let angle = TAU * d as f32 / DIRECTIONS as f32;
                let (dx, dz) = (angle.cos(), angle.sin());
                if !self.sound(x + dx * reach, z + dz * reach) {
                    found = Some(match found {
                        Some((sx, sz)) => (sx + dx, sz + dz),
                        None => (dx, dz),
                    });
                }
            }
            if let Some((dx, dz)) = found {
                let length = (dx * dx + dz * dz).sqrt();
                let way = if length < 1e-3 {
                    let out = (x * x + z * z).sqrt().max(1e-3);
                    (x / out, z / out)
                } else {
                    (dx / length, dz / length)
                };
                return (reach, way);
            }
        }
        let out = (x * x + z * z).sqrt().max(1e-3);
        (4.0, (x / out, z / out))
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    fn partners(&self, a: u8, b: u8) -> bool {
        crate::bout::allied(self.allies.get(&a).copied(), self.allies.get(&b).copied())
    }

    pub fn core_at(&self, time: f32) -> f32 {
        let worn = ((time - CORE_SHRINK.0) / (CORE_SHRINK.1 - CORE_SHRINK.0)).clamp(0.0, 1.0);
        self.base_core + (CORE_MIN_RADIUS - self.base_core) * worn
    }

    fn crumble(&mut self, dt: f32, events: &mut Vec<Value>) {
        let time = self.time;
        self.core = self.core_at(time);
        for (index, section) in self.sections.iter_mut().enumerate() {
            match section.state {
                Footing::Solid if time >= section.crack_at => {
                    section.state = Footing::Cracking;
                    section.timer = CRACK_SECONDS;
                    events.push(json!({ "kind": "ledge_crack", "section": index }));
                }
                Footing::Cracking => {
                    section.timer -= dt;
                    if section.timer <= 0.0 {
                        section.state = Footing::Falling;
                        section.timer = FALL_SECONDS;
                        events.push(json!({ "kind": "ledge_fall", "section": index }));
                    }
                }
                Footing::Falling => {
                    section.timer -= dt;
                    if section.timer <= 0.0 {
                        section.state = Footing::Gone;
                    }
                }
                _ => {}
            }
        }
    }

    /// Moves everyone and returns who let a shove go this step.
    fn walk(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) -> Vec<usize> {
        let mut thrown = Vec::new();
        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let player = &mut self.players[index];
            if !player.standing() {
                if player.y > OUT_DEPTH {
                    player.vy -= GRAVITY * dt;
                    player.y += player.vy * dt;
                    player.run.x += player.run.vx * dt;
                    player.run.z += player.run.vz * dt;
                }
                continue;
            }
            player.cooldown = (player.cooldown - dt).max(0.0);
            if player.skidding() {
                player.stagger = (player.stagger - dt).max(0.0);
                player.windup = 0.0;
                player.still = 0.0;
                let keep = (-SLIDE_DRAG * dt).exp();
                player.run.vx *= keep;
                player.run.vz *= keep;
                player.run.x += player.run.vx * dt;
                player.run.z += player.run.vz * dt;
                continue;
            }
            let stick = input.stick.clamped();
            let push = (stick.x * stick.x + stick.z * stick.z).sqrt();
            if input.fire && player.cooldown <= 0.0 && !player.winding() {
                player.windup = WINDUP_SECONDS;
                player.cooldown = SHOVE_COOLDOWN;
                if push > 0.2 {
                    player.facing = stick.x.atan2(stick.z);
                }
                events.push(json!({ "kind": "ledge_windup", "seat": seat, "yaw": player.facing }));
            }
            let pace = if player.winding() { WINDUP_PACE } else { 1.0 };
            ground::step_runner(&mut player.run, stick, dt, ground::RUN_SPEED * pace);
            if !player.winding() && push > 0.2 {
                player.facing = stick.x.atan2(stick.z);
            }
            player.still = if push < STILL_STICK && !player.winding() {
                player.still + dt
            } else {
                0.0
            };
            if player.winding() {
                player.windup -= dt;
                if player.windup <= 0.0 {
                    player.windup = 0.0;
                    thrown.push(index);
                }
            }
        }
        self.separate();
        thrown
    }

    /// Keeps everyone out of each other's space. Someone braced does not give
    /// ground, so walking into them is no way round the brace.
    fn separate(&mut self) {
        let reach = ground::RUNNER_RADIUS * 2.0;
        for i in 0..self.players.len() {
            for j in (i + 1)..self.players.len() {
                let (a, b) = (&self.players[i], &self.players[j]);
                if !a.standing() || !b.standing() {
                    continue;
                }
                let (dx, dz) = (b.run.x - a.run.x, b.run.z - a.run.z);
                let gap = (dx * dx + dz * dz).sqrt();
                if gap >= reach || gap < 1e-4 {
                    continue;
                }
                let share_a = match (a.braced(), b.braced()) {
                    (true, false) => 0.0,
                    (false, true) => 1.0,
                    _ => 0.5,
                };
                let (nx, nz) = (dx / gap, dz / gap);
                let overlap = reach - gap;
                self.players[i].run.x -= nx * overlap * share_a;
                self.players[i].run.z -= nz * overlap * share_a;
                self.players[j].run.x += nx * overlap * (1.0 - share_a);
                self.players[j].run.z += nz * overlap * (1.0 - share_a);
            }
        }
    }

    fn throw(&mut self, index: usize, events: &mut Vec<Value>) {
        let time = self.time;
        let shover = self.players[index].clone();
        if !shover.standing() || shover.skidding() {
            return;
        }
        let (fx, fz) = shover.ahead();
        {
            let me = &mut self.players[index];
            me.run.vx += fx * LUNGE_SPEED;
            me.run.vz += fz * LUNGE_SPEED;
        }
        let (mut landed, mut blocked) = (false, false);
        for target in 0..self.players.len() {
            let other = self.players[target].clone();
            if target == index
                || !other.standing()
                || self.partners(shover.seat, other.seat)
                || !shover.covers(&other, SHOVE_REACH)
            {
                continue;
            }
            let (dx, dz) = (other.run.x - shover.run.x, other.run.z - shover.run.z);
            let gap = (dx * dx + dz * dz).sqrt();
            let (nx, nz) = (dx / gap, dz / gap);
            let at = (
                (shover.run.x + other.run.x) / 2.0,
                (shover.run.z + other.run.z) / 2.0,
            );
            if other.braced() {
                blocked = true;
                {
                    let them = &mut self.players[target];
                    them.run.vx += nx * SHOVE_SPEED * BRACED_TAKES;
                    them.run.vz += nz * SHOVE_SPEED * BRACED_TAKES;
                }
                let me = &mut self.players[index];
                me.run.vx = -nx * RECOIL_SPEED;
                me.run.vz = -nz * RECOIL_SPEED;
                me.stagger = RECOIL_STAGGER;
                me.hit_by = Some((other.seat, time));
                events.push(json!({
                    "kind": "ledge_block",
                    "seat": other.seat,
                    "by": shover.seat,
                    "x": at.0,
                    "z": at.1,
                }));
            } else {
                landed = true;
                let them = &mut self.players[target];
                them.run.vx = nx * SHOVE_SPEED + them.run.vx * 0.2;
                them.run.vz = nz * SHOVE_SPEED + them.run.vz * 0.2;
                them.stagger = SHOVE_STAGGER;
                them.windup = 0.0;
                them.still = 0.0;
                them.hit_by = Some((shover.seat, time));
                events.push(json!({
                    "kind": "ledge_hit",
                    "seat": other.seat,
                    "by": shover.seat,
                    "x": at.0,
                    "z": at.1,
                }));
            }
        }
        events.push(json!({
            "kind": "ledge_shove",
            "seat": shover.seat,
            "x": shover.run.x,
            "z": shover.run.z,
            "yaw": shover.facing,
            "landed": landed,
            "blocked": blocked,
        }));
    }

    fn drop_the_fallen(&mut self, events: &mut Vec<Value>) {
        let time = self.time;
        let mut fell: Vec<(usize, Option<u8>)> = Vec::new();
        for (index, player) in self.players.iter().enumerate() {
            if player.standing() && !self.holds(player.run.x, player.run.z) {
                let by = player
                    .hit_by
                    .filter(|(_, at)| time - at <= CREDIT_SECONDS)
                    .map(|(seat, _)| seat);
                fell.push((index, by));
            }
        }
        for (index, by) in fell {
            let player = &mut self.players[index];
            player.fell_at = Some(time);
            player.vy = 1.4;
            player.stagger = 0.0;
            player.windup = 0.0;
            let seat = player.seat;
            if let Some(by) = by
                && let Some(shover) = self.players.iter_mut().find(|p| p.seat == by)
            {
                shover.knockoffs += 1;
            }
            events.push(json!({ "kind": "over", "seat": seat, "by": by }));
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let standing = self.players.iter().filter(|p| p.standing()).count();
        if standing > 1 && self.time < ROUND_SECONDS {
            return;
        }
        let mut order: Vec<(f32, u8)> = self
            .players
            .iter()
            .map(|p| (p.fell_at.unwrap_or(f32::INFINITY), p.seat))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group))
                    if (prev - at).abs() < 1e-3 || (prev.is_infinite() && at.is_infinite()) =>
                {
                    group.push(seat)
                }
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }

    /// The nearest ground that is not about to go, leaning toward the middle.
    fn safe_spot(&self, from: (f32, f32)) -> (f32, f32) {
        let mut best = ((0.0, 0.0), (from.0 * from.0 + from.1 * from.1).sqrt());
        for (index, section) in self.sections.iter().enumerate() {
            if section.state != Footing::Solid {
                continue;
            }
            let ring = self.rings[section.ring];
            let slot = index - ring.first;
            let angle = ring.turn + TAU * (slot as f32 + 0.5) / ring.count as f32;
            let out = (ring.inner + ring.outer) / 2.0;
            let spot = (angle.cos() * out, angle.sin() * out);
            let cost = ((spot.0 - from.0).powi(2) + (spot.1 - from.1).powi(2)).sqrt()
                + (section.ring + 1) as f32 * 1.2;
            if cost < best.1 {
                best = (spot, cost);
            }
        }
        best.0
    }

    /// Get on the far side of a mark from the drop nearest them, then close in.
    fn line_up(&self, me: &Shover, mark: &Shover) -> (Stick, bool) {
        let here = (me.run.x, me.run.z);
        let (_, (wx, wz)) = self.nearest_drop(mark.run.x, mark.run.z);
        let gap = me.run.distance_to(mark.run.x, mark.run.z);
        let (tx, tz) = (
            (mark.run.x - me.run.x) / gap.max(1e-3),
            (mark.run.z - me.run.z) / gap.max(1e-3),
        );
        let lined_up = tx * wx + tz * wz > 0.35;
        if lined_up || gap > 4.5 {
            return (ground::steer(here, (mark.run.x, mark.run.z)), lined_up);
        }
        let stage = (mark.run.x - wx * 1.7, mark.run.z - wz * 1.7);
        if self.sound(stage.0, stage.1) {
            (ground::steer(here, stage), false)
        } else {
            (ground::steer(here, (mark.run.x, mark.run.z)), false)
        }
    }
}

impl Bout for LedgeShuffle {
    fn key(&self) -> &'static str {
        "ledge"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "ledge",
            "rule": "Shove rivals off the crumbling ledge. Stand still to brace. Last one up wins.",
            "radius": self.radius(),
            "core": self.base_core,
            "min_core": CORE_MIN_RADIUS,
            "rings": self.rings.iter().map(|ring| json!({
                "inner": ring.inner,
                "outer": ring.outer,
                "count": ring.count,
                "turn": ring.turn,
            })).collect::<Vec<_>>(),
            "reach": SHOVE_REACH,
            "cone": SHOVE_CONE,
            "windup": WINDUP_SECONDS,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.crumble(dt, events);
        let thrown = self.walk(inputs, dt, events);
        for index in thrown {
            self.throw(index, events);
        }
        self.drop_the_fallen(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.players.iter().find(|p| p.seat == seat).cloned() else {
            return Input::default();
        };
        if !me.standing() || me.skidding() {
            return Input::default();
        }
        let skill = self.skill(seat);
        let rivals: Vec<Shover> = self
            .players
            .iter()
            .filter(|p| p.seat != seat && p.standing() && !self.partners(seat, p.seat))
            .cloned()
            .collect();
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        brain.pause -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = self.rng.random_range(0.3..0.6) + skill.reaction() * 0.4;
            let wobble = skill.aim_error() * 0.6;
            brain.wobble = self.rng.random_range(-wobble..wobble);
            if self.rng.random_bool(skill.hesitation() * 0.35) {
                brain.pause = self.rng.random_range(0.3..0.8);
            }
            let (margin, _) = self.nearest_drop(me.run.x, me.run.z);
            brain.guard =
                margin < 3.0 && self.rng.random_bool((0.15 + skill.level() * 0.45) as f64);
            // Whoever is nearest a drop, with a nudge toward whoever is near;
            // someone braced is a poor mark.
            brain.target = rivals
                .iter()
                .map(|r| {
                    let (margin, _) = self.nearest_drop(r.run.x, r.run.z);
                    let near = me.run.distance_to(r.run.x, r.run.z);
                    let braced = if r.braced() { 2.5 } else { 0.0 };
                    (margin * 1.5 + near * 0.7 + braced, r.seat)
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, s)| s);
        }

        let here = (me.run.x, me.run.z);
        let (margin, (wx, wz)) = self.nearest_drop(here.0, here.1);
        let threat = rivals
            .iter()
            .find(|r| r.winding() && r.covers(&me, SHOVE_REACH + 0.6));
        match threat {
            Some(shover) if brain.saw != Some(shover.seat) => {
                brain.saw = Some(shover.seat);
                brain.dodge = self.rng.random_bool((0.15 + skill.level() * 0.6) as f64);
            }
            None => brain.saw = None,
            _ => {}
        }
        let mut fire = false;
        let stick = if !self.sound(here.0, here.1) || margin < 1.0 {
            brain.aim = None;
            ground::steer(here, self.safe_spot(here))
        } else if let Some(shover) = threat.filter(|_| brain.dodge) {
            if me.braced() && margin > 1.5 {
                Stick::default()
            } else {
                // Step out of the cone, to whichever side is further from a drop.
                let (fx, fz) = shover.ahead();
                let (sx, sz) = (-fz, fx);
                let side = if sx * wx + sz * wz > 0.0 { -1.0 } else { 1.0 };
                Stick {
                    x: sx * side,
                    z: sz * side,
                }
            }
        } else if brain.pause > 0.0
            || brain.guard
                && rivals
                    .iter()
                    .any(|r| r.run.distance_to(here.0, here.1) < 3.5)
        {
            Stick::default()
        } else if let Some(mark) = brain
            .target
            .and_then(|t| rivals.iter().find(|r| r.seat == t))
        {
            let gap = me.run.distance_to(mark.run.x, mark.run.z);
            let (approach, lined_up) = self.line_up(&me, mark);
            if mark.braced() && gap <= SHOVE_REACH {
                // Shoving someone braced only bounces you off, so wait them out.
                Stick::default()
            } else if gap <= SHOVE_REACH * 0.9 && (lined_up || gap < 1.6) {
                let wait = brain
                    .aim
                    .get_or_insert_with(|| skill.reaction() * 0.5)
                    .max(0.0);
                if wait <= 0.0 && me.cooldown <= 0.0 && !me.winding() {
                    fire = true;
                    brain.aim = None;
                } else if let Some(left) = brain.aim.as_mut() {
                    *left -= dt;
                }
                ground::steer(here, (mark.run.x, mark.run.z))
            } else {
                brain.aim = None;
                approach
            }
        } else {
            ground::steer(here, (0.0, 0.0))
        };
        let (cos, sin) = (brain.wobble.cos(), brain.wobble.sin());
        let stick = Stick {
            x: stick.x * cos - stick.z * sin,
            z: stick.x * sin + stick.z * cos,
        };
        self.brains.insert(seat, brain);
        let pace = skill.pace();
        Input {
            stick: Stick {
                x: stick.x * pace,
                z: stick.z * pace,
            }
            .clamped(),
            fire,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "ledge",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "core": round(self.core),
            "sections": self.sections.iter().map(|s| s.state.code()).collect::<Vec<_>>(),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "y": round(p.y),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "yaw": round(p.facing),
                "skidding": p.skidding(),
                "winding": p.winding(),
                "braced": p.braced(),
                "ready": p.cooldown <= 0.0,
                "knockoffs": p.knockoffs,
                "out": !p.standing(),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
#[path = "ledge_tests.rs"]
mod tests;
