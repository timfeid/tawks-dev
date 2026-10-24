//! Bumper Balls: everyone rides a ball on a round platform over the water.
//! Bump the others off; the last one standing wins. The platform starts to
//! shrink partway through so a round always ends.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use tawks_bout::{Bout, BoutSpec, Contestant, Controls, Format, Input, Skill, Stick};

pub const SPEC: BoutSpec = BoutSpec {
    key: "bumper",
    name: "Bumper Balls",
    rules: "Roll, bump, and be the last one above the water.",
    controls: Controls::Move,
    format: Format::FreeForAll,
    min_players: 2,
    max_players: 4,
    create: |contestants, seed| Box::new(BumperBalls::new(contestants, seed)),
};

pub const ARENA_RADIUS: f32 = 9.0;
pub const ARENA_MIN_RADIUS: f32 = 5.0;
pub const BALL_RADIUS: f32 = 0.85;
pub const ROUND_SECONDS: f32 = 60.0;
pub const SHRINK_START_SECONDS: f32 = 25.0;
pub const WATER_Y: f32 = -3.0;

const ACCEL: f32 = 16.0;
const MAX_SPEED: f32 = 7.0;
const COAST_DRAG: f32 = 1.4;
const ROLL_DRAG: f32 = 0.3;
const OVERSPEED_DRAG: f32 = 2.6;
const RESTITUTION: f32 = 1.3;
const MIN_BUMP_IMPULSE: f32 = 2.2;
const BUMP_EVENT_COOLDOWN: f32 = 0.15;
const GRAVITY: f32 = 22.0;
const OUT_DEPTH: f32 = -9.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallState {
    Rolling,
    Falling,
    Out,
}

#[derive(Clone, Debug)]
pub struct Ball {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub y: f32,
    pub vy: f32,
    pub state: BallState,
    pub fell_at: Option<f32>,
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    target: Option<u8>,
    think_in: f32,
    wobble: f32,
    idle_for: f32,
}

pub struct BumperBalls {
    pub balls: Vec<Ball>,
    pub time: f32,
    pub radius: f32,
    rng: StdRng,
    cooldowns: HashMap<(u8, u8), f32>,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

pub fn spawn_point(index: usize, count: usize) -> (f32, f32) {
    let angle =
        std::f32::consts::TAU * index as f32 / count.max(1) as f32 + std::f32::consts::FRAC_PI_4;
    let distance = ARENA_RADIUS * 0.55;
    (angle.cos() * distance, angle.sin() * distance)
}

impl BumperBalls {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len();
        let balls = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (x, z) = spawn_point(i, count);
                Ball {
                    seat: c.seat,
                    x,
                    z,
                    vx: 0.0,
                    vz: 0.0,
                    y: 0.0,
                    vy: 0.0,
                    state: BallState::Rolling,
                    fell_at: None,
                }
            })
            .collect();
        let brains = contestants
            .iter()
            .filter(|c| c.is_bot)
            .map(|c| {
                (
                    c.seat,
                    BotBrain {
                        think_in: 0.2,
                        ..Default::default()
                    },
                )
            })
            .collect();
        BumperBalls {
            balls,
            time: 0.0,
            radius: ARENA_RADIUS,
            rng: StdRng::seed_from_u64(seed),
            cooldowns: HashMap::new(),
            brains,
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

    fn radius_at(time: f32) -> f32 {
        if time <= SHRINK_START_SECONDS {
            return ARENA_RADIUS;
        }
        let t = ((time - SHRINK_START_SECONDS) / (ROUND_SECONDS - SHRINK_START_SECONDS))
            .clamp(0.0, 1.0);
        ARENA_RADIUS + (ARENA_MIN_RADIUS - ARENA_RADIUS) * t
    }

    fn ball(&self, seat: u8) -> Option<&Ball> {
        self.balls.iter().find(|b| b.seat == seat)
    }

    fn rolling_count(&self) -> usize {
        self.balls
            .iter()
            .filter(|b| b.state == BallState::Rolling)
            .count()
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let falling = self.balls.iter().any(|b| b.state == BallState::Falling);
        let timed_out = self.time >= ROUND_SECONDS;
        if !(timed_out || (self.rolling_count() <= 1 && !falling)) {
            return;
        }
        // Survivors share first place; everyone else ranks by how long they lasted.
        let mut order: Vec<(f32, u8)> = self
            .balls
            .iter()
            .map(|b| (b.fell_at.unwrap_or(f32::INFINITY), b.seat))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group))
                    if (prev - at).abs() < 1e-4 || (prev.is_infinite() && at.is_infinite()) =>
                {
                    group.push(seat)
                }
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }
}

