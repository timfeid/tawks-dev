//! Reef Bloom is a short coral-gardening contest. Polyps spread to open reef
//! cells, live briefly, then fade, so every new planting changes the garden.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "reefbloom";
pub const COLS: usize = 9;
pub const ROWS: usize = 7;
pub const CELLS: usize = COLS * ROWS;
pub const LIMIT: f32 = 63.0;
pub const GROW_EVERY: f32 = 1.2;
pub const PLANT_COOLDOWN: f32 = 2.4;
pub const PLANT_POINTS: u32 = 30;
const CODE: f32 = 850.0;

#[derive(Clone, Copy, Default)]
pub struct Coral {
    pub owner: Option<u8>,
    pub life: u8,
}

pub struct Gardener {
    pub seat: u8,
    pub body_width: f32,
    pub score: u32,
    pub planted: u32,
    pub cooldown: f32,
    pub gone: bool,
    seen: f32,
    next_bot: f32,
    skill: f32,
    bot: bool,
}

pub struct ReefBloom {
    pub time: f32,
    pub growth: u32,
    pub cells: [Coral; CELLS],
    pub gardeners: Vec<Gardener>,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn plant_code(index: usize) -> f32 {
    CODE + index as f32
}

fn decode(input: &Input) -> Option<usize> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    (0..CELLS).find(|i| (input.yaw - plant_code(*i)).abs() < 0.01)
}

pub fn neighbors(index: usize) -> impl Iterator<Item = usize> {
    let x = index % COLS;
    let y = index / COLS;
    [
        x.checked_sub(1).map(|x| y * COLS + x),
        (x + 1 < COLS).then_some(y * COLS + x + 1),
        y.checked_sub(1).map(|y| y * COLS + x),
        (y + 1 < ROWS).then_some((y + 1) * COLS + x),
    ]
    .into_iter()
    .flatten()
}

impl ReefBloom {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let gardeners = contestants
            .iter()
            .map(|c| Gardener {
                seat: c.seat,
                body_width: BodyProfile::new(c.body_width).width,
                score: 0,
                planted: 0,
                cooldown: 0.,
                gone: false,
                seen: 0.,
                next_bot: rng.random_range(0.4..1.2),
                skill: c.skill.clamp(0., 1.),
                bot: c.is_bot,
            })
            .collect();
        Self {
            time: 0.,
            growth: 0,
            cells: [Coral::default(); CELLS],
            gardeners,
            rng,
            places: None,
        }
    }

    fn grow(&mut self, events: &mut Vec<Value>) {
        let old = self.cells;
        let mut next = old;
        for (index, cell) in old.iter().enumerate() {
            if cell.owner.is_some() {
                next[index].life = cell.life.saturating_sub(1);
                if next[index].life == 0 {
                    next[index].owner = None;
                }
                continue;
            }
            let mut counts = [0u8; 8];
            for near in neighbors(index) {
                if let Some(owner) = old[near].owner {
                    if old[near].life >= 2 {
                        counts[owner as usize] += 1;
                    }
                }
            }
            let max = *counts.iter().max().unwrap();
            if max == 0 {
                continue;
            }
            let leaders = counts
                .iter()
                .enumerate()
                .filter(|(_, n)| **n == max)
                .collect::<Vec<_>>();
            if leaders.len() == 1 {
                next[index] = Coral {
                    owner: Some(leaders[0].0 as u8),
                    life: 3,
                };
            }
        }
        self.cells = next;
        for p in &mut self.gardeners {
            p.score += self
                .cells
                .iter()
                .filter(|c| c.owner == Some(p.seat))
                .count() as u32;
        }
        events.push(json!({"kind":"bloom","growth":self.growth}));
    }

    fn finish(&mut self) {
        let mut sorted = self
            .gardeners
            .iter()
            .map(|p| (p.seat, p.score))
            .collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, score) in sorted {
            if last == Some(score) {
                places.last_mut().unwrap().push(seat)
            } else {
                places.push(vec![seat]);
                last = Some(score)
            }
        }
        self.places = Some(places);
    }

    fn bot_cell(&mut self, skill: f32) -> Option<usize> {
        let options = (0..CELLS)
            .filter(|i| self.cells[*i].owner.is_none())
            .collect::<Vec<_>>();
        if options.is_empty() {
            return None;
        }
        if self.rng.random::<f32>() > (0.32 + skill * 0.58) {
            return Some(options[self.rng.random_range(0..options.len())]);
        }
        let mut best = Vec::new();
        let mut high = i32::MIN;
        for index in options {
            let fresh = neighbors(index)
                .filter(|n| self.cells[*n].owner.is_none())
                .count() as i32;
            let same = neighbors(index)
                .filter(|n| self.cells[*n].owner.is_some())
                .count() as i32;
            let score = fresh * 3 - same + self.rng.random_range(0..3);
            if score > high {
                high = score;
                best.clear();
                best.push(index)
            } else if score == high {
                best.push(index)
            }
        }
        Some(best[self.rng.random_range(0..best.len())])
    }
}

