//! Dodgeball: five balls on the centre line of a gym. Run one down, throw it
//! at a rival to knock them out for a few seconds, or catch one out of the air
//! to send the thrower off instead. Most hits when the whistle goes wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::fpmove::{self, Block, Body, EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_RADIUS};
use crate::bout::{Contestant, Input, Bout, Skill, Stick};

pub const COURT_HALF_X: f32 = 12.0;
pub const COURT_HALF_Z: f32 = 16.0;
pub const BALL_RADIUS: f32 = 0.33;
pub const BALL_COUNT: usize = 5;
pub const THROW_SPEED: f32 = 22.0;
pub const LIVE_SECONDS: f32 = 2.2;
pub const PICKUP_RANGE: f32 = 1.3;
pub const CATCH_RANGE: f32 = 2.6;
pub const OUT_SECONDS: f32 = 4.0;
pub const ROUND_SECONDS: f32 = 75.0;

const BALL_GRAVITY: f32 = 9.0;
const BALL_BOUNCE: f32 = 0.45;
const BALL_ROLL_DRAG: f32 = 1.8;
const THROW_COOLDOWN: f32 = 0.35;
const BOT_AIM_DELAY: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BallState {
    Idle,
    Held(u8),
    Live(u8),
}

#[derive(Clone, Debug)]
pub struct Ball {
    pub id: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub state: BallState,
    pub live_for: f32,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub seat: u8,
    pub body: Body,
    pub yaw: f32,
    pub pitch: f32,
    pub hits: u32,
    pub outs: u32,
    pub out_for: f32,
    pub cooldown: f32,
    pub end: f32,
}

impl Player {
    pub fn playing(&self) -> bool {
        self.out_for <= 0.0
    }

    fn eye(&self) -> (f32, f32, f32) {
        (self.body.x, self.body.y + EYE_HEIGHT, self.body.z)
    }

    fn chest(&self) -> (f32, f32, f32) {
        (self.body.x, self.body.y + PLAYER_HEIGHT * 0.55, self.body.z)
    }
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    think_in: f32,
    aim_for: f32,
    strafe: f32,
    aim_error: f32,
}

/// Low crates to duck behind, away from the centre line.
pub fn court() -> Vec<Block> {
    let mut blocks = Vec::new();
    for (sx, sz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        blocks.push(Block {
            x: 7.5 * sx,
            z: 7.0 * sz,
            hx: 1.4,
            hz: 1.0,
            top: 1.0,
            bottom: 0.0,
        });
    }
    blocks
}

