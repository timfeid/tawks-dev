//! A downhill precision slide. Steer through wind corridors at speed, or
//! brake for a cleaner line and give up the speed bonus.
use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use serde_json::{Value, json};

pub const KEY: &str = "ravine_draft";
pub const LIMIT: f32 = 64.0;
pub const HALF_WIDTH: f32 = 6.4;
pub const FINISH_Z: f32 = 332.0;
pub const GATES: usize = 24;
const LEAVE_SECONDS: f32 = 4.0;
const LANES: [f32; 5] = [-4.8, -2.4, 0., 2.4, 4.8];

#[derive(Clone)]
struct Gate {
    z: f32,
    safe: f32,
    rock: f32,
}
#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    body_width: f32,
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    yaw: f32,
    score: u32,
    clean: u32,
    misses: u32,
    rocks: u32,
    next: usize,
    spin: f32,
    braking: bool,
    missing: f32,
    left: bool,
    finished: bool,
    plan_x: f32,
    plan_at: f32,
}
pub struct RavineDraft {
    players: Vec<Player>,
    gates: Vec<Gate>,
    seed: u64,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl RavineDraft {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let slot = (i + seed as usize % count) % count;
                let x = if count == 1 {
                    0.
                } else {
                    -4.8 + slot as f32 * 9.6 / (count - 1) as f32
                };
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0., 1.),
                    body_width: BodyProfile::new(c.body_width).width,
                    x,
                    z: 0.,
                    vx: 0.,
                    vz: 0.,
                    yaw: 0.,
                    score: 0,
                    clean: 0,
                    misses: 0,
                    rocks: 0,
                    next: 0,
                    spin: 0.,
                    braking: false,
                    missing: 0.,
                    left: false,
                    finished: false,
                    plan_x: x,
                    plan_at: -1.,
                }
            })
            .collect();
        let gates = (0..GATES)
            .map(|i| {
                let pick = (seed
                    .wrapping_add((i as u64).wrapping_mul(0xD1B54A32D192ED03))
                    .wrapping_mul(0x9E3779B97F4A7C15)
                    >> 32) as usize;
                let safe = LANES[pick % LANES.len()];
                let rock = LANES[(pick + 2 + (i % 2)) % LANES.len()];
                Gate {
                    z: 19. + i as f32 * 12.1,
                    safe,
                    rock,
                }
            })
            .collect();
        Self {
            players,
            gates,
            seed,
            time: 0.,
            placed: None,
        }
    }
    fn finish(&mut self) {
        let mut ranked = self.players.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| b.clean.cmp(&a.clean))
                .then_with(|| b.z.total_cmp(&a.z))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left
                        && q.score == p.score
                        && q.clean == p.clean
                        && (q.z - p.z).abs() < 0.01
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

