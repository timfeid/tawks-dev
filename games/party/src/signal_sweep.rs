//! Race a personal alarm by disarming the lit signal stations in order.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "signal_sweep";
pub const LIMIT: f32 = 64.0;
pub const ALARM: f32 = 11.0;
const RADIUS: f32 = 9.5;
const SPEED: f32 = 6.5;
const STATIONS: usize = 8;
const REACH: f32 = 1.15;
const LEAVE_AFTER: f32 = 4.0;

fn station(id: usize) -> (f32, f32) {
    let angle = TAU * id as f32 / STATIONS as f32;
    (angle.sin() * 6.5, angle.cos() * 6.5)
}
fn hash(seed: u64) -> u64 {
    let mut n = seed ^ (seed >> 30);
    n = n.wrapping_mul(0xbf58476d1ce4e5b9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}
fn route(seed: u64, seat: u8, round: u32) -> [usize; 3] {
    let salt = seed
        .wrapping_add(seat as u64 * 0x10001)
        .wrapping_add(round as u64 * 0x9e37);
    let a = (hash(salt) % STATIONS as u64) as usize;
    let b = (a + 2 + (hash(salt + 1) % 5) as usize) % STATIONS;
    let mut c = (b + 2 + (hash(salt + 2) % 5) as usize) % STATIONS;
    if c == a {
        c = (c + 1) % STATIONS;
    }
    [a, b, c]
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    x: f32,
    z: f32,
    yaw: f32,
    route: [usize; 3],
    stage: usize,
    round: u32,
    clock: f32,
    points: u32,
    chains: u32,
    misses: u32,
    held: bool,
    missing: f32,
    left: bool,
}
pub struct SignalSweep {
    seed: u64,
    players: Vec<Player>,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl SignalSweep {
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
                    x: angle.sin() * 8.,
                    z: angle.cos() * 8.,
                    yaw: angle,
                    route: route(seed, c.seat, 0),
                    stage: 0,
                    round: 0,
                    clock: ALARM,
                    points: 0,
                    chains: 0,
                    misses: 0,
                    held: false,
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
                .then_with(|| b.chains.cmp(&a.chains))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| q.left == p.left && q.points == p.points && q.chains == p.chains);
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}

impl Bout for SignalSweep {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"limit":LIMIT,"alarm":ALARM,
            "stations":(0..STATIONS).map(|id| { let (x,z)=station(id); json!({"id":id,"x":x,"z":z}) }).collect::<Vec<_>>()})
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
                    events.push(json!({"kind":"signal_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
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
            p.clock -= dt;
            if p.clock <= 0. {
                p.misses += 1;
                p.round += 1;
                p.route = route(self.seed, p.seat, p.round);
                p.stage = 0;
                p.clock = ALARM;
                events.push(json!({"kind":"signal_timeout","seat":p.seat}));
            }
            let fire = input.fire || input.jump;
            if fire && !p.held {
                let target = p.route[p.stage];
                let (tx, tz) = station(target);
                if (p.x - tx).hypot(p.z - tz) <= REACH {
                    p.points += 1;
                    p.stage += 1;
                    events.push(json!({"kind":"signal_disarm","seat":p.seat,"station":target}));
                    if p.stage == 3 {
                        p.points += 3;
                        p.chains += 1;
                        p.round += 1;
                        p.route = route(self.seed, p.seat, p.round);
                        p.stage = 0;
                        p.clock = ALARM;
                        events.push(json!({"kind":"signal_chain","seat":p.seat}));
                    }
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
        let (tx, tz) = station(p.route[p.stage]);
        let dx = tx - p.x;
        let dz = tz - p.z;
        let distance = dx.hypot(dz);
        let pace = 0.72 + p.skill * 0.28;
        let wobble = (1. - p.skill) * 0.34 * (self.time * 3.3 + seat as f32 * 2.1).sin();
        let (x, z) = if distance > 0.001 {
            (
                (dx / distance + wobble).clamp(-1., 1.),
                (dz / distance - wobble).clamp(-1., 1.),
            )
        } else {
            (0., 0.)
        };
        let fire = distance <= REACH * (0.78 + p.skill * 0.12) && !p.held;
        Input {
            stick: Stick {
                x: x * pace,
                z: z * pace,
            },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"yaw":p.yaw,
                "route":p.route,"stage":p.stage,"target":p.route[p.stage],"clock":p.clock.max(0.),
                "points":p.points,"chains":p.chains,"misses":p.misses,"left":p.left})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seats() -> Vec<Contestant> {
        (0..4).map(|s| Contestant::bot(s, 0.5)).collect()
    }
    #[test]
    fn routes_have_three_distinct_stations() {
        for seed in 0..200 {
            for seat in 0..8 {
                let r = route(seed, seat, 0);
                assert_ne!(r[0], r[1]);
                assert_ne!(r[1], r[2]);
                assert_ne!(r[0], r[2]);
            }
        }
    }
    #[test]
    fn only_the_lit_station_accepts_a_new_press() {
        let mut g = SignalSweep::new(&seats(), 1);
        let target = g.players[0].route[0];
        let (x, z) = station(target);
        g.players[0].x = x;
        g.players[0].z = z;
        let mut events = Vec::new();
        g.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            1. / 60.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 1);
        assert_eq!(g.players[0].stage, 1);
        g.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            1. / 60.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 1);
    }
    #[test]
    fn timeout_resets_route_without_erasing_score() {
        let mut g = SignalSweep::new(&seats(), 3);
        g.players[0].points = 2;
        g.players[0].clock = 0.01;
        g.step(&[(0, Input::default())], 0.02, &mut Vec::new());
        assert_eq!(g.players[0].stage, 0);
        assert_eq!(g.players[0].round, 1);
        assert_eq!(g.players[0].points, 2);
        assert_eq!(g.players[0].misses, 1);
    }
    #[test]
    fn bots_finish_with_rankings_and_real_disarms() {
        let seats = seats();
        let mut g = SignalSweep::new(&seats, 6);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.points).sum::<u32>() >= 8);
    }
    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = seats();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut chains = 0;
        for seed in 0..160 {
            let mut g = SignalSweep::new(&seats, seed);
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
                chains += p.chains;
            }
        }
        println!("Signal Sweep equal skill: first={first:?}, scores={scores:?}, chains={chains}");
        assert!(chains > 1_000);
        assert!(first.iter().max().unwrap() - first.iter().min().unwrap() <= 25);
        assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() <= 250);
    }
}
