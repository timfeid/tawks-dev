//! Beacon Bloom: tune a revolving lantern toward a drifting sky mote, then
//! pulse when the two halos meet. Each successful pulse grows a short streak.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::f32::consts::TAU;

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "beacon";
pub const LIMIT: f32 = 64.0;
const TURN: f32 = 2.7;
pub const HIT: f32 = 0.24;
const COOLDOWN: f32 = 0.55;
const BLOOM: f32 = 0.48;

#[derive(Clone, Debug)]
pub struct Keeper {
    seat: u8,
    body_width: f32,
    aim: f32,
    mote: f32,
    drift: f32,
    score: u32,
    streak: u8,
    cooldown: f32,
    bloom: f32,
    bot_delay: f32,
    skill: f32,
    misses: u32,
}

pub struct BeaconBloom {
    players: Vec<Keeper>,
    time: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}

fn angular_gap(a: f32, b: f32) -> f32 {
    ((a - b + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI).abs()
}

impl BeaconBloom {
    fn turn_rate(p: &Keeper) -> f32 {
        TURN * (1.0 - (p.body_width - 1.0) * 0.10)
    }

    fn hit_arc(p: &Keeper) -> f32 {
        HIT * p.body_width
    }

    fn pulse_cooldown(p: &Keeper) -> f32 {
        COOLDOWN * (1.0 + (p.body_width - 1.0) * 0.12)
    }

    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let players = contestants
            .iter()
            .map(|c| Keeper {
                seat: c.seat,
                body_width: BodyProfile::new(c.body_width).width,
                aim: rng.random_range(0.0..TAU),
                mote: rng.random_range(0.0..TAU),
                drift: if rng.random_bool(0.5) { 0.42 } else { -0.42 },
                score: 0,
                streak: 0,
                cooldown: 0.0,
                bloom: 0.0,
                bot_delay: rng.random_range(0.3..1.0),
                skill: c.skill.clamp(0.0, 1.0),
                misses: 0,
            })
            .collect();
        Self {
            players,
            time: 0.0,
            rng,
            result: None,
        }
    }

    fn finish(&mut self) {
        let mut scores: Vec<_> = self.players.iter().map(|p| (p.seat, p.score)).collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut previous = None;
        for (seat, score) in scores {
            if previous != Some(score) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            previous = Some(score);
        }
        self.result = Some(places);
    }
}

