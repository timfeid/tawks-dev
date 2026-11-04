//! Bumper Balls: everyone rides a ball on a round platform over the water.
//! Bump the others off; the last one standing wins. The platform starts to
//! shrink partway through so a round always ends.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use tawks_bout::{BodyProfile, Bout, BoutSpec, Contestant, Controls, Format, Input, Skill, Stick, crowd_scale};

pub const SPEC: BoutSpec = BoutSpec {
    key: "bumper",
    name: "Bumper Balls",
    rules: "Roll, bump, and be the last one above the water.",
    controls: Controls::Move,
    format: Format::FreeForAll,
    min_players: 2,
    max_players: 8,
    create: |contestants, seed| Box::new(BumperBalls::new(contestants, seed)),
};

pub const ARENA_RADIUS: f32 = 9.0;
pub const ARENA_MIN_RADIUS: f32 = 5.0;
pub const BALL_RADIUS: f32 = 0.85;
pub const ROUND_SECONDS: f32 = 60.0;
pub const SHRINK_START_SECONDS: f32 = 25.0;
pub const WATER_Y: f32 = -3.0;

const ACCEL: f32 = 10.5;
const MAX_SPEED: f32 = 11.0;
const COAST_DRAG: f32 = 1.4;
const ROLL_DRAG: f32 = 0.22;
const OVERSPEED_DRAG: f32 = 2.0;
const BUMP_EVENT_MIN_SPEED: f32 = 0.4;
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
    pub body_width: f32,
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
    /// How much wider than the platform for four, for the size of the field.
    size: f32,
    rng: StdRng,
    cooldowns: HashMap<(u8, u8), f32>,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
    /// Partners, when played in pairs: they roll apart but never bump each other off.
    allies: HashMap<u8, u8>,
}

