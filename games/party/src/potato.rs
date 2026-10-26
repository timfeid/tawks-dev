//! Hot Potato Panic: someone is holding a lit bomb. Touch a rival to hand it
//! over; whoever has it when the fuse runs out is knocked out and a new bomb
//! is lit. Carrying it makes you quicker, so nobody is ever safe for long.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::ground::{self, Runner};
use crate::bout::{Contestant, Input, Bout, Skill, Stick};

pub const ARENA_RADIUS: f32 = 13.0;
pub const PASS_RANGE: f32 = 1.5;
/// A moment of grace after a pass so the bomb does not bounce straight back.
pub const PASS_LOCK: f32 = 0.85;
pub const CARRIER_SPEED: f32 = 1.16;
pub const FUSE_MIN: f32 = 7.0;
pub const FUSE_MAX: f32 = 13.0;
pub const RELIGHT_SECONDS: f32 = 2.0;
pub const ROUND_SECONDS: f32 = 95.0;

#[derive(Clone, Debug)]
pub struct Carrier {
    pub seat: u8,
    pub run: Runner,
    pub out_at: Option<f32>,
}

impl Carrier {
    pub fn playing(&self) -> bool {
        self.out_at.is_none()
    }
}

pub struct HotPotato {
    pub players: Vec<Carrier>,
    pub holder: Option<u8>,
    pub fuse: f32,
    pub time: f32,
    lock: f32,
    relight: f32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl HotPotato {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let spread = ground::ring(contestants.len().max(1), ARENA_RADIUS * 0.6, 0.2);
        let players: Vec<Carrier> = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Carrier {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                out_at: None,
            })
            .collect();
        let mut rng = StdRng::seed_from_u64(seed);
        let holder = players
            .get(rng.random_range(0..players.len().max(1)) % players.len().max(1))
            .map(|p| p.seat);
        let fuse = rng.random_range(FUSE_MIN..FUSE_MAX);
        HotPotato {
            players,
            holder,
            fuse,
            time: 0.0,
            lock: PASS_LOCK,
            relight: 0.0,
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        }
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    pub fn holder_seat(&self) -> Option<u8> {
        self.holder
    }

    fn player(&self, seat: u8) -> Option<&Carrier> {
        self.players.iter().find(|p| p.seat == seat)
    }

    fn still_in(&self) -> Vec<u8> {
        self.players
            .iter()
            .filter(|p| p.playing())
            .map(|p| p.seat)
            .collect()
    }

    /// Hand the bomb to whoever the carrier just ran into.
    fn try_pass(&mut self, events: &mut Vec<Value>) {
        let Some(holder) = self.holder else { return };
        if self.lock > 0.0 {
            return;
        }
        let Some(carrier) = self.player(holder).cloned() else {
            return;
        };
        let caught = self
            .players
            .iter()
            .filter(|p| p.playing() && p.seat != holder)
            .map(|p| (p.seat, carrier.run.distance_to(p.run.x, p.run.z)))
            .filter(|(_, away)| *away <= PASS_RANGE + ground::RUNNER_RADIUS)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((seat, _)) = caught {
            self.holder = Some(seat);
            self.lock = PASS_LOCK;
            events.push(json!({ "kind": "pass", "from": holder, "to": seat, "fuse": self.fuse }));
        }
    }

    fn blow_up(&mut self, events: &mut Vec<Value>) {
        let Some(holder) = self.holder.take() else {
            return;
        };
        let time = self.time;
        let at = self
            .player(holder)
            .map(|p| (p.run.x, p.run.z))
            .unwrap_or((0.0, 0.0));
        if let Some(player) = self.players.iter_mut().find(|p| p.seat == holder) {
            player.out_at = Some(time);
            player.run.vx = 0.0;
            player.run.vz = 0.0;
        }
        events.push(json!({ "kind": "boom", "seat": holder, "x": at.0, "z": at.1 }));
        self.relight = RELIGHT_SECONDS;
    }

