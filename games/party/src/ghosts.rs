//! Ghost Busting: everyone gets a vacuum in a haunted hall. Ghosts drift
//! about, slip out of the way of vacuum cones and now and then phase through
//! a wall. Hold a ghost in your cone and it is drawn in: a small one is worth
//! a point, a golden one three, and a big one takes two cones at once and
//! splits four points between them. Most points when the clock runs out wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::fpmove::{self, Block, Body, EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_RADIUS};

pub const KEY: &str = "ghosts";
pub const HALL_HALF: f32 = 13.0;
pub const ROUND_SECONDS: f32 = 60.0;

/// The vacuum: how far it reaches, how wide its cone is either side of where
/// you look, and how much it keeps at the edge of the cone and the end of its
/// reach. Matches the board vacuum's feel: strongest dead centre.
pub const RANGE: f32 = 9.0;
pub const CONE: f32 = 0.42;
pub const EDGE: f32 = 0.25;
pub const FAR: f32 = 0.4;
pub const SPIN_UP: f32 = 0.45;
/// Held fire arrives as a stream of shots, one every 280 ms, so the motor
/// stays on until it has missed a little more than one of them.
pub const HOLD: f32 = 0.35;
/// Seconds of full strength to draw a ghost all the way in.
pub const CATCH_SECONDS: f32 = 1.0;
pub const BIG_SECONDS: f32 = 1.5;
/// How fast a ghost let go of slips back out, in progress per second.
const SLIP: f32 = 1.2;

