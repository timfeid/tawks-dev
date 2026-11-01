//! Sumo: everyone stands on one round platform and shoves. Run into a rival to
//! send them skidding, and the faster you were going the further they go. The
//! platform keeps closing in, so a stalemate in the middle cannot last. Step
//! off the edge and you are out; the last one still up wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, Runner};

/// The platform for four; a bigger field gets a wider one, closing to a
/// wider last circle.
pub const ARENA_RADIUS: f32 = 11.0;
pub const ARENA_MIN_RADIUS: f32 = 2.9;
pub const SHRINK_FROM: f32 = 14.0;
pub const SHRINK_UNTIL: f32 = 58.0;
pub const ROUND_SECONDS: f32 = 75.0;
/// A touch closing slower than this just moves the two of them apart.
pub const SHOVE_FLOOR: f32 = 1.4;
/// What the speed you ran in with is worth to whoever takes it.
pub const SHOVE_GAIN: f32 = 1.15;
/// Seconds of lost footing per unit of speed a hit adds or takes away.
pub const STAGGER_PER_SPEED: f32 = 0.035;
pub const STAGGER_MAX: f32 = 0.38;

/// Skidding keeps far more of its speed than running does.
const SLIDE_DRAG: f32 = 1.1;
const GRAVITY: f32 = 22.0;
const OUT_DEPTH: f32 = -9.0;
const BUMP_EVENT_COOLDOWN: f32 = 0.12;
/// How far in from the rim a bot starts worrying about its own footing.
const EDGE_CARE: f32 = 1.9;
/// Where a bot lines up behind a mark before running through them.
const STAGE_BACK: f32 = 3.2;

#[derive(Clone, Debug)]
pub struct Fighter {
    pub seat: u8,
    pub run: Runner,
    pub y: f32,
    pub vy: f32,
    pub stagger: f32,
    pub facing: f32,
    pub fell_at: Option<f32>,
}

impl Fighter {
    pub fn standing(&self) -> bool {
        self.fell_at.is_none()
    }

    pub fn skidding(&self) -> bool {
        self.stagger > 0.0
    }
}

#[derive(Clone, Debug, Default)]
struct Brain {
    target: Option<u8>,
    think_in: f32,
    wobble: f32,
    pause: f32,
}

