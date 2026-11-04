//! A moving shock wave crosses a circular court. Players can move to change
//! when it reaches them, then spring over it for points.
use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "flux_skip";
pub const LIMIT: f32 = 58.0;
pub const RADIUS: f32 = 10.0;
pub const WAVE_SPEED: f32 = 10.0;
pub const WAVE_FIRST: f32 = 2.8;
pub const WAVE_PERIOD: f32 = 4.2;
const SPEED: f32 = 5.3;
const LEAVE_SECONDS: f32 = 4.0;
const JUMP_COOLDOWN: f32 = 0.67;

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
    score: i32,
    clears: u32,
    perfects: u32,
    hits: u32,
    last_jump: f32,
    stunned_until: f32,
    held_fire: bool,
    resolved: i32,
    missing: f32,
    left: bool,
}

pub struct FluxSkip {
    players: Vec<Player>,
    seed: u64,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl FluxSkip {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle = TAU * i as f32 / count as f32;
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0.0, 1.0),
                    body_width: BodyProfile::new(c.body_width).width,
                    x: angle.sin() * 5.7,
                    z: angle.cos() * 5.7,
                    vx: 0.0,
                    vz: 0.0,
                    yaw: angle,
                    score: 0,
                    clears: 0,
                    perfects: 0,
                    hits: 0,
                    last_jump: -10.0,
                    stunned_until: -10.0,
                    held_fire: false,
                    resolved: -1,
                    missing: 0.0,
                    left: false,
                }
            })
            .collect();
        Self {
            players,
            seed,
            time: 0.0,
            placed: None,
        }
    }

    fn cycle_at(time: f32) -> i32 {
        ((time - WAVE_FIRST) / WAVE_PERIOD).floor() as i32
    }

    fn wave(&self) -> (i32, f32) {
        let cycle = Self::cycle_at(self.time);
        let elapsed = self.time - WAVE_FIRST - cycle as f32 * WAVE_PERIOD;
        (cycle, elapsed)
    }

    fn contact_radius(p: &Player) -> f32 {
        p.x.hypot(p.z) - 0.45 * BodyProfile::new(p.body_width).radius
    }

    fn finish(&mut self) {
        let mut ranked: Vec<_> = self.players.iter().collect();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| b.perfects.cmp(&a.perfects))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left && q.score == p.score && q.perfects == p.perfects
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