    fn relight_bomb(&mut self, events: &mut Vec<Value>) {
        let left = self.still_in();
        if left.len() < 2 {
            return;
        }
        let seat = left[self.rng.random_range(0..left.len())];
        self.holder = Some(seat);
        self.fuse = self.rng.random_range(FUSE_MIN..FUSE_MAX);
        self.lock = PASS_LOCK;
        events.push(json!({ "kind": "light", "seat": seat }));
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let left = self.still_in();
        if left.len() > 1 && self.time < ROUND_SECONDS {
            return;
        }
        let mut order: Vec<(f32, u8)> = self
            .players
            .iter()
            .map(|p| (p.out_at.unwrap_or(f32::INFINITY), p.seat))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if (prev - at).abs() < 1e-3 => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }
}

impl Bout for HotPotato {
    fn key(&self) -> &'static str {
        "hotpotato"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "hotpotato",
            "rule": "Touch someone to pass the bomb. Holding it when the fuse ends puts you out.",
            "radius": ARENA_RADIUS,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.lock = (self.lock - dt).max(0.0);

        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let carrying = self.holder == Some(seat);
            let stick = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| input.stick)
                .unwrap_or_default();
            let player = &mut self.players[index];
            if !player.playing() {
                continue;
            }
            let speed = ground::RUN_SPEED * if carrying { CARRIER_SPEED } else { 1.0 };
            ground::step_runner(&mut player.run, stick, dt, speed);
            ground::hold_inside(&mut player.run, ARENA_RADIUS);
        }
        let mut runners: Vec<Runner> = self.players.iter().map(|p| p.run).collect();
        ground::separate(&mut runners);
        for (player, run) in self.players.iter_mut().zip(runners) {
            if player.playing() {
                player.run = run;
                ground::hold_inside(&mut player.run, ARENA_RADIUS);
            }
        }

        if self.holder.is_some() {
            self.try_pass(events);
            self.fuse -= dt;
            if self.fuse <= 0.0 {
                self.blow_up(events);
            }
        } else if self.relight > 0.0 {
            self.relight -= dt;
            if self.relight <= 0.0 {
                self.relight_bomb(events);
            }
        }

        self.settle();
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        if !me.playing() {
            return Input::default();
        }
        let here = (me.run.x, me.run.z);
        let stick = if self.holder == Some(seat) {
            // Carrying: run down whoever is closest.
            let mark = self
                .players
                .iter()
                .filter(|p| p.playing() && p.seat != seat)
                .min_by(|a, b| {
                    me.run
                        .distance_to(a.run.x, a.run.z)
                        .partial_cmp(&me.run.distance_to(b.run.x, b.run.z))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            match mark {
                Some(mark) => ground::steer(here, (mark.run.x, mark.run.z)),
                None => Stick::default(),
            }
        } else if let Some(holder) = self.holder.and_then(|h| self.player(h)) {
            // Not carrying: put distance between you and the bomb, but stay
            // inside the arena rather than hugging the wall.
            let away = ground::steer((holder.run.x, holder.run.z), here);
            let pull_in = ground::steer(here, (0.0, 0.0));
            let weight = (me.run.distance_to(0.0, 0.0) / ARENA_RADIUS)
                .clamp(0.0, 1.0)
                .powi(2);
            Stick {
                x: away.x * (1.0 - weight) + pull_in.x * weight,
                z: away.z * (1.0 - weight) + pull_in.z * weight,
            }
        } else {
            Stick::default()
        };
        let pace = self.skill(seat).pace();
        Input {
            stick: Stick {
                x: stick.x * pace,
                z: stick.z * pace,
            }
            .clamped(),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "hotpotato",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "holder": self.holder,
            "fuse": round(self.fuse.max(0.0)),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "out": !p.playing(),
                "holding": self.holder == Some(p.seat),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