pub struct Sumo {
    /// Partners, when played in pairs: they bump but never shove each other off.
    allies: HashMap<u8, u8>,
    pub players: Vec<Fighter>,
    pub time: f32,
    pub radius: f32,
    /// How much wider than the platform for four, for the size of the field.
    size: f32,
    rng: StdRng,
    bumps: HashMap<(u8, u8), f32>,
    brains: HashMap<u8, Brain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl Sumo {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let size = crate::bout::crowd_scale(contestants.len());
        let spread = ground::ring(contestants.len().max(1), ARENA_RADIUS * size * 0.6, 0.35);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Fighter {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                y: 0.0,
                vy: 0.0,
                stagger: 0.0,
                facing: 0.0,
                fell_at: None,
            })
            .collect();
        Sumo {
            allies: contestants.iter().filter_map(|c| c.ally.map(|ally| (c.seat, ally))).collect(),
            players,
            time: 0.0,
            radius: ARENA_RADIUS * size,
            size,
            rng: StdRng::seed_from_u64(seed),
            bumps: HashMap::new(),
            brains: contestants
                .iter()
                .filter(|c| c.is_bot)
                .map(|c| (c.seat, Brain::default()))
                .collect(),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        }
    }

    pub fn radius_at(&self, time: f32) -> f32 {
        let closed = ((time - SHRINK_FROM) / (SHRINK_UNTIL - SHRINK_FROM)).clamp(0.0, 1.0);
        (ARENA_RADIUS + (ARENA_MIN_RADIUS - ARENA_RADIUS) * closed) * self.size
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    fn player(&self, seat: u8) -> Option<&Fighter> {
        self.players.iter().find(|p| p.seat == seat)
    }

    fn standing_count(&self) -> usize {
        self.players.iter().filter(|p| p.standing()).count()
    }

    fn walk(&mut self, inputs: &[(u8, Input)], dt: f32) {
        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let stick = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| input.stick)
                .unwrap_or_default();
            let player = &mut self.players[index];
            if !player.standing() {
                if player.y <= OUT_DEPTH {
                    continue;
                }
                player.vy -= GRAVITY * dt;
                player.y += player.vy * dt;
                let keep = (-0.6 * dt).exp();
                player.run.vx *= keep;
                player.run.vz *= keep;
                player.run.x += player.run.vx * dt;
                player.run.z += player.run.vz * dt;
                continue;
            }
            player.stagger = (player.stagger - dt).max(0.0);
            if player.skidding() {
                let keep = (-SLIDE_DRAG * dt).exp();
                player.run.vx *= keep;
                player.run.vz *= keep;
                player.run.x += player.run.vx * dt;
                player.run.z += player.run.vz * dt;
            } else {
                ground::step_runner(&mut player.run, stick, dt, ground::RUN_SPEED);
            }
            if player.run.speed() > 0.3 {
                player.facing = player.run.facing();
            }
        }
    }

    /// Two fighters meeting trade their closing speed: each is sent back at
    /// what the other ran in with, so a standing target takes the whole hit.
    fn resolve_shoves(&mut self, events: &mut Vec<Value>) {
        let reach = ground::RUNNER_RADIUS * 2.0;
        let standing: Vec<usize> = (0..self.players.len())
            .filter(|&i| self.players[i].standing())
            .collect();
        for (n, &i) in standing.iter().enumerate() {
            for &j in &standing[n + 1..] {
                let (a, b) = (self.players[i].run, self.players[j].run);
                let (mut dx, mut dz) = (b.x - a.x, b.z - a.z);
                let mut gap = (dx * dx + dz * dz).sqrt();
                if gap >= reach {
                    continue;
                }
                if gap < 1e-4 {
                    let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
                    (dx, dz, gap) = (angle.cos() * 1e-3, angle.sin() * 1e-3, 1e-3);
                }
                let (nx, nz) = (dx / gap, dz / gap);
                let apart = (reach - gap) / 2.0;
                // Partners only stand apart; there is no shove between them.
                let (sa, sb) = (self.players[i].seat, self.players[j].seat);
                if crate::bout::allied(self.allies.get(&sa).copied(), self.allies.get(&sb).copied()) {
                    self.players[i].run.x -= nx * apart;
                    self.players[i].run.z -= nz * apart;
                    self.players[j].run.x += nx * apart;
                    self.players[j].run.z += nz * apart;
                    continue;
                }
                let into_a = a.vx * nx + a.vz * nz;
                let into_b = -(b.vx * nx + b.vz * nz);
                let closing = into_a + into_b;
                let (away_a, away_b) = if closing > SHOVE_FLOOR {
                    (into_b * SHOVE_GAIN, into_a * SHOVE_GAIN)
                } else {
                    (-into_a, -into_b)
                };
                let (hit_a, hit_b) = (-away_a - into_a, away_b + into_b);
                {
                    let a = &mut self.players[i];
                    a.run.x -= nx * apart;
                    a.run.z -= nz * apart;
                    a.run.vx += nx * hit_a;
                    a.run.vz += nz * hit_a;
                    a.stagger = a.stagger.max(stagger_for(hit_a));
                }
                {
                    let b = &mut self.players[j];
                    b.run.x += nx * apart;
                    b.run.z += nz * apart;
                    b.run.vx += nx * hit_b;
                    b.run.vz += nz * hit_b;
                    b.stagger = b.stagger.max(stagger_for(hit_b));
                }
                if closing <= SHOVE_FLOOR {
                    continue;
                }
                let pair = (
                    self.players[i].seat.min(self.players[j].seat),
                    self.players[i].seat.max(self.players[j].seat),
                );
                if self.bumps.get(&pair).copied().unwrap_or(0.0) > 0.0 {
                    continue;
                }
                self.bumps.insert(pair, BUMP_EVENT_COOLDOWN);
                events.push(json!({
                    "kind": "bump",
                    "a": self.players[i].seat,
                    "b": self.players[j].seat,
                    "strength": (closing / (ground::RUN_SPEED * 1.5)).min(2.0),
                    "x": (a.x + b.x) / 2.0,
                    "z": (a.z + b.z) / 2.0,
                }));
            }
        }
    }

    fn drop_the_fallen(&mut self, events: &mut Vec<Value>) {
        let (radius, time) = (self.radius, self.time);
        for player in self.players.iter_mut() {
            if !player.standing() || player.run.distance_to(0.0, 0.0) <= radius {
                continue;
            }
            player.fell_at = Some(time);
            player.vy = 1.4;
            player.stagger = 0.0;
            events.push(json!({ "kind": "over", "seat": player.seat }));
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        if self.standing_count() > 1 && self.time < ROUND_SECONDS {
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

    /// Get on the inside of a mark, then run out through them.
    fn charge(&self, me: &Fighter, mark: &Fighter, wobble: f32) -> Stick {
        let here = (me.run.x, me.run.z);
        let lead = (
            mark.run.x + mark.run.vx * 0.28,
            mark.run.z + mark.run.vz * 0.28,
        );
        let out = (lead.0 * lead.0 + lead.1 * lead.1).sqrt();
        let aim = if out < 1.0 {
            ground::steer(here, lead)
        } else {
            let (ox, oz) = (lead.0 / out, lead.1 / out);
            let stage = (lead.0 - ox * STAGE_BACK, lead.1 - oz * STAGE_BACK);
            let run_in = ground::steer(here, lead);
            let lined_up = run_in.x * ox + run_in.z * oz;
            if lined_up > 0.5 || me.run.distance_to(lead.0, lead.1) < 2.4 {
                run_in
            } else {
                ground::steer(here, stage)
            }
        };
        let (cos, sin) = (wobble.cos(), wobble.sin());
        Stick {
            x: aim.x * cos - aim.z * sin,
            z: aim.x * sin + aim.z * cos,
        }
    }

    /// The inside of a shove is the winning side of it, so a fighter caught on
    /// the outside gives ground sideways instead of trading the hit.
    fn slip_inside(&self, me: &Fighter, rival: &Fighter) -> Stick {
        let off = ground::steer((rival.run.x, rival.run.z), (me.run.x, me.run.z));
        let inward = ground::steer((me.run.x, me.run.z), (0.0, 0.0));
        let (sx, sz) = (-off.z, off.x);
        let (sx, sz) = if sx * inward.x + sz * inward.z >= 0.0 {
            (sx, sz)
        } else {
            (-sx, -sz)
        };
        Stick {
            x: sx * 0.8 + inward.x * 0.6,
            z: sz * 0.8 + inward.z * 0.6,
        }
    }
}

/// A hit rattles whoever took it, shover included, so nobody can simply lean
/// on a rival and walk them off the edge.
fn stagger_for(hit: f32) -> f32 {
    (hit.abs() * STAGGER_PER_SPEED).min(STAGGER_MAX)
}

impl Bout for Sumo {
    fn key(&self) -> &'static str {
        "sumo"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "sumo",
            "rule": "Run into a rival to shove them off. The faster you go, the harder it lands.",
            "radius": ARENA_RADIUS * self.size,
            "min_radius": ARENA_MIN_RADIUS * self.size,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.radius = self.radius_at(self.time);
        for cooldown in self.bumps.values_mut() {
            *cooldown -= dt;
        }
        self.walk(inputs, dt);
        self.resolve_shoves(events);
        self.drop_the_fallen(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        if !me.standing() {
            return Input::default();
        }
        let radius = self.radius;
        let rivals: Vec<Fighter> = self
            .players
            .iter()
            .filter(|p| p.seat != seat && p.standing())
            .cloned()
            .collect();
        let skill = self.skill(seat);
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        brain.pause -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = self.rng.random_range(0.25..0.6) + skill.reaction() * 0.4;
            let wobble = skill.aim_error() * 0.7;
            brain.wobble = self.rng.random_range(-wobble..wobble);
            if self.rng.random_bool(skill.hesitation() * 0.4) {
                brain.pause = self.rng.random_range(0.2..0.6);
            }
            // Whoever is nearest the rim, with a nudge toward whoever is near.
            brain.target = rivals
                .iter()
                .map(|r| {
                    let rim = radius - r.run.distance_to(0.0, 0.0);
                    let near = me.run.distance_to(r.run.x, r.run.z);
                    (rim * 1.5 + near * 0.7, r.seat)
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, s)| s);
        }
        let here = (me.run.x, me.run.z);
        let out = me.run.distance_to(0.0, 0.0);
        let safe = (radius - EDGE_CARE).max(radius * 0.45);
        let pressing = rivals
            .iter()
            .filter(|r| {
                me.run.distance_to(r.run.x, r.run.z) < 3.0
                    && r.run.distance_to(0.0, 0.0) < out - 0.4
            })
            .min_by(|a, b| {
                me.run
                    .distance_to(a.run.x, a.run.z)
                    .partial_cmp(&me.run.distance_to(b.run.x, b.run.z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        let stick = if out > safe {
            ground::steer(here, (0.0, 0.0))
        } else if brain.pause > 0.0 {
            Stick::default()
        } else if let Some(rival) = pressing {
            self.slip_inside(&me, rival)
        } else {
            match brain
                .target
                .and_then(|t| rivals.iter().find(|r| r.seat == t))
            {
                Some(mark) => self.charge(&me, mark, brain.wobble),
                None => ground::steer(here, (0.0, 0.0)),
            }
        };
        self.brains.insert(seat, brain);
        let pace = skill.pace();
        Input::from_stick(
            Stick {
                x: stick.x * pace,
                z: stick.z * pace,
            }
            .clamped(),
        )
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "sumo",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "radius": round(self.radius),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "y": round(p.y),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "yaw": round(p.facing),
                "skidding": p.skidding(),
                "out": !p.standing(),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