impl Bout for BumperBalls {
    fn key(&self) -> &'static str {
        "bumper"
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.radius = Self::radius_at(self.time);
        for cooldown in self.cooldowns.values_mut() {
            *cooldown -= dt;
        }

        for ball in self.balls.iter_mut() {
            match ball.state {
                BallState::Rolling => {
                    let stick = inputs
                        .iter()
                        .find(|(s, _)| *s == ball.seat)
                        .map(|(_, input)| input.stick.clamped())
                        .unwrap_or_default();
                    let (ax, az) = (stick.x * ACCEL, stick.z * ACCEL);
                    let speed = (ball.vx * ball.vx + ball.vz * ball.vz).sqrt();
                    // Input can always steer or brake, but cannot push past top speed.
                    if speed < MAX_SPEED || ball.vx * ax + ball.vz * az < 0.0 {
                        ball.vx += ax * dt;
                        ball.vz += az * dt;
                    }
                    let pushing = stick.x.abs() + stick.z.abs() > 0.1;
                    let mut drag = if pushing { ROLL_DRAG } else { COAST_DRAG };
                    if speed > MAX_SPEED {
                        drag += OVERSPEED_DRAG;
                    }
                    let keep = (-drag * dt).exp();
                    ball.vx *= keep;
                    ball.vz *= keep;
                    ball.x += ball.vx * dt;
                    ball.z += ball.vz * dt;
                }
                BallState::Falling => {
                    let above = ball.y > WATER_Y;
                    ball.vy -= GRAVITY * dt;
                    ball.y += ball.vy * dt;
                    let keep = (-0.6 * dt).exp();
                    ball.vx *= keep;
                    ball.vz *= keep;
                    ball.x += ball.vx * dt;
                    ball.z += ball.vz * dt;
                    if above && ball.y <= WATER_Y {
                        events.push(json!({ "kind": "splash", "seat": ball.seat, "x": ball.x, "z": ball.z }));
                    }
                    if ball.y < OUT_DEPTH {
                        ball.state = BallState::Out;
                    }
                }
                BallState::Out => {}
            }
        }

        let rolling: Vec<usize> = (0..self.balls.len())
            .filter(|&i| self.balls[i].state == BallState::Rolling)
            .collect();
        for (n, &i) in rolling.iter().enumerate() {
            for &j in &rolling[n + 1..] {
                let (a, b) = (self.balls[i].clone(), self.balls[j].clone());
                let (mut dx, mut dz) = (b.x - a.x, b.z - a.z);
                let mut dist = (dx * dx + dz * dz).sqrt();
                if dist >= BALL_RADIUS * 2.0 {
                    continue;
                }
                if dist < 1e-4 {
                    let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
                    (dx, dz, dist) = (angle.cos() * 1e-3, angle.sin() * 1e-3, 1e-3);
                }
                let (nx, nz) = (dx / dist, dz / dist);
                let push = (BALL_RADIUS * 2.0 - dist) / 2.0;
                let approach = (b.vx - a.vx) * nx + (b.vz - a.vz) * nz;
                let impulse = if approach < 0.0 {
                    (-(1.0 + RESTITUTION) * approach / 2.0).max(MIN_BUMP_IMPULSE)
                } else {
                    0.0
                };
                {
                    let a = &mut self.balls[i];
                    a.x -= nx * push;
                    a.z -= nz * push;
                    a.vx -= nx * impulse;
                    a.vz -= nz * impulse;
                }
                {
                    let b = &mut self.balls[j];
                    b.x += nx * push;
                    b.z += nz * push;
                    b.vx += nx * impulse;
                    b.vz += nz * impulse;
                }
                let pair = (a.seat.min(b.seat), a.seat.max(b.seat));
                if impulse > 0.0 && self.cooldowns.get(&pair).copied().unwrap_or(0.0) <= 0.0 {
                    self.cooldowns.insert(pair, BUMP_EVENT_COOLDOWN);
                    events.push(json!({
                        "kind": "bump",
                        "a": a.seat,
                        "b": b.seat,
                        "strength": (impulse / MAX_SPEED).min(2.0),
                        "x": (a.x + b.x) / 2.0,
                        "z": (a.z + b.z) / 2.0,
                    }));
                }
            }
        }

