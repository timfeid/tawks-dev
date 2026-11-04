//! Prism Foundry: a forge drops one molten crystal into a mold at a time.
//! Slide the cooling mold into place and quench as it reaches the gold line.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "prismforge";
pub const LIMIT: f32 = 63.0;
pub const DELIVERY: f32 = 3.5;
pub const ARRIVAL: f32 = 2.1;
pub const CHUTES: usize = 3;
const MOVE_DELAY: f32 = 0.19;

fn move_delay(width: f32) -> f32 {
    MOVE_DELAY / BodyProfile::new(width).speed
}

fn catch_window(width: f32) -> f32 {
    0.5 * BodyProfile::new(width).radius
}

fn precision_window(width: f32, base: f32) -> f32 {
    base / BodyProfile::new(width).radius
}

#[derive(Clone)]
pub struct Sorter {
    pub seat: u8,
    pub body_width: f32,
    pub lane: u8,
    pub score: i32,
    pub caught: u32,
    pub misses: u32,
    pub last_points: i32,
    pub gone: bool,
    bot: bool,
    skill: f32,
    answered: bool,
    move_at: f32,
    seen: f32,
    plan_lane: u8,
    notice_at: f32,
    catch_at: f32,
}

pub struct ParcelPanic {
    pub time: f32,
    pub parcel: u32,
    pub chute: u8,
    pub style: u8,
    pub sorters: Vec<Sorter>,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

impl ParcelPanic {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let chute = rng.random_range(0..CHUTES as u8);
        let style = rng.random_range(0..3);
        let sorters = contestants
            .iter()
            .map(|c| Sorter {
                seat: c.seat,
                body_width: BodyProfile::new(c.body_width).width,
                lane: (c.seat % CHUTES as u8),
                score: 0,
                caught: 0,
                misses: 0,
                last_points: 0,
                gone: false,
                bot: c.is_bot,
                skill: c.skill.clamp(0., 1.),
                answered: false,
                move_at: 0.,
                seen: 0.,
                plan_lane: chute,
                notice_at: 0.,
                catch_at: ARRIVAL,
            })
            .collect();
        let mut game = Self {
            time: 0.,
            parcel: 0,
            chute,
            style,
            sorters,
            rng,
            places: None,
        };
        game.plan_bots();
        game
    }

    fn plan_bots(&mut self) {
        for p in &mut self.sorters {
            if !p.bot {
                continue;
            }
            let skill = p.skill;
            p.notice_at = self.rng.random_range(0.15..0.55) + (1. - skill) * 0.5;
            p.plan_lane = if self.rng.random_bool((0.78 + skill as f64 * 0.18).min(0.98)) {
                self.chute
            } else {
                (self.chute + self.rng.random_range(1..3)) % 3
            };
            p.catch_at = ARRIVAL + self.rng.random_range(-0.5..0.5) * (1.4 - skill * 0.75);
        }
    }

    fn finish(&mut self) {
        let mut scores = self
            .sorters
            .iter()
            .map(|p| (p.seat, p.score))
            .collect::<Vec<_>>();
        scores.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, score) in scores {
            if last == Some(score) {
                places.last_mut().unwrap().push(seat)
            } else {
                places.push(vec![seat]);
                last = Some(score)
            }
        }
        self.places = Some(places);
    }
}

impl Bout for ParcelPanic {
    fn key(&self) -> &'static str {
        KEY
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        let next = (self.time / DELIVERY).floor() as u32;
        if next != self.parcel && self.time < LIMIT {
            self.parcel = next;
            self.chute = self.rng.random_range(0..CHUTES as u8);
            self.style = self.rng.random_range(0..3);
            for p in &mut self.sorters {
                p.answered = false;
                p.last_points = 0;
            }
            self.plan_bots();
            events
                .push(json!({"kind":"parcel","index":next,"chute":self.chute,"style":self.style}));
        }
        let age = self.time - self.parcel as f32 * DELIVERY;
        for p in &mut self.sorters {
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone {
                    continue;
                }
                if self.time >= p.move_at && input.stick.x.is_finite() && input.stick.x.abs() > 0.55
                {
                    p.lane =
                        (p.lane as i8 + if input.stick.x > 0. { 1 } else { -1 }).clamp(0, 2) as u8;
                    p.move_at = self.time + move_delay(p.body_width);
                }
                if input.fire && !p.answered && age >= 0.55 && age <= 2.75 {
                    p.answered = true;
                    let distance = (age - ARRIVAL).abs();
                    let window = catch_window(p.body_width);
                    let points = if p.lane != self.chute || distance > window {
                        -1
                    } else if distance <= precision_window(p.body_width, 0.14) {
                        5
                    } else if distance <= precision_window(p.body_width, 0.31) {
                        3
                    } else {
                        1
                    };
                    if points > 0 {
                        p.caught += 1
                    } else {
                        p.misses += 1
                    }
                    p.score += points;
                    p.last_points = points;
                    events
                        .push(json!({"kind":"catch","seat":p.seat,"points":points,"lane":p.lane}));
                }
            } else if self.time - p.seen > 1. {
                p.gone = true;
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.sorters.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if !p.bot || p.gone {
            return Input::default();
        }
        let age = self.time - self.parcel as f32 * DELIVERY;
        let x = if age >= p.notice_at && p.lane != p.plan_lane {
            if p.lane < p.plan_lane { 1. } else { -1. }
        } else {
            0.
        };
        Input {
            stick: crate::bout::Stick { x, z: 0. },
            fire: !p.answered && age >= p.catch_at,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"parcel":self.parcel,
            "chute":self.chute,"style":self.style,"age":self.time-self.parcel as f32*DELIVERY,
            "arrival":ARRIVAL,"players":self.sorters.iter().map(|p|json!({"seat":p.seat,"lane":p.lane,
                "score":p.score,"caught":p.caught,"misses":p.misses,"last_points":p.last_points,
                "answered":p.answered,"gone":p.gone,"body_width":p.body_width,
                "move_delay":move_delay(p.body_width),"catch_window":catch_window(p.body_width),
                "perfect_window":precision_window(p.body_width,0.14)})).collect::<Vec<_>>()})
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
    fn at_lane(game: &mut ParcelPanic, seat: u8, lane: u8) {
        game.sorters[seat as usize].lane = lane;
    }

