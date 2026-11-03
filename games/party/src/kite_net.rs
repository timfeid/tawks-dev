//! Place three stakes around wandering wind sprites to spring a temporary net.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "kite_net";
pub const LIMIT: f32 = 62.0;
const RADIUS: f32 = 10.0;
const SPEED: f32 = 6.3;
const MOTES: usize = 12;
const MIN_AREA: f32 = 3.6;
const MAX_AREA: f32 = 36.0;
const LEAVE_AFTER: f32 = 4.0;

fn cluster_center(id: usize, time: f32, seed: u64) -> (f32, f32) {
    let phase = seed as f32 * 0.127;
    let angle = TAU * id as f32 / 3.0 + phase + time * (0.075 + id as f32 * 0.011);
    let radius = 4.35 + (time * 0.31 + id as f32 + phase).sin() * 0.48;
    (angle.sin() * radius, angle.cos() * radius)
}
fn mote_pos(id: usize, time: f32, seed: u64) -> (f32, f32) {
    let group = id / 4;
    let slot = id % 4;
    let (cx, cz) = cluster_center(group, time, seed);
    let angle = TAU * slot as f32 / 4.0 + time * (0.8 + group as f32 * 0.13) + id as f32 * 0.27;
    let radius = 0.58 + (time * 1.7 + id as f32).sin() * 0.18;
    (cx + angle.sin() * radius, cz + angle.cos() * radius)
}
fn area(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs() * 0.5
}
fn inside(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let cross = |u: (f32, f32), v: (f32, f32), w: (f32, f32)| {
        (v.0 - u.0) * (w.1 - u.1) - (v.1 - u.1) * (w.0 - u.0)
    };
    let x = cross(a, b, p);
    let y = cross(b, c, p);
    let z = cross(c, a, p);
    (x >= -0.001 && y >= -0.001 && z >= -0.001) || (x <= 0.001 && y <= 0.001 && z <= 0.001)
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    x: f32,
    z: f32,
    yaw: f32,
    stakes: Vec<(f32, f32)>,
    points: u32,
    nets: u32,
    caught: u32,
    held: bool,
    cooldown: f32,
    missing: f32,
    left: bool,
    bot_cluster: usize,
    bot_angle: f32,
    bot_plan: bool,
}
pub struct KiteNet {
    seed: u64,
    players: Vec<Player>,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}
