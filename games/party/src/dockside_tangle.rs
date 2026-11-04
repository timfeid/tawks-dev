//! Untangle the ropes by trading their right hooks. Each player has the same
//! knot but their own hooks, so choices and timing are server-authoritative.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "docksidetangle";
pub const ROPES: usize = 6;
pub const LIMIT: f32 = 58.;
pub const CLICK_COOLDOWN: f32 = 0.10;
const CODE: f32 = 1480.;

fn hook_cooldown(width: f32, span: usize) -> f32 {
    let body = BodyProfile::new(width);
    CLICK_COOLDOWN / body.speed
        + 0.18 * (span.min(ROPES - 1).saturating_sub(1) as f32 / (ROPES - 2) as f32)
            / body.reach.powi(6)
}

pub struct Rigger {
    pub seat: u8,
    pub body_width: f32,
    pub order: [u8; ROPES],
    pub selected: Option<u8>,
    pub score: u32,
    pub knots: u32,
    pub swaps: u32,
    pub wrong: u32,
    pub cooldown: f32,
    pub gone: bool,
    started: f32,
    seen: f32,
    bot_time: f32,
    bot_second: Option<u8>,
    skill: f32,
    bot: bool,
}

pub struct DocksideTangle {
    pub time: f32,
    pub riggers: Vec<Rigger>,
    seed: u64,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn hook_code(hook: usize) -> f32 {
    CODE + hook as f32
}
fn decode(input: &Input) -> Option<usize> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    (0..ROPES).find(|hook| (input.yaw - hook_code(*hook)).abs() < 0.01)
}

pub fn crossings(order: &[u8; ROPES]) -> u8 {
    let mut count = 0;
    for a in 0..ROPES {
        for b in a + 1..ROPES {
            count += (order[a] > order[b]) as u8;
        }
    }
    count
}

fn puzzle(seed: u64, round: u32) -> [u8; ROPES] {
    let mut rng = StdRng::seed_from_u64(seed ^ (round as u64 + 1).wrapping_mul(0x9e37_79b9));
    loop {
        let mut order = std::array::from_fn(|i| i as u8);
        order.shuffle(&mut rng);
        if crossings(&order) >= 5 {
            return order;
        }
    }
}

fn best_pair(order: &[u8; ROPES], rng: &mut StdRng) -> (usize, usize) {
    let before = crossings(order);
    let mut best = Vec::new();
    let mut gain = 0;
    for a in 0..ROPES {
        for b in a + 1..ROPES {
            let mut next = *order;
            next.swap(a, b);
            let improvement = before.saturating_sub(crossings(&next));
            if improvement > gain {
                gain = improvement;
                best.clear();
                best.push((a, b));
            } else if improvement == gain {
                best.push((a, b));
            }
        }
    }
    best[rng.random_range(0..best.len())]
}

impl DocksideTangle {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let order = puzzle(seed, 0);
        let riggers = contestants
            .iter()
            .map(|c| Rigger {
                seat: c.seat,
                body_width: BodyProfile::new(c.body_width).width,
                order,
                selected: None,
                score: 0,
                knots: 0,
                swaps: 0,
                wrong: 0,
                cooldown: 0.,
                gone: false,
                started: 0.,
                seen: 0.,
                bot_time: rng.random_range(0.6..1.6),
                bot_second: None,
                // A small speed edge compounds across many hook swaps.
                skill: 0.5 + 0.5 * c.skill.clamp(0., 1.).powi(2),
                bot: c.is_bot,
            })
            .collect();
        Self {
            time: 0.,
            riggers,
            seed,
            rng,
            places: None,
        }
    }
    fn finish(&mut self) {
        let mut ranked = self
            .riggers
            .iter()
            .map(|p| (p.seat, p.score, crossings(&p.order)))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = vec![];
        let mut prior = None;
        for (seat, score, cross) in ranked {
            if prior == Some((score, cross)) {
                places.last_mut().unwrap().push(seat)
            } else {
                places.push(vec![seat]);
                prior = Some((score, cross))
            }
        }
        self.places = Some(places);
    }
}