    #[test]
    fn body_width_trades_slide_speed_for_catch_reach() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = ParcelPanic::new(&[small, broad], 3);
        assert!(move_delay(game.sorters[0].body_width) < move_delay(game.sorters[1].body_width));
        assert!(
            catch_window(game.sorters[0].body_width) < catch_window(game.sorters[1].body_width)
        );
        let chute = game.chute;
        at_lane(&mut game, 0, chute);
        at_lane(&mut game, 1, chute);
        game.time = ARRIVAL + 0.48 - 0.01;
        game.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (
                    1,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
            ],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.sorters[0].score, -1);
        assert_eq!(game.sorters[1].score, 1);
        assert_eq!(
            game.snapshot()["players"][0]["body_width"]
                .as_f64()
                .unwrap() as f32,
            0.7
        );
    }

    #[test]
    fn hatch_scores_only_the_right_chute_at_arrival() {
        let mut game = ParcelPanic::new(&people(2), 3);
        let chute = game.chute;
        at_lane(&mut game, 0, chute);
        at_lane(&mut game, 1, (chute + 1) % 3);
        game.time = ARRIVAL - 0.01;
        game.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (
                    1,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
            ],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.sorters[0].score, 5);
        assert_eq!(game.sorters[1].score, -1);
        game.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.sorters[0].score, 5);
    }

    #[test]
    fn early_press_and_move_rate_are_validated() {
        let mut game = ParcelPanic::new(&people(2), 4);
        game.sorters[0].lane = 0;
        for _ in 0..6 {
            game.step(
                &[(
                    0,
                    Input {
                        stick: crate::bout::Stick { x: 1., z: 0. },
                        fire: true,
                        ..Default::default()
                    },
                )],
                0.01,
                &mut Vec::new(),
            );
        }
        assert_eq!(game.sorters[0].lane, 1);
        assert_eq!(game.sorters[0].score, 0);
        game.step(
            &[(
                0,
                Input {
                    stick: crate::bout::Stick { x: f32::NAN, z: 0. },
                    ..Default::default()
                },
            )],
            0.1,
            &mut Vec::new(),
        );
        assert_eq!(game.sorters[0].lane, 1);
    }

    #[test]
    fn leavers_and_ties_finish() {
        let mut game = ParcelPanic::new(&people(3), 7);
        for _ in 0..64 * 60 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.sorters[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }

    #[test]
    fn bots_catch_parcels_and_sharper_bots_score_more() {
        let mut top = 0;
        let mut low = 0;
        for seed in 0..20 {
            let seats = vec![
                Contestant::bot(0, 1.),
                Contestant::bot(1, 0.15),
                Contestant::bot(2, 0.4),
            ];
            let mut game = ParcelPanic::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = (0..3)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            top += game.sorters[0].score;
            low += game.sorters[1].score;
        }
        assert!(top > low + 30, "top {top} low {low}");
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
            let mut game = ParcelPanic::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.sorters {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.sorters.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Prism Foundry mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 400);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 20);
    }

    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1., 0.7), (0.7, 0.4)] {
            let mut wins = 0;
            for seed in 0..200 {
                let seats = (0..4)
                    .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&seats, seed + 15000, 1.);
                let mut game = ParcelPanic::new(&formed, seed + 15000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
            }
            println!("Prism Foundry {favorite} vs {field}: {wins}/200 favorite wins");
        }
    }
}