impl Bout for BeaconBloom {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game": KEY, "seconds": LIMIT})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        for p in &mut self.players {
            p.mote = (p.mote + p.drift * dt).rem_euclid(TAU);
            p.cooldown = (p.cooldown - dt).max(0.0);
            p.bloom = (p.bloom - dt).max(0.0);
            let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) else {
                continue;
            };
            let turn = input.stick.clamped().x;
            p.aim = (p.aim + turn * Self::turn_rate(p) * dt).rem_euclid(TAU);
            if !input.fire || p.cooldown > 0.0 {
                continue;
            }
            p.cooldown = Self::pulse_cooldown(p);
            let hit = angular_gap(p.aim, p.mote) <= Self::hit_arc(p);
            if hit {
                p.streak = (p.streak + 1).min(5);
                let gained = p.streak as u32;
                p.score += gained;
                p.bloom = BLOOM;
                p.mote = (p.mote + self.rng.random_range(1.5..4.8)).rem_euclid(TAU);
                p.drift = if self.rng.random_bool(0.5) {
                    0.42
                } else {
                    -0.42
                };
                events.push(json!({"kind":"beacon_bloom","seat":p.seat,"points":gained}));
            } else {
                p.streak = 0;
                p.misses += 1;
                events.push(json!({"kind":"beacon_miss","seat":p.seat}));
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(p) = self.players.iter_mut().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.result.is_some() {
            return Input::default();
        }
        p.bot_delay -= dt.max(0.0);
        let delta = (p.mote - p.aim + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI;
        let error = 0.0;
        let fire =
            p.cooldown <= 0.0 && p.bot_delay <= 0.0 && delta.abs() < Self::hit_arc(p) + error;
        if fire {
            p.bot_delay = 0.45 + (1.0 - p.skill) * 0.22 + self.rng.random_range(0.0..0.35);
        }
        Input {
            stick: crate::bout::Stick {
                x: (delta * 3.5).clamp(-1.0, 1.0),
                z: 0.0,
            },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        let count = self.players.len();
        let columns = count.min(4).max(1);
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,
            "players":self.players.iter().enumerate().map(|(index, p)| json!({"seat":p.seat,
                "x":(index % columns) as f32 * 4.4 - (columns as f32 - 1.0) * 2.2,
                "z":(index / columns) as f32 * 4.5 - if count > 4 { 2.25 } else { 0.0 } + 2.1,
                "yaw":p.aim,"aim":p.aim,"mote":p.mote,
                "body_width":p.body_width,"turn_rate":Self::turn_rate(p),
                "hit_arc":Self::hit_arc(p),"pulse_cooldown":Self::pulse_cooldown(p),
                "score":p.score,"streak":p.streak,"cooldown":p.cooldown,"bloom":p.bloom,"misses":p.misses})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn body_width_changes_lantern_turn_arc_and_cooldown() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = BeaconBloom::new(&[small, broad], 5);
        assert!(
            BeaconBloom::turn_rate(&game.players[0]) > BeaconBloom::turn_rate(&game.players[1])
        );
        assert!(BeaconBloom::hit_arc(&game.players[0]) < BeaconBloom::hit_arc(&game.players[1]));
        assert!(
            BeaconBloom::pulse_cooldown(&game.players[0])
                < BeaconBloom::pulse_cooldown(&game.players[1])
        );
        for p in &mut game.players {
            p.aim = 0.0;
            p.mote = 0.25;
        }
        let fire = Input {
            fire: true,
            ..Default::default()
        };
        game.step(&[(0, fire), (1, fire)], 1.0 / 60.0, &mut Vec::new());
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.players[1].score, 1);
    }

    #[test]
    fn pulse_scores_only_when_aligned_and_obeys_cooldown() {
        let mut game = BeaconBloom::new(&[Contestant::person(0), Contestant::person(1)], 5);
        game.players[0].aim = 1.0;
        game.players[0].mote = 1.0;
        let fire = Input {
            fire: true,
            ..Default::default()
        };
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 1);
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 1);
        game.players[0].mote = 4.0;
        for _ in 0..36 {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].streak, 0);
        assert_eq!(game.players[0].misses, 1);
    }
    #[test]
    fn ties_and_unseated_inputs_are_safe() {
        let mut game = BeaconBloom::new(&[Contestant::person(2), Contestant::person(7)], 19);
        game.step(
            &[(
                99,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut vec![],
        );
        assert_eq!(game.players.iter().map(|p| p.score).sum::<u32>(), 0);
        game.step(&[], LIMIT * 2.0, &mut vec![]);
        assert!(
            game.placements().is_none(),
            "a malformed long tick cannot skip a round"
        );
        for _ in 0..(LIMIT as usize * 60 + 1) {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements(), Some(vec![vec![2, 7]]));
    }
    #[test]
    fn bots_finish_and_leavers_do_not_stall() {
        let seats: Vec<_> = (0..8).map(|i| Contestant::bot(i, 0.4)).collect();
        let mut game = BeaconBloom::new(&seats, 9);
        for _ in 0..(LIMIT as usize * 60 + 2) {
            let inputs: Vec<_> = (0..7).map(|i| (i, game.bot_input(i, 1.0 / 60.0))).collect();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        let places = game.placements().unwrap();
        assert_eq!(places.iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.score > 0));
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
            let mut game = BeaconBloom::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.players {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.players.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Beacon Bloom mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 700);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 30);
    }
}
