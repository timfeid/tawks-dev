//! Move a projected shadow tip onto drifting seals, then stamp to score.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "shadow_stamp";
pub const LIMIT: f32 = 64.0;
pub const SHADOW_LENGTH: f32 = 3.2;
const RADIUS: f32 = 10.0;
const SPEED: f32 = 6.3;
const SEALS: usize = 4;
const HIT_RADIUS: f32 = 0.8;
const PERFECT_RADIUS: f32 = 0.33;
const LEAVE_AFTER: f32 = 4.0;

fn lamp_angle(seed: u64, time: f32) -> f32 {
    seed as f32 * 0.41 + time * 0.47
}
fn shadow_tip(x: f32, z: f32, seed: u64, time: f32) -> (f32, f32) {
    let angle = lamp_angle(seed, time);
    (
        x + angle.sin() * SHADOW_LENGTH,
        z + angle.cos() * SHADOW_LENGTH,
    )
}
fn seal_pos(id: usize, time: f32) -> (f32, f32) {
    let angle = TAU * id as f32 / SEALS as f32 + time * (0.20 + id as f32 * 0.035);
    let radius = 4.3 + (time * 0.8 + id as f32 * 1.7).sin() * 0.55;
    (angle.sin() * radius, angle.cos() * radius)
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    x: f32,
    z: f32,
    yaw: f32,
    points: u32,
    stamps: u32,
    perfects: u32,
    held: bool,
    cooldown: f32,
    seal_ready: [f32; SEALS],
    missing: f32,
    left: bool,
}
pub struct ShadowStamp {
    seed: u64,
    players: Vec<Player>,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl ShadowStamp {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle = TAU * ((i + seed as usize % count) % count) as f32 / count as f32;
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0., 1.),
                    x: angle.sin() * 7.5,
                    z: angle.cos() * 7.5,
                    yaw: angle,
                    points: 0,
                    stamps: 0,
                    perfects: 0,
                    held: false,
                    cooldown: 0.,
                    seal_ready: [0.; SEALS],
                    missing: 0.,
                    left: false,
                }
            })
            .collect();
        Self {
            seed,
            players,
            time: 0.,
            placed: None,
        }
    }
    fn finish(&mut self) {
        let mut ranked = self.players.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.points.cmp(&a.points))
                .then_with(|| b.perfects.cmp(&a.perfects))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left && q.points == p.points && q.perfects == p.perfects
                });
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}