/// Where the `index`th of `count` balls starts. A bigger field starts further
/// out, on the wider platform it plays on.
pub fn spawn_point(index: usize, count: usize) -> (f32, f32) {
    let angle =
        std::f32::consts::TAU * index as f32 / count.max(1) as f32 + std::f32::consts::FRAC_PI_4;
    let distance = ARENA_RADIUS * crowd_scale(count) * 0.55;
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
                    body_width: c.body_width,
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
            allies: contestants
                .iter()
                .filter_map(|c| c.ally.map(|ally| (c.seat, ally)))
                .collect(),
            balls,
            time: 0.0,
            radius: ARENA_RADIUS * crowd_scale(count),
            size: crowd_scale(count),
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

    fn radius_at(&self, time: f32) -> f32 {
        if time <= SHRINK_START_SECONDS {
            return ARENA_RADIUS * self.size;
        }
        let t = ((time - SHRINK_START_SECONDS) / (ROUND_SECONDS - SHRINK_START_SECONDS))
            .clamp(0.0, 1.0);
        // Crowded rounds need a wider start, but the same tight finish to avoid ties.
        ARENA_RADIUS * self.size * (1.0 - t) + ARENA_MIN_RADIUS * t
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

    fn setup(&self) -> Value {
        json!({
            "game": "bumper",
            "radius": ARENA_RADIUS * self.size,
            "min_radius": ARENA_MIN_RADIUS,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.radius = self.radius_at(self.time);
        for cooldown in self.cooldowns.values_mut() {
            *cooldown -= dt;
        }

        for ball in self.balls.iter_mut() {
            match ball.state {
                BallState::Rolling => {
                    let body = BodyProfile::new(ball.body_width);
                    let stick = inputs
                        .iter()
                        .find(|(s, _)| *s == ball.seat)
                        .map(|(_, input)| input.stick.clamped())
                        .unwrap_or_default();
                    let (ax, az) = (stick.x * ACCEL * body.acceleration, stick.z * ACCEL * body.acceleration);
                    let speed = (ball.vx * ball.vx + ball.vz * ball.vz).sqrt();
                    let top_speed = MAX_SPEED * body.speed;
                    // Input can always steer or brake, but cannot push past top speed.
                    if speed < top_speed || ball.vx * ax + ball.vz * az < 0.0 {
                        ball.vx += ax * dt;
                        ball.vz += az * dt;
                    }
                    let pushing = stick.x.abs() + stick.z.abs() > 0.1;
                    let mut drag = if pushing { ROLL_DRAG } else { COAST_DRAG };
                    if speed > top_speed {
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
                let partners = tawks_bout::allied(
                    self.allies.get(&a.seat).copied(),
                    self.allies.get(&b.seat).copied(),
                );
                let (mass_a, mass_b) = (BodyProfile::new(a.body_width).mass, BodyProfile::new(b.body_width).mass);
                let closing_speed = (-approach).max(0.0);
                let impulse = if partners {
                    // Partners only roll apart.
                    0.0
                } else if closing_speed > 0.0 {
                    // A slow touch yields; a fast, direct hit transfers momentum.
                    // Only velocity toward the other ball counts, so glancing
                    // contacts keep their sideways motion.
                    let restitution = (0.45 + closing_speed / MAX_SPEED * 0.55).clamp(0.45, 1.0);
                    (1.0 + restitution) * closing_speed / (1.0 / mass_a + 1.0 / mass_b)
                } else {
                    0.0
                };
                {
                    let a = &mut self.balls[i];
                    a.x -= nx * push * (2.0 * mass_b / (mass_a + mass_b));
                    a.z -= nz * push * (2.0 * mass_b / (mass_a + mass_b));
                    a.vx -= nx * impulse / mass_a;
                    a.vz -= nz * impulse / mass_a;
                }
                {
                    let b = &mut self.balls[j];
                    b.x += nx * push * (2.0 * mass_a / (mass_a + mass_b));
                    b.z += nz * push * (2.0 * mass_a / (mass_a + mass_b));
                    b.vx += nx * impulse / mass_b;
                    b.vz += nz * impulse / mass_b;
                }
                let pair = (a.seat.min(b.seat), a.seat.max(b.seat));
                if closing_speed >= BUMP_EVENT_MIN_SPEED
                    && impulse > 0.0
                    && self.cooldowns.get(&pair).copied().unwrap_or(0.0) <= 0.0
                {
                    self.cooldowns.insert(pair, BUMP_EVENT_COOLDOWN);
                    events.push(json!({
                        "kind": "bump",
                        "a": a.seat,
                        "b": b.seat,
                        "strength": (closing_speed / MAX_SPEED).min(2.0),
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
        let skill = self.skill(seat).level();
        let outward_speed = if from_center > 0.01 {
            ((me.x * me.vx + me.z * me.vz) / from_center).max(0.0)
        } else {
            0.0
        };
        let stopping_distance = outward_speed * outward_speed / (2.0 * ACCEL);
        let stick = if from_center + stopping_distance > radius - BALL_RADIUS - 0.3 {
            let lean = 1.0 / from_center.max(0.01);
            // Brake outward momentum as well as steering toward the middle.
            let brake = 0.22 * skill;
            let x = -me.x * lean - me.vx * brake;
            let z = -me.z * lean - me.vz * brake;
            let length = (x * x + z * z).sqrt().max(0.01);
            Stick {
                x: x / length,
                z: z / length,
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
        // Stronger riders balance their chase with braking and collision room.
        let defend = ((skill - 0.4) / 0.6).clamp(0., 1.) * 0.4;
        let stick = if defend > 0. && brain.idle_for <= 0. && from_center <= radius * 0.74 {
            let mut safe_x = -me.x * 0.18 - me.vx * 0.25;
            let mut safe_z = -me.z * 0.18 - me.vz * 0.25;
            if let Some(threat) = opponents.iter().min_by(|a, b| {
                let gap = |o: &Ball| (me.x - o.x - o.vx * 0.35).hypot(me.z - o.z - o.vz * 0.35);
                gap(a).total_cmp(&gap(b))
            }) {
                let dx = me.x - threat.x - threat.vx * 0.35;
                let dz = me.z - threat.z - threat.vz * 0.35;
                let gap = dx.hypot(dz).max(0.01);
                let avoid = ((4. - gap) / 4.).clamp(0., 1.) * 1.5;
                safe_x += dx / gap * avoid;
                safe_z += dz / gap * avoid;
            }
            let safe_len = safe_x.hypot(safe_z);
            if safe_len > 0.01 {
                let x = stick.x * (1. - defend) + safe_x / safe_len * defend;
                let z = stick.z * (1. - defend) + safe_z / safe_len * defend;
                let len = x.hypot(z).max(0.01);
                Stick {
                    x: x / len,
                    z: z / len,
                }
            } else {
                stick
            }
        } else {
            stick
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
                team: seat % 2,
                ally: None,
                body_width: 1.0,
                pet: 4,
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
    fn broad_rider_keeps_more_momentum_in_a_collision() {
        let mut seats = contestants(2, false);
        seats[0].body_width = 1.4;
        seats[1].body_width = 0.7;
        let mut game = BumperBalls::new(&seats, 3);
        game.balls[0].x = -0.9;
        game.balls[1].x = 0.9;
        game.balls[0].z = 0.0;
        game.balls[1].z = 0.0;
        game.balls[0].vx = 6.0;
        game.balls[1].vx = -6.0;
        game.step(&[], TICK_DT, &mut Vec::new());
        assert!((game.balls[0].vx - 6.0).abs() < (game.balls[1].vx + 6.0).abs());
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
    fn holding_a_direction_builds_speed_beyond_the_old_ceiling() {
        let mut game = BumperBalls::new(&contestants(2, false), 21);
        game.balls[0].x = 0.0;
        game.balls[0].z = 0.0;
        game.balls[1].x = -7.0;
        game.balls[1].z = 0.0;
        let push = [(0, Stick { x: 1.0, z: 0.0 })];
        run(&mut game, 0.2, &push);
        let early = game.balls[0].vx;
        run(&mut game, 0.8, &push);
        let built = game.balls[0].vx;
        assert!(
            early < 4.0,
            "the ball should not instantly reach top speed: {early}"
        );
        assert!(
            built > 7.0,
            "holding the stick should pass the former limit: {built}"
        );
        assert!(
            built <= MAX_SPEED + 0.2,
            "movement remains bounded: {built}"
        );
    }

    #[test]
    fn closing_speed_sets_knockback_and_grazes_do_not_create_a_hard_hit() {
        let impact = |speed: f32, sideways: bool| {
            let mut game = BumperBalls::new(&contestants(2, false), 22);
            game.balls[0].x = 0.0;
            game.balls[0].z = 0.0;
            game.balls[1].x = 1.6;
            game.balls[1].z = 0.0;
            if sideways {
                game.balls[0].vz = speed;
            } else {
                game.balls[0].vx = speed;
            }
            let mut events = Vec::new();
            game.step(&[], TICK_DT, &mut events);
            (game.balls[1].vx, events)
        };
        let (soft, soft_events) = impact(1.0, false);
        let (hard, hard_events) = impact(8.0, false);
        assert!(
            hard > soft * 5.0,
            "speed must transfer through the collision: {soft}, {hard}"
        );
        let strength = |events: &[Value]| {
            events.iter().find(|event| event["kind"] == "bump").unwrap()["strength"]
                .as_f64()
                .unwrap()
        };
        assert!(strength(&hard_events) > strength(&soft_events) * 5.0);
        let (grazed, graze_events) = impact(8.0, true);
        assert!(
            grazed.abs() < 0.01,
            "sideways motion should not launch the other ball"
        );
        assert!(!graze_events.iter().any(|event| event["kind"] == "bump"));
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
    fn a_bigger_field_starts_wider_but_closes_to_a_decisive_radius() {
        assert_eq!(
            BumperBalls::new(&contestants(4, false), 1).radius,
            ARENA_RADIUS,
            "four play it as built"
        );
        let (x, z) = spawn_point(0, 4);
        assert!(
            (x.hypot(z) - ARENA_RADIUS * 0.55).abs() < 1e-4,
            "and start where they did"
        );
        let eight = BumperBalls::new(&contestants(8, false), 1);
        assert!(
            (eight.radius / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4,
            "eight get the room of two fours"
        );
        assert_eq!(
            eight.setup()["radius"],
            json!(eight.radius),
            "and the client is told"
        );
        assert_eq!(eight.setup()["min_radius"], json!(ARENA_MIN_RADIUS));
        let closed = eight.radius_at(ROUND_SECONDS);
        assert!(
            (closed - ARENA_MIN_RADIUS).abs() < 0.05,
            "it closes to the same final radius as four, {closed}"
        );
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