impl Bout for ReefBloom {
    fn key(&self) -> &'static str {
        KEY
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        for p in &mut self.gardeners {
            p.cooldown = (p.cooldown - dt).max(0.);
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone || p.cooldown > 0. {
                    continue;
                }
                let Some(index) = decode(input) else { continue };
                if self.cells[index].owner.is_some() {
                    continue;
                }
                let body = BodyProfile::new(p.body_width);
                self.cells[index] = Coral {
                    owner: Some(p.seat),
                    life: if body.width < 0.9 {
                        5
                    } else if body.width > 1.1 {
                        7
                    } else {
                        6
                    },
                };
                p.cooldown = PLANT_COOLDOWN * body.mass;
                p.planted += 1;
                p.score += PLANT_POINTS;
                p.next_bot = self.time
                    + p.cooldown
                    + 0.25
                    + (1. - p.skill) * 4.5
                    + self.rng.random_range(-0.2..0.2);
                events.push(json!({"kind":"plant","seat":p.seat,"cell":index}));
            } else if self.time - p.seen > 1. {
                p.gone = true;
            }
        }
        let next = (self.time / GROW_EVERY).floor() as u32;
        while self.growth < next {
            self.growth += 1;
            self.grow(events);
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.gardeners.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if !p.bot || p.gone || p.cooldown > 0. || self.time < p.next_bot {
            return Input::default();
        }
        let skill = p.skill;
        self.bot_cell(skill).map_or(Input::default(), |cell| Input {
            fire: true,
            yaw: plant_code(cell),
            ..Default::default()
        })
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"growth":self.growth,
            "cols":COLS,"rows":ROWS,
            "cells":self.cells.iter().map(|c|json!({"owner":c.owner,"life":c.life})).collect::<Vec<_>>(),
            "players":self.gardeners.iter().map(|p|json!({"seat":p.seat,"score":p.score,
                "planted":p.planted,"cooldown":p.cooldown,"gone":p.gone,"body_width":p.body_width})).collect::<Vec<_>>()})
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
    fn plant(index: usize) -> Input {
        Input {
            fire: true,
            yaw: plant_code(index),
            ..Default::default()
        }
    }
    #[test]
    fn body_size_trades_planting_pace_for_coral_lifespan() {
        let mut seats = people(2);
        seats[0].body_width = 0.7;
        seats[1].body_width = 1.4;
        let mut game = ReefBloom::new(&seats, 8);
        game.step(&[(0, plant(10)), (1, plant(50))], 0.01, &mut vec![]);
        assert!(game.gardeners[0].cooldown < game.gardeners[1].cooldown);
        assert!(game.cells[10].life < game.cells[50].life);
        for _ in 0..150 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut vec![],
            );
        }
        assert_eq!(game.gardeners[0].cooldown, 0.);
        assert!(game.gardeners[1].cooldown > 0.);
    }
    #[test]
    fn mixed_size_bots_keep_scoring() {
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
            let mut game = ReefBloom::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut vec![]);
            }
            for p in &game.gardeners {
                scores[p.seat as usize] += p.score;
            }
        }
        let least = *scores.iter().min().unwrap();
        let most = *scores.iter().max().unwrap();
        assert!(least > 0 && most < least * 2, "scores={scores:?}");
    }
    #[test]
    fn only_open_cells_take_valid_seeds() {
        let mut game = ReefBloom::new(&people(2), 3);
        let mut events = Vec::new();
        game.step(&[(0, plant(10)), (1, plant(10))], 0.01, &mut events);
        assert_eq!(game.cells[10].owner, Some(0));
        assert_eq!(game.gardeners[0].score, PLANT_POINTS);
        assert_eq!(game.gardeners[1].planted, 0);
        game.step(
            &[
                (0, plant(11)),
                (
                    1,
                    Input {
                        fire: true,
                        yaw: f32::NAN,
                        ..Default::default()
                    },
                ),
            ],
            0.01,
            &mut events,
        );
        assert_eq!(game.gardeners[0].planted, 1);
        assert_eq!(game.gardeners[1].planted, 0);
    }
    #[test]
    fn coral_spreads_fades_and_scores_living_area() {
        let mut game = ReefBloom::new(&people(2), 4);
        game.step(
            &[(0, plant(31)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        for _ in 0..(GROW_EVERY * 60.) as usize {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.cells.iter().filter(|c| c.owner == Some(0)).count() >= 5);
        assert!(game.gardeners[0].score >= 5);
        while game.growth < 6 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.cells[31].owner.is_none(), "the first polyp faded");
    }
    #[test]
    fn ties_and_leavers_finish() {
        let mut game = ReefBloom::new(&people(3), 5);
        for _ in 0..64 * 60 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.gardeners[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }
    #[test]
    fn bots_plant_repeatedly_and_skill_helps() {
        let mut top = 0;
        let mut low = 0;
        for seed in 0..20 {
            let people = vec![
                Contestant::bot(0, 1.),
                Contestant::bot(1, 0.15),
                Contestant::bot(2, 0.4),
            ];
            let mut game = ReefBloom::new(&people, seed);
            while game.placements().is_none() {
                let inputs = (0..3)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            top += game.gardeners[0].score;
            low += game.gardeners[1].score;
            assert!(game.gardeners.iter().all(|p| p.planted > 5));
        }
        assert!(top > low, "top {top} low {low}");
    }
    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1., 0.7), (0.7, 0.4)] {
            let mut wins = 0;
            let mut score_edge = 0i64;
            let mut plant_edge = 0i64;
            for seed in 0..200 {
                let seats = (0..4)
                    .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&seats, seed + 18000, 1.);
                let mut game = ReefBloom::new(&formed, seed + 18000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
                score_edge += game.gardeners[0].score as i64 - game.gardeners[1].score as i64;
                plant_edge += game.gardeners[0].planted as i64 - game.gardeners[1].planted as i64;
            }
            println!(
                "Reef Bloom {favorite} vs {field}: {wins}/200 favorite wins, score edge {:.1}, plant edge {:.1}",
                score_edge as f32 / 200.,
                plant_edge as f32 / 200.
            );
        }
    }
}