impl KiteNet {
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
                    x: angle.sin() * 7.6,
                    z: angle.cos() * 7.6,
                    yaw: angle,
                    stakes: Vec::new(),
                    points: 0,
                    nets: 0,
                    caught: 0,
                    held: false,
                    cooldown: 0.,
                    missing: 0.,
                    left: false,
                    bot_cluster: 0,
                    bot_angle: 0.,
                    bot_plan: false,
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
                .then_with(|| b.caught.cmp(&a.caught))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| q.left == p.left && q.points == p.points && q.caught == p.caught);
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}
impl Bout for KiteNet {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"limit":LIMIT,"motes":MOTES})
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
                    events.push(json!({"kind":"kite_left","seat":p.seat}));
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
                p.stakes.push((p.x, p.z));
                p.cooldown = 0.2;
                events.push(json!({"kind":"kite_stake","seat":p.seat,"index":p.stakes.len()}));
                if p.stakes.len() == 3 {
                    let [a, b, c] = [p.stakes[0], p.stakes[1], p.stakes[2]];
                    let size = area(a, b, c);
                    let caught = if (MIN_AREA..=MAX_AREA).contains(&size) {
                        (0..MOTES)
                            .filter(|id| inside(mote_pos(*id, self.time, self.seed), a, b, c))
                            .count() as u32
                    } else {
                        0
                    };
                    let points = caught * (if size <= 10. { 2 } else { 1 })
                        + (if caught >= 4 { 2 } else { 0 });
                    p.points += points;
                    p.caught += caught;
                    p.nets += 1;
                    p.stakes.clear();
                    p.bot_plan = false;
                    events.push(json!({"kind":"kite_net","seat":p.seat,"caught":caught,"points":points,"area":size}));
                }
            }
            p.held = fire;
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.left) {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        if !self.players[index].bot_plan {
            let p = &self.players[index];
            let (cluster, center, _) = (0..3)
                .map(|id| {
                    let c = cluster_center(id, self.time, self.seed);
                    (id, c, (p.x - c.0).hypot(p.z - c.1))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2))
                .unwrap();
            let p = &mut self.players[index];
            p.bot_cluster = cluster;
            p.bot_angle = (p.x - center.0).atan2(p.z - center.1);
            p.bot_plan = true;
        }
        let p = &self.players[index];
        let (cx, cz) = cluster_center(p.bot_cluster, self.time, self.seed);
        let angle = p.bot_angle + TAU * p.stakes.len() as f32 / 3.;
        let target = (cx + angle.sin() * 1.9, cz + angle.cos() * 1.9);
        let dx = target.0 - p.x;
        let dz = target.1 - p.z;
        let distance = dx.hypot(dz);
        let pace = 0.71 + p.skill * 0.29;
        let wobble = (1. - p.skill) * 0.18 * (self.time * 2.7 + seat as f32 * 2.0).sin();
        let (x, z) = if distance > 0.001 {
            (
                (dx / distance + wobble) * pace,
                (dz / distance - wobble) * pace,
            )
        } else {
            (0., 0.)
        };
        let fire = distance <= 0.28 + p.skill * 0.15 && !p.held && p.cooldown == 0.;
        Input {
            stick: Stick { x, z },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "motes":(0..MOTES).map(|id|{let (x,z)=mote_pos(id,self.time,self.seed);json!({"id":id,"x":x,"z":z})}).collect::<Vec<_>>(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"yaw":p.yaw,
                "stakes":p.stakes.iter().map(|(x,z)|json!({"x":x,"z":z})).collect::<Vec<_>>(),
                "points":p.points,"nets":p.nets,"caught":p.caught,"left":p.left})).collect::<Vec<_>>()})
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
    fn triangle_geometry_rejects_outside_and_degenerate_nets() {
        let a = (0., 0.);
        let b = (4., 0.);
        let c = (0., 4.);
        assert!(inside((1., 1.), a, b, c));
        assert!(!inside((3., 3.), a, b, c));
        assert_eq!(area(a, b, (2., 0.)), 0.);
    }
    #[test]
    fn a_completed_compact_net_catches_the_flower_of_sprites() {
        let mut g = KiteNet::new(&seats(), 1);
        let center = cluster_center(0, 0., 1);
        let mut events = Vec::new();
        for stage in 0..3 {
            let angle = TAU * stage as f32 / 3.;
            g.players[0].x = center.0 + angle.sin() * 1.9;
            g.players[0].z = center.1 + angle.cos() * 1.9;
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
            g.step(&[(0, Input::default())], 0.1, &mut events);
            g.step(&[(0, Input::default())], 0.1, &mut events);
        }
        assert_eq!(g.players[0].nets, 1);
        assert!(g.players[0].caught >= 3);
        assert!(g.players[0].points >= 6);
    }
    #[test]
    fn bots_finish_and_rank_everyone() {
        let seats = seats();
        let mut g = KiteNet::new(&seats, 7);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.caught).sum::<u32>() > 8);
    }
    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = seats();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut caught = 0u32;
        for seed in 0..160 {
            let mut g = KiteNet::new(&seats, seed);
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
                caught += p.caught;
            }
        }
        println!("Kite Net equal skill: first={first:?}, scores={scores:?}, caught={caught}");
        assert!(caught > 5_000);
        assert!(first.iter().max().unwrap() - first.iter().min().unwrap() <= 15);
        assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() <= 1_500);
    }
}