impl Bout for RavineDraft {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":16.0,"half_width":HALF_WIDTH,"finish_z":FINISH_Z,"limit":LIMIT,
            "gates":self.gates.iter().map(|g|json!({"z":g.z,"safe":g.safe,"rock":g.rock})).collect::<Vec<_>>()})
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
            if p.left || p.finished {
                continue;
            }
            let Some(input) = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, i)| *i)
            else {
                p.missing += dt;
                p.vx = 0.;
                p.vz = 0.;
                if p.missing >= LEAVE_SECONDS {
                    p.left = true;
                    events.push(json!({"kind":"ravine_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            let body = BodyProfile::new(p.body_width);
            let stick = input.stick.clamped();
            p.braking = input.fire || input.jump;
            p.spin = (p.spin - dt).max(0.);
            let steer = (if p.braking { 6.2 } else { 4.8 }) * body.speed;
            p.vx += (stick.x * steer - p.vx) * (dt * 5. * body.acceleration).min(1.);
            if p.spin > 0. {
                p.vx *= 0.7;
            }
            let edge = HALF_WIDTH - 0.3 * body.radius;
            p.x = (p.x + p.vx * dt).clamp(-edge, edge);
            let throttle = stick.z.clamp(-1., 1.);
            p.vz = (5.35 + (body.mass - 1.) * 0.9 + throttle * 1.8
                - if p.braking { 2.2 } else { 0. }
                - if p.spin > 0. { 1.8 } else { 0. })
            .clamp(1.6, 8.0);
            p.z = (p.z + p.vz * dt).min(FINISH_Z);
            if p.vx.abs() > 0.1 {
                p.yaw = p.vx.atan2(p.vz);
            }
            while let Some(gate) = self.gates.get(p.next).filter(|g| p.z >= g.z) {
                let near = (p.x - gate.safe).abs();
                let clearance = (body.radius - 1.) * 0.8;
                if near <= 1.25 - clearance {
                    let fast = p.vz >= 6.3 && !p.braking && p.spin <= 0.;
                    p.score += 3 + u32::from(fast);
                    p.clean += 1;
                    events.push(
                        json!({"kind":"ravine_clean","seat":p.seat,"gate":p.next,"fast":fast}),
                    );
                } else if near <= 2.35 - clearance {
                    p.score += 1;
                    events.push(json!({"kind":"ravine_glance","seat":p.seat,"gate":p.next}));
                } else {
                    p.misses += 1;
                    events.push(json!({"kind":"ravine_miss","seat":p.seat,"gate":p.next}));
                }
                if (p.x - gate.rock).abs() < 0.95 * body.radius {
                    p.rocks += 1;
                    p.score = p.score.saturating_sub(1);
                    p.spin = 0.75 * body.stun;
                    events.push(json!({"kind":"ravine_rock","seat":p.seat,"gate":p.next}));
                }
                p.next += 1;
            }
            if p.z >= FINISH_Z {
                p.finished = true;
                events.push(json!({"kind":"ravine_finish","seat":p.seat}));
            }
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.finished || p.left) {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self
            .players
            .iter()
            .position(|p| p.seat == seat && !p.left && !p.finished)
        else {
            return Input::default();
        };
        if self.time >= self.players[index].plan_at {
            let p = &self.players[index];
            let gate = self.gates.get(p.next);
            let target = gate.map(|g| g.safe).unwrap_or(0.);
            let sample = (self.seed
                ^ (seat as u64).wrapping_mul(0xD1B54A32D192ED03)
                ^ (p.next as u64).wrapping_mul(0x94D049BB133111EB))
            .wrapping_mul(0x9E3779B97F4A7C15);
            let noise = ((sample >> 32) as f32 / u32::MAX as f32 - 0.5) * (1. - p.skill) * 8.0;
            let p = &mut self.players[index];
            p.plan_x = (target + noise).clamp(-HALF_WIDTH, HALF_WIDTH);
            p.plan_at = self.time + 0.13 + (1. - p.skill) * 0.4;
        }
        let p = &self.players[index];
        let error = p.plan_x - p.x;
        let gate_distance = self.gates.get(p.next).map_or(f32::INFINITY, |g| g.z - p.z);
        let brake = gate_distance < 8. && error.abs() > 1.8;
        Input {
            stick: Stick {
                x: (error * 0.7).clamp(-1., 1.),
                z: 1.,
            },
            fire: brake,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"y":-p.z*0.018,"z":p.z,
                "vx":p.vx,"vz":p.vz,"yaw":p.yaw,"score":p.score,"clean":p.clean,"misses":p.misses,"rocks":p.rocks,
                "next":p.next,"speed":p.vz,"brake":p.braking,"spin":p.spin,"left":p.left,"finished":p.finished,"body_width":p.body_width})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> RavineDraft {
        RavineDraft::new(&[Contestant::person(0), Contestant::person(1)], 7)
    }
    #[test]
    fn size_changes_steering_speed_and_gate_clearance() {
        let mut seats = [Contestant::person(0), Contestant::person(1)];
        seats[0].body_width = 0.7;
        seats[1].body_width = 1.4;
        let mut game = RavineDraft::new(&seats, 7);
        for p in &mut game.players {
            p.x = 0.;
            p.z = 0.;
        }
        let input = Input::from_stick(Stick { x: 1., z: 1. });
        game.step(&[(0, input), (1, input)], 1. / 60., &mut vec![]);
        assert!(game.players[0].vx > game.players[1].vx);
        assert!(game.players[1].vz > game.players[0].vz);
        let safe = game.gates[0].safe;
        let z = game.gates[0].z - 0.02;
        for p in &mut game.players {
            p.x = safe + 1.25;
            p.z = z;
            p.vx = 0.;
        }
        game.step(
            &[(0, Input::default()), (1, Input::default())],
            1. / 60.,
            &mut vec![],
        );
        assert_eq!(game.players[0].clean, 1);
        assert_eq!(game.players[1].clean, 0);
    }
    #[test]
    fn mixed_size_bots_finish_with_competitive_scores() {
        let seats = [0.7, 0.9, 1.2, 1.4]
            .iter()
            .enumerate()
            .map(|(seat, width)| {
                let mut c = Contestant::bot(seat as u8, 0.6);
                c.body_width = *width;
                c
            })
            .collect::<Vec<_>>();
        let mut scores = [0u32; 4];
        for seed in 0..24 {
            let mut game = RavineDraft::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut vec![]);
            }
            for p in &game.players {
                scores[p.seat as usize] += p.score;
            }
        }
        let least = *scores.iter().min().unwrap();
        let most = *scores.iter().max().unwrap();
        assert!(least > 0 && most < least * 2, "scores={scores:?}");
    }
    #[test]
    fn safe_gate_scores_and_rock_penalizes() {
        let mut g = game();
        let mut events = Vec::new();
        g.players[0].x = g.gates[0].safe;
        g.players[0].z = g.gates[0].z - 0.02;
        g.players[1].x = g.gates[0].rock;
        g.players[1].z = g.gates[0].z - 0.02;
        g.step(
            &[(0, Input::default()), (1, Input::default())],
            1. / 60.,
            &mut events,
        );
        assert!(g.players[0].score >= 3);
        assert_eq!(g.players[0].clean, 1);
        assert_eq!(g.players[1].rocks, 1);
        assert!(g.players[1].spin > 0.);
    }
    #[test]
    fn braking_trades_speed_for_steering() {
        let mut fast = game();
        let mut slow = game();
        let mut events = Vec::new();
        for _ in 0..60 {
            fast.step(
                &[
                    (0, Input::from_stick(Stick { x: 1., z: 1. })),
                    (1, Input::default()),
                ],
                1. / 60.,
                &mut events,
            );
            slow.step(
                &[
                    (
                        0,
                        Input {
                            stick: Stick { x: 1., z: 1. },
                            fire: true,
                            ..Default::default()
                        },
                    ),
                    (1, Input::default()),
                ],
                1. / 60.,
                &mut events,
            );
        }
        assert!(fast.players[0].z > slow.players[0].z + 1.);
        assert!(slow.players[0].vx > fast.players[0].vx);
    }
    #[test]
    fn invalid_input_stays_bounded_and_leaver_places_last() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..250 {
            g.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::NAN,
                        z: f32::INFINITY,
                    }),
                )],
                1. / 60.,
                &mut events,
            );
        }
        assert!(g.players[0].x.is_finite() && g.players[0].z.is_finite());
        assert!(g.players[1].left);
        while g.placements().is_none() {
            g.step(&[(0, Input::default())], 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().last(), Some(&vec![1]));
    }
    #[test]
    fn bots_finish_with_points() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.6))
            .collect::<Vec<_>>();
        let mut g = RavineDraft::new(&seats, 3);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().all(|p| p.score > 0 && p.next > 10));
    }

    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.5))
            .collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut seconds = 0.0;
        for seed in 0..160 {
            let mut g = RavineDraft::new(&seats, seed);
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
                scores[p.seat as usize] += p.score;
            }
            seconds += g.time;
        }
        println!(
            "Ravine Draft equal skill: first={first:?}, scores={scores:?}, mean_seconds={:.1}",
            seconds / 160.0
        );
        assert!(*first.iter().max().unwrap() - *first.iter().min().unwrap() <= 30);
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 300);
        assert!((45.0..=64.0).contains(&(seconds / 160.0)));
    }
}