impl Bout for ShadowStamp {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"limit":LIMIT,"shadow_length":SHADOW_LENGTH,"seals":SEALS})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0., 0.1)
        } else {
            0.
        };
        self.time += dt;
        for p in &mut self.players {
            if p.left {
                continue;
            }
            let Some(input) = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, input)| *input)
            else {
                p.missing += dt;
                if p.missing >= LEAVE_AFTER {
                    p.left = true;
                    events.push(json!({"kind":"shadow_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            p.cooldown = (p.cooldown - dt).max(0.);
            let stick = input.stick.clamped();
            p.x += stick.x * SPEED * dt;
            p.z += stick.z * SPEED * dt;
            let distance = p.x.hypot(p.z);
            if distance > RADIUS - 0.45 {
                let scale = (RADIUS - 0.45) / distance;
                p.x *= scale;
                p.z *= scale;
            }
            if stick.x.hypot(stick.z) > 0.1 {
                p.yaw = stick.x.atan2(stick.z);
            }
            let fire = input.fire || input.jump;
            if fire && !p.held && p.cooldown == 0. {
                let (tip_x, tip_z) = shadow_tip(p.x, p.z, self.seed, self.time);
                let hit = (0..SEALS)
                    .filter(|id| p.seal_ready[*id] <= self.time)
                    .map(|id| {
                        let (sx, sz) = seal_pos(id, self.time);
                        (id, (tip_x - sx).hypot(tip_z - sz))
                    })
                    .filter(|(_, distance)| *distance <= HIT_RADIUS)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((id, distance)) = hit {
                    let perfect = distance <= PERFECT_RADIUS;
                    p.points += if perfect { 3 } else { 1 };
                    p.stamps += 1;
                    if perfect {
                        p.perfects += 1;
                    }
                    p.cooldown = 0.35;
                    p.seal_ready[id] = self.time + 2.5;
                    events.push(
                        json!({"kind":"shadow_stamp","seat":p.seat,"seal":id,"perfect":perfect}),
                    );
                }
            }
            p.held = fire;
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.left) {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.players.iter().find(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        let angle = lamp_angle(self.seed, self.time);
        let offset = (angle.sin() * SHADOW_LENGTH, angle.cos() * SHADOW_LENGTH);
        let target = (0..SEALS)
            .filter(|id| p.seal_ready[*id] <= self.time)
            .map(|id| {
                let (sx, sz) = seal_pos(id, self.time);
                let at = (sx - offset.0, sz - offset.1);
                (id, at, (p.x - at.0).hypot(p.z - at.1))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2));
        let Some((id, at, distance)) = target else {
            return Input::default();
        };
        let dx = at.0 - p.x;
        let dz = at.1 - p.z;
        let wobble = (1. - p.skill) * 0.26 * (self.time * 2.7 + seat as f32 * 1.9).sin();
        let pace = 0.73 + p.skill * 0.27;
        let (x, z) = if distance > 0.001 {
            (
                (dx / distance + wobble) * pace,
                (dz / distance - wobble) * pace,
            )
        } else {
            (0., 0.)
        };
        let (sx, sz) = seal_pos(id, self.time);
        let (tip_x, tip_z) = shadow_tip(p.x, p.z, self.seed, self.time);
        let error = (tip_x - sx).hypot(tip_z - sz);
        let fire = error <= HIT_RADIUS * (0.74 + p.skill * 0.18) && !p.held && p.cooldown == 0.;
        Input {
            stick: Stick { x, z },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "lamp_angle":lamp_angle(self.seed,self.time),
            "seals":(0..SEALS).map(|id|{let (x,z)=seal_pos(id,self.time);
                json!({"id":id,"x":x,"z":z})}).collect::<Vec<_>>(),
            "players":self.players.iter().map(|p|{let (tip_x,tip_z)=shadow_tip(p.x,p.z,self.seed,self.time);
                json!({"seat":p.seat,"x":p.x,"z":p.z,"yaw":p.yaw,"tip":{"x":tip_x,"z":tip_z},
                    "points":p.points,"stamps":p.stamps,"perfects":p.perfects,"ready":p.seal_ready,"left":p.left})}).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seats() -> Vec<Contestant> {
        (0..4).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }
    #[test]
    fn stamp_needs_the_projected_tip_on_a_live_seal() {
        let mut g = ShadowStamp::new(&seats(), 1);
        let (sx, sz) = seal_pos(0, 0.);
        let angle = lamp_angle(g.seed, 0.);
        g.players[0].x = sx - angle.sin() * SHADOW_LENGTH;
        g.players[0].z = sz - angle.cos() * SHADOW_LENGTH;
        let mut events = Vec::new();
        g.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 3);
        assert_eq!(g.players[0].perfects, 1);
        g.players[0].x = 0.;
        g.players[0].z = 0.;
        g.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 3);
    }
    #[test]
    fn misses_do_not_score_and_leavers_place_last() {
        let mut g = ShadowStamp::new(&seats(), 2);
        let mut events = Vec::new();
        g.players[0].x = 0.;
        g.players[0].z = 0.;
        g.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 0);
        for _ in 0..50 {
            g.step(
                &[
                    (0, Input::default()),
                    (2, Input::default()),
                    (3, Input::default()),
                ],
                0.1,
                &mut events,
            );
        }
        assert!(g.players[1].left);
    }
    #[test]
    fn bots_stamp_and_everyone_ranks_before_limit() {
        let seats = seats();
        let mut g = ShadowStamp::new(&seats, 7);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.stamps).sum::<u32>() > 12);
    }
    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = seats();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut stamps = 0u32;
        for seed in 0..160 {
            let mut g = ShadowStamp::new(&seats, seed);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1. / 60., &mut events);
            }
            for &seat in &g.placements().unwrap()[0] {
                first[seat as usize] += 1;
            }
            for p in &g.players {
                scores[p.seat as usize] += p.points;
                stamps += p.stamps;
            }
        }
        println!("Shadow Stamp equal skill: first={first:?}, scores={scores:?}, stamps={stamps}");
        assert!(stamps > 2_000);
        assert!(first.iter().max().unwrap() - first.iter().min().unwrap() <= 20);
        assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() <= 180);
    }
}