pub const SMALL_GHOSTS: usize = 5;
pub const SMALL_POINTS: u32 = 1;
pub const GOLD_POINTS: u32 = 3;
pub const BIG_POINTS: u32 = 4;
/// How long a golden ghost hangs about before it phases out for good.
pub const GOLD_LIFE: f32 = 14.0;
/// How long a ghost coming in through a wall stays out of reach.
const PHASE_SECONDS: f32 = 1.0;
const LOW: f32 = 0.8;
const HIGH: f32 = 2.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Small,
    Gold,
    Big,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Small => "small",
            Kind::Gold => "gold",
            Kind::Big => "big",
        }
    }

    pub fn speed(self) -> f32 {
        match self {
            Kind::Small => 3.0,
            Kind::Gold => 6.0,
            Kind::Big => 1.9,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Kind::Small => 0.45,
            Kind::Gold => 0.4,
            Kind::Big => 0.95,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Ghost {
    pub id: u16,
    pub kind: Kind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    heading: f32,
    wander_in: f32,
    height: f32,
    /// Seconds left passing through walls; nobody can catch it meanwhile.
    pub phase: f32,
    next_phase: f32,
    /// How far each vacuum has drawn it in, from 0 to 1.
    pub drawn: HashMap<u8, f32>,
    /// A big ghost's progress, which only rises with two cones on it.
    pub together: f32,
    /// Who had it in their cone last tick.
    pub pullers: Vec<u8>,
    pub age: f32,
}

impl Ghost {
    pub fn progress(&self) -> f32 {
        match self.kind {
            Kind::Big => self.together,
            _ => self.drawn.values().copied().fold(0.0, f32::max),
        }
    }

    fn at(&self) -> (f32, f32, f32) {
        (self.x, self.y, self.z)
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub seat: u8,
    pub body: Body,
    pub yaw: f32,
    pub pitch: f32,
    pub points: u32,
    pub ghosts: u32,
    /// Seconds since the trigger was last pulled.
    since_fire: f32,
    /// The motor's speed, from 0 to 1, spinning up while it is on.
    pub power: f32,
}

impl Player {
    pub fn sucking(&self) -> bool {
        self.since_fire < HOLD
    }

    fn eye(&self) -> (f32, f32, f32) {
        (self.body.x, self.body.y + EYE_HEIGHT, self.body.z)
    }
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    target: Option<u16>,
    think_in: f32,
    aim_yaw: f32,
    aim_pitch: f32,
    strafe: f32,
    idle: bool,
}

struct Spawn {
    kind: Kind,
    after: f32,
}

/// Columns and a pair of low tombs. Ghosts drift through them; vacuums do not
/// reach through them.
pub fn hall() -> Vec<Block> {
    let mut blocks = Vec::new();
    for (sx, sz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        blocks.push(Block {
            x: 6.0 * sx,
            z: 6.0 * sz,
            hx: 0.7,
            hz: 0.7,
            top: 4.2,
            bottom: 0.0,
        });
    }
    for sx in [1.0f32, -1.0] {
        blocks.push(Block {
            x: 0.0,
            z: 4.0 * sx,
            hx: 1.4,
            hz: 0.6,
            top: 0.9,
            bottom: 0.0,
        });
    }
    blocks
}

/// How hard a vacuum at `eye`, looking along `look`, pulls on a ghost of
/// `radius` at `at`, from 0 to 1, or None when it is out of the cone.
pub fn strength(eye: (f32, f32, f32), look: (f32, f32, f32), at: (f32, f32, f32), radius: f32) -> Option<f32> {
    let (dx, dy, dz) = (at.0 - eye.0, at.1 - eye.1, at.2 - eye.2);
    let away = (dx * dx + dy * dy + dz * dz).sqrt();
    if away > RANGE + radius {
        return None;
    }
    let near = (1.0 - (1.0 - FAR) * away / RANGE).clamp(FAR, 1.0);
    if away < 1.0 {
        return Some(near);
    }
    let along = ((dx * look.0 + dy * look.1 + dz * look.2) / away).clamp(-1.0, 1.0);
    let off = (along.acos() - (radius / away).atan()).max(0.0);
    let across = off / CONE;
    (across <= 1.0).then(|| near * (1.0 - (1.0 - EDGE) * across * across))
}

/// A spot on the ring at `angle`, moved in or out until it is clear of the
/// columns and tombs.
fn clear_spot(angle: f32, ring: f32, half: f32, blocks: &[Block]) -> (f32, f32) {
    let at = |r: f32| (angle.sin() * r, angle.cos() * r);
    let clear = |(x, z): (f32, f32)| !blocks.iter().any(|b| b.contains_xz(x, z, PLAYER_RADIUS + 0.4));
    (0..12)
        .flat_map(|step| [ring + step as f32 * 0.5, ring - step as f32 * 0.5])
        .filter(|r| *r < half - 1.0)
        .map(at)
        .find(|spot| clear(*spot))
        .unwrap_or_else(|| at(ring))
}

fn wrap_angle(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn round(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

pub struct GhostBusting {
    pub players: Vec<Player>,
    pub ghosts: Vec<Ghost>,
    pub time: f32,
    blocks: Vec<Block>,
    half: f32,
    rng: StdRng,
    next_id: u16,
    spawns: Vec<Spawn>,
    gold_in: f32,
    gold_most: usize,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl GhostBusting {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len();
        let size = crate::bout::crowd_scale(count);
        let half = HALL_HALF * size;
        let ring = half * 0.72;
        let blocks = crate::shooter::stretch(hall(), size);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle = i as f32 / count.max(1) as f32 * std::f32::consts::TAU;
                let (x, z) = clear_spot(angle, ring, half, &blocks);
                Player {
                    seat: c.seat,
                    body: Body {
                        x,
                        z,
                        on_ground: true,
                        ..Body::default()
                    },
                    yaw: x.atan2(z),
                    pitch: 0.0,
                    points: 0,
                    ghosts: 0,
                    since_fire: HOLD,
                    power: 0.0,
                }
            })
            .collect();
        let mut game = GhostBusting {
            players,
            ghosts: Vec::new(),
            time: 0.0,
            blocks,
            half,
            rng: StdRng::seed_from_u64(seed),
            next_id: 0,
            spawns: Vec::new(),
            gold_in: 0.0,
            gold_most: crate::bout::crowd_count(1, count),
            brains: contestants
                .iter()
                .filter(|c| c.is_bot)
                .map(|c| (c.seat, BotBrain::default()))
                .collect(),
            skills: contestants.iter().map(|c| (c.seat, Skill(c.skill))).collect(),
            placements: None,
        };
        game.gold_in = game.rng.random_range(6.0..10.0);
        for _ in 0..crate::bout::crowd_count(SMALL_GHOSTS, count) {
            game.add_ghost(Kind::Small, false);
        }
        for _ in 0..crate::bout::crowd_count(1, count) {
            game.add_ghost(Kind::Big, false);
        }
        game
    }

    pub fn half(&self) -> f32 {
        self.half
    }

    pub fn player(&self, seat: u8) -> Option<&Player> {
        self.players.iter().find(|p| p.seat == seat)
    }

    pub fn player_mut(&mut self, seat: u8) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.seat == seat)
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    /// A ghost somewhere in the hall, or slipping in through a wall.
    pub fn add_ghost(&mut self, kind: Kind, through_wall: bool) -> u16 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let inner = self.half - 1.5;
        let (x, z, heading, phase) = if through_wall {
            let along = self.rng.random_range(-inner..inner);
            let side = self.rng.random_range(0..4);
            let edge = self.half - 0.3;
            let (x, z) = match side {
                0 => (edge, along),
                1 => (-edge, along),
                2 => (along, edge),
                _ => (along, -edge),
            };
            (x, z, x.atan2(z), PHASE_SECONDS)
        } else {
            let mut spot = (0.0, 0.0);
            for _ in 0..20 {
                spot = (self.rng.random_range(-inner..inner), self.rng.random_range(-inner..inner));
                if !self.blocks.iter().any(|b| b.contains_xz(spot.0, spot.1, 1.0)) {
                    break;
                }
            }
            (spot.0, spot.1, self.rng.random_range(-3.2..3.2), 0.0)
        };
        let height = self.rng.random_range(LOW..HIGH);
        let next_phase = self.rng.random_range(6.0..13.0);
        self.ghosts.push(Ghost {
            id,
            kind,
            x,
            y: height,
            z,
            vx: 0.0,
            vz: 0.0,
            heading,
            wander_in: self.rng.random_range(0.8..2.0),
            height,
            phase,
            next_phase,
            drawn: HashMap::new(),
            together: 0.0,
            pullers: Vec::new(),
            age: 0.0,
        });
        id
    }

    fn step_players(&mut self, inputs: &[(u8, Input)], dt: f32) {
        let blocks = self.blocks.clone();
        let half = self.half;
        for player in self.players.iter_mut() {
            let input = inputs
                .iter()
                .find(|(s, _)| *s == player.seat)
                .map(|(_, i)| *i)
                .unwrap_or_default();
            if input.yaw.is_finite() {
                player.yaw = input.yaw;
            }
            if input.pitch.is_finite() {
                player.pitch = input.pitch.clamp(-1.4, 1.4);
            }
            player.since_fire = if input.fire { 0.0 } else { player.since_fire + dt };
            player.power = if player.sucking() {
                (player.power + dt / SPIN_UP).min(1.0)
            } else {
                (player.power - dt * 3.0).max(0.0)
            };
            fpmove::step_body(&mut player.body, input, dt, &blocks, half, half);
        }
    }

    /// Each ghost's pull from every vacuum on it this tick: who, how hard,
    /// and the nozzle it is drawn toward.
    fn pulls(&self, ghost: &Ghost) -> Vec<(u8, f32, (f32, f32, f32))> {
        if ghost.phase > 0.0 {
            return Vec::new();
        }
        self.players
            .iter()
            .filter(|p| p.power > 0.0 && p.sucking())
            .filter_map(|p| {
                let eye = p.eye();
                let look = fpmove::look(p.yaw, p.pitch);
                let pull = strength(eye, look, ghost.at(), ghost.kind.radius())?;
                if !fpmove::can_see(eye, ghost.at(), &self.blocks, self.half, self.half) {
                    return None;
                }
                let nozzle = (eye.0 + look.0 * 0.8, eye.1 - 0.3 + look.1 * 0.8, eye.2 + look.2 * 0.8);
                Some((p.seat, pull * p.power, nozzle))
            })
            .collect()
    }

    fn step_ghosts(&mut self, dt: f32, events: &mut Vec<Value>) {
        let cones: Vec<((f32, f32, f32), (f32, f32, f32), bool)> = self
            .players
            .iter()
            .map(|p| (p.eye(), fpmove::look(p.yaw, p.pitch), p.sucking()))
            .collect();
        let mut caught: Vec<(usize, Vec<u8>)> = Vec::new();
        let half = self.half;
        let blocks = self.blocks.clone();
        for index in 0..self.ghosts.len() {
            let pulls = self.pulls(&self.ghosts[index]);
            let turn = self.rng.random_range(-1.6f32..1.6);
            let wander = self.rng.random_range(0.8..2.2);
            let height = self.rng.random_range(LOW..HIGH);
            let rest = self.rng.random_range(6.0..13.0);
            let ghost = &mut self.ghosts[index];
            ghost.age += dt;
            ghost.pullers = pulls.iter().map(|(seat, _, _)| *seat).collect();

            match ghost.kind {
                Kind::Big => {
                    let mut strengths: Vec<f32> = pulls.iter().map(|(_, s, _)| *s).collect();
                    strengths.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                    if strengths.len() >= 2 {
                        ghost.together += (strengths[0] + strengths[1]) / 2.0 * dt / BIG_SECONDS;
                    } else {
                        ghost.together = (ghost.together - SLIP * dt).max(0.0);
                    }
                    if ghost.together >= 1.0 {
                        let mut ranked = pulls.clone();
                        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                        caught.push((index, ranked.iter().take(2).map(|(seat, _, _)| *seat).collect()));
                    }
                }
                Kind::Small | Kind::Gold => {
                    ghost.drawn.retain(|seat, drawn| {
                        if pulls.iter().any(|(s, _, _)| s == seat) {
                            true
                        } else {
                            *drawn -= SLIP * dt;
                            *drawn > 0.0
                        }
                    });
                    for (seat, pull, _) in &pulls {
                        *ghost.drawn.entry(*seat).or_insert(0.0) += pull * dt / CATCH_SECONDS;
                    }
                    let best = ghost
                        .drawn
                        .iter()
                        .filter(|(_, drawn)| **drawn >= 1.0)
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(seat, _)| *seat);
                    if let Some(seat) = best {
                        caught.push((index, vec![seat]));
                    }
                }
            }

            let speed = ghost.kind.speed();
            ghost.wander_in -= dt;
            if ghost.wander_in <= 0.0 {
                ghost.wander_in = wander;
                ghost.heading += turn;
                ghost.height = height;
            }
            let mut want = (-ghost.heading.sin() * speed, -ghost.heading.cos() * speed);

            // Slip sideways out of any cone pointed this way, harder from a
            // vacuum that is running.
            for (eye, look, on) in &cones {
                let (dx, dy, dz) = (ghost.x - eye.0, ghost.y - eye.1, ghost.z - eye.2);
                let away = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
                if away > RANGE * 1.3 {
                    continue;
                }
                let along = (dx * look.0 + dy * look.1 + dz * look.2) / away;
                if along < (CONE * 1.8).cos() {
                    continue;
                }
                let (mut sx, mut sz) = (dx - look.0 * along * away, dz - look.2 * along * away);
                let side = (sx * sx + sz * sz).sqrt();
                if side < 0.05 {
                    (sx, sz) = (-look.2, look.0);
                } else {
                    sx /= side;
                    sz /= side;
                }
                let urgency = match (*on, ghost.kind) {
                    (false, _) => 0.5,
                    (true, Kind::Gold) => 2.6,
                    (true, _) => 2.0,
                };
                want.0 += sx * speed * urgency;
                want.1 += sz * speed * urgency;
            }

            // Held near the middle of a cone it barely struggles; at the
            // edge it can still wriggle free.
            let total: f32 = (pulls.iter().map(|(_, s, _)| *s).sum::<f32>() * 1.2).min(1.0);
            let held = match ghost.kind {
                Kind::Big if pulls.len() < 2 => 0.6,
                _ => 0.85,
            };
            want.0 *= 1.0 - total * held;
            want.1 *= 1.0 - total * held;
            let blend = (dt * 4.0).min(1.0);
            ghost.vx += (want.0 - ghost.vx) * blend;
            ghost.vz += (want.1 - ghost.vz) * blend;
            ghost.x += ghost.vx * dt;
            ghost.z += ghost.vz * dt;
            ghost.y += (ghost.height - ghost.y) * (dt * 1.5).min(1.0);

            let progress = ghost.progress();
            if let Some((_, _, nozzle)) = pulls
                .iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                && progress > 0.0
            {
                let reel = (progress * 2.5 * dt).min(0.5);
                ghost.x += (nozzle.0 - ghost.x) * reel;
                ghost.y += (nozzle.1 - ghost.y) * reel;
                ghost.z += (nozzle.2 - ghost.z) * reel;
            }

            if ghost.phase > 0.0 {
                ghost.phase -= dt;
                // Through the outer wall and out of the one opposite.
                let edge = half + 0.6;
                if ghost.x.abs() > edge {
                    ghost.x = -ghost.x.signum() * (half - 0.4);
                }
                if ghost.z.abs() > edge {
                    ghost.z = -ghost.z.signum() * (half - 0.4);
                }
                if ghost.phase <= 0.0 {
                    let inner = half - 0.6;
                    ghost.x = ghost.x.clamp(-inner, inner);
                    ghost.z = ghost.z.clamp(-inner, inner);
                }
            } else {
                let r = ghost.kind.radius();
                let inner = half - r - 0.2;
                if ghost.x.abs() > inner {
                    ghost.x = ghost.x.clamp(-inner, inner);
                    ghost.vx = -ghost.vx;
                    ghost.heading = (-ghost.vx).atan2(-ghost.vz);
                }
                if ghost.z.abs() > inner {
                    ghost.z = ghost.z.clamp(-inner, inner);
                    ghost.vz = -ghost.vz;
                    ghost.heading = (-ghost.vx).atan2(-ghost.vz);
                }
                for block in &blocks {
                    if ghost.y - r > block.top || !block.contains_xz(ghost.x, ghost.z, r) {
                        continue;
                    }
                    let push_x = block.hx + r - (ghost.x - block.x).abs();
                    let push_z = block.hz + r - (ghost.z - block.z).abs();
                    if push_x < push_z {
                        ghost.x += push_x * (ghost.x - block.x).signum();
                        ghost.vx = -ghost.vx;
                    } else {
                        ghost.z += push_z * (ghost.z - block.z).signum();
                        ghost.vz = -ghost.vz;
                    }
                    ghost.heading = (-ghost.vx).atan2(-ghost.vz);
                }
                ghost.next_phase -= dt;
                if ghost.next_phase <= 0.0 && progress < 0.2 {
                    ghost.next_phase = rest;
                    ghost.drawn.clear();
                    ghost.together = 0.0;
                    // Off to the nearest wall, through it, and in from the far side.
                    let (ax, az) = (ghost.x.abs(), ghost.z.abs());
                    let to_wall = match (ax > az, ghost.x > 0.0, ghost.z > 0.0) {
                        (true, true, _) => -std::f32::consts::FRAC_PI_2,
                        (true, false, _) => std::f32::consts::FRAC_PI_2,
                        (false, _, true) => std::f32::consts::PI,
                        (false, _, false) => 0.0,
                    };
                    let pace = speed * 1.4;
                    ghost.phase = ((half - ax.max(az)) / pace + 0.7).clamp(0.8, 3.0);
                    ghost.heading = to_wall;
                    ghost.vx = -to_wall.sin() * pace;
                    ghost.vz = -to_wall.cos() * pace;
                    events.push(json!({ "kind": "phase", "id": ghost.id }));
                }
            }
        }

        caught.sort_by(|a, b| b.0.cmp(&a.0));
        for (index, seats) in caught {
            let ghost = self.ghosts.remove(index);
            let points = match ghost.kind {
                Kind::Small => SMALL_POINTS,
                Kind::Gold => GOLD_POINTS,
                Kind::Big => BIG_POINTS,
            };
            let share = points / seats.len().max(1) as u32;
            for seat in &seats {
                if let Some(player) = self.player_mut(*seat) {
                    player.points += share;
                    player.ghosts += 1;
                }
            }
            events.push(json!({
                "kind": "caught",
                "id": ghost.id,
                "ghost": ghost.kind.name(),
                "seats": seats,
                "points": share,
                "x": round(ghost.x),
                "y": round(ghost.y),
                "z": round(ghost.z),
            }));
            if ghost.kind != Kind::Gold {
                let after = self.rng.random_range(1.5..3.0);
                self.spawns.push(Spawn { kind: ghost.kind, after });
            }
        }

        // A golden ghost that nobody caught slips away for good.
        let mut gone = Vec::new();
        self.ghosts.retain(|g| {
            let leave = g.kind == Kind::Gold && g.age > GOLD_LIFE && g.progress() < 0.2;
            if leave {
                gone.push(g.id);
            }
            !leave
        });
        for id in gone {
            events.push(json!({ "kind": "escape", "id": id }));
        }
    }

    fn step_spawns(&mut self, dt: f32, events: &mut Vec<Value>) {
        let mut ready = Vec::new();
        self.spawns.retain_mut(|spawn| {
            spawn.after -= dt;
            if spawn.after <= 0.0 {
                ready.push(spawn.kind);
                false
            } else {
                true
            }
        });
        self.gold_in -= dt;
        let golds = self.ghosts.iter().filter(|g| g.kind == Kind::Gold).count();
        if self.gold_in <= 0.0 && golds < self.gold_most && self.time < ROUND_SECONDS - 4.0 {
            self.gold_in = self.rng.random_range(9.0..15.0);
            ready.push(Kind::Gold);
        }
        for kind in ready {
            let id = self.add_ghost(kind, true);
            events.push(json!({ "kind": "spawn", "id": id, "ghost": kind.name() }));
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < ROUND_SECONDS {
            return;
        }
        let mut rows: Vec<(u32, u32, u8)> = self.players.iter().map(|p| (p.points, p.ghosts, p.seat)).collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, u32)> = None;
        for (points, ghosts, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (points, ghosts) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((points, ghosts));
        }
        self.placements = Some(groups);
    }

    /// What a bot is worth chasing: the ghost and how much it wants it.
    fn pick_target(&self, seat: u8) -> Option<u16> {
        let me = self.player(seat)?;
        let eye = me.eye();
        let skill = self.skill(seat).level();
        self.ghosts
            .iter()
            .filter(|g| g.phase <= 0.0)
            .map(|g| {
                let (dx, dz) = (g.x - me.body.x, g.z - me.body.z);
                let away = (dx * dx + dz * dz).sqrt();
                let others = self
                    .brains
                    .iter()
                    .any(|(s, b)| *s != seat && b.target == Some(g.id))
                    || g.pullers.iter().any(|s| *s != seat);
                let value = match g.kind {
                    Kind::Small => 1.0,
                    Kind::Gold => 1.5 + 2.0 * skill,
                    Kind::Big if others => 2.5,
                    Kind::Big => 0.25,
                };
                let seen = fpmove::can_see(eye, g.at(), &self.blocks, self.half, self.half);
                let score = value / (away + 3.0) * if seen { 1.0 } else { 0.5 };
                (g.id, score)
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| id)
    }
}

impl Bout for GhostBusting {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "half_x": self.half,
            "half_z": self.half,
            "eye": EYE_HEIGHT,
            "height": PLAYER_HEIGHT,
            "radius": PLAYER_RADIUS,
            "range": RANGE,
            "cone": CONE,
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
        self.step_players(inputs, dt);
        self.step_ghosts(dt, events);
        self.step_spawns(dt, events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        let skill = self.skill(seat);
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = skill.reaction() * self.rng.random_range(0.6..1.2);
            brain.target = self.pick_target(seat);
            let wobble = skill.aim_error() * 0.5;
            brain.aim_yaw = self.rng.random_range(-wobble..=wobble);
            brain.aim_pitch = self.rng.random_range(-wobble..=wobble) * 0.5;
            brain.strafe = [-1.0, 0.0, 1.0][self.rng.random_range(0..3)];
            brain.idle = self.rng.random_bool(skill.hesitation() * 0.5);
        }
        let mut input = Input {
            yaw: me.yaw,
            pitch: me.pitch,
            ..Default::default()
        };
        let target = brain.target.and_then(|id| self.ghosts.iter().find(|g| g.id == id)).cloned();
        if let Some(ghost) = target {
            let eye = me.eye();
            let (dx, dy, dz) = (ghost.x - eye.0, ghost.y - eye.1, ghost.z - eye.2);
            let flat = (dx * dx + dz * dz).sqrt().max(0.01);
            // A shaky hand drifts off the ghost now and then.
            let sway = (1.0 - skill.level()) * 0.3 * (self.time * 2.3 + seat as f32 * 1.7).sin();
            let want_yaw = (-dx).atan2(-dz) + brain.aim_yaw + sway;
            let want_pitch = dy.atan2(flat) + brain.aim_pitch;
            let track = (skill.tracking() * dt).min(1.0);
            input.yaw = me.yaw + wrap_angle(want_yaw - me.yaw) * track;
            input.pitch = (me.pitch + (want_pitch - me.pitch) * track).clamp(-1.4, 1.4);
            let look = fpmove::look(input.yaw, input.pitch);
            let away = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
            let along = (dx * look.0 + dy * look.1 + dz * look.2) / away;
            let on_it = along > (CONE * 0.9).cos() && away < RANGE;
            // Keep the motor running once the ghost is coming in.
            input.fire = !brain.idle && (on_it || (me.sucking() && ghost.progress() > 0.0));
            let forward = if away > RANGE * 0.55 {
                1.0
            } else if away < 2.5 {
                -0.6
            } else {
                0.0
            };
            input.stick = Stick {
                x: brain.strafe * 0.4,
                z: -forward,
            }
            .clamped();
            input.stick.x *= skill.pace();
            input.stick.z *= skill.pace();
        } else {
            input.yaw = me.yaw + dt * 1.2;
            input.stick = Stick { x: 0.0, z: -0.5 * skill.pace() };
        }
        self.brains.insert(seat, brain);
        input
    }

    fn snapshot(&self) -> Value {
        json!({
            "game": KEY,
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
                "ground": p.body.on_ground,
                "power": round(if p.sucking() { p.power } else { 0.0 }),
                "points": p.points,
                "ghosts": p.ghosts,
            })).collect::<Vec<_>>(),
            "ghosts": self.ghosts.iter().map(|g| json!({
                "id": g.id,
                "kind": g.kind.name(),
                "x": round(g.x),
                "y": round(g.y),
                "z": round(g.z),
                "vx": round(g.vx),
                "vz": round(g.vz),
                "phase": g.phase > 0.0,
                "drawn": round(g.progress().min(1.0)),
                "pullers": g.pullers,
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
    use crate::bout::TICK_DT;

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    /// A game with only the ghosts a test puts in, and nobody moving them.
    fn empty(count: u8) -> GhostBusting {
        let mut game = GhostBusting::new(&people(count), 3);
        game.ghosts.clear();
        game.gold_in = f32::MAX;
        game
    }

    /// Stands `seat` at `x, z` looking straight down -z at eye height.
    fn stand(game: &mut GhostBusting, seat: u8, x: f32, z: f32) {
        let player = game.player_mut(seat).unwrap();
        player.body.x = x;
        player.body.z = z;
        player.body.y = 0.0;
        player.yaw = 0.0;
        player.pitch = 0.0;
    }

    /// A ghost of `kind` parked at `x, z` at eye height, too slow to get away.
    fn park(game: &mut GhostBusting, kind: Kind, x: f32, z: f32) -> u16 {
        let id = game.add_ghost(kind, false);
        let ghost = game.ghosts.iter_mut().find(|g| g.id == id).unwrap();
        ghost.x = x;
        ghost.z = z;
        ghost.y = EYE_HEIGHT;
        ghost.height = EYE_HEIGHT;
        ghost.next_phase = 100.0;
        id
    }

    fn suck(seats: &[u8]) -> Vec<(u8, Input)> {
        seats
            .iter()
            .map(|s| (*s, Input { fire: true, ..Default::default() }))
            .collect()
    }

    /// Everyone in `seats` sucking with their eyes on ghost `id`, as a
    /// player following it with the mouse would.
    fn chase(game: &GhostBusting, seats: &[u8], id: u16) -> Vec<(u8, Input)> {
        let Some(ghost) = game.ghosts.iter().find(|g| g.id == id) else {
            return suck(seats);
        };
        seats
            .iter()
            .map(|s| {
                let eye = game.player(*s).unwrap().eye();
                let (dx, dy, dz) = (ghost.x - eye.0, ghost.y - eye.1, ghost.z - eye.2);
                let input = Input {
                    fire: true,
                    yaw: (-dx).atan2(-dz),
                    pitch: dy.atan2((dx * dx + dz * dz).sqrt()),
                    ..Default::default()
                };
                (*s, input)
            })
            .collect()
    }

    fn chase_for(game: &mut GhostBusting, seats: &[u8], id: u16, seconds: f32, events: &mut Vec<Value>) {
        for _ in 0..(seconds / TICK_DT) as usize {
            let inputs = chase(game, seats, id);
            game.step(&inputs, TICK_DT, events);
        }
    }

    fn run(game: &mut GhostBusting, inputs: &[(u8, Input)], seconds: f32, events: &mut Vec<Value>) {
        for _ in 0..(seconds / TICK_DT) as usize {
            game.step(inputs, TICK_DT, events);
        }
    }

    fn caught(events: &[Value]) -> Vec<&Value> {
        events.iter().filter(|e| e["kind"] == "caught").collect()
    }

    #[test]
    fn the_cone_pulls_hardest_dead_centre_and_not_at_all_outside() {
        let eye = (0.0, 1.0, 0.0);
        let look = fpmove::look(0.0, 0.0);
        let centre = strength(eye, look, (0.0, 1.0, -4.0), 0.0).unwrap();
        let edge = strength(eye, look, (4.0 * (CONE * 0.9).sin(), 1.0, -4.0 * (CONE * 0.9).cos()), 0.0).unwrap();
        assert!(centre > edge * 2.0, "centre {centre} against edge {edge}");
        assert!(strength(eye, look, (0.0, 1.0, -(RANGE + 1.0)), 0.0).is_none(), "out of reach");
        assert!(strength(eye, look, (4.0, 1.0, 0.0), 0.0).is_none(), "off to the side");
        assert!(strength(eye, look, (0.0, 1.0, 4.0), 0.0).is_none(), "behind");
    }

    #[test]
    fn holding_a_small_ghost_in_the_cone_catches_it_for_a_point_in_about_a_second() {
        let mut game = empty(2);
        stand(&mut game, 0, 0.0, 9.0);
        stand(&mut game, 1, 9.0, -9.0);
        let id = park(&mut game, Kind::Small, 0.0, 6.0);
        let mut events = Vec::new();
        let mut seconds = 0.0;
        while caught(&events).is_empty() && seconds < 3.0 {
            let inputs = chase(&game, &[0], id);
            game.step(&inputs, TICK_DT, &mut events);
            seconds += TICK_DT;
        }
        let catch = caught(&events);
        assert_eq!(catch.len(), 1, "the ghost was drawn in");
        assert_eq!(catch[0]["seats"], json!([0]));
        assert!((0.8..1.8).contains(&seconds), "it took about a second: {seconds}");
        let me = game.player(0).unwrap();
        assert_eq!((me.points, me.ghosts), (SMALL_POINTS, 1));
        assert_eq!(game.player(1).unwrap().points, 0);
        assert!(game.ghosts.is_empty(), "the caught ghost is gone");

        run(&mut game, &[], 3.5, &mut events);
        assert_eq!(game.ghosts.len(), 1, "another drifts in through a wall");
        assert!(events.iter().any(|e| e["kind"] == "spawn"));
    }

    #[test]
    fn a_ghost_let_go_slips_back_out() {
        let mut game = empty(1);
        stand(&mut game, 0, 0.0, 9.0);
        let id = park(&mut game, Kind::Small, 0.0, 6.0);
        let mut events = Vec::new();
        chase_for(&mut game, &[0], id, 0.6, &mut events);
        let drawn = game.ghosts.iter().find(|g| g.id == id).unwrap().progress();
        assert!(drawn > 0.1, "it was coming in: {drawn}");
        run(&mut game, &[], 1.5, &mut events);
        assert!(caught(&events).is_empty());
        let ghost = game.ghosts.iter().find(|g| g.id == id).unwrap();
        assert_eq!(ghost.progress(), 0.0, "and got away");
    }

    #[test]
    fn a_big_ghost_needs_two_cones_and_splits_four_points() {
        let mut game = empty(3);
        stand(&mut game, 0, -1.0, 9.0);
        stand(&mut game, 1, 1.0, 9.0);
        stand(&mut game, 2, -9.0, -9.0);
        let id = park(&mut game, Kind::Big, 0.0, 5.0);
        let mut events = Vec::new();
        chase_for(&mut game, &[0], id, 4.0, &mut events);
        assert!(caught(&events).is_empty(), "one vacuum alone cannot hold it");
        assert_eq!(game.ghosts[0].progress(), 0.0);
        assert_eq!(game.ghosts[0].pullers, vec![0], "though it was in the cone");

        chase_for(&mut game, &[0, 1], id, 3.0, &mut events);
        let catch = caught(&events);
        assert_eq!(catch.len(), 1, "two together draw it in");
        assert_eq!(catch[0]["ghost"], "big");
        for seat in [0, 1] {
            let player = game.player(seat).unwrap();
            assert_eq!((player.points, player.ghosts), (BIG_POINTS / 2, 1), "seat {seat} gets half");
        }
        assert_eq!(game.player(2).unwrap().points, 0);
    }

    #[test]
    fn a_golden_ghost_is_worth_three_and_a_phasing_one_cannot_be_caught() {
        let mut game = empty(1);
        stand(&mut game, 0, 0.0, 9.0);
        let id = park(&mut game, Kind::Gold, 0.0, 6.0);
        game.ghosts[0].phase = 100.0;
        let mut events = Vec::new();
        chase_for(&mut game, &[0], id, 1.0, &mut events);
        assert!(caught(&events).is_empty(), "it slips through the vacuum while phasing");
        let ghost = game.ghosts.iter_mut().find(|g| g.id == id).unwrap();
        ghost.phase = 0.0;
        ghost.x = 0.0;
        ghost.z = 6.0;
        ghost.y = EYE_HEIGHT;
        chase_for(&mut game, &[0], id, 2.0, &mut events);
        assert_eq!(caught(&events).len(), 1);
        assert_eq!(game.player(0).unwrap().points, GOLD_POINTS);
    }

    #[test]
    fn a_column_shields_a_ghost_behind_it() {
        let mut game = empty(1);
        let column = game.blocks[0];
        stand(&mut game, 0, column.x, column.z + 4.0);
        let player = game.player_mut(0).unwrap();
        player.power = 1.0;
        player.since_fire = 0.0;
        park(&mut game, Kind::Small, column.x, column.z - 2.0);
        assert!(game.pulls(&game.ghosts[0]).is_empty(), "behind the column");
        game.ghosts[0].x = column.x;
        game.ghosts[0].z = column.z + 1.5;
        assert_eq!(game.pulls(&game.ghosts[0]).len(), 1, "in front of it");
    }

    #[test]
    fn a_ghost_phases_out_through_a_wall_and_in_from_the_far_side() {
        let mut game = empty(1);
        stand(&mut game, 0, -9.0, -9.0);
        let id = park(&mut game, Kind::Small, HALL_HALF - 2.0, 0.0);
        game.ghosts[0].next_phase = 0.0;
        let mut events = Vec::new();
        run(&mut game, &[], TICK_DT * 2.0, &mut events);
        assert!(events.iter().any(|e| e["kind"] == "phase" && e["id"] == id));
        let mut crossed = false;
        for _ in 0..(3.0 / TICK_DT) as usize {
            game.step(&[], TICK_DT, &mut events);
            crossed |= game.ghosts[0].x < 0.0;
        }
        assert!(crossed, "it came back in on the other side");
        assert!(game.ghosts[0].x.abs() < HALL_HALF, "and is back inside the hall");
    }

    #[test]
    fn ghosts_dodge_a_running_vacuum() {
        let mut game = empty(1);
        stand(&mut game, 0, 0.0, 9.0);
        let id = park(&mut game, Kind::Small, 0.0, 1.0);
        let mut events = Vec::new();
        run(&mut game, &suck(&[0]), 1.0, &mut events);
        let ghost = game.ghosts.iter().find(|g| g.id == id).unwrap();
        assert!(ghost.x.abs() > 1.0, "it slid out of the cone: {}", ghost.x);
    }

    #[test]
    fn the_round_ends_on_the_clock_and_ranks_by_points_then_ghosts() {
        let mut game = empty(4);
        game.player_mut(0).unwrap().points = 3;
        game.player_mut(0).unwrap().ghosts = 1;
        game.player_mut(1).unwrap().points = 3;
        game.player_mut(1).unwrap().ghosts = 3;
        game.player_mut(2).unwrap().points = 1;
        game.player_mut(2).unwrap().ghosts = 1;
        game.player_mut(3).unwrap().points = 1;
        game.player_mut(3).unwrap().ghosts = 1;
        let mut events = Vec::new();
        run(&mut game, &[], ROUND_SECONDS - 1.0, &mut events);
        assert!(game.placements().is_none());
        run(&mut game, &[], 1.5, &mut events);
        assert_eq!(game.placements(), Some(vec![vec![1], vec![0], vec![2, 3]]));
    }

    #[test]
    fn bots_catch_ghosts_and_sharp_ones_catch_more() {
        let haul = |skill: f32| {
            let seats: Vec<Contestant> = (0..4).map(|s| Contestant::bot(s, skill)).collect();
            let mut total = 0;
            for seed in 0..3 {
                let mut game = GhostBusting::new(&seats, seed);
                let mut events = Vec::new();
                while game.placements().is_none() {
                    let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
                    game.step(&inputs, TICK_DT, &mut events);
                }
                assert_eq!(game.placements().unwrap().iter().map(|g| g.len()).sum::<usize>(), 4);
                total += caught(&events).len();
            }
            total
        };
        let (sharp, relaxed) = (haul(1.0), haul(0.15));
        assert!(relaxed > 0, "even relaxed bots catch some: {relaxed}");
        assert!(sharp > relaxed, "sharp {sharp} against relaxed {relaxed}");
    }

    #[test]
    fn bots_team_up_on_a_big_ghost() {
        let seats: Vec<Contestant> = (0..4).map(|s| Contestant::bot(s, 0.7)).collect();
        let mut bigs = 0;
        for seed in 0..4 {
            let mut game = GhostBusting::new(&seats, seed);
            let mut events = Vec::new();
            while game.placements().is_none() {
                let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
                game.step(&inputs, TICK_DT, &mut events);
            }
            bigs += caught(&events).iter().filter(|e| e["ghost"] == "big").count();
        }
        assert!(bigs > 0, "bots caught a big ghost together");
    }

    #[test]
    fn a_bigger_crowd_gets_a_bigger_hall_and_more_ghosts() {
        let four = GhostBusting::new(&people(4), 1);
        let eight = GhostBusting::new(&people(8), 1);
        assert_eq!(four.half(), HALL_HALF);
        assert!((eight.half() / HALL_HALF - 2f32.sqrt()).abs() < 1e-4);
        let small = |g: &GhostBusting| g.ghosts.iter().filter(|g| g.kind == Kind::Small).count();
        let big = |g: &GhostBusting| g.ghosts.iter().filter(|g| g.kind == Kind::Big).count();
        assert_eq!((small(&four), big(&four)), (SMALL_GHOSTS, 1));
        assert_eq!((small(&eight), big(&eight)), (SMALL_GHOSTS * 2, 2));
        assert!(eight.players.iter().all(|p| p.body.x.abs() < eight.half() - 1.0 && p.body.z.abs() < eight.half() - 1.0));
    }

    #[test]
    fn nobody_starts_inside_a_column_or_a_tomb() {
        for count in 2..=8 {
            let game = GhostBusting::new(&people(count), 1);
            for player in &game.players {
                let (x, z) = (player.body.x, player.body.z);
                assert!(
                    !game.blocks.iter().any(|b| b.contains_xz(x, z, PLAYER_RADIUS)),
                    "seat {} of {count} starts at {x}, {z}",
                    player.seat
                );
            }
        }
    }
}
