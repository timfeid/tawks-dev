//! Rocker Rally: pump a spring racer by alternating left and right, steer for
//! boost pads, and stay clear of mud. First over the line wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Contestant, Input, Bout, Skill};

pub const TRACK_HALF: f32 = 11.0;
pub const START_Z: f32 = -38.0;
pub const FINISH_Z: f32 = 38.0;
pub const ROUND_SECONDS: f32 = 55.0;
const MIN_PUMP_GAP: f32 = 0.075;
const MAX_PUMP_GAP: f32 = 0.72;
const PUMP_SPEED: f32 = 2.25;
const MAX_SPEED: f32 = 13.5;
const COAST_DRAG: f32 = 1.45;
const STEER_SPEED: f32 = 9.0;

#[derive(Clone, Debug)]
pub struct Racer {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub speed: f32,
    pub bounce: f32,
    pub finished_at: Option<f32>,
    pump_side: i8,
    last_pump: f32,
    boost_mask: u8,
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    side: i8,
    pump_in: f32,
    lane: f32,
}

pub struct RockerRally {
    pub racers: Vec<Racer>,
    pub time: f32,
    brains: HashMap<u8, BotBrain>,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

pub const BOOSTS: &[(f32, f32)] = &[(-5.5, -17.0), (5.5, -2.0), (-4.0, 15.0), (4.5, 29.0)];
const MUD: &[(f32, f32, f32)] = &[(0.0, -9.0, 3.0), (-5.0, 7.0, 2.6), (5.0, 23.0, 2.4)];

impl RockerRally {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let racers = contestants
            .iter()
            .enumerate()
            .map(|(index, contestant)| Racer {
                seat: contestant.seat,
                x: ((index as f32 + 0.5) / count as f32 - 0.5) * 14.0,
                z: START_Z,
                speed: 0.0,
                bounce: 0.0,
                finished_at: None,
                pump_side: 0,
                last_pump: -10.0,
                boost_mask: 0,
            })
            .collect();
        let brains = contestants
            .iter()
            .filter(|c| c.is_bot)
            .map(|c| {
                (
                    c.seat,
                    BotBrain {
                        side: -1,
                        pump_in: 0.05,
                        ..Default::default()
                    },
                )
            })
            .collect();
        RockerRally {
            racers,
            time: 0.0,
            brains,
            rng: StdRng::seed_from_u64(seed),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        }
    }

    fn input<'a>(inputs: &'a [(u8, Input)], seat: u8) -> Input {
        inputs
            .iter()
            .find(|(s, _)| *s == seat)
            .map(|(_, input)| *input)
            .unwrap_or_default()
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        if self.time < ROUND_SECONDS && self.racers.iter().any(|r| r.finished_at.is_none()) {
            return;
        }
        let mut ranking: Vec<(f32, u8)> = self
            .racers
            .iter()
            .map(|r| {
                // Finished racers always lead; unfinished racers rank by distance.
                (r.finished_at.map(|at| 10_000.0 - at).unwrap_or(r.z), r.seat)
            })
            .collect();
        ranking.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        self.placements = Some(ranking.into_iter().map(|(_, seat)| vec![seat]).collect());
    }
}

impl Bout for RockerRally {
    fn key(&self) -> &'static str {
        "race"
    }

    fn setup(&self) -> Value {
        json!({ "game": "race", "half": TRACK_HALF, "start": START_Z, "finish": FINISH_Z, "boosts": BOOSTS, "mud": MUD })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        let mut finish_place = self
            .racers
            .iter()
            .filter(|r| r.finished_at.is_some())
            .count();
        for racer in &mut self.racers {
            if racer.finished_at.is_some() {
                continue;
            }
            let input = Self::input(inputs, racer.seat);
            let side = if input.stick.x > 0.45 {
                1
            } else if input.stick.x < -0.45 {
                -1
            } else {
                0
            };
            let since = self.time - racer.last_pump;
            if side != 0 && side != racer.pump_side {
                // The first press establishes a side. Every following alternating
                // press within the rhythm window gives the racer a shove.
                if racer.last_pump < 0.0 {
                    racer.last_pump = self.time;
                } else if since >= MIN_PUMP_GAP && since <= MAX_PUMP_GAP {
                    racer.speed = (racer.speed + PUMP_SPEED).min(MAX_SPEED);
                    racer.bounce = 1.0;
                    racer.last_pump = self.time;
                    events.push(json!({ "kind": "pump", "seat": racer.seat, "side": side }));
                } else if since > MAX_PUMP_GAP {
                    // Missing a beat should cost momentum, not make the
                    // racer permanently unresponsive. This press starts a
                    // fresh left/right rhythm.
                    racer.last_pump = self.time;
                }
            }
            if side != 0 {
                racer.pump_side = side;
            }
            racer.x = (racer.x + input.stick.z.clamp(-1.0, 1.0) * STEER_SPEED * dt)
                .clamp(-TRACK_HALF + 0.7, TRACK_HALF - 0.7);
            for (index, &(x, z)) in BOOSTS.iter().enumerate() {
                if racer.boost_mask & (1 << index) == 0
                    && (racer.z - z).abs() < 0.75
                    && (racer.x - x).abs() < 2.0
                {
                    racer.boost_mask |= 1 << index;
                    racer.speed = (racer.speed + 4.2).min(MAX_SPEED + 2.0);
                    events.push(json!({ "kind": "boost", "seat": racer.seat, "x": x, "z": z }));
                }
            }
            let muddy = MUD.iter().any(|&(x, z, radius)| {
                (racer.x - x).powi(2) + (racer.z - z).powi(2) < radius.powi(2)
            });
            racer.speed *= (-(COAST_DRAG + if muddy { 3.2 } else { 0.0 }) * dt).exp();
            racer.z += racer.speed * dt;
            racer.bounce = (racer.bounce - dt * 4.0).max(0.0);
            if racer.z >= FINISH_Z {
                racer.z = FINISH_Z;
                racer.finished_at = Some(self.time);
                finish_place += 1;
                events.push(json!({ "kind": "finish", "seat": racer.seat, "place": finish_place }));
            }
        }
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let brain = self.brains.entry(seat).or_default();
        brain.pump_in -= dt;
        if brain.pump_in <= 0.0 {
            brain.side = if brain.side > 0 { -1 } else { 1 };
            let skill = self.skills.get(&seat).copied().unwrap_or_default();
            brain.pump_in = self.rng.random_range(0.12..0.21) + (1.0 - skill.level()) * 0.16;
        }
        let racer = self.racers.iter().find(|r| r.seat == seat);
        if let Some(racer) = racer {
            if self.rng.random_bool((dt * 0.9).min(0.15) as f64) {
                brain.lane = BOOSTS
                    .iter()
                    .min_by(|a, b| {
                        (a.1 - racer.z)
                            .abs()
                            .partial_cmp(&(b.1 - racer.z).abs())
                            .unwrap()
                    })
                    .map(|p| p.0)
                    .unwrap_or(0.0);
            }
            Input {
                stick: crate::bout::Stick {
                    x: brain.side as f32,
                    z: ((brain.lane - racer.x) / 4.0).clamp(-1.0, 1.0),
                },
                ..Default::default()
            }
        } else {
            Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        json!({ "game": "race", "time": self.time, "limit": ROUND_SECONDS, "players": self.racers.iter().map(|r| json!({ "seat": r.seat, "x": r.x, "z": r.z, "speed": r.speed, "bounce": r.bounce, "finished": r.finished_at.is_some() })).collect::<Vec<_>>() })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