impl Bout for FluxSkip {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({"game": KEY, "radius": RADIUS, "wave_speed": WAVE_SPEED, "period": WAVE_PERIOD, "limit": LIMIT})
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        self.time += dt;
        let (cycle, wave_elapsed) = self.wave();
        let wave_radius = wave_elapsed * WAVE_SPEED;
        for p in &mut self.players {
            if p.left {
                continue;
            }
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, input)| *input);
            let Some(input) = input else {
                p.missing += dt;
                p.vx = 0.0;
                p.vz = 0.0;
                if p.missing >= LEAVE_SECONDS {
                    p.left = true;
                    events.push(json!({"kind":"flux_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.0;
            let stick = input.stick.clamped();
            let body = BodyProfile::new(p.body_width);
            let stagger = if self.time < p.stunned_until {
                0.45
            } else {
                1.0
            };
            p.vx = stick.x * SPEED * body.speed * stagger;
            p.vz = stick.z * SPEED * body.speed * stagger;
            p.x += p.vx * dt;
            p.z += p.vz * dt;
            let distance = p.x.hypot(p.z);
            let outer = RADIUS - 0.6 * body.radius;
            let inner = 2.2 + 0.4 * (body.radius - 1.0);
            if distance > outer || distance < inner {
                let safe = distance.clamp(inner, outer);
                let scale = if distance > 0.01 {
                    safe / distance
                } else {
                    1.0
                };
                p.x *= scale;
                p.z *= scale;
            }
            if p.vx.hypot(p.vz) > 0.1 {
                p.yaw = p.vx.atan2(p.vz);
            }
            let fire = input.fire || input.jump;
            if fire
                && !p.held_fire
                && self.time - p.last_jump >= JUMP_COOLDOWN
                && self.time >= p.stunned_until
            {
                p.last_jump = self.time;
                events.push(json!({"kind":"flux_jump","seat":p.seat}));
            }
            p.held_fire = fire;
            if cycle >= 0 && p.resolved != cycle && wave_radius >= Self::contact_radius(p) {
                p.resolved = cycle;
                let age = self.time - p.last_jump;
                if (0.15..=0.58).contains(&age) {
                    let perfect = (0.25..=0.43).contains(&age);
                    p.score += if perfect { 3 } else { 2 };
                    p.clears += 1;
                    p.perfects += u32::from(perfect);
                    events.push(json!({"kind":"flux_clear","seat":p.seat,"perfect":perfect}));
                } else {
                    p.score = (p.score - 1).max(0);
                    p.hits += 1;
                    p.stunned_until = self.time + 0.55 * body.stun;
                    events.push(json!({"kind":"flux_hit","seat":p.seat}));
                }
            }
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.left) {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.players.iter().find(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        let current = Self::cycle_at(self.time).max(0);
        let cycle = if p.resolved == current {
            current + 1
        } else {
            current
        };
        let arrival =
            WAVE_FIRST + cycle as f32 * WAVE_PERIOD + Self::contact_radius(p) / WAVE_SPEED;
        let hash = self
            .seed
            .wrapping_add(seat as u64 * 101)
            .wrapping_add(cycle as u64 * 7919);
        let offset = ((hash.wrapping_mul(0x9E3779B97F4A7C15) >> 48) as f32 / 65535.0 * 2.0 - 1.0)
            * (0.31 - p.skill * 0.24);
        let desired = arrival - 0.34 + offset;
        let fire =
            self.time >= desired && self.time < desired + 0.12 && p.last_jump < desired - 0.2;
        let target = 5.5;
        let radius = p.x.hypot(p.z);
        let side = if radius > target + 0.3 {
            -1.0
        } else if radius < target - 0.3 {
            1.0
        } else {
            0.0
        };
        Input {
            stick: Stick {
                x: side * p.x / radius.max(0.1) * 0.45,
                z: side * p.z / radius.max(0.1) * 0.45,
            },
            fire,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let (cycle, elapsed) = self.wave();
        let radius = if cycle >= 0 && elapsed * WAVE_SPEED <= RADIUS {
            (elapsed * WAVE_SPEED).max(0.0)
        } else {
            0.0
        };
        let next = if cycle < 0 {
            WAVE_FIRST - self.time
        } else {
            (WAVE_FIRST + (cycle + 1) as f32 * WAVE_PERIOD - self.time).max(0.0)
        };
        json!({
            "game": KEY, "time": self.time, "limit": LIMIT,
            "wave_radius": radius, "wave_cycle": cycle, "next_wave": next,
            "done": self.placed.is_some(),
            "players": self.players.iter().map(|p| json!({
                "seat":p.seat,"x":p.x,"z":p.z,"yaw":p.yaw,"vx":p.vx,"vz":p.vz,
                "body_width":p.body_width,"run_speed":SPEED * BodyProfile::new(p.body_width).speed,
                "recovery":0.55 * BodyProfile::new(p.body_width).stun,
                "stun_left":(p.stunned_until-self.time).max(0.0),
                "score":p.score,"clears":p.clears,"perfects":p.perfects,"hits":p.hits,
                "jump_age":self.time-p.last_jump,"left":p.left
            })).collect::<Vec<_>>()
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> FluxSkip {
        FluxSkip::new(&[Contestant::person(0), Contestant::person(1)], 7)
    }
    #[test]
    fn jump_timing_scores_and_misses_are_server_owned() {
        let mut g = game();
        let mut events = Vec::new();
        let arrival = WAVE_FIRST + FluxSkip::contact_radius(&g.players[0]) / WAVE_SPEED;
        while g.time < arrival - 0.35 {
            g.step(
                &[(0, Input::default()), (1, Input::default())],
                1.0 / 60.0,
                &mut events,
            );
        }
        g.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (1, Input::default()),
            ],
            1.0 / 60.0,
            &mut events,
        );
        while g.time < arrival + 0.1 {
            g.step(
                &[(0, Input::default()), (1, Input::default())],
                1.0 / 60.0,
                &mut events,
            );
        }
        assert_eq!(g.players[0].score, 3);
        assert_eq!(g.players[1].score, 0);
        assert_eq!(g.players[1].hits, 1);
    }
    #[test]
    fn malformed_stick_and_fire_spam_cannot_escape_or_farm_jumps() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..240 {
            g.step(
                &[
                    (
                        0,
                        Input {
                            stick: Stick {
                                x: f32::NAN,
                                z: f32::INFINITY,
                            },
                            fire: true,
                            ..Default::default()
                        },
                    ),
                    (1, Input::default()),
                ],
                1.0 / 60.0,
                &mut events,
            );
        }
        assert!(g.players[0].x.is_finite() && g.players[0].z.is_finite());
        assert_eq!(
            events.iter().filter(|e| e["kind"] == "flux_jump").count(),
            1
        );
    }
    #[test]
    fn leavers_place_after_active_players_and_ties_group() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..250 {
            g.step(&[(0, Input::default())], 1.0 / 60.0, &mut events);
        }
        assert!(g.players[1].left);
        while g.placements().is_none() {
            g.step(&[(0, Input::default())], 1.0 / 60.0, &mut events);
        }
        assert_eq!(g.placements().unwrap()[1], vec![1]);
        let mut same = game();
        same.finish();
        assert_eq!(same.placements().unwrap(), vec![vec![0, 1]]);
    }
    #[test]
    fn bots_finish_with_varied_scores() {
        let contestants = (0..4)
            .map(|i| Contestant::bot(i, 0.3 + i as f32 * 0.2))
            .collect::<Vec<_>>();
        let mut g = FluxSkip::new(&contestants, 98);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = contestants
                .iter()
                .map(|c| (c.seat, g.bot_input(c.seat, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1.0 / 60.0, &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().any(|p| p.clears > 0));
    }

    #[test]
    fn body_width_changes_run_speed_contact_and_recovery() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut g = FluxSkip::new(&[small, broad], 7);
        let (small_contact, broad_contact) = (
            FluxSkip::contact_radius(&g.players[0]),
            FluxSkip::contact_radius(&g.players[1]),
        );
        assert!(broad_contact < small_contact);
        assert!(
            BodyProfile::new(g.players[0].body_width).stun
                > BodyProfile::new(g.players[1].body_width).stun
        );
        let mut events = Vec::new();
        let forward = Input {
            stick: Stick { x: 0.0, z: 1.0 },
            ..Default::default()
        };
        g.step(&[(0, forward), (1, forward)], 0.1, &mut events);
        assert!(g.players[0].vz > g.players[1].vz);
        let players = g.snapshot()["players"].as_array().unwrap().clone();
        assert!((players[0]["body_width"].as_f64().unwrap() - 0.7).abs() < 0.001);
        assert!((players[1]["body_width"].as_f64().unwrap() - 1.4).abs() < 0.001);
        assert!(
            players[0]["recovery"].as_f64().unwrap() > players[1]["recovery"].as_f64().unwrap()
        );
        while g.time < WAVE_FIRST + 0.7 {
            g.step(
                &[(0, Input::default()), (1, Input::default())],
                1.0 / 60.0,
                &mut events,
            );
        }
        assert_eq!(g.players[0].hits, 1);
        assert_eq!(g.players[1].hits, 1);
        assert!(g.players[0].stunned_until > g.players[1].stunned_until);
    }

    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let contestants = (0..4)
            .map(|seat| Contestant::bot(seat, 0.5))
            .collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        for seed in 0..160 {
            let mut g = FluxSkip::new(&contestants, seed);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, g.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1.0 / 60.0, &mut events);
            }
            for &seat in &g.placements().unwrap()[0] {
                first[seat as usize] += 1;
            }
            for p in &g.players {
                scores[p.seat as usize] += p.score as u32;
            }
        }
        println!("Flux Skip equal skill: first={first:?}, total_scores={scores:?}");
        assert!(*first.iter().max().unwrap() - *first.iter().min().unwrap() <= 32);
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 180);
    }

    #[test]
    #[ignore]
    fn mixed_body_balance_report() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        let mut scores = [0u32; 4];
        let mut wins = [0u32; 4];
        for seed in 0..128u64 {
            let contestants = (0..4u8)
                .map(|seat| {
                    let mut c = Contestant::bot(seat, 0.5);
                    c.body_width = widths[(seat as usize + seed as usize) % 4];
                    c
                })
                .collect::<Vec<_>>();
            let mut g = FluxSkip::new(&contestants, seed);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, g.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1.0 / 60.0, &mut events);
            }
            for p in &g.players {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score as u32;
            }
            for &seat in &g.placements().unwrap()[0] {
                let p = g.players.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Flux Skip mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 250);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 40);
    }
}
