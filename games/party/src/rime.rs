//! Rime Rink: steer with momentum across ice to ring the next lit floor bell.
//! A bell moves after a clean crossing or when its short window expires.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::cmp::Ordering;

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};

pub const KEY: &str = "rime";
pub const LIMIT: f32 = 68.0;
pub const HALF: f32 = 11.0;
pub const BELL_RADIUS: f32 = 1.2;
const SKATER_RADIUS: f32 = 0.48;
const ACCEL: f32 = 11.0;
const DRAG: f32 = 0.8;
const MAX_SPEED: f32 = 7.2;
const BELL_WINDOW: f32 = 5.5;
const RELOCATE: f32 = 0.8;
const MARKS: [(f32, f32); 8] = [
    (-7.0, -6.8),
    (0.0, -7.5),
    (7.0, -6.8),
    (8.0, 0.0),
    (7.0, 6.8),
    (0.0, 7.5),
    (-7.0, 6.8),
    (-8.0, 0.0),
];

#[derive(Clone, Debug)]
pub struct Skater {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub score: u32,
    pub bells: u32,
    pub body_width: f32,
    skill: f32,
    bot_at: f32,
    bot_bias: f32,
}
pub struct RimeRink {
    pub players: Vec<Skater>,
    pub time: f32,
    pub bell: usize,
    pub window: f32,
    pub waiting: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
impl RimeRink {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle = i as f32 * std::f32::consts::TAU / contestants.len().max(1) as f32;
                Skater {
                    seat: c.seat,
                    x: angle.cos() * 3.5,
                    z: angle.sin() * 3.5,
                    vx: 0.0,
                    vz: 0.0,
                    score: 0,
                    bells: 0,
                    body_width: BodyProfile::new(c.body_width).width,
                    skill: c.skill.clamp(0.0, 1.0),
                    bot_at: rng.random_range(0.0..0.6),
                    bot_bias: rng.random_range(-0.18..0.18),
                }
            })
            .collect();
        Self {
            players,
            time: 0.0,
            bell: rng.random_range(0..MARKS.len()),
            window: BELL_WINDOW,
            waiting: 0.0,
            rng,
            result: None,
        }
    }
    fn next_bell(&mut self) {
        let old = self.bell;
        self.bell = (old + 2 + self.rng.random_range(0..MARKS.len() - 3)) % MARKS.len();
        self.window = BELL_WINDOW;
        self.waiting = 0.0;
    }
    fn finish(&mut self) {
        let mut ranked = self
            .players
            .iter()
            .map(|p| (p.seat, p.score, p.bells))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score, bells) in ranked {
            if prior != Some((score, bells)) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some((score, bells));
        }
        self.result = Some(places);
    }
}
impl Bout for RimeRink {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"half":HALF,"seconds":LIMIT})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        for p in &mut self.players {
            let body = BodyProfile::new(p.body_width);
            let stick = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, input)| input.stick.clamped())
                .unwrap_or_default();
            let drag = (1.0 - DRAG * dt / body.mass).max(0.0);
            p.vx = (p.vx + stick.x * ACCEL * body.acceleration * dt) * drag;
            p.vz = (p.vz + stick.z * ACCEL * body.acceleration * dt) * drag;
            let speed = (p.vx * p.vx + p.vz * p.vz).sqrt();
            let cap = MAX_SPEED * body.speed;
            if speed > cap {
                p.vx *= cap / speed;
                p.vz *= cap / speed;
            }
            p.x += p.vx * dt;
            p.z += p.vz * dt;
            for (at, velocity) in [(&mut p.x, &mut p.vx), (&mut p.z, &mut p.vz)] {
                let edge = HALF - SKATER_RADIUS * body.radius;
                if at.abs() > edge {
                    *at = at.clamp(-edge, edge);
                    *velocity *= -0.32;
                }
            }
        }
        for a in 0..self.players.len() {
            for b in a + 1..self.players.len() {
                let (left, right) = self.players.split_at_mut(b);
                let p = &mut left[a];
                let q = &mut right[0];
                let a_body = BodyProfile::new(p.body_width);
                let b_body = BodyProfile::new(q.body_width);
                let dx = q.x - p.x;
                let dz = q.z - p.z;
                let d = (dx * dx + dz * dz).sqrt();
                let contact = SKATER_RADIUS * (a_body.radius + b_body.radius);
                if d > 0.001 && d < contact {
                    let nx = dx / d;
                    let nz = dz / d;
                    let overlap = contact - d;
                    let total_mass = a_body.mass + b_body.mass;
                    p.x -= nx * overlap * b_body.mass / total_mass;
                    q.x += nx * overlap * a_body.mass / total_mass;
                    p.z -= nz * overlap * b_body.mass / total_mass;
                    q.z += nz * overlap * a_body.mass / total_mass;
                    let rel = (p.vx - q.vx) * nx + (p.vz - q.vz) * nz;
                    if rel > 0.0 {
                        let impulse = 1.1 * rel / (1.0 / a_body.mass + 1.0 / b_body.mass);
                        p.vx -= nx * impulse / a_body.mass;
                        p.vz -= nz * impulse / a_body.mass;
                        q.vx += nx * impulse / b_body.mass;
                        q.vz += nz * impulse / b_body.mass;
                    }
                }
            }
        }
        if self.waiting > 0.0 {
            self.waiting -= dt;
            if self.waiting <= 0.0 {
                self.next_bell();
            }
        } else {
            self.window -= dt;
            let (bx, bz) = MARKS[self.bell];
            let winner = self
                .players
                .iter()
                .filter(|p| (p.x - bx).hypot(p.z - bz)
                    < BELL_RADIUS * BodyProfile::new(p.body_width).reach)
                .min_by(|a, b| {
                    let da = (a.x - bx).hypot(a.z - bz);
                    let db = (b.x - bx).hypot(b.z - bz);
                    da.partial_cmp(&db)
                        .unwrap_or(Ordering::Equal)
                        .then(a.seat.cmp(&b.seat))
                })
                .map(|p| p.seat);
            if let Some(seat) = winner {
                let p = self.players.iter_mut().find(|p| p.seat == seat).unwrap();
                p.score += 2;
                p.bells += 1;
                events.push(json!({"kind":"rime_bell","seat":seat,"bell":self.bell}));
                self.waiting = RELOCATE;
            } else if self.window <= 0.0 {
                events.push(json!({"kind":"rime_move"}));
                self.waiting = RELOCATE;
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.result.is_some() || self.waiting > 0.0 {
            return Input::default();
        }
        let Some(p) = self.players.iter_mut().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.time < p.bot_at {
            return Input::default();
        }
        let (bx, bz) = MARKS[self.bell];
        let look = 0.05 + p.skill * 0.35;
        let dx = bx - p.x - p.vx * look + p.bot_bias;
        let dz = bz - p.z - p.vz * look - p.bot_bias;
        let length = (dx * dx + dz * dz).sqrt().max(0.001);
        let pace = 0.55 + p.skill * 0.45;
        Input::from_stick(Stick {
            x: dx / length * pace,
            z: dz / length * pace,
        })
    }
    fn snapshot(&self) -> Value {
        let (bx, bz) = MARKS[self.bell];
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,"bell":self.bell,"bell_x":bx,"bell_z":bz,"window":self.window.max(0.0),"waiting":self.waiting.max(0.0),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"yaw":p.vx.atan2(p.vz),"score":p.score,"bells":p.bells,"body_width":p.body_width})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_rings_once_then_relocates() {
        let mut game = RimeRink::new(&[Contestant::person(0), Contestant::person(1)], 8);
        let first_bell = game.bell;
        let (x, z) = MARKS[first_bell];
        game.players[0].x = x;
        game.players[0].z = z;
        game.step(&[], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 2);
        game.step(&[], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 2);
        for _ in 0..60 {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        assert_ne!(game.bell, first_bell);
    }
    #[test]
    fn malformed_motion_cannot_escape_rink() {
        let mut game = RimeRink::new(&[Contestant::person(0), Contestant::person(1)], 8);
        for _ in 0..1000 {
            game.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::NAN,
                        z: 100.0,
                    }),
                )],
                1.0 / 60.0,
                &mut vec![],
            );
        }
        assert!(game.players[0].x.abs() <= HALF && game.players[0].z.abs() <= HALF);
    }
    #[test]
    fn bots_finish_with_absent_player() {
        let players = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = RimeRink::new(&players, 8);
        for _ in 0..(LIMIT as usize * 60 + 2) {
            let inputs = (0..7)
                .map(|s| (s, game.bot_input(s, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.score > 0));
    }
    #[test]
    fn small_skater_turns_faster_while_broad_skater_glides_and_holds_space() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = RimeRink::new(&[small, broad], 8);
        game.players[0].x = -4.0;
        game.players[1].x = 4.0;
        game.players[0].z = 0.0;
        game.players[1].z = 0.0;
        for _ in 0..60 {
            game.step(&[(0, Input::from_stick(Stick { x: 1.0, z: 0.0 })),
                (1, Input::from_stick(Stick { x: 1.0, z: 0.0 }))], 1.0 / 60.0, &mut Vec::new());
        }
        assert!(game.players[0].vx > game.players[1].vx);
        let before = [game.players[0].vx, game.players[1].vx];
        game.step(&[], 0.1, &mut Vec::new());
        assert!(game.players[1].vx / before[1] > game.players[0].vx / before[0]);

        game.players[0].x = 0.8;
        game.players[1].x = 0.0;
        game.players[0].z = 0.0;
        game.players[1].z = 0.0;
        game.players[0].vx = 0.0;
        game.players[1].vx = 0.0;
        game.step(&[], 0.0, &mut Vec::new());
        assert!((game.players[1].x).abs() < game.players[0].x - 0.8,
            "the broad skater yields less space in a collision");
    }
    #[test]
    fn broad_skater_can_ring_a_bell_from_farther_away() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = RimeRink::new(&[small, broad], 8);
        let (x, z) = MARKS[game.bell];
        game.players[0].x = x + 1.21;
        game.players[0].z = z;
        game.players[1].x = x - 1.21;
        game.players[1].z = z;
        game.step(&[], 0.0, &mut Vec::new());
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.players[1].score, 2);
    }
    #[test]
    fn mixed_build_bots_each_find_bells_across_seeds() {
        let mut totals = [0u32; 4];
        for seed in 0..8 {
            let players = (0..4).map(|seat| {
                let mut c = Contestant::bot(seat, 0.7);
                c.body_width = [0.7, 0.9, 1.2, 1.4][seat as usize];
                c
            }).collect::<Vec<_>>();
            let mut game = RimeRink::new(&players, seed);
            for _ in 0..(LIMIT as usize * 60 + 2) {
                let inputs = players.iter().map(|c|
                    (c.seat, game.bot_input(c.seat, 1.0 / 60.0))).collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for (total, player) in totals.iter_mut().zip(&game.players) {
                *total += player.score;
            }
        }
        assert!(totals.iter().all(|&score| score > 0), "mixed builds: {totals:?}");
    }
}
