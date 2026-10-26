//! Ledge Shuffle: pedestals over a long drop, always one fewer than there are
//! players. Roam while the horn is quiet; when it sounds, get on a ledge. Miss
//! out and you go over the side, and another ledge sinks for the next round.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::ground::{self, Runner};
use crate::bout::{Contestant, Input, Bout, Skill, Stick};

pub const ARENA_RADIUS: f32 = 15.0;
pub const LEDGE_RADIUS: f32 = 1.35;
pub const RING_RADIUS: f32 = 8.5;
pub const ROAM_MIN: f32 = 4.5;
pub const ROAM_MAX: f32 = 9.0;
/// How long you have to find a ledge once the horn goes.
pub const SCRAMBLE_SECONDS: f32 = 3.0;
pub const ROUND_SECONDS: f32 = 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Roam,
    Scramble,
    Settle,
}

#[derive(Clone, Debug)]
pub struct Ledge {
    pub x: f32,
    pub z: f32,
    pub taken: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Shuffler {
    pub seat: u8,
    pub run: Runner,
    pub y: f32,
    pub out_at: Option<f32>,
    pub safe: bool,
}

impl Shuffler {
    pub fn playing(&self) -> bool {
        self.out_at.is_none()
    }
}

pub struct LedgeShuffle {
    pub players: Vec<Shuffler>,
    pub ledges: Vec<Ledge>,
    pub phase: Phase,
    pub timer: f32,
    pub time: f32,
    pub round: u32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    react: std::collections::HashMap<u8, f32>,
    placements: Option<Vec<Vec<u8>>>,
}

impl LedgeShuffle {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let spread = ground::ring(contestants.len().max(1), RING_RADIUS * 1.5, 0.3);
        let players: Vec<Shuffler> = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Shuffler {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                y: 0.0,
                out_at: None,
                safe: false,
            })
            .collect();
        let ledges = Self::lay_ledges(players.len().saturating_sub(1).max(1));
        let timer = rng.random_range(ROAM_MIN..ROAM_MAX);
        LedgeShuffle {
            players,
            ledges,
            phase: Phase::Roam,
            timer,
            time: 0.0,
            round: 1,
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            react: Default::default(),
            placements: None,
        }
    }

    fn lay_ledges(count: usize) -> Vec<Ledge> {
        ground::ring(count, RING_RADIUS, 0.25)
            .into_iter()
            .map(|(x, z)| Ledge { x, z, taken: None })
            .collect()
    }

    fn still_in(&self) -> Vec<u8> {
        self.players
            .iter()
            .filter(|p| p.playing())
            .map(|p| p.seat)
            .collect()
    }

    /// The first player to reach a free ledge claims it for this round.
    fn claim_ledges(&mut self, events: &mut Vec<Value>) {
        for index in 0..self.ledges.len() {
            if self.ledges[index].taken.is_some() {
                continue;
            }
            let (lx, lz) = (self.ledges[index].x, self.ledges[index].z);
            let standing = self
                .players
                .iter()
                .filter(|p| p.playing() && !p.safe)
                .map(|p| (p.seat, p.run.distance_to(lx, lz)))
                .filter(|(_, away)| *away <= LEDGE_RADIUS)
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            if let Some((seat, _)) = standing {
                self.ledges[index].taken = Some(seat);
                if let Some(player) = self.players.iter_mut().find(|p| p.seat == seat) {
                    player.safe = true;
                }
                events.push(json!({ "kind": "claim", "seat": seat, "x": lx, "z": lz }));
            }
        }
    }

    fn end_scramble(&mut self, events: &mut Vec<Value>) {
        let time = self.time;
        let mut lost: Vec<u8> = Vec::new();
        for player in self.players.iter_mut().filter(|p| p.playing()) {
            if !player.safe {
                player.out_at = Some(time);
                player.run.vx = 0.0;
                player.run.vz = 0.0;
                lost.push(player.seat);
            }
        }
        for seat in lost {
            events.push(json!({ "kind": "over", "seat": seat }));
        }
        self.phase = Phase::Settle;
        self.timer = 1.4;
    }

    fn next_round(&mut self, events: &mut Vec<Value>) {
        let left = self.still_in().len();
        self.round += 1;
        self.ledges = Self::lay_ledges(left.saturating_sub(1).max(1));
        for player in self.players.iter_mut() {
            player.safe = false;
        }
        self.phase = Phase::Roam;
        self.timer = self.rng.random_range(ROAM_MIN..ROAM_MAX);
        events.push(json!({ "kind": "music", "round": self.round, "ledges": self.ledges.len() }));
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

impl Bout for LedgeShuffle {
    fn key(&self) -> &'static str {
        "ledge"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "ledge",
            "rule": "When the horn sounds, get on a ledge. There is always one short.",
            "radius": ARENA_RADIUS,
            "ledge": LEDGE_RADIUS,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.timer -= dt;

        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let stick = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| input.stick)
                .unwrap_or_default();
            let player = &mut self.players[index];
            if !player.playing() {
                player.y -= 12.0 * dt;
                continue;
            }
            ground::step_runner(&mut player.run, stick, dt, ground::RUN_SPEED);
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

        match self.phase {
            Phase::Roam => {
                if self.timer <= 0.0 {
                    self.phase = Phase::Scramble;
                    self.timer = SCRAMBLE_SECONDS;
                    events.push(json!({ "kind": "horn", "ledges": self.ledges.len() }));
                }
            }
            Phase::Scramble => {
                self.claim_ledges(events);
                let everyone_safe = !self.players.iter().any(|p| p.playing() && !p.safe);
                if self.timer <= 0.0 || everyone_safe {
                    self.end_scramble(events);
                }
            }
            Phase::Settle => {
                if self.timer <= 0.0 {
                    if self.still_in().len() > 1 {
                        self.next_round(events);
                    } else {
                        self.settle();
                    }
                }
            }
        }

        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let skill = self.skills.get(&seat).copied().unwrap_or_default();
        let Some(me) = self.players.iter().find(|p| p.seat == seat).cloned() else {
            return Input::default();
        };
        if !me.playing() || me.safe {
            return Input::default();
        }
        let here = (me.run.x, me.run.z);
        let free = self
            .ledges
            .iter()
            .filter(|ledge| ledge.taken.is_none())
            .min_by(|a, b| {
                me.run
                    .distance_to(a.x, a.z)
                    .partial_cmp(&me.run.distance_to(b.x, b.z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let stick = match self.phase {
            // Circling near the ring beats standing still when the horn goes.
            Phase::Roam => {
                self.react.insert(seat, skill.reaction() * 0.8);
                let out = me.run.distance_to(0.0, 0.0).max(0.01);
                let drift = Stick {
                    x: -me.run.z / out,
                    z: me.run.x / out,
                };
                let pull = ground::steer(here, (0.0, 0.0));
                let far = (out / RING_RADIUS - 1.0).clamp(-1.0, 1.0);
                Stick {
                    x: drift.x * 0.7 + pull.x * far,
                    z: drift.z * 0.7 + pull.z * far,
                }
            }
            Phase::Scramble => {
                // The horn takes a moment to register.
                let waited = self.react.entry(seat).or_insert(skill.reaction() * 0.8);
                *waited -= dt;
                match free {
                    Some(ledge) if *waited <= 0.0 => ground::steer(here, (ledge.x, ledge.z)),
                    _ => Stick::default(),
                }
            }
            Phase::Settle => Stick::default(),
        };
        Input {
            stick: stick.clamped(),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "ledge",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "phase": match self.phase {
                Phase::Roam => "roam",
                Phase::Scramble => "scramble",
                Phase::Settle => "settle",
            },
            "timer": round(self.timer.max(0.0)),
            "round": self.round,
            "ledges": self.ledges.iter().map(|ledge| json!({
                "x": ledge.x,
                "z": ledge.z,
                "taken": ledge.taken,
            })).collect::<Vec<_>>(),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "y": round(p.y),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "safe": p.safe,
                "out": !p.playing(),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
