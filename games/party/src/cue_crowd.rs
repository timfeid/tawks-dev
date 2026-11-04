//! A tiny theatre contest. Every five seconds the audience calls for a mood.
//! Perform one of three gestures near the spotlight pulse to win applause.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "cuecrowd";
pub const LIMIT: f32 = 60.0;
pub const CUE_SECONDS: f32 = 5.0;
pub const MOODS: [&str; 3] = ["CHEER", "GASP", "SIGH"];
const CODE_BASE: f32 = 410.0;

#[derive(Clone)]
pub struct Performer {
    pub seat: u8,
    body_width: f32,
    pub score: i32,
    pub correct: u32,
    pub last: Option<u8>,
    pub last_points: i32,
    answered: bool,
    seen: f32,
    gone: bool,
    bot: bool,
    skill: f32,
    bot_delay: f32,
    bot_choice: u8,
}

pub struct CueCrowd {
    pub time: f32,
    pub cue: u32,
    pub target: u8,
    pub performers: Vec<Performer>,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn action_code(choice: u8) -> f32 {
    CODE_BASE + choice as f32
}

fn decode(input: &Input) -> Option<u8> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    (0..3).find(|choice| (input.yaw - action_code(*choice)).abs() < 0.01)
}

fn timing_windows(width: f32) -> (f32, f32) {
    let delta = BodyProfile::new(width).width - 1.0;
    (0.35 - delta * 0.04, 0.9 + delta * 0.12)
}

impl CueCrowd {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let target = rng.random_range(0..3);
        let performers = contestants
            .iter()
            .map(|c| Performer {
                seat: c.seat,
                body_width: BodyProfile::new(c.body_width).width,
                score: 0,
                correct: 0,
                last: None,
                last_points: 0,
                answered: false,
                seen: 0.0,
                gone: false,
                bot: c.is_bot,
                skill: c.skill.clamp(0.0, 1.0),
                bot_delay: 2.0,
                bot_choice: target,
            })
            .collect();
        let mut game = Self {
            time: 0.0,
            cue: 0,
            target,
            performers,
            rng,
            places: None,
        };
        game.plan_bots();
        game
    }

    fn plan_bots(&mut self) {
        for p in &mut self.performers {
            if !p.bot {
                continue;
            }
            let skill = p.skill;
            p.bot_delay =
                (2.7 + self.rng.random_range(-0.95..0.95) * (1.2 - skill * 0.65)).clamp(0.8, 4.4);
            p.bot_choice = if self
                .rng
                .random_bool((0.52 + skill as f64 * 0.44).clamp(0.0, 0.98))
            {
                self.target
            } else {
                (self.target + self.rng.random_range(1..3)) % 3
            };
        }
    }

    fn finish(&mut self) {
        let mut ranked: Vec<(u8, i32)> =
            self.performers.iter().map(|p| (p.seat, p.score)).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score) in ranked {
            if prior == Some(score) {
                places.last_mut().unwrap().push(seat);
            } else {
                places.push(vec![seat]);
                prior = Some(score);
            }
        }
        self.places = Some(places);
    }
}

