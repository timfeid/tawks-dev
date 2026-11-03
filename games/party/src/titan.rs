//! Titan Siege: everyone against one huge toadstool titan. It telegraphs a
//! slam, a sweep or a charge; get clear, then pile in while it is winded and
//! its core is open. Whoever lands the most hits takes the round, and if the
//! titan falls everyone did well.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, Runner};

pub const ARENA_RADIUS: f32 = 18.0;
pub const TITAN_RADIUS: f32 = 3.2;
pub const TITAN_HEALTH: i32 = 60;
pub const CORE_RANGE: f32 = 4.6;
pub const STRIKE_COOLDOWN: f32 = 0.55;
pub const DOWN_SECONDS: f32 = 2.5;
pub const ROUND_SECONDS: f32 = 100.0;
/// How long the core stays open after an attack.
pub const WINDED_SECONDS: f32 = 3.0;
/// The open core throbs on a beat; a strike on the throb is a critical.
pub const CORE_PERIOD: f32 = 0.8;
pub const CORE_WINDOW: f32 = 0.15;
pub const CRITICAL_DAMAGE: i32 = 2;
/// An off-beat swing glances off and leaves the raider off balance.
pub const GLANCE_COOLDOWN: f32 = 0.9;
const SLAM_RADIUS: f32 = 9.5;
const SWEEP_RANGE: f32 = 8.0;
const CHARGE_SPEED: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Stalk,
    Winding,
    Striking,
    Winded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attack {
    Slam,
    Sweep,
    Charge,
}

impl Attack {
    fn wind_up(self) -> f32 {
        match self {
            Attack::Slam => 1.3,
            Attack::Sweep => 1.0,
            Attack::Charge => 1.5,
        }
    }