impl Bout for DocksideTangle {
    fn key(&self) -> &'static str {
        KEY
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        for p in &mut self.riggers {
            p.cooldown = (p.cooldown - dt).max(0.);
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone || p.cooldown > 0. {
                    continue;
                }
                let Some(hook) = decode(input) else {
                    continue;
                };
                if let Some(first) = p.selected.take() {
                    let span = first.abs_diff(hook as u8) as usize;
                    p.cooldown = hook_cooldown(p.body_width, span);
                    if first as usize == hook {
                        events.push(json!({"kind":"unselect","seat":p.seat,"hook":hook}));
                        continue;
                    }
                    let before = crossings(&p.order);
                    p.order.swap(first as usize, hook);
                    let after = crossings(&p.order);
                    p.swaps += 1;
                    if after >= before {
                        p.wrong += 1;
                    }
                    p.bot_time = self.time
                        + 0.58
                        + (1. - p.skill) * 0.35
                        + self.rng.random_range(0.0..0.28)
                        + hook_cooldown(p.body_width, span)
                        - CLICK_COOLDOWN;
                    events.push(
                        json!({"kind":"swap","seat":p.seat,"a":first,"b":hook,"crossings":after}),
                    );
                    if after == 0 {
                        let quick = (35. - (self.time - p.started) * 1.6).max(0.) as u32;
                        p.score += 100 + quick;
                        p.knots += 1;
                        p.order = puzzle(self.seed, p.knots);
                        p.started = self.time;
                        p.bot_time = self.time + self.rng.random_range(1.0..2.0);
                        events.push(json!({"kind":"clear","seat":p.seat,"knots":p.knots}));
                    }
                } else {
                    p.selected = Some(hook as u8);
                    p.cooldown = hook_cooldown(p.body_width, 0);
                    p.bot_time = self.time
                        + 0.19
                        + (1. - p.skill) * 0.08
                        + self.rng.random_range(0.0..0.1)
                        + p.cooldown
                        - CLICK_COOLDOWN;
                    events.push(json!({"kind":"select","seat":p.seat,"hook":hook}));
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
        let Some(index) = self.riggers.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let p = &mut self.riggers[index];
        if !p.bot || p.gone || p.cooldown > 0. || self.time < p.bot_time {
            return Input::default();
        }
        let hook = if p.selected.is_some() {
            p.bot_second
                .take()
                .unwrap_or_else(|| best_pair(&p.order, &mut self.rng).1 as u8) as usize
        } else {
            let (a, b) = if self.rng.random::<f32>() < (1. - p.skill) * 0.06 {
                let a = self.rng.random_range(0..ROPES);
                let mut b = self.rng.random_range(0..ROPES - 1);
                if b >= a {
                    b += 1;
                }
                (a, b)
            } else {
                best_pair(&p.order, &mut self.rng)
            };
            p.bot_second = Some(b as u8);
            a
        };
        Input {
            fire: true,
            yaw: hook_code(hook),
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,
            "players":self.riggers.iter().map(|p|json!({"seat":p.seat,"order":p.order,"selected":p.selected,
                "crossings":crossings(&p.order),"score":p.score,"knots":p.knots,"swaps":p.swaps,
                "wrong":p.wrong,"cooldown":p.cooldown,"gone":p.gone,
                "body_width":p.body_width,"near_pull":hook_cooldown(p.body_width,1),
                "far_pull":hook_cooldown(p.body_width,ROPES-1)})).collect::<Vec<_>>()})
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
    fn press(i: usize) -> Input {
        Input {
            fire: true,
            yaw: hook_code(i),
            ..Default::default()
        }
    }
    fn advance(game: &mut DocksideTangle, seconds: f32) {
        for _ in 0..(seconds * 60.) as usize {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
    }
    #[test]
    fn body_width_trades_near_grabs_for_far_reach() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = DocksideTangle::new(&[small, broad], 3);
        assert!(hook_cooldown(0.7, 1) < hook_cooldown(1.4, 1));
        assert!(hook_cooldown(1.4, ROPES - 1) < hook_cooldown(0.7, ROPES - 1));
        game.riggers[0].selected = Some(0);
        game.riggers[1].selected = Some(0);
        game.step(&[(0, press(5)), (1, press(5))], 0.01, &mut Vec::new());
        assert_eq!(game.riggers[0].swaps, 1);
        assert_eq!(game.riggers[1].swaps, 1);
        assert!(game.riggers[1].cooldown < game.riggers[0].cooldown);
        assert_eq!(
            game.snapshot()["players"][0]["body_width"]
                .as_f64()
                .unwrap() as f32,
            0.7
        );
    }
    #[test]
    fn puzzles_are_valid_and_tangled() {
        for seed in 0..200 {
            let order = puzzle(seed, 0);
            let mut sorted = order;
            sorted.sort();
            assert_eq!(sorted, std::array::from_fn(|i| i as u8));
            assert!(crossings(&order) >= 5);
        }
    }
    #[test]
    fn two_valid_hooks_swap_and_cooldown_blocks_repeats() {
        let mut game = DocksideTangle::new(&people(2), 3);
        let start = game.riggers[0].order;
        game.step(
            &[(0, press(0)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.riggers[0].selected, Some(0));
        game.step(
            &[(0, press(1)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.riggers[0].order, start);
        advance(&mut game, 0.2);
        game.step(
            &[(0, press(1)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        let mut expected = start;
        expected.swap(0, 1);
        assert_eq!(game.riggers[0].order, expected);
        assert_eq!(game.riggers[0].swaps, 1);
    }
    #[test]
    fn invalid_inputs_do_not_select_or_score() {
        let mut game = DocksideTangle::new(&people(2), 5);
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
                (1, press(99)),
            ],
            0.01,
            &mut Vec::new(),
        );
        assert!(
            game.riggers
                .iter()
                .all(|p| p.selected.is_none() && p.score == 0)
        );
    }
    #[test]
    fn a_clear_scores_and_starts_a_new_knot() {
        let mut game = DocksideTangle::new(&people(2), 8);
        game.riggers[0].order = [1, 0, 2, 3, 4, 5];
        game.step(
            &[(0, press(0)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        advance(&mut game, 0.2);
        game.step(
            &[(0, press(1)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.riggers[0].knots, 1);
        assert!(game.riggers[0].score >= 100);
        assert!(crossings(&game.riggers[0].order) >= 5);
    }
    #[test]
    fn leavers_and_ties_finish() {
        let mut game = DocksideTangle::new(&people(3), 9);
        while game.placements().is_none() {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.riggers[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }
    #[test]
    fn bots_solve_knots() {
        for seed in 0..20 {
            let people = (0..4).map(|s| Contestant::bot(s, 0.7)).collect::<Vec<_>>();
            let mut game = DocksideTangle::new(&people, seed);
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            assert!(game.riggers.iter().all(|p| p.knots >= 1));
        }
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
            let mut game = DocksideTangle::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.riggers {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.riggers.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Dockside Tangle mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 1500);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 15);
    }
    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1., 0.7), (0.7, 0.4), (0.7, 0.7)] {
            let mut wins = 0;
            for seed in 0..1000 {
                let people = (0..4)
                    .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&people, seed + 61000, 1.);
                let mut game = DocksideTangle::new(&formed, seed + 61000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
            }
            println!("Dockside Tangle {favorite} vs {field}: {wins}/1000 favorite wins");
        }
    }
}