pub struct Dodgeball {
    pub players: Vec<Player>,
    pub balls: Vec<Ball>,
    pub time: f32,
    blocks: Vec<Block>,
    rng: StdRng,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl Dodgeball {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        // Halves take turns, so a pair faces off across the line.
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let end = if i % 2 == 0 { -1.0 } else { 1.0 };
                let spread = ((i / 2) as f32 - 0.5) * 6.0;
                let mut player = Player {
                    seat: c.seat,
                    body: Body::default(),
                    yaw: if end < 0.0 { std::f32::consts::PI } else { 0.0 },
                    pitch: 0.0,
                    hits: 0,
                    outs: 0,
                    out_for: 0.0,
                    cooldown: 0.0,
                    end,
                };
                player.body.x = spread;
                player.body.z = end * (COURT_HALF_Z - 2.0);
                player.body.on_ground = true;
                player
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
        Dodgeball {
            players,
            balls: line_of_balls(),
            time: 0.0,
            blocks: court(),
            rng: StdRng::seed_from_u64(seed),
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

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    fn player(&self, seat: u8) -> Option<&Player> {
        self.players.iter().find(|p| p.seat == seat)
    }

    fn held_by(&self, seat: u8) -> Option<usize> {
        self.balls
            .iter()
            .position(|b| b.state == BallState::Held(seat))
    }

    fn step_balls(&mut self, dt: f32, events: &mut Vec<Value>) {
        struct Carrier {
            seat: u8,
            eye: (f32, f32, f32),
            look: (f32, f32, f32),
        }
        let carriers: Vec<Carrier> = self
            .players
            .iter()
            .map(|p| Carrier {
                seat: p.seat,
                eye: p.eye(),
                look: fpmove::look(p.yaw, p.pitch),
            })
            .collect();
        for ball in self.balls.iter_mut() {
            match ball.state {
                BallState::Held(seat) => {
                    if let Some(carrier) = carriers.iter().find(|c| c.seat == seat) {
                        let (eye, look) = (carrier.eye, carrier.look);
                        ball.x = eye.0 + look.0 * 0.55;
                        ball.y = eye.1 - 0.25 + look.1 * 0.55;
                        ball.z = eye.2 + look.2 * 0.55;
                        ball.vx = 0.0;
                        ball.vy = 0.0;
                        ball.vz = 0.0;
                    }
                }
                BallState::Idle | BallState::Live(_) => {
                    if let BallState::Live(_) = ball.state {
                        ball.live_for -= dt;
                        if ball.live_for <= 0.0 {
                            ball.state = BallState::Idle;
                        }
                    }
                    ball.vy -= BALL_GRAVITY * dt;
                    ball.x += ball.vx * dt;
                    ball.y += ball.vy * dt;
                    ball.z += ball.vz * dt;
                    if ball.y <= BALL_RADIUS {
                        ball.y = BALL_RADIUS;
                        if ball.vy < -0.5 {
                            ball.vy = -ball.vy * BALL_BOUNCE;
                            events.push(json!({ "kind": "bounce", "id": ball.id, "x": round(ball.x), "z": round(ball.z) }));
                        } else {
                            ball.vy = 0.0;
                            let keep = (-BALL_ROLL_DRAG * dt).exp();
                            ball.vx *= keep;
                            ball.vz *= keep;
                        }
                    }
                    let bound_x = COURT_HALF_X - BALL_RADIUS;
                    let bound_z = COURT_HALF_Z - BALL_RADIUS;
                    if ball.x.abs() > bound_x {
                        ball.x = ball.x.clamp(-bound_x, bound_x);
                        ball.vx = -ball.vx * BALL_BOUNCE;
                    }
                    if ball.z.abs() > bound_z {
                        ball.z = ball.z.clamp(-bound_z, bound_z);
                        ball.vz = -ball.vz * BALL_BOUNCE;
                    }
                }
            }
        }
    }

    /// A live ball meeting someone knocks them out and dies where it lands.
    fn resolve_hits(&mut self, events: &mut Vec<Value>) {
        let mut knocked: Vec<(u8, u8, u8)> = Vec::new();
        for ball in self.balls.iter_mut() {
            let BallState::Live(thrower) = ball.state else {
                continue;
            };
            for player in self.players.iter() {
                if player.seat == thrower || !player.playing() {
                    continue;
                }
                let dx = ball.x - player.body.x;
                let dz = ball.z - player.body.z;
                let reach = BALL_RADIUS + PLAYER_RADIUS;
                let low = player.body.y - BALL_RADIUS;
                let high = player.body.y + PLAYER_HEIGHT + BALL_RADIUS;
                if dx * dx + dz * dz <= reach * reach && ball.y >= low && ball.y <= high {
                    ball.state = BallState::Idle;
                    ball.vx *= 0.1;
                    ball.vz *= 0.1;
                    ball.vy = 0.0;
                    knocked.push((player.seat, thrower, ball.id));
                    break;
                }
            }
        }
        for (seat, thrower, ball) in knocked {
            self.knock_out(seat, thrower, ball, events);
        }
    }

    fn knock_out(&mut self, seat: u8, by: u8, ball: u8, events: &mut Vec<Value>) {
        // Whatever they were holding rolls free.
        if let Some(index) = self.held_by(seat) {
            let dropped = &mut self.balls[index];
            dropped.state = BallState::Idle;
            dropped.y = BALL_RADIUS;
            dropped.vx = 0.0;
            dropped.vz = 0.0;
        }
        if let Some(player) = self.players.iter_mut().find(|p| p.seat == seat) {
            player.out_for = OUT_SECONDS;
            player.outs += 1;
            player.body.vx = 0.0;
            player.body.vz = 0.0;
        }
        if let Some(thrower) = self.players.iter_mut().find(|p| p.seat == by) {
            thrower.hits += 1;
        }
        events.push(json!({ "kind": "out", "seat": seat, "by": by, "ball": ball }));
    }

    fn pick_up(&mut self, seat: u8, events: &mut Vec<Value>) {
        if self.held_by(seat).is_some() {
            return;
        }
        let Some(player) = self.player(seat).cloned() else {
            return;
        };
        let reach = PICKUP_RANGE * PICKUP_RANGE;
        let found = self.balls.iter().position(|b| {
            b.state == BallState::Idle
                && (b.x - player.body.x).powi(2) + (b.z - player.body.z).powi(2) <= reach
                && b.y < player.body.y + PLAYER_HEIGHT
        });
        if let Some(index) = found {
            self.balls[index].state = BallState::Held(seat);
            events.push(json!({ "kind": "pickup", "seat": seat, "id": self.balls[index].id }));
        }
    }

    /// Throw what you are holding, or reach for a ball flying at you.
    fn use_ball(&mut self, seat: u8, events: &mut Vec<Value>) {
        let Some(player) = self.player(seat).cloned() else {
            return;
        };
        if player.cooldown > 0.0 {
            return;
        }
        if let Some(index) = self.held_by(seat) {
            let (ox, oy, oz) = player.eye();
            let (dx, dy, dz) = fpmove::look(player.yaw, player.pitch);
            let ball = &mut self.balls[index];
            ball.state = BallState::Live(seat);
            ball.live_for = LIVE_SECONDS;
            ball.x = ox + dx * 0.7;
            ball.y = oy + dy * 0.7;
            ball.z = oz + dz * 0.7;
            ball.vx = dx * THROW_SPEED + player.body.vx * 0.3;
            ball.vy = dy * THROW_SPEED;
            ball.vz = dz * THROW_SPEED + player.body.vz * 0.3;
            let id = ball.id;
            if let Some(thrower) = self.players.iter_mut().find(|p| p.seat == seat) {
                thrower.cooldown = THROW_COOLDOWN;
            }
            events.push(json!({ "kind": "throw", "seat": seat, "id": id }));
            return;
        }
        // Empty handed: try to take one out of the air.
        let catchable = self
            .balls
            .iter()
            .enumerate()
            .filter_map(|(i, ball)| match ball.state {
                BallState::Live(by) if by != seat => {
                    let dx = ball.x - player.body.x;
                    let dz = ball.z - player.body.z;
                    let distance = (dx * dx + dz * dz).sqrt();
                    let closing = ball.vx * -dx + ball.vz * -dz > 0.0;
                    if distance <= CATCH_RANGE && closing {
                        Some((i, by, distance))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        if let Some((index, thrower, _)) = catchable {
            self.balls[index].state = BallState::Held(seat);
            if let Some(catcher) = self.players.iter_mut().find(|p| p.seat == seat) {
                catcher.cooldown = THROW_COOLDOWN;
            }
            let id = self.balls[index].id;
            events.push(json!({ "kind": "catch", "seat": seat, "off": thrower, "id": id }));
            self.knock_out(thrower, seat, id, events);
        }
    }

    fn respawn(&mut self, seat: u8, events: &mut Vec<Value>) {
        if let Some(player) = self.players.iter_mut().find(|p| p.seat == seat) {
            player.body = Body {
                x: 0.0,
                y: 0.0,
                z: player.end * (COURT_HALF_Z - 1.5),
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                on_ground: true,
            };
            player.yaw = if player.end < 0.0 {
                std::f32::consts::PI
            } else {
                0.0
            };
            player.pitch = 0.0;
            player.out_for = 0.0;
        }
        events.push(json!({ "kind": "back", "seat": seat }));
    }

    /// The nearest live ball heading our way, and how far off it is.
    fn incoming(&self, seat: u8) -> Option<(f32, f32, f32)> {
        let player = self.player(seat)?;
        self.balls
            .iter()
            .filter_map(|ball| match ball.state {
                BallState::Live(by) if by != seat => {
                    let dx = player.body.x - ball.x;
                    let dz = player.body.z - ball.z;
                    let distance = (dx * dx + dz * dz).sqrt().max(0.01);
                    let closing = (ball.vx * dx + ball.vz * dz) / distance;
                    if closing > 2.0 {
                        Some((distance, ball.vx, ball.vz))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < ROUND_SECONDS {
            return;
        }
        let mut rows: Vec<(u32, i64, u8)> = self
            .players
            .iter()
            .map(|p| (p.hits, -(p.outs as i64), p.seat))
            .collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, i64)> = None;
        for (hits, outs, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (hits, outs) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((hits, outs));
        }
        self.placements = Some(groups);
    }
}

fn line_of_balls() -> Vec<Ball> {
    (0..BALL_COUNT)
        .map(|i| {
            let spread = (i as f32 - (BALL_COUNT as f32 - 1.0) / 2.0) * 3.4;
            Ball {
                id: i as u8,
                x: spread,
                y: BALL_RADIUS,
                z: 0.0,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                state: BallState::Idle,
                live_for: 0.0,
            }
        })
        .collect()
}

fn round(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut angle = a % tau;
    if angle > std::f32::consts::PI {
        angle -= tau;
    }
    if angle < -std::f32::consts::PI {
        angle += tau;
    }
    angle
}

impl Bout for Dodgeball {
    fn key(&self) -> &'static str {
        "dodgeball"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "dodgeball",
            "half_x": COURT_HALF_X,
            "half_z": COURT_HALF_Z,
            "eye": EYE_HEIGHT,
            "height": PLAYER_HEIGHT,
            "radius": PLAYER_RADIUS,
            "ball_radius": BALL_RADIUS,
            "blocks": self.blocks.iter().map(|b| json!({
                "x": b.x, "z": b.z, "hx": b.hx, "hz": b.hz, "top": b.top,
            })).collect::<Vec<_>>(),
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
                .map(|(_, i)| *i)
                .unwrap_or_default();
            let player = &mut self.players[index];
            player.cooldown = (player.cooldown - dt).max(0.0);
            player.yaw = input.yaw;
            player.pitch = input.pitch.clamp(-1.4, 1.4);
            if !player.playing() {
                player.out_for -= dt;
                if player.out_for <= 0.0 {
                    self.respawn(seat, events);
                }
                continue;
            }
            let blocks = self.blocks.clone();
            let player = &mut self.players[index];
            fpmove::step_body(
                &mut player.body,
                input,
                dt,
                &blocks,
                COURT_HALF_X,
                COURT_HALF_Z,
            );
            self.pick_up(seat, events);
            if input.fire {
                self.use_ball(seat, events);
            }
        }

        self.step_balls(dt, events);
        self.resolve_hits(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = self.rng.random_range(0.3..0.8);
            brain.strafe = [-1.0, 0.0, 1.0][self.rng.random_range(0..3)];
            let wobble = self.skill(seat).aim_error() * 0.6;
            brain.aim_error = self.rng.random_range(-wobble..wobble);
        }
        let mut input = Input {
            yaw: me.yaw,
            pitch: me.pitch,
            ..Default::default()
        };
        if !me.playing() {
            self.brains.insert(seat, brain);
            return input;
        }

        let holding = self.held_by(seat).is_some();
        let dodging = self.incoming(seat);
        let target = self
            .players
            .iter()
            .filter(|p| p.seat != seat && p.playing())
            .filter(|p| {
                fpmove::can_see(
                    me.eye(),
                    p.chest(),
                    &self.blocks,
                    COURT_HALF_X,
                    COURT_HALF_Z,
                )
            })
            .min_by(|a, b| {
                let near =
                    |p: &Player| (p.body.x - me.body.x).powi(2) + (p.body.z - me.body.z).powi(2);
                near(a)
                    .partial_cmp(&near(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();

        if holding {
            if let Some(target) = target {
                let (dx, dz) = (target.body.x - me.body.x, target.body.z - me.body.z);
                let distance = (dx * dx + dz * dz).sqrt().max(0.01);
                let want_yaw = (-dx).atan2(-dz) + brain.aim_error;
                // Lob it just enough to reach them.
                let lift = (BALL_GRAVITY * distance / (THROW_SPEED * THROW_SPEED)).clamp(-1.0, 1.0);
                let want_pitch = 0.5 * lift.asin() + (target.body.y - me.body.y) * 0.04;
                input.yaw = me.yaw + wrap_angle(want_yaw - me.yaw) * (7.0 * dt).min(1.0);
                input.pitch = me.pitch + (want_pitch - me.pitch) * (7.0 * dt).min(1.0);
                let (fx, fz) = (-input.yaw.sin(), -input.yaw.cos());
                let facing = (fx * dx + fz * dz) / distance;
                brain.aim_for = if facing > 0.99 {
                    brain.aim_for + dt
                } else {
                    0.0
                };
                let skill = self.skill(seat);
                if brain.aim_for > BOT_AIM_DELAY + skill.reaction() * 0.6 && distance < 26.0 {
                    input.fire = !self.rng.random_bool(skill.hesitation());
                    brain.aim_for = 0.0;
                }
                // Hold the line rather than charging into the throw.
                input.stick = Stick {
                    x: brain.strafe * 0.7,
                    z: if distance > 14.0 { -0.5 } else { 0.0 },
                }
                .clamped();
            }
        } else if let Some(ball) = self
            .balls
            .iter()
            .filter(|b| b.state == BallState::Idle)
            .min_by(|a, b| {
                let near =
                    |ball: &Ball| (ball.x - me.body.x).powi(2) + (ball.z - me.body.z).powi(2);
                near(a)
                    .partial_cmp(&near(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
        {
            let (dx, dz) = (ball.x - me.body.x, ball.z - me.body.z);
            let want_yaw = (-dx).atan2(-dz);
            input.yaw = me.yaw + wrap_angle(want_yaw - me.yaw) * (8.0 * dt).min(1.0);
            input.stick = Stick { x: 0.0, z: -1.0 };
        }

        // Jump aside from anything flying at us, and sometimes go for the catch.
        if let Some((distance, _, _)) = dodging
            && distance < 7.0
        {
            {
                input.stick = Stick {
                    x: if brain.strafe >= 0.0 { 1.0 } else { -1.0 },
                    z: 0.0,
                };
                let catch_odds = 0.01 + self.skill(seat).level() as f64 * 0.05;
                if !holding && distance < CATCH_RANGE * 0.8 && self.rng.random_bool(catch_odds) {
                    input.fire = true;
                }
            }
        }
        self.brains.insert(seat, brain);
        input
    }

    fn snapshot(&self) -> Value {
        json!({
            "game": "dodgeball",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.body.x),
                "y": round(p.body.y),
                "z": round(p.body.z),
                "vx": round(p.body.vx),
                "vz": round(p.body.vz),
                "yaw": round(p.yaw),
                "pitch": round(p.pitch),
                "out": !p.playing(),
                "back_in": round(p.out_for.max(0.0)),
                "holding": self.held_by(p.seat).is_some(),
                "hits": p.hits,
                "outs": p.outs,
                "ground": p.body.on_ground,
            })).collect::<Vec<_>>(),
            "balls": self.balls.iter().map(|b| json!({
                "id": b.id,
                "x": round(b.x),
                "y": round(b.y),
                "z": round(b.z),
                "state": match b.state {
                    BallState::Idle => "idle",
                    BallState::Held(_) => "held",
                    BallState::Live(_) => "live",
                },
                "by": match b.state {
                    BallState::Held(seat) | BallState::Live(seat) => Some(seat),
                    BallState::Idle => None,
                },
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
