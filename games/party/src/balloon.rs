//! Balloon Brawl: three balloons each, tied to your back. Dash into someone to
//! burst one of theirs. Lose all three and you are out of the brawl.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::ground::{self, Runner};
use crate::bout::{Contestant, Input, Bout, Skill, Stick};

pub const ARENA_HALF: f32 = 13.0;
pub const BALLOONS: u8 = 4;
pub const DASH_SECONDS: f32 = 0.32;
pub const DASH_COOLDOWN: f32 = 2.0;
pub const DASH_SPEED: f32 = 2.6;
pub const POP_RANGE: f32 = 1.45;
/// A breather after being hit, so one dash cannot take two balloons.
pub const SAFE_SECONDS: f32 = 2.2;
pub const KNOCKBACK: f32 = 9.0;
pub const ROUND_SECONDS: f32 = 80.0;

#[derive(Clone, Debug)]
pub struct Brawler {
    pub seat: u8,
    pub run: Runner,
    pub balloons: u8,
    pub dash: f32,
    pub cooldown: f32,
    pub safe: f32,
    pub facing: f32,
    pub out_at: Option<f32>,
}

impl Brawler {
    pub fn playing(&self) -> bool {
        self.balloons > 0
    }

    pub fn dashing(&self) -> bool {
        self.dash > 0.0
    }
}