    fn strike(self) -> f32 {
        match self {
            Attack::Slam => 0.5,
            Attack::Sweep => 0.45,
            Attack::Charge => 1.4,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Attack::Slam => "slam",
            Attack::Sweep => "sweep",
            Attack::Charge => "charge",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Raider {
    pub seat: u8,
    pub run: Runner,
    pub hits: u32,
    pub downs: u32,
    pub down_for: f32,
    pub cooldown: f32,
}

impl Raider {
    pub fn up(&self) -> bool {
        self.down_for <= 0.0
    }
}

pub struct TitanSiege {
    pub raiders: Vec<Raider>,
    pub x: f32,
    pub z: f32,
    pub facing: f32,
    pub health: i32,
    pub state: Move,
    pub attack: Attack,
    pub timer: f32,
    pub time: f32,
    /// The yard's radius, wider than the one built for four on a bigger field.
    pub radius: f32,
    /// How much it can take, more when more raiders are swinging at it.
    pub max_health: i32,
    /// How much wider than the yard for four, for the size of the field.
    size: f32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    target_offset: usize,
    bot_last_swing: std::collections::HashMap<u8, f32>,
    placements: Option<Vec<Vec<u8>>>,
}

impl TitanSiege {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let size = crate::bout::crowd_scale(contestants.len());
        let radius = ARENA_RADIUS * size;
        // Twice the raiders would fell it twice as fast, so it gets the
        // health of one titan per four of them and the siege runs as long.
        let max_health = crate::bout::crowd_count(TITAN_HEALTH as usize, contestants.len()) as i32;
        let mut placement_rng = StdRng::seed_from_u64(seed ^ 0x71_7A_4E);
        let turn = placement_rng.random_range(0.0..std::f32::consts::TAU);
        let spread = ground::ring(contestants.len().max(1), radius * 0.7, turn);
        let raiders = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Raider {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                hits: 0,
                downs: 0,
                down_for: 0.0,
                cooldown: 0.0,
            })
            .collect();
        let target_offset = placement_rng.random_range(0..contestants.len().max(1));
        TitanSiege {
            raiders,
            x: 0.0,
            z: 0.0,
            facing: 0.0,
            health: max_health,
            state: Move::Stalk,
            attack: Attack::Slam,
            timer: 2.0,
            time: 0.0,
            radius,
            max_health,
            size,
            rng: StdRng::seed_from_u64(seed),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            target_offset,
            bot_last_swing: Default::default(),
            placements: None,
        }
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    /// True while the core is open and a hit will land.
    pub fn open(&self) -> bool {
        self.state == Move::Winded
    }

    /// Where the open core is in its throb, 0 to 1 with the peak at a half.
    pub fn core_phase(&self, earlier: f32) -> f32 {
        ((WINDED_SECONDS - self.timer - earlier) / CORE_PERIOD).rem_euclid(1.0)
    }

    /// On the throb now, or within the last 150 ms of network delay.
    pub fn on_beat(&self) -> bool {
        (0..=3).any(|i| (self.core_phase(0.05 * i as f32) - 0.5).abs() <= CORE_WINDOW)
    }

    fn nearest_raider(&self) -> Option<&Raider> {
        let count = self.raiders.len();
        self.raiders
            .iter()
            .enumerate()
            .filter(|(_, raider)| raider.up())
            .min_by(|(ai, a), (bi, b)| {
                let near = |r: &Raider| (r.run.x - self.x).powi(2) + (r.run.z - self.z).powi(2);
                let (ad, bd) = (near(a), near(b));
                // The starting ring is equidistant; give each seat a fair
                // chance to be the first target instead of always picking 0.
                if (ad - bd).abs() < 0.01 {
                    let rank = |i: usize| (i + count - self.target_offset) % count;
                    rank(*ai).cmp(&rank(*bi))
                } else {
                    ad.total_cmp(&bd)
                }
            })
            .map(|(_, raider)| raider)
    }

    fn knock_down(&mut self, seat: u8, how: &str, events: &mut Vec<Value>) {
        let (tx, tz) = (self.x, self.z);
        if let Some(raider) = self.raiders.iter_mut().find(|r| r.seat == seat) {
            if !raider.up() {
                return;
            }
            raider.down_for = DOWN_SECONDS;
            raider.downs += 1;
            let (dx, dz) = (raider.run.x - tx, raider.run.z - tz);
            let away = (dx * dx + dz * dz).sqrt().max(0.01);
            raider.run.vx = dx / away * 12.0;
            raider.run.vz = dz / away * 12.0;
        }
        events.push(json!({ "kind": "floored", "seat": seat, "by": how }));
    }

    fn land_attack(&mut self, events: &mut Vec<Value>) {
        let caught: Vec<u8> = match self.attack {
            Attack::Slam => self
                .raiders
                .iter()
                .filter(|r| r.up() && r.run.distance_to(self.x, self.z) <= SLAM_RADIUS)
                .map(|r| r.seat)
                .collect(),
            Attack::Sweep => {
                let (fx, fz) = (-self.facing.sin(), -self.facing.cos());
                self.raiders
                    .iter()
                    .filter(|r| {
                        if !r.up() {
                            return false;
                        }
                        let (dx, dz) = (r.run.x - self.x, r.run.z - self.z);
                        let away = (dx * dx + dz * dz).sqrt().max(0.01);
                        away <= SWEEP_RANGE && (fx * dx + fz * dz) / away > 0.15
                    })
                    .map(|r| r.seat)
                    .collect()
            }
            Attack::Charge => self
                .raiders
                .iter()
                .filter(|r| r.up() && r.run.distance_to(self.x, self.z) <= TITAN_RADIUS + 1.6)
                .map(|r| r.seat)
                .collect(),
        };
        let how = self.attack.name();
        for seat in caught {
            self.knock_down(seat, how, events);
        }
    }

    fn pick_attack(&mut self) {
        let close = self
            .nearest_raider()
            .map(|r| r.run.distance_to(self.x, self.z))
            .unwrap_or(99.0);
        self.attack = if close < SWEEP_RANGE * 0.8 {
            if self.rng.random_bool(0.5) {
                Attack::Sweep
            } else {
                Attack::Slam
            }
        } else if close > 12.0 * self.size {
            Attack::Charge
        } else if self.rng.random_bool(0.5) {
            Attack::Slam
        } else {
            Attack::Charge
        };
    }

    fn step_titan(&mut self, dt: f32, events: &mut Vec<Value>) {
        self.timer -= dt;
        if let Some(mark) = self.nearest_raider() {
            let (dx, dz) = (mark.run.x - self.x, mark.run.z - self.z);
            let want = (-dx).atan2(-dz);
            let turn = if self.state == Move::Stalk { 1.6 } else { 0.4 };
            let mut delta = (want - self.facing) % std::f32::consts::TAU;
            if delta > std::f32::consts::PI {
                delta -= std::f32::consts::TAU;
            }
            if delta < -std::f32::consts::PI {
                delta += std::f32::consts::TAU;
            }
            self.facing += delta * (turn * dt).min(1.0);
        }

        match self.state {
            Move::Stalk => {
                // Lumber after the nearest raider until it is ready to swing.
                if let Some(mark) = self.nearest_raider().cloned() {
                    let step = ground::steer((self.x, self.z), (mark.run.x, mark.run.z));
                    self.x += step.x * 3.1 * dt;
                    self.z += step.z * 3.1 * dt;
                }
                if self.timer <= 0.0 {
                    self.pick_attack();
                    self.state = Move::Winding;
                    self.timer = self.attack.wind_up();
                    events.push(json!({ "kind": "wind_up", "attack": self.attack.name() }));
                }
            }
            Move::Winding => {
                if self.timer <= 0.0 {
                    self.state = Move::Striking;
                    self.timer = self.attack.strike();
                    events.push(json!({ "kind": "strike", "attack": self.attack.name(), "x": self.x, "z": self.z }));
                    if self.attack != Attack::Charge {
                        self.land_attack(events);
                    }
                }
            }
            Move::Striking => {
                if self.attack == Attack::Charge {
                    let (fx, fz) = (-self.facing.sin(), -self.facing.cos());
                    self.x += fx * CHARGE_SPEED * dt;
                    self.z += fz * CHARGE_SPEED * dt;
                    self.land_attack(events);
                }
                if self.timer <= 0.0 {
                    self.state = Move::Winded;
                    self.timer = WINDED_SECONDS;
                    events.push(json!({ "kind": "winded" }));
                }
            }
            Move::Winded => {
                if self.timer <= 0.0 {
                    self.state = Move::Stalk;
                    self.timer = self.rng.random_range(1.6..3.0);
                    events.push(json!({ "kind": "recovered" }));
                }
            }
        }

        let out = (self.x * self.x + self.z * self.z).sqrt();
        let edge = self.radius - TITAN_RADIUS;
        if out > edge && out > 0.01 {
            self.x = self.x / out * edge;
            self.z = self.z / out * edge;
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        if self.health > 0 && self.time < ROUND_SECONDS {
            return;
        }
        let mut rows: Vec<(u32, i64, u8)> = self
            .raiders
            .iter()
            .map(|r| (r.hits, -(r.downs as i64), r.seat))
            .collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, i64)> = None;
        for (hits, downs, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (hits, downs) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((hits, downs));
        }
        self.placements = Some(groups);
    }
}

impl Bout for TitanSiege {
    fn key(&self) -> &'static str {
        "titan"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "titan",
            "rule": "Dodge the wind-up, then strike the core on its throb while the titan is winded.",
            "radius": self.radius,
            "titan": TITAN_RADIUS,
            "health": self.max_health,
            "core": CORE_RANGE,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        let core_open = self.open();
        let on_beat = core_open && self.on_beat();
        let radius = self.radius;
        let (titan_x, titan_z) = (self.x, self.z);
        for index in 0..self.raiders.len() {
            let seat = self.raiders[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let raider = &mut self.raiders[index];
            raider.cooldown = (raider.cooldown - dt).max(0.0);
            if !raider.up() {
                raider.down_for -= dt;
                ground::step_runner(&mut raider.run, Stick::default(), dt, ground::RUN_SPEED);
                ground::hold_inside(&mut raider.run, radius);
                continue;
            }
            ground::step_runner(&mut raider.run, input.stick, dt, ground::RUN_SPEED);
            ground::hold_inside(&mut raider.run, radius);

            // Strike the core: close enough, swinging, and the titan is winded.
            let close = raider.run.distance_to(titan_x, titan_z) <= TITAN_RADIUS + CORE_RANGE;
            if input.fire && raider.cooldown <= 0.0 && close && core_open && !on_beat {
                raider.cooldown = GLANCE_COOLDOWN;
                events.push(json!({ "kind": "glance", "seat": seat }));
            } else if input.fire && raider.cooldown <= 0.0 && close && core_open {
                raider.cooldown = STRIKE_COOLDOWN;
                raider.hits += 1;
                let (rx, rz) = (raider.run.x, raider.run.z);
                self.health = (self.health - CRITICAL_DAMAGE).max(0);
                events.push(json!({
                    "kind": "strike_core",
                    "seat": seat,
                    "x": rx,
                    "z": rz,
                    "left": self.health,
                    "critical": true,
                }));
                if self.health == 0 {
                    events.push(json!({ "kind": "titan_down" }));
                }
            } else if input.fire && raider.cooldown <= 0.0 {
                raider.cooldown = STRIKE_COOLDOWN;
                events.push(json!({ "kind": "swing", "seat": seat }));
            }
        }

        let mut runners: Vec<Runner> = self.raiders.iter().map(|r| r.run).collect();
        ground::separate(&mut runners);
        for (raider, run) in self.raiders.iter_mut().zip(runners) {
            raider.run = run;
            ground::hold_inside(&mut raider.run, radius);
        }

        // Nobody walks through the titan.
        let (tx, tz) = (self.x, self.z);
        for raider in self.raiders.iter_mut() {
            let (dx, dz) = (raider.run.x - tx, raider.run.z - tz);
            let away = (dx * dx + dz * dz).sqrt();
            let reach = TITAN_RADIUS + ground::RUNNER_RADIUS;
            if away < reach && away > 0.01 {
                raider.run.x = tx + dx / away * reach;
                raider.run.z = tz + dz / away * reach;
            }
        }

        self.step_titan(dt, events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.raiders.iter().find(|r| r.seat == seat).cloned() else {
            return Input::default();
        };
        if !me.up() {
            return Input::default();
        }
        let here = (me.run.x, me.run.z);
        let away = me.run.distance_to(self.x, self.z);
        let toward = ground::steer(here, (self.x, self.z));
        let back = Stick {
            x: -toward.x,
            z: -toward.z,
        };
        let stick = match self.state {
            // Get out of range while it is loading up a swing.
            Move::Winding | Move::Striking => {
                // A slow reader is still ambling in when the swing lands.
                let reading = self.timer < self.attack.wind_up() - self.skill(seat).reaction();
                if away < SLAM_RADIUS + 1.5 && reading {
                    back
                } else {
                    Stick {
                        x: -toward.z,
                        z: toward.x,
                    }
                }
            }
            Move::Winded => {
                if away > TITAN_RADIUS + CORE_RANGE * 0.7 {
                    toward
                } else {
                    Stick::default()
                }
            }
            Move::Stalk => {
                if away < 9.0 {
                    back
                } else {
                    Stick {
                        x: -toward.z,
                        z: toward.x,
                    }
                }
            }
        };
        let skill = self.skill(seat);
        let on_throb = (self.core_phase(0.0) - 0.5).abs() <= CORE_WINDOW * 0.6;
        // One decision per throb gives a missed beat a real cost.
        let can_try = self.open()
            && away <= TITAN_RADIUS + CORE_RANGE
            && me.cooldown <= 0.0
            && on_throb
            && self.time - self.bot_last_swing.get(&seat).copied().unwrap_or(-10.0)
                >= CORE_PERIOD * 0.75;
        if can_try {
            self.bot_last_swing.insert(seat, self.time);
        }
        Input {
            stick: stick.clamped(),
            fire: can_try
                && !self
                    .rng
                    .random_bool(((1.0 - skill.level()) * 1.5).min(0.7) as f64),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "titan",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "titan": {
                "x": round(self.x),
                "z": round(self.z),
                "facing": round(self.facing),
                "health": self.health,
                "max_health": self.max_health,
                "state": match self.state {
                    Move::Stalk => "stalk",
                    Move::Winding => "winding",
                    Move::Striking => "striking",
                    Move::Winded => "winded",
                },
                "attack": self.attack.name(),
                "timer": round(self.timer.max(0.0)),
                "radius": TITAN_RADIUS,
                "core_phase": round(if self.open() { self.core_phase(0.0) } else { 0.0 }),
                "core_period": CORE_PERIOD,
                "core_window": CORE_WINDOW,
            },
            "players": self.raiders.iter().map(|r| json!({
                "seat": r.seat,
                "x": round(r.run.x),
                "z": round(r.run.z),
                "vx": round(r.run.vx),
                "vz": round(r.run.vz),
                "hits": r.hits,
                "downs": r.downs,
                "out": !r.up(),
                "ready": r.cooldown <= 0.0,
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raiders(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn opening_target_is_shared_across_seats() {
        let mut starts = [0; 4];
        for seed in 0..128 {
            let game = TitanSiege::new(&raiders(4), seed);
            let target = game.nearest_raider().expect("a raider").seat;
            starts[target as usize] += 1;
        }
        assert!(
            starts.iter().all(|count| (15..=50).contains(count)),
            "{starts:?}"
        );
    }

    #[test]
    fn a_bigger_siege_gets_a_wider_yard_and_a_tougher_titan() {
        let four = TitanSiege::new(&raiders(4), 1);
        assert_eq!(four.radius, ARENA_RADIUS, "four play it as built");
        assert_eq!(four.health, TITAN_HEALTH);
        let eight = TitanSiege::new(&raiders(8), 1);
        assert!(
            (eight.radius / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4,
            "eight get the room of two fours"
        );
        assert_eq!(
            eight.health,
            TITAN_HEALTH * 2,
            "and twice the titan to chop down"
        );
        assert!(
            eight
                .raiders
                .iter()
                .any(|r| r.run.distance_to(0.0, 0.0) > ARENA_RADIUS * 0.75)
        );
        assert_eq!(
            eight.setup()["radius"],
            json!(eight.radius),
            "and the client is told"
        );
        assert_eq!(eight.setup()["health"], json!(TITAN_HEALTH * 2));
        assert_eq!(
            eight.snapshot()["titan"]["max_health"],
            json!(TITAN_HEALTH * 2)
        );
    }
}
