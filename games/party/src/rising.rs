//! Rising Water: the arena floods from the bottom while a tower of floating
//! platforms climbs out of it. Jump from platform to platform to stay dry.
//! Every platform gives way a moment after someone lands on it, so nobody can
//! sit on the top. Last one above the water wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, Runner};

pub const KEY: &str = "rising";
pub const ARENA_RADIUS: f32 = 9.5;
pub const PAD_RADIUS: f32 = 1.7;
/// Height between one ring of platforms and the next, inside one jump.
pub const STEP: f32 = 1.5;
pub const LEVELS: usize = 16;
/// Platforms on each ring for four; a bigger field gets more of them.
pub const PADS_PER_LEVEL: usize = 4;
pub const GRAVITY: f32 = 30.0;
pub const JUMP_SPEED: f32 = 11.5;
/// How long a platform holds once someone has landed on it.
pub const CRUMBLE_SECONDS: f32 = 0.9;
pub const REGROW_SECONDS: f32 = 5.0;
/// Feet this far under the surface is under.
pub const WADE: f32 = 0.5;
pub const ROUND_SECONDS: f32 = 90.0;
const WATER_START: f32 = -2.0;
const WATER_SPEED: f32 = 0.2;
const WATER_ACCEL: f32 = 0.0022;
const PLAYER_HEIGHT: f32 = 1.0;

fn jump_speed(width: f32) -> f32 {
    JUMP_SPEED * (1.0 - (BodyProfile::new(width).width - 1.0) * 0.12)
}

fn pad_lifetime(width: f32) -> f32 {
    CRUMBLE_SECONDS / BodyProfile::new(width).mass.sqrt()
}