impl Bout for CueCrowd {
    fn key(&self) -> &'static str {
        KEY
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time = (self.time + dt).min(LIMIT);
        let next_cue = (self.time / CUE_SECONDS).floor() as u32;
        if next_cue != self.cue && self.time < LIMIT {
            self.cue = next_cue;
            self.target = self.rng.random_range(0..3);
            for p in &mut self.performers {
                p.answered = false;
                p.last = None;
                p.last_points = 0;
            }
            self.plan_bots();
            events.push(json!({"kind":"cue","cue":self.cue,"target":self.target}));
        }
        let within = self.time - self.cue as f32 * CUE_SECONDS;
        for p in &mut self.performers {
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone {
                    continue;
                }
                if p.answered || within < 0.5 || within > 4.65 {
                    continue;
                }
                let Some(choice) = decode(input) else {
                    continue;
                };
                p.answered = true;
                p.last = Some(choice);
                let (perfect, good) = timing_windows(p.body_width);
                let points = if choice == self.target {
                    p.correct += 1;
                    if (within - 2.7).abs() <= perfect {
                        5
                    } else if (within - 2.7).abs() <= good {
                        3
                    } else {
                        1
                    }
                } else {
                    -1
                };
                p.last_points = points;
                p.score += points;
                events
                    .push(json!({"kind":"perform","seat":p.seat,"choice":choice,"points":points}));
            } else if self.time - p.seen > 1.0 {
                p.gone = true;
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.performers.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if p.bot && !p.answered && self.time - self.cue as f32 * CUE_SECONDS >= p.bot_delay {
            Input {
                fire: true,
                yaw: action_code(p.bot_choice),
                ..Default::default()
            }
        } else {
            Input::default()
        }
    }

    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"cue":self.cue,"target":self.target,
            "pulse":self.time - self.cue as f32 * CUE_SECONDS,"players":self.performers.iter().map(|p| json!({
                "seat":p.seat,"score":p.score,"correct":p.correct,"last":p.last,
                "body_width":p.body_width,"perfect_window":timing_windows(p.body_width).0,
                "good_window":timing_windows(p.body_width).1,
                "last_points":p.last_points,"answered":p.answered,"gone":p.gone
            })).collect::<Vec<_>>()})
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.places.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn people(n: u8) -> Vec<Contestant> {
        (0..n).map(Contestant::person).collect()
    }
    fn tap(choice: u8) -> Input {
        Input {
            fire: true,
            yaw: action_code(choice),
            ..Default::default()
        }
    }

    #[test]
    fn body_width_changes_perfect_and_good_spotlight_timing() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = CueCrowd::new(&[small.clone(), broad.clone()], 4);
        let target = game.target;
        game.time = 2.7 + 0.345;
        game.step(&[(0, tap(target)), (1, tap(target))], 0.01, &mut Vec::new());
        assert_eq!((game.performers[0].score, game.performers[1].score), (5, 3));
        let players = game.snapshot()["players"].as_array().unwrap().clone();
        assert!(
            players[0]["perfect_window"].as_f64().unwrap()
                > players[1]["perfect_window"].as_f64().unwrap()
        );
        assert!(
            players[0]["good_window"].as_f64().unwrap()
                < players[1]["good_window"].as_f64().unwrap()
        );
        let mut game = CueCrowd::new(&[small, broad], 4);
        let target = game.target;
        game.time = 2.7 + 0.895;
        game.step(&[(0, tap(target)), (1, tap(target))], 0.01, &mut Vec::new());
        assert_eq!((game.performers[0].score, game.performers[1].score), (1, 3));
    }

    #[test]
    fn only_one_valid_performance_per_cue_scores() {
        let mut game = CueCrowd::new(&people(2), 4);
        let target = game.target;
        let mut events = Vec::new();
        game.step(&[(0, tap(target))], 0.1, &mut events);
        assert_eq!(game.performers[0].score, 0);
        for _ in 0..26 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                0.1,
                &mut events,
            );
        }
        game.step(
            &[(0, tap(target)), (1, tap((target + 1) % 3))],
            0.01,
            &mut events,
        );
        assert_eq!(game.performers[0].score, 5);
        assert_eq!(game.performers[1].score, -1);
        game.step(&[(0, tap(target)), (1, tap(target))], 0.01, &mut events);
        assert_eq!(game.performers[0].score, 5);
        assert_eq!(game.performers[1].score, -1);
    }

    #[test]
    fn malformed_and_late_actions_are_rejected() {
        let mut game = CueCrowd::new(&people(2), 7);
        game.time = 4.7;
        game.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        yaw: f32::NAN,
                        ..Default::default()
                    },
                ),
                (1, tap(game.target)),
            ],
            0.01,
            &mut Vec::new(),
        );
        assert!(game.performers.iter().all(|p| p.score == 0));
    }

    #[test]
    fn bots_finish_and_skill_makes_a_difference() {
        let mut high = 0;
        let mut low = 0;
        for seed in 0..24 {
            let seats = vec![
                Contestant::bot(0, 1.0),
                Contestant::bot(1, 0.15),
                Contestant::bot(2, 0.4),
                Contestant::bot(3, 0.4),
            ];
            let mut game = CueCrowd::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            high += game.performers[0].score;
            low += game.performers[1].score;
        }
        assert!(high > low + 100, "high {high}, low {low}");
    }

    #[test]
    fn tied_scores_and_leavers_keep_their_place() {
        let mut game = CueCrowd::new(&people(3), 9);
        for _ in 0..61 * 60 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.performers[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }

    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1.0, 0.7), (0.7, 0.4)] {
            let mut wins = 0;
            let mut score_edge = 0;
            for seed in 0..200 {
                let seats = (0..4)
                    .map(|seat| Contestant::bot(seat, if seat == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&seats, seed + 10000, 1.0);
                let mut game = CueCrowd::new(&formed, seed + 10000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
                score_edge += game.performers[0].score - game.performers[1].score;
            }
            println!(
                "Cue the Crowd {favorite:.1} vs {field:.1}: {wins}/200 favorite wins, average edge {:.1}",
                score_edge as f32 / 200.
            );
        }
    }

    #[test]
    #[ignore]
    fn mixed_body_balance_report() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        let mut scores = [0i32; 4];
        let mut wins = [0u32; 4];
        for seed in 0..128u64 {
            let contestants = (0..4u8)
                .map(|seat| {
                    let mut c = Contestant::bot(seat, 0.5);
                    c.body_width = widths[(seat as usize + seed as usize) % 4];
                    c
                })
                .collect::<Vec<_>>();
            let mut game = CueCrowd::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.performers {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.performers.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Cue Crowd mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 250);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 20);
    }
}