pub struct BalloonBrawl {
    pub players: Vec<Brawler>,
    pub time: f32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl BalloonBrawl {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let spread = ground::ring(contestants.len().max(1), ARENA_HALF * 0.85, 0.6);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Brawler {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                balloons: BALLOONS,
                dash: 0.0,
                cooldown: 0.0,
                safe: 0.0,
                facing: 0.0,
                out_at: None,
            })
            .collect();
        BalloonBrawl {
            players,
            time: 0.0,
            rng: StdRng::seed_from_u64(seed),
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

    fn player(&self, seat: u8) -> Option<&Brawler> {
        self.players.iter().find(|p| p.seat == seat)
    }

    /// A dash that catches someone bursts a balloon and shoves them away.
    fn resolve_dashes(&mut self, events: &mut Vec<Value>) {
        let dashers: Vec<(u8, f32, f32, f32, f32)> = self
            .players
            .iter()
            .filter(|p| p.playing() && p.dashing())
            .map(|p| (p.seat, p.run.x, p.run.z, p.run.vx, p.run.vz))
            .collect();
        for (seat, x, z, vx, vz) in dashers {
            let reach = POP_RANGE + ground::RUNNER_RADIUS;
            let hit = self
                .players
                .iter()
                .filter(|p| p.playing() && p.seat != seat && p.safe <= 0.0)
                .map(|p| {
                    (
                        p.seat,
                        ((p.run.x - x).powi(2) + (p.run.z - z).powi(2)).sqrt(),
                    )
                })
                .filter(|(_, away)| *away <= reach)
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let Some((mark, _)) = hit else { continue };
            let time = self.time;
            let mut popped = 0;
            let mut out = false;
            if let Some(target) = self.players.iter_mut().find(|p| p.seat == mark) {
                target.balloons = target.balloons.saturating_sub(1);
                target.safe = SAFE_SECONDS;
                let push = (vx * vx + vz * vz).sqrt().max(1.0);
                target.run.vx += vx / push * KNOCKBACK;
                target.run.vz += vz / push * KNOCKBACK;
                popped = target.balloons;
                if !target.playing() {
                    target.out_at = Some(time);
                    out = true;
                }
            }
            if let Some(hitter) = self.players.iter_mut().find(|p| p.seat == seat) {
                hitter.dash = 0.0;
            }
            events.push(json!({ "kind": "pop", "seat": mark, "by": seat, "left": popped }));
            if out {
                events.push(json!({ "kind": "burst", "seat": mark, "by": seat }));
            }
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let standing = self.players.iter().filter(|p| p.playing()).count();
        if standing > 1 && self.time < ROUND_SECONDS {
            return;
        }
        let mut order: Vec<(u8, f32, u8)> = self
            .players
            .iter()
            .map(|p| (p.balloons, p.out_at.unwrap_or(f32::INFINITY), p.seat))
            .collect();
        order.sort_by(|a, b| {
            (b.0, b.1)
                .partial_cmp(&(a.0, a.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u8, f32)> = None;
        for (balloons, at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev.0 == balloons && (prev.1 - at).abs() < 1e-3 => {
                    group.push(seat)
                }
                _ => groups.push(vec![seat]),
            }
            last = Some((balloons, at));
        }
        self.placements = Some(groups);
    }
}

impl Bout for BalloonBrawl {
    fn key(&self) -> &'static str {
        "balloon"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "balloon",
            "rule": "Dash into a rival to burst a balloon. Lose all three and you are out.",
            "half": ARENA_HALF,
            "balloons": BALLOONS,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let player = &mut self.players[index];
            player.cooldown = (player.cooldown - dt).max(0.0);
            player.safe = (player.safe - dt).max(0.0);
            player.dash = (player.dash - dt).max(0.0);
            if !player.playing() {
                continue;
            }
            if input.fire && player.cooldown <= 0.0 && !player.dashing() {
                player.dash = DASH_SECONDS;
                player.cooldown = DASH_COOLDOWN;
                let aim = if input.stick.x.abs() + input.stick.z.abs() > 0.1 {
                    input.stick
                } else {
                    Stick {
                        x: player.run.vx,
                        z: player.run.vz,
                    }
                }
                .clamped();
                let length = (aim.x * aim.x + aim.z * aim.z).sqrt().max(0.01);
                player.run.vx = aim.x / length * ground::RUN_SPEED * DASH_SPEED;
                player.run.vz = aim.z / length * ground::RUN_SPEED * DASH_SPEED;
            }
            let speed = if player.dashing() {
                ground::RUN_SPEED * DASH_SPEED
            } else {
                ground::RUN_SPEED
            };
            let stick = if player.dashing() {
                Stick::default()
            } else {
                input.stick
            };
            ground::step_runner(&mut player.run, stick, dt, speed);
            ground::hold_square(&mut player.run, ARENA_HALF);
            if player.run.speed() > 0.2 {
                player.facing = player.run.facing();
            }
        }

        let mut runners: Vec<Runner> = self.players.iter().map(|p| p.run).collect();
        ground::separate(&mut runners);
        for (player, run) in self.players.iter_mut().zip(runners) {
            if player.playing() {
                player.run = run;
                ground::hold_square(&mut player.run, ARENA_HALF);
            }
        }

        self.resolve_dashes(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        if !me.playing() {
            return Input::default();
        }
        let mark = self
            .players
            .iter()
            .filter(|p| p.playing() && p.seat != seat)
            .min_by(|a, b| {
                me.run
                    .distance_to(a.run.x, a.run.z)
                    .partial_cmp(&me.run.distance_to(b.run.x, b.run.z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();
        let Some(mark) = mark else {
            return Input::default();
        };
        let away = me.run.distance_to(mark.run.x, mark.run.z);
        let chase = ground::steer((me.run.x, me.run.z), (mark.run.x, mark.run.z));
        // Back off while the dash recharges, then commit when close.
        let stick = if me.cooldown > 0.4 && away < 4.0 {
            Stick {
                x: -chase.x,
                z: -chase.z,
            }
        } else {
            chase
        };
        Input {
            stick,
            fire: me.cooldown <= 0.0
                && away < POP_RANGE + 2.2
                && self
                    .rng
                    .random_bool(0.004 + self.skill(seat).level() as f64 * 0.03),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "balloon",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "yaw": round(p.facing),
                "balloons": p.balloons,
                "dashing": p.dashing(),
                "safe": p.safe > 0.0,
                "ready": p.cooldown <= 0.0,
                "out": !p.playing(),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
