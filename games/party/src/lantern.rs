//! Spark Courier: gather drifting sparks and carry them to the relay core.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Stick};

pub const KEY: &str = "lantern";
pub const LIMIT: f32 = 65.0;
const ARENA: f32 = 13.0;
const SPEED: f32 = 6.2;
const SPARKS: usize = 5;
const CAPACITY: u8 = 3;

#[derive(Clone, Debug)]
struct Runner {
    seat: u8,
    x: f32,
    z: f32,
    carried: u8,
    score: u32,
    deposits: u32,
    skill: f32,
    is_bot: bool,
    pause: f32,
}

#[derive(Clone, Debug)]
struct Spark {
    angle: f32,
    radius: f32,
    speed: f32,
    dormant: f32,
}

impl Spark {
    fn at(&self, time: f32) -> (f32, f32) {
        let angle = self.angle + time * self.speed;
        (angle.cos() * self.radius, angle.sin() * self.radius)
    }
}

pub struct LanternRun {
    players: Vec<Runner>,
    sparks: Vec<Spark>,
    rng: StdRng,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}

impl LanternRun {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed ^ 0x1a71e2);
        let offset = rng.random_range(0.0..std::f32::consts::TAU);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let a = i as f32 * std::f32::consts::TAU / contestants.len().max(1) as f32;
                Runner {
                    seat: c.seat,
                    x: a.cos() * 2.8,
                    z: a.sin() * 2.8,
                    carried: 0,
                    score: 0,
                    deposits: 0,
                    skill: c.skill.clamp(0., 1.),
                    is_bot: c.is_bot,
                    pause: 0.,
                }
            })
            .collect();
        let sparks = (0..SPARKS)
            .map(|i| Spark {
                angle: offset + i as f32 * std::f32::consts::TAU / SPARKS as f32,
                radius: if i % 2 == 0 { 7.0 } else { 9.0 },
                speed: if i % 2 == 0 { 0.22 } else { -0.18 },
                dormant: 0.,
            })
            .collect();
        Self {
            players,
            sparks,
            rng,
            time: 0.,
            placed: None,
        }
    }

    fn place(&self) -> Vec<Vec<u8>> {
        let mut seats: Vec<_> = self.players.iter().map(|p| (p.seat, p.score)).collect();
        seats.sort_by_key(|(seat, score)| (std::cmp::Reverse(*score), *seat));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, score) in seats {
            if last != Some(score) {
                groups.push(Vec::new());
                last = Some(score);
            }
            groups.last_mut().unwrap().push(seat);
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::TICK_DT;

    #[test]
    fn only_a_nearby_carrier_can_bank_and_three_give_nine() {
        let mut game = LanternRun::new(&[Contestant::person(0), Contestant::person(1)], 2);
        game.players[0].carried = 3;
        game.players[0].x = 8.;
        game.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            TICK_DT,
            &mut vec![],
        );
        assert_eq!(game.players[0].score, 0);
        game.players[0].x = 0.;
        game.players[0].z = 0.;
        game.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            TICK_DT,
            &mut vec![],
        );
        assert_eq!(game.players[0].score, 9);
        assert_eq!(game.players[0].carried, 0);
        assert_eq!(game.players[1].score, 0);
    }

    #[test]
    fn all_bot_bouts_finish_with_each_seat_once() {
        for seed in 0..32 {
            let seats: Vec<_> = (0..4).map(|seat| Contestant::bot(seat, 0.4)).collect();
            let mut game = LanternRun::new(&seats, seed);
            for _ in 0..(LIMIT / TICK_DT) as usize + 2 {
                let inputs: Vec<_> = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                    .collect();
                game.step(&inputs, TICK_DT, &mut vec![]);
            }
            let groups = game.placements().unwrap();
            let mut all: Vec<_> = groups.into_iter().flatten().collect();
            all.sort();
            assert_eq!(all, vec![0, 1, 2, 3]);
            assert!(game.players.iter().any(|p| p.score > 0), "seed {seed}");
        }
    }

    #[test]
    fn absent_input_stops_a_player_and_ties_share_a_place() {
        let mut game = LanternRun::new(&[Contestant::person(0), Contestant::person(1)], 5);
        let x = game.players[0].x;
        game.step(&[], 0.1, &mut vec![]);
        assert_eq!(game.players[0].x, x);
        game.time = LIMIT - 0.01;
        game.step(&[], 0.02, &mut vec![]);
        assert_eq!(game.placements(), Some(vec![vec![0, 1]]));
    }

    #[test]
    fn malformed_and_oversized_sticks_cannot_escape_the_arena() {
        let mut game = LanternRun::new(&[Contestant::person(0)], 9);
        for _ in 0..1000 {
            game.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::INFINITY,
                        z: 9999.,
                    }),
                )],
                TICK_DT,
                &mut vec![],
            );
        }
        assert!(game.players[0].x.is_finite());
        assert!(game.players[0].z.is_finite());
        assert!(game.players[0].x.hypot(game.players[0].z) <= ARENA - 0.6 + 0.001);
    }

    #[test]
    #[ignore]
    fn seeded_bot_balance_report() {
        let mut wins = [0u32; 4];
        let mut points = [0u32; 4];
        for seed in 1000..1200 {
            let seats: Vec<_> = (0..4).map(|seat| Contestant::bot(seat, 0.4)).collect();
            let mut game = LanternRun::new(&seats, seed);
            for _ in 0..(LIMIT / TICK_DT) as usize + 2 {
                let inputs: Vec<_> = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                    .collect();
                game.step(&inputs, TICK_DT, &mut vec![]);
            }
            for seat in &game.placements().unwrap()[0] {
                wins[*seat as usize] += 1;
            }
            for player in &game.players {
                points[player.seat as usize] += player.score;
            }
        }
        println!("equal-skill wins {wins:?}, total points {points:?}");
        assert!(points.iter().all(|p| *p > 0));
    }
}