        let radius = self.radius;
        let time = self.time;
        for ball in self.balls.iter_mut() {
            if ball.state == BallState::Rolling
                && (ball.x * ball.x + ball.z * ball.z).sqrt() > radius
            {
                ball.state = BallState::Falling;
                ball.vy = 1.5;
                ball.fell_at = Some(time);
                events.push(json!({ "kind": "fall", "seat": ball.seat }));
            }
        }
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.ball(seat).cloned() else {
            return Input::default();
        };
        if me.state != BallState::Rolling {
            return Input::default();
        }
        let radius = self.radius;
        let opponents: Vec<Ball> = self
            .balls
            .iter()
            .filter(|b| b.seat != seat && b.state == BallState::Rolling)
            .cloned()
            .collect();
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        brain.idle_for -= dt;
        if brain.think_in <= 0.0 {
            let skill = self.skill(seat);
            brain.think_in = self.rng.random_range(0.35..0.8) + skill.reaction() * 0.5;
            let wobble = 0.15 + skill.aim_error();
            brain.wobble = self.rng.random_range(-wobble..wobble);
            if self.rng.random_bool(0.08 + skill.hesitation() * 0.4) {
                brain.idle_for = self.rng.random_range(0.2..0.5);
            }
            // Prefer whoever is closest to the edge, then whoever is near.
            brain.target = opponents
                .iter()
                .map(|o| {
                    let edge = radius - (o.x * o.x + o.z * o.z).sqrt();
                    let near = ((o.x - me.x).powi(2) + (o.z - me.z).powi(2)).sqrt();
                    (edge * 1.4 + near * 0.6, o.seat)
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, s)| s);
        }
        let from_center = (me.x * me.x + me.z * me.z).sqrt();
        let stick = if from_center > radius * 0.74 {
            let lean = 1.0 / from_center.max(0.01);
            Stick {
                x: -me.x * lean,
                z: -me.z * lean,
            }
        } else if brain.idle_for > 0.0 {
            Stick::default()
        } else if let Some(target) = brain
            .target
            .and_then(|t| opponents.iter().find(|o| o.seat == t))
        {
            let (tx, tz) = (
                target.x + target.vx * 0.3 - me.x,
                target.z + target.vz * 0.3 - me.z,
            );
            let len = (tx * tx + tz * tz).sqrt().max(0.01);
            let (cos, sin) = (brain.wobble.cos(), brain.wobble.sin());
            let (dx, dz) = (tx / len, tz / len);
            Stick {
                x: dx * cos - dz * sin,
                z: dx * sin + dz * cos,
            }
        } else {
            Stick {
                x: -me.x * 0.1,
                z: -me.z * 0.1,
            }
        };
        self.brains.insert(seat, brain);
        Input::from_stick(stick.clamped())
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 1000.0).round() / 1000.0;
        json!({
            "game": "bumper",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "radius": round(self.radius),
            "ball_radius": BALL_RADIUS,
            "balls": self.balls.iter().map(|b| json!({
                "seat": b.seat,
                "x": round(b.x),
                "z": round(b.z),
                "y": round(b.y),
                "vx": round(b.vx),
                "vz": round(b.vz),
                "state": match b.state { BallState::Rolling => "rolling", BallState::Falling => "falling", BallState::Out => "out" },
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
    use serde_json::Value;
    use tawks_bout::{DEFAULT_SKILL, TICK_DT};

    fn contestants(n: u8, bots: bool) -> Vec<Contestant> {
        (0..n)
            .map(|seat| Contestant {
                seat,
                is_bot: bots,
                skill: DEFAULT_SKILL,
            })
            .collect()
    }

    fn run(game: &mut BumperBalls, seconds: f32, sticks: &[(u8, Stick)]) -> Vec<Value> {
        let inputs: Vec<(u8, Input)> = sticks
            .iter()
            .map(|(seat, stick)| (*seat, Input::from_stick(*stick)))
            .collect();
        let mut events = Vec::new();
        for _ in 0..(seconds / TICK_DT) as usize {
            game.step(&inputs, TICK_DT, &mut events);
        }
        events
    }

    #[test]
    fn a_head_on_bump_sends_both_balls_back() {
        let mut game = BumperBalls::new(&contestants(2, false), 1);
        game.balls[0].x = -2.0;
        game.balls[0].z = 0.0;
        game.balls[1].x = 2.0;
        game.balls[1].z = 0.0;
        let events = run(
            &mut game,
            0.6,
            &[
                (0, Stick { x: 1.0, z: 0.0 }),
                (1, Stick { x: -1.0, z: 0.0 }),
            ],
        );
        assert!(events.iter().any(|e| e["kind"] == "bump"), "the balls met");
        assert!(
            game.balls[0].vx < 0.0 && game.balls[1].vx > 0.0,
            "each bounced away"
        );
        assert!(
            game.balls[1].x - game.balls[0].x >= 1.69,
            "the balls do not overlap"
        );
    }

    #[test]
    fn rolling_off_the_edge_falls_and_the_survivor_places_first() {
        let mut game = BumperBalls::new(&contestants(2, false), 2);
        game.balls[0].x = 0.0;
        game.balls[0].z = 0.0;
        game.balls[1].x = ARENA_RADIUS - 0.5;
        game.balls[1].z = 0.0;
        let events = run(&mut game, 3.0, &[(1, Stick { x: 1.0, z: 0.0 })]);
        assert!(events.iter().any(|e| e["kind"] == "fall" && e["seat"] == 1));
        assert!(
            events
                .iter()
                .any(|e| e["kind"] == "splash" && e["seat"] == 1)
        );
        assert_eq!(game.balls[1].state, BallState::Out);
        assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
    }

    #[test]
    fn time_up_shares_first_place_among_survivors() {
        let mut game = BumperBalls::new(&contestants(3, false), 3);
        game.balls[2].x = ARENA_RADIUS + 1.0;
        run(&mut game, ROUND_SECONDS + 1.0, &[]);
        let placements = game.placements().expect("round over");
        assert_eq!(placements[0].len(), 2, "both idle balls survived");
        assert_eq!(placements[1], vec![2]);
    }

    #[test]
    fn practice_bots_finish_a_round_on_their_own() {
        let mut game = BumperBalls::new(&contestants(4, true), 4);
        let mut events = Vec::new();
        let mut steps = 0;
        while game.placements().is_none() && steps < 5000 {
            let inputs: Vec<(u8, Input)> =
                (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
            game.step(&inputs, TICK_DT, &mut events);
            steps += 1;
        }
        assert!(
            game.placements().is_some(),
            "the round ended within the time limit"
        );
        assert!(
            events.iter().any(|e| e["kind"] == "bump"),
            "bots went after each other"
        );
    }
}