/// Where the water stands a given number of seconds in. It starts slow and
/// keeps speeding up, and passes the top of the tower before the clock ends.
pub fn water_at(time: f32) -> f32 {
    WATER_START + WATER_SPEED * time + WATER_ACCEL * time * time
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadState {
    Solid,
    Crumbling,
    Gone,
}

#[derive(Clone, Debug)]
pub struct Pad {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub state: PadState,
    pub timer: f32,
}

impl Pad {
    pub fn standable(&self) -> bool {
        self.state != PadState::Gone
    }

    fn under(&self, x: f32, z: f32, width: f32) -> bool {
        let stance = BodyProfile::new(width).radius;
        (self.x - x).hypot(self.z - z) <= PAD_RADIUS + (stance - 1.0) * 0.8
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    Air,
    Floor,
    Pad(usize),
}

#[derive(Clone, Debug)]
pub struct Climber {
    pub seat: u8,
    pub run: Runner,
    pub y: f32,
    pub vy: f32,
    pub footing: Footing,
    pub out_at: Option<f32>,
}

impl Climber {
    pub fn up(&self) -> bool {
        self.out_at.is_none()
    }
}

pub struct RisingWater {
    pub players: Vec<Climber>,
    pub pads: Vec<Pad>,
    pub time: f32,
    pub water: f32,
    /// How far the tank reaches, wider for a bigger field.
    pub radius: f32,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    react: HashMap<u8, f32>,
    falling: HashMap<u8, f32>,
    aims: HashMap<u8, usize>,
    placements: Option<Vec<Vec<u8>>>,
}

/// `per_level` platforms on every ring, set out `size` times as far from the
/// middle as the tower for four so the extra ones have room.
fn tower(rng: &mut StdRng, per_level: usize, size: f32) -> Vec<Pad> {
    let mut pads = Vec::new();
    for level in 1..=LEVELS {
        // Each ring turns a little from the one below, so the way up spirals.
        let turn = level as f32 * 0.5 + rng.random_range(-0.15..0.15);
        for index in 0..per_level {
            let angle = turn + index as f32 * std::f32::consts::TAU / per_level as f32;
            let out = rng.random_range(3.5..6.0) * size;
            pads.push(Pad {
                x: angle.cos() * out,
                y: level as f32 * STEP,
                z: angle.sin() * out,
                state: PadState::Solid,
                timer: 0.0,
            });
        }
    }
    pads
}

impl RisingWater {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let size = crate::bout::crowd_scale(contestants.len());
        let radius = ARENA_RADIUS * size;
        let pads = tower(
            &mut rng,
            crate::bout::crowd_count(PADS_PER_LEVEL, contestants.len()),
            size,
        );
        let spread = ground::ring(contestants.len().max(1), radius * 0.6, 0.5);
        RisingWater {
            players: contestants
                .iter()
                .enumerate()
                .map(|(index, one)| Climber {
                    seat: one.seat,
                    run: Runner::at(spread[index].0, spread[index].1).with_width(one.body_width),
                    y: 0.0,
                    vy: 0.0,
                    footing: Footing::Floor,
                    out_at: None,
                })
                .collect(),
            pads,
            time: 0.0,
            water: water_at(0.0),
            radius,
            rng,
            skills: contestants
                .iter()
                .map(|one| (one.seat, Skill(one.skill)))
                .collect(),
            react: HashMap::new(),
            falling: HashMap::new(),
            aims: HashMap::new(),
            placements: None,
        }
    }

    /// The highest platform caught by someone dropping from `from` to `to`.
    fn landing(&self, x: f32, z: f32, width: f32, from: f32, to: f32) -> Option<usize> {
        self.pads
            .iter()
            .enumerate()
            .filter(|(_, pad)| {
                pad.standable() && pad.under(x, z, width) && pad.y <= from + 0.05 && pad.y >= to
            })
            .max_by(|a, b| a.1.y.total_cmp(&b.1.y))
            .map(|(index, _)| index)
    }

    fn move_climber(&mut self, index: usize, input: Input, dt: f32) {
        let mut climber = self.players[index].clone();
        ground::step_runner(&mut climber.run, input.stick, dt, ground::RUN_SPEED);
        ground::hold_inside(&mut climber.run, self.radius);

        if let Footing::Pad(pad) = climber.footing
            && !(self.pads[pad].standable() && self.pads[pad].under(climber.run.x, climber.run.z, climber.run.width))
        {
            climber.footing = Footing::Air;
        }
        if (input.jump || input.fire) && climber.footing != Footing::Air {
            climber.footing = Footing::Air;
            climber.vy = jump_speed(climber.run.width);
        }
        if climber.footing == Footing::Air {
            climber.vy -= GRAVITY * dt;
            let next = climber.y + climber.vy * dt;
            let caught = if climber.vy <= 0.0 {
                self.landing(climber.run.x, climber.run.z, climber.run.width, climber.y, next)
            } else {
                None
            };
            if let Some(pad) = caught {
                climber.y = self.pads[pad].y;
                climber.vy = 0.0;
                climber.footing = Footing::Pad(pad);
            } else if next <= 0.0 {
                climber.y = 0.0;
                climber.vy = 0.0;
                climber.footing = Footing::Floor;
            } else {
                climber.y = next;
            }
        }
        self.players[index] = climber;
    }

    /// Players only push each other about when they are at the same height.
    fn separate(&mut self) {
        for i in 0..self.players.len() {
            for j in (i + 1)..self.players.len() {
                let (a, b) = (&self.players[i], &self.players[j]);
                if !a.up() || !b.up() || (a.y - b.y).abs() > PLAYER_HEIGHT {
                    continue;
                }
                let a_body = BodyProfile::new(a.run.width);
                let b_body = BodyProfile::new(b.run.width);
                let reach = ground::RUNNER_RADIUS * (a_body.radius + b_body.radius);
                let (dx, dz) = (b.run.x - a.run.x, b.run.z - a.run.z);
                let apart = dx.hypot(dz);
                if apart >= reach || apart < 1e-4 {
                    continue;
                }
                let push = reach - apart;
                let (nx, nz) = (dx / apart, dz / apart);
                let a_share = b_body.mass / (a_body.mass + b_body.mass);
                let b_share = a_body.mass / (a_body.mass + b_body.mass);
                self.players[i].run.x -= nx * push * a_share;
                self.players[i].run.z -= nz * push * a_share;
                self.players[j].run.x += nx * push * b_share;
                self.players[j].run.z += nz * push * b_share;
            }
        }
    }

    fn wear_pads(&mut self, dt: f32, events: &mut Vec<Value>) {
        for climber in self.players.iter().filter(|climber| climber.up()) {
            if let Footing::Pad(index) = climber.footing
            {
                let lifetime = pad_lifetime(climber.run.width);
                if self.pads[index].state == PadState::Solid {
                    self.pads[index].state = PadState::Crumbling;
                    self.pads[index].timer = lifetime;
                    events.push(json!({ "kind": "crack", "seat": climber.seat, "pad": index }));
                } else if self.pads[index].state == PadState::Crumbling {
                    self.pads[index].timer = self.pads[index].timer.min(lifetime);
                }
            }
        }
        for (index, pad) in self.pads.iter_mut().enumerate() {
            match pad.state {
                PadState::Solid => {}
                PadState::Crumbling => {
                    pad.timer -= dt;
                    if pad.timer <= 0.0 {
                        pad.state = PadState::Gone;
                        pad.timer = REGROW_SECONDS;
                        events.push(json!({ "kind": "crumble", "pad": index }));
                    }
                }
                PadState::Gone => {
                    pad.timer -= dt;
                    // A platform the water has already reached stays gone.
                    if pad.timer <= 0.0 && pad.y > self.water + WADE {
                        pad.state = PadState::Solid;
                        events.push(json!({ "kind": "regrow", "pad": index }));
                    }
                }
            }
        }
    }

    fn drown(&mut self, events: &mut Vec<Value>) {
        for climber in self.players.iter_mut() {
            if climber.up() && self.water > climber.y + WADE {
                climber.out_at = Some(self.time);
                climber.run.vx = 0.0;
                climber.run.vz = 0.0;
                climber.vy = 0.0;
                events.push(json!({ "kind": "dunked", "seat": climber.seat }));
            }
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let up = self.players.iter().filter(|climber| climber.up()).count();
        let last_standing = up == 0 || (up == 1 && self.players.len() > 1);
        if !last_standing && self.time < ROUND_SECONDS {
            return;
        }
        // Whoever is still dry beats everyone who went under, highest first.
        let mut order: Vec<(f32, u8)> = self
            .players
            .iter()
            .map(|climber| match climber.out_at {
                Some(at) => (at, climber.seat),
                None => (ROUND_SECONDS * 10.0 + climber.y, climber.seat),
            })
            .collect();
        order.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if (previous - at).abs() < 1e-3 => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }

    /// The highest platform still standing below a falling player, within
    /// drifting distance.
    fn catch_pad(&self, me: &Climber) -> Option<usize> {
        self.pads
            .iter()
            .enumerate()
            .filter(|(_, pad)| {
                pad.standable()
                    && pad.y < me.y - 0.05
                    && pad.y > self.water
                    && me.run.distance_to(pad.x, pad.z) <= PAD_RADIUS + 3.0
            })
            .max_by(|a, b| a.1.y.total_cmp(&b.1.y))
            .map(|(index, _)| index)
    }

    /// A platform worth heading for: the nearest standing one a level up, or
    /// failing that another on this level, inside a jump's reach. With
    /// nothing in reach, walk toward the nearest one up.
    fn next_pad(&self, me: &Climber) -> Option<usize> {
        let here = match me.footing {
            Footing::Pad(index) => Some(index),
            _ => None,
        };
        let pick = |rise: f32, reach: f32| {
            self.pads
                .iter()
                .enumerate()
                .filter(|(index, pad)| {
                    Some(*index) != here
                        && pad.state == PadState::Solid
                        && (pad.y - (me.y + rise)).abs() < 0.3
                        && me.run.distance_to(pad.x, pad.z) <= reach
                })
                .min_by(|a, b| {
                    me.run
                        .distance_to(a.1.x, a.1.z)
                        .total_cmp(&me.run.distance_to(b.1.x, b.1.z))
                })
                .map(|(index, _)| index)
        };
        let reach = PAD_RADIUS * 2.0 + 3.2;
        pick(STEP, reach)
            .or_else(|| pick(0.0, reach))
            .or_else(|| pick(STEP, f32::INFINITY))
    }
}

impl Bout for RisingWater {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "radius": self.radius,
            "pad": PAD_RADIUS,
            "step": STEP,
            "pads": self.pads.iter().map(|pad| json!({ "x": pad.x, "y": pad.y, "z": pad.z })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.water = water_at(self.time);

        for index in 0..self.players.len() {
            if !self.players[index].up() {
                self.players[index].y -= 3.0 * dt;
                continue;
            }
            let seat = self.players[index].seat;
            let input = inputs
                .iter()
                .find(|(one, _)| *one == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let was = self.players[index].footing;
            self.move_climber(index, input, dt);
            if was != Footing::Air
                && self.players[index].footing == Footing::Air
                && self.players[index].vy > 0.0
            {
                events.push(json!({ "kind": "jump", "seat": seat }));
            }
        }
        self.separate();
        let radius = self.radius;
        for climber in self.players.iter_mut().filter(|climber| climber.up()) {
            ground::hold_inside(&mut climber.run, radius);
        }
        self.wear_pads(dt, events);
        self.drown(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let skill = self.skills.get(&seat).copied().unwrap_or_default();
        let Some(me) = self.players.iter().find(|one| one.seat == seat).cloned() else {
            return Input::default();
        };
        if !me.up() {
            return Input::default();
        }
        let here = (me.run.x, me.run.z);

        if me.footing == Footing::Air {
            let aimed = self.aims.get(&seat).copied().filter(|index| {
                let pad = &self.pads[*index];
                pad.standable() && (me.vy > 0.0 || pad.y <= me.y + 0.05)
            });
            let aim = match aimed {
                Some(index) => Some(index),
                None => {
                    // Dropping with nothing under you: a sharp bot finds
                    // somewhere to land before it has fallen far.
                    let waited = self.falling.entry(seat).or_insert(skill.reaction() * 0.6);
                    *waited -= dt;
                    if *waited > 0.0 {
                        None
                    } else {
                        self.catch_pad(&me)
                    }
                }
            };
            let Some(index) = aim else {
                return Input::default();
            };
            self.aims.insert(seat, index);
            let pad = &self.pads[index];
            let away = me.run.distance_to(pad.x, pad.z);
            let stick = ground::steer(here, (pad.x, pad.z));
            let ease = (away / 1.2).min(1.0) * skill.pace();
            return Input::from_stick(Stick {
                x: stick.x * ease,
                z: stick.z * ease,
            });
        }
        self.falling.remove(&seat);

        // Landing takes a moment to register, longer for a clumsy bot.
        let waited = self.react.entry(seat).or_insert(skill.reaction() * 0.5);
        *waited -= dt;
        if *waited > 0.0 {
            return Input::default();
        }
        let Some(target) = self.next_pad(&me) else {
            return Input::default();
        };
        self.aims.insert(seat, target);
        let pad = self.pads[target].clone();
        let away = me.run.distance_to(pad.x, pad.z);
        // A sharp bot takes off from just the right spot; a clumsy one guesses.
        let wobble = (1.0 - skill.level()) * 1.4;
        let lead = PAD_RADIUS + 1.8 + self.rng.random_range(-wobble..=wobble);
        let jump = away <= lead.max(0.5) || (pad.y > me.y + 0.3 && away < PAD_RADIUS);
        if jump {
            self.react.remove(&seat);
        }
        let stick = ground::steer(here, (pad.x, pad.z));
        Input {
            stick: Stick {
                x: stick.x * skill.pace(),
                z: stick.z * skill.pace(),
            },
            jump,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "water": round(self.water),
            "players": self.players.iter().map(|climber| json!({
                "seat": climber.seat,
                "body_width": climber.run.width,
                "x": round(climber.run.x),
                "y": round(climber.y),
                "z": round(climber.run.z),
                "vx": round(climber.run.vx),
                "vz": round(climber.run.vz),
                "air": climber.footing == Footing::Air,
                "out": !climber.up(),
            })).collect::<Vec<_>>(),
            "pads": self.pads.iter().map(|pad| match pad.state {
                PadState::Solid => "solid",
                PadState::Crumbling => "crumbling",
                PadState::Gone => "gone",
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_a_wider_tank_and_more_platforms() {
        let four = RisingWater::new(&field(4), 1);
        assert_eq!(four.radius, ARENA_RADIUS, "four play it as built");
        assert_eq!(four.pads.len(), LEVELS * PADS_PER_LEVEL);
        assert!(four.pads.iter().all(|pad| pad.x.hypot(pad.z) <= 6.0));
        let eight = RisingWater::new(&field(8), 1);
        assert!((eight.radius / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4, "eight get the room of two fours");
        assert_eq!(eight.pads.len(), LEVELS * PADS_PER_LEVEL * 2, "and twice the platforms to land on");
        assert!(
            eight.pads.iter().all(|pad| pad.x.hypot(pad.z) + PAD_RADIUS < eight.radius),
            "all of them inside the tank"
        );
        let setup = eight.setup();
        assert_eq!(setup["radius"], json!(eight.radius), "and the client is told");
        assert_eq!(setup["pads"].as_array().map(|pads| pads.len()), Some(eight.pads.len()));
    }
    #[test]
    fn build_size_changes_jump_footing_pad_wear_and_shoves() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = RisingWater::new(&[small, broad], 4);
        assert!(jump_speed(0.7) > jump_speed(1.4));
        assert!(pad_lifetime(0.7) > pad_lifetime(1.4));
        let pad = &game.pads[0];
        assert!(!pad.under(pad.x + 1.75, pad.z, 0.7));
        assert!(pad.under(pad.x + 1.75, pad.z, 1.4));

        game.players[0].run.x = 0.0;
        game.players[0].run.z = 0.0;
        game.players[1].run.x = 0.5;
        game.players[1].run.z = 0.0;
        game.separate();
        assert!(game.players[0].run.x.abs() > game.players[1].run.x - 0.5);

        game.players[0].footing = Footing::Pad(0);
        game.players[1].footing = Footing::Pad(1);
        game.wear_pads(0.0, &mut Vec::new());
        assert!(game.pads[0].timer > game.pads[1].timer);
        assert!((game.snapshot()["players"][1]["body_width"].as_f64().unwrap() - 1.4).abs() < 0.001);
    }
    #[test]
    #[ignore = "manual body build balance audit"]
    fn body_build_balance_audit() {
        let mut wins = [0u32; 2];
        let mut ties = 0u32;
        for seed in 0..24 {
            let broad_seat = (seed % 2) as usize;
            let mut contestants = [Contestant::bot(0, 0.7), Contestant::bot(1, 0.7)];
            contestants[broad_seat].body_width = 1.4;
            contestants[1 - broad_seat].body_width = 0.7;
            let mut game = RisingWater::new(&contestants, seed);
            let mut events = Vec::new();
            for _ in 0..(ROUND_SECONDS as usize * 60 + 2) {
                let inputs = contestants.iter().map(|c|
                    (c.seat, game.bot_input(c.seat, 1.0 / 60.0))).collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut events);
                if game.placements().is_some() { break; }
            }
            let first = &game.placements().unwrap()[0];
            if first.len() == 2 { ties += 1; }
            else { wins[usize::from(first[0] != broad_seat as u8)] += 1; }
        }
        println!("Rising Water broad wins={} small wins={} ties={ties}", wins[0], wins[1]);
    }
}