impl Bout for LanternRun {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":ARENA,
        "rule":"Steer through drifting sparks. Carry up to three to the middle and press Space or A to bank them."})
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        let dt = dt.clamp(0., 0.05);
        self.time += dt;
        for spark in &mut self.sparks {
            spark.dormant = (spark.dormant - dt).max(0.);
        }
        for p in &mut self.players {
            p.pause = (p.pause - dt).max(0.);
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let stick = input.stick.clamped();
            p.x += stick.x * SPEED * dt;
            p.z += stick.z * SPEED * dt;
            let length = p.x.hypot(p.z);
            if length > ARENA - 0.6 {
                p.x *= (ARENA - 0.6) / length;
                p.z *= (ARENA - 0.6) / length;
            }
            if input.fire && p.carried > 0 && p.x.hypot(p.z) < 2.0 {
                let gained = p.carried as u32 * p.carried as u32;
                p.score += gained;
                p.deposits += 1;
                events.push(json!({"kind":"bank","seat":p.seat,"gain":gained,"score":p.score}));
                p.carried = 0;
            }
        }
        for spark in &mut self.sparks {
            if spark.dormant > 0. {
                continue;
            }
            let (x, z) = spark.at(self.time);
            let nearest = self
                .players
                .iter()
                .enumerate()
                .filter(|(_, p)| p.carried < CAPACITY)
                .map(|(i, p)| (i, (p.x - x).hypot(p.z - z)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((index, distance)) = nearest {
                if distance < 0.9 {
                    let p = &mut self.players[index];
                    p.carried += 1;
                    p.pause = 0.15;
                    events.push(json!({"kind":"spark","seat":p.seat,"carried":p.carried}));
                    spark.dormant = self.rng.random_range(2.6..3.8);
                }
            }
        }
        if self.time >= LIMIT {
            self.placed = Some(self.place());
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(p) = self.players.iter().find(|p| p.seat == seat && p.is_bot) else {
            return Input::default();
        };
        let skill = p.skill;
        let (x, z, carried, pause) = (p.x, p.z, p.carried, p.pause);
        if pause > 0. {
            return Input::default();
        }
        let target = if carried == CAPACITY || (carried > 0 && self.time > LIMIT - 5.) {
            (0., 0.)
        } else {
            self.sparks
                .iter()
                .filter(|s| s.dormant <= 0.3)
                .map(|s| {
                    let at = s.at(self.time + 0.2 + skill * 0.35);
                    (at, (at.0 - x).hypot(at.1 - z))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|v| v.0)
                .unwrap_or((0., 0.))
        };
        let error = (1. - skill) * 0.22;
        let dx = target.0 - x + self.rng.random_range(-error..=error);
        let dz = target.1 - z + self.rng.random_range(-error..=error);
        let len = dx.hypot(dz).max(0.01);
        if self
            .rng
            .random_bool((dt * (0.7 - skill * 0.5)).clamp(0., 1.) as f64)
        {
            if let Some(p) = self.players.iter_mut().find(|p| p.seat == seat) {
                p.pause = self.rng.random_range(0.2..0.6);
            }
        }
        Input {
            stick: Stick {
                x: dx / len * (0.68 + skill * 0.32),
                z: dz / len * (0.68 + skill * 0.32),
            },
            fire: carried > 0 && x.hypot(z) < 1.7 && target == (0., 0.),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,
        "players":self.players.iter().map(|p| json!({"seat":p.seat,"x":p.x,"z":p.z,"score":p.score,"carried":p.carried,"deposits":p.deposits})).collect::<Vec<_>>(),
        "sparks":self.sparks.iter().map(|s| { let (x,z)=s.at(self.time); json!({"x":x,"z":z,"live":s.dormant<=0.}) }).collect::<Vec<_>>() })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}
