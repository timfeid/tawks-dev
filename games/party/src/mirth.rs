//! Mirth Chain: broadcast an expression to a bystander and let matching
//! neighbors echo it, one after another. Longer live chains score more.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::collections::VecDeque;

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};

pub const KEY: &str = "mirth";
pub const LIMIT: f32 = 64.0;
pub const HALF: f32 = 10.0;
pub const BYSTANDERS: usize = 18;
const SPEED: f32 = 5.8;
const START_RADIUS: f32 = 2.35;
const ECHO_RADIUS: f32 = 3.25;
const COOLDOWN: f32 = 1.15;
const ECHO_COOLDOWN: f32 = 2.8;

#[derive(Clone, Debug)]
pub struct Performer {
    pub seat: u8,
    pub body_width: f32,
    pub x: f32,
    pub z: f32,
    pub mood: u8,
    pub score: u32,
    pub best: u32,
    pub broadcasts: u32,
    cooldown: f32,
    skill: f32,
    bot_target: usize,
    bot_mood: u8,
    bot_at: f32,
}
#[derive(Clone, Debug)]
pub struct Bystander {
    pub x: f32,
    pub z: f32,
    pub mood: u8,
    pub spark: f32,
    pub pulse_delay: f32,
    pub source: Option<u8>,
    home_x: f32,
    home_z: f32,
    phase: f32,
    cooldown: f32,
    change_at: f32,
}
pub struct MirthChain {
    pub players: Vec<Performer>,
    pub crowd: Vec<Bystander>,
    pub time: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
fn dist(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    (ax - bx).hypot(az - bz)
}
fn mood(value: f32) -> Option<u8> {
    if !value.is_finite() {
        return None;
    }
    let n = value.round();
    if (value - n).abs() > 0.01 || !(0.0..=2.0).contains(&n) {
        None
    } else {
        Some(n as u8)
    }
}
impl MirthChain {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let a = (i + (seed as usize % count)) as f32 * std::f32::consts::TAU / count as f32;
                Performer {
                    seat: c.seat,
                    body_width: BodyProfile::new(c.body_width).width,
                    x: a.cos() * 8.5,
                    z: a.sin() * 7.0,
                    mood: 0,
                    score: 0,
                    best: 0,
                    broadcasts: 0,
                    cooldown: 0.0,
                    skill: c.skill.clamp(0.0, 1.0),
                    bot_target: i % BYSTANDERS,
                    bot_mood: 0,
                    bot_at: rng.random_range(0.0..0.6),
                }
            })
            .collect();
        let crowd = (0..BYSTANDERS)
            .map(|i| {
                let col = i % 6;
                let row = i / 6;
                let x = (col as f32 - 2.5) * 2.65;
                let z = (row as f32 - 1.0) * 3.0;
                Bystander {
                    x,
                    z,
                    home_x: x,
                    home_z: z,
                    mood: rng.random_range(0..3),
                    spark: 0.0,
                    pulse_delay: 0.0,
                    source: None,
                    phase: rng.random_range(0.0..6.28),
                    cooldown: 0.0,
                    change_at: rng.random_range(3.0..6.0),
                }
            })
            .collect();
        Self {
            players,
            crowd,
            time: 0.0,
            rng,
            result: None,
        }
    }
    fn chain_size(&self, start: usize, target: u8) -> usize {
        let mut seen = [false; BYSTANDERS];
        let mut queue = vec![start];
        seen[start] = true;
        let mut total = 0;
        while let Some(i) = queue.pop() {
            let p = &self.crowd[i];
            if p.cooldown > 0.0 || !(i == start || p.mood == target) {
                continue;
            }
            total += 1;
            for (j, q) in self.crowd.iter().enumerate() {
                if !seen[j]
                    && q.mood == target
                    && q.cooldown <= 0.0
                    && dist(p.x, p.z, q.x, q.z) < ECHO_RADIUS
                {
                    seen[j] = true;
                    queue.push(j);
                }
            }
        }
        total
    }
    fn broadcast(&mut self, seat: u8, target: u8, events: &mut Vec<Value>) {
        let Some(player_index) = self.players.iter().position(|p| p.seat == seat) else {
            return;
        };
        if self.players[player_index].cooldown > 0.0 {
            return;
        }
        let (x, z) = (self.players[player_index].x, self.players[player_index].z);
        let body = BodyProfile::new(self.players[player_index].body_width);
        let reach = START_RADIUS * body.radius;
        self.players[player_index].cooldown = COOLDOWN * body.mass;
        self.players[player_index].broadcasts += 1;
        let seed = self
            .crowd
            .iter()
            .enumerate()
            .filter(|(_, p)| p.cooldown <= 0.0 && dist(x, z, p.x, p.z) <= reach)
            .min_by(|(_, a), (_, b)| {
                dist(x, z, a.x, a.z)
                    .partial_cmp(&dist(x, z, b.x, b.z))
                    .unwrap()
            })
            .map(|(i, _)| i);
        let Some(seed) = seed else {
            events.push(json!({"kind":"mirth_miss","seat":seat}));
            return;
        };
        self.crowd[seed].mood = target;
        let mut seen = [false; BYSTANDERS];
        let mut queue = VecDeque::from([(seed, 0u8)]);
        seen[seed] = true;
        let mut chain = 0u32;
        while let Some((i, depth)) = queue.pop_front() {
            if self.crowd[i].cooldown > 0.0 || self.crowd[i].mood != target {
                continue;
            }
            chain += 1;
            self.crowd[i].cooldown = ECHO_COOLDOWN;
            self.crowd[i].spark = 1.1;
            self.crowd[i].pulse_delay = depth as f32 * 0.15;
            self.crowd[i].source = Some(seat);
            let (px, pz) = (self.crowd[i].x, self.crowd[i].z);
            for (j, q) in self.crowd.iter().enumerate() {
                if !seen[j]
                    && q.cooldown <= 0.0
                    && q.mood == target
                    && dist(px, pz, q.x, q.z) < ECHO_RADIUS
                {
                    seen[j] = true;
                    queue.push_back((j, depth + 1));
                }
            }
        }
        let gained = chain + (chain.saturating_sub(3));
        let p = &mut self.players[player_index];
        p.score += gained;
        p.best = p.best.max(chain);
        events.push(
            json!({"kind":"mirth_chain","seat":seat,"mood":target,"chain":chain,"points":gained}),
        );
    }
    fn finish(&mut self) {
        let mut ranked = self
            .players
            .iter()
            .map(|p| (p.seat, p.score, p.best))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score, best) in ranked {
            if prior != Some((score, best)) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some((score, best));
        }
        self.result = Some(places);
    }
}
impl Bout for MirthChain {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"seconds":LIMIT,"bystanders":BYSTANDERS})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        for p in &mut self.players {
            let body = BodyProfile::new(p.body_width);
            let input = inputs
                .iter()
                .find(|(s, _)| *s == p.seat)
                .map(|(_, v)| *v)
                .unwrap_or_default();
            let stick = input.stick.clamped();
            let edge = (body.radius - 1.).max(0.) * 0.35;
            p.x = (p.x + stick.x * SPEED * body.speed * dt).clamp(-HALF + edge, HALF - edge);
            p.z = (p.z + stick.z * SPEED * body.speed * dt).clamp(-HALF + edge, HALF - edge);
            p.cooldown = (p.cooldown - dt).max(0.0);
            if let Some(m) = mood(input.yaw) {
                p.mood = m;
            }
        }
        for p in &mut self.crowd {
            p.x = p.home_x + 0.45 * (self.time * 1.1 + p.phase).sin();
            p.z = p.home_z + 0.35 * (self.time * 0.8 + p.phase).cos();
            p.cooldown = (p.cooldown - dt).max(0.0);
            if p.pulse_delay > 0.0 {
                p.pulse_delay = (p.pulse_delay - dt).max(0.0);
            } else {
                p.spark = (p.spark - dt).max(0.0);
            }
            if self.time >= p.change_at && p.cooldown == 0.0 {
                p.mood = self.rng.random_range(0..3);
                p.change_at = self.time + self.rng.random_range(2.6..5.5);
            }
        }
        for (seat, input) in inputs {
            if input.fire {
                if let Some(m) = mood(input.yaw) {
                    self.broadcast(*seat, m, events);
                }
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.result.is_some() {
            return Input::default();
        }
        let Some(i) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.time >= self.players[i].bot_at {
            let p = &self.players[i];
            let mut options = Vec::new();
            for (j, c) in self.crowd.iter().enumerate() {
                if c.cooldown > 0.0 {
                    continue;
                }
                for m in 0..3 {
                    let chain = self.chain_size(j, m);
                    let travel = dist(p.x, p.z, c.x, c.z);
                    let quality = chain as f32 * 2.0 - travel * (0.25 + p.skill * 0.15)
                        + self.rng.random_range(-1.0..1.0) * (1.0 - p.skill) * 3.0;
                    options.push((quality, j, m));
                }
            }
            options.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let choice =
                if !options.is_empty() && self.rng.random_bool(((1.0 - p.skill) * 0.22) as f64) {
                    self.rng.random_range(0..options.len().min(8))
                } else {
                    0
                };
            if let Some((_, target, m)) = options.get(choice).copied() {
                self.players[i].bot_target = target;
                self.players[i].bot_mood = m;
            }
            self.players[i].bot_at = self.time
                + 0.55
                + (1.0 - self.players[i].skill) * 0.8
                + self.rng.random_range(0.0..0.4);
        }
        let p = &self.players[i];
        let c = &self.crowd[p.bot_target];
        let dx = c.x - p.x;
        let dz = c.z - p.z;
        let length = dx.hypot(dz).max(0.001);
        let pace = 0.65 + p.skill * 0.35;
        let ready = length < START_RADIUS * BodyProfile::new(p.body_width).radius * 0.85
            && p.cooldown <= 0.0;
        let fire = ready && self.rng.random_bool((0.48 + p.skill * 0.5) as f64);
        Input {
            stick: Stick {
                x: dx / length * pace,
                z: dz / length * pace,
            },
            yaw: p.bot_mood as f32,
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,
        "crowd":self.crowd.iter().map(|p|json!({"x":p.x,"z":p.z,"mood":p.mood,"spark":p.spark,"pulse_delay":p.pulse_delay,"source":p.source,"cooldown":p.cooldown})).collect::<Vec<_>>(),
        "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"yaw":0.0,"mood":p.mood,"score":p.score,"best":p.best,"broadcasts":p.broadcasts,"cooldown":p.cooldown,"body_width":p.body_width})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn people() -> Vec<Contestant> {
        vec![Contestant::person(0), Contestant::person(1)]
    }
    #[test]
    fn body_size_trades_broadcast_reach_for_movement_and_retry() {
        let mut seats = people();
        seats[0].body_width = 0.7;
        seats[1].body_width = 1.4;
        let mut game = MirthChain::new(&seats, 4);
        for face in &mut game.crowd {
            face.cooldown = 4.;
        }
        game.crowd[0].x = 0.;
        game.crowd[0].z = 0.;
        game.crowd[0].cooldown = 0.;
        for p in &mut game.players {
            p.x = 2.4;
            p.z = 0.;
        }
        game.broadcast(0, 0, &mut vec![]);
        game.broadcast(1, 0, &mut vec![]);
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.players[1].score, 1);
        assert!(game.players[0].cooldown < game.players[1].cooldown);
        for p in &mut game.players {
            p.x = 0.;
            p.z = 0.;
        }
        let right = Input::from_stick(Stick { x: 1., z: 0. });
        game.step(&[(0, right), (1, right)], 0.1, &mut vec![]);
        assert!(game.players[0].x > game.players[1].x);
    }
    #[test]
    fn mixed_size_bots_all_build_chains() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        let mut scores = [0u32; 4];
        for seed in 0..80 {
            let seats = (0..4)
                .map(|seat| {
                    let mut c = Contestant::bot(seat, 0.6);
                    c.body_width = widths[(seat as usize + seed as usize) % 4];
                    c
                })
                .collect::<Vec<_>>();
            let mut game = MirthChain::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut vec![]);
            }
            for p in &game.players {
                scores[(p.seat as usize + seed as usize) % 4] += p.score;
            }
        }
        let least = *scores.iter().min().unwrap();
        let most = *scores.iter().max().unwrap();
        println!("Mirth Chain mixed-size scores: {scores:?}");
        assert!(least > 0 && most < least * 2, "scores={scores:?}");
    }
    #[test]
    fn broadcast_chains_only_matching_neighbors() {
        let mut game = MirthChain::new(&people(), 4);
        for p in &mut game.crowd {
            p.x = 9.0;
            p.z = 9.0;
            p.cooldown = 4.0;
        }
        game.crowd[0].x = 0.0;
        game.crowd[0].z = 0.0;
        game.crowd[0].mood = 1;
        game.crowd[0].cooldown = 0.0;
        game.crowd[1].x = 2.0;
        game.crowd[1].z = 0.0;
        game.crowd[1].mood = 1;
        game.crowd[1].cooldown = 0.0;
        game.crowd[2].x = 2.0;
        game.crowd[2].z = 2.0;
        game.crowd[2].mood = 2;
        game.crowd[2].cooldown = 0.0;
        game.players[0].x = 0.0;
        game.players[0].z = 0.0;
        game.broadcast(0, 1, &mut vec![]);
        assert_eq!(game.players[0].score, 2);
        assert!(game.crowd[0].spark > 0.0);
        assert_eq!(game.crowd[0].pulse_delay, 0.0);
        assert!(game.crowd[1].pulse_delay > 0.0);
        assert_eq!(game.crowd[2].spark, 0.0);
    }
    #[test]
    fn invalid_mood_and_cooldown_cannot_double_score() {
        let mut game = MirthChain::new(&people(), 5);
        game.players[0].x = game.crowd[0].x;
        game.players[0].z = game.crowd[0].z;
        game.step(
            &[(
                0,
                Input {
                    fire: true,
                    yaw: f32::NAN,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut vec![],
        );
        assert_eq!(game.players[0].score, 0);
        let action = Input {
            fire: true,
            yaw: 1.0,
            ..Default::default()
        };
        game.step(&[(0, action)], 1.0 / 60.0, &mut vec![]);
        let score = game.players[0].score;
        game.step(&[(0, action)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, score);
    }
    #[test]
    fn bots_and_missing_seat_finish() {
        let people = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = MirthChain::new(&people, 7);
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
    fn bot_waits_when_every_face_is_recovering() {
        let mut game = MirthChain::new(&people(), 9);
        game.time = 1.0;
        for face in &mut game.crowd {
            face.cooldown = ECHO_COOLDOWN;
        }
        let input = game.bot_input(0, 1.0 / 60.0);
        assert!(input.yaw.is_finite());
    }
}
