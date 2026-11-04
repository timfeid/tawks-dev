//! Rotate wind ducts to carry a bellows' gust to a rooftop pinwheel.
//! Every player gets the same generated puzzle, but solves it independently.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "whirligig";
pub const SIDE: usize = 4;
pub const TILES: usize = SIDE * SIDE;
pub const LIMIT: f32 = 60.;
pub const TURN_COOLDOWN: f32 = 0.14;
const CODE: f32 = 980.;
const N: u8 = 1;
const E: u8 = 2;
const S: u8 = 4;
const W: u8 = 8;

fn turn_cooldown(width: f32) -> f32 {
    TURN_COOLDOWN / BodyProfile::new(width).speed.powi(2)
}

fn gust_points(width: f32) -> u32 {
    (100. + (BodyProfile::new(width).width - 1.) * 20.).round() as u32
}

#[derive(Clone, Copy, Default)]
pub struct Duct {
    pub ports: u8,
    target: u8,
}

pub struct Tinker {
    pub seat: u8,
    pub body_width: f32,
    pub score: u32,
    pub circuits: u32,
    pub turns: u32,
    pub board: [Duct; TILES],
    pub cooldown: f32,
    pub gone: bool,
    started: f32,
    seen: f32,
    next_bot: f32,
    skill: f32,
    bot: bool,
}

pub struct Whirligig {
    pub time: f32,
    pub tinkers: Vec<Tinker>,
    seed: u64,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn turn_code(index: usize) -> f32 {
    CODE + index as f32
}

fn decode(input: &Input) -> Option<usize> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    (0..TILES).find(|i| (input.yaw - turn_code(*i)).abs() < 0.01)
}

fn rotated(mask: u8) -> u8 {
    ((mask << 1) | (mask >> 3)) & 15
}

fn adjacent(index: usize) -> impl Iterator<Item = (usize, u8, u8)> {
    let x = index % SIDE;
    let y = index / SIDE;
    [
        y.checked_sub(1).map(|_| (index - SIDE, N, S)),
        (x + 1 < SIDE).then_some((index + 1, E, W)),
        (y + 1 < SIDE).then_some((index + SIDE, S, N)),
        x.checked_sub(1).map(|_| (index - 1, W, E)),
    ]
    .into_iter()
    .flatten()
}

fn path_from(rng: &mut StdRng, at: usize, used: u16, path: &mut Vec<usize>) -> bool {
    if at == TILES - 1 {
        return (6..=11).contains(&path.len());
    }
    if path.len() >= 11 {
        return false;
    }
    let mut choices = adjacent(at).map(|(i, _, _)| i).collect::<Vec<_>>();
    choices.shuffle(rng);
    for next in choices {
        if used & (1 << next) != 0 {
            continue;
        }
        path.push(next);
        if path_from(rng, next, used | 1 << next, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn connection(from: usize, to: usize) -> u8 {
    if to == from + 1 {
        E
    } else if from == to + 1 {
        W
    } else if to == from + SIDE {
        S
    } else {
        N
    }
}

fn puzzle(seed: u64, round: u32) -> [Duct; TILES] {
    let mut rng = StdRng::seed_from_u64(seed ^ ((round as u64 + 1) * 0x9e37_79b9));
    loop {
        let mut path = vec![0];
        if !path_from(&mut rng, 0, 1, &mut path) {
            continue;
        }
        let mut board = [Duct::default(); TILES];
        for duct in &mut board {
            let shape = if rng.random_bool(0.7) { N | E } else { N | S };
            duct.ports = (0..rng.random_range(0..4)).fold(shape, |m, _| rotated(m));
        }
        for (place, &index) in path.iter().enumerate() {
            let enter = if place == 0 {
                W
            } else {
                connection(index, path[place - 1])
            };
            let exit = if place + 1 == path.len() {
                E
            } else {
                connection(index, path[place + 1])
            };
            let target = enter | exit;
            board[index].target = target;
            board[index].ports = (0..rng.random_range(0..4)).fold(target, |m, _| rotated(m));
        }
        if path
            .iter()
            .filter(|&&i| board[i].ports != board[i].target)
            .count()
            >= 4
            && !connected(&board).0
        {
            return board;
        }
    }
}

pub fn connected(board: &[Duct; TILES]) -> (bool, u8) {
    if board[0].ports & W == 0 {
        return (false, 0);
    }
    let mut visited: u16 = 1;
    let mut fringe = vec![0];
    while let Some(index) = fringe.pop() {
        for (next, out, into) in adjacent(index) {
            if board[index].ports & out != 0
                && board[next].ports & into != 0
                && visited & (1 << next) == 0
            {
                visited |= 1 << next;
                fringe.push(next);
            }
        }
    }
    (
        visited & (1 << (TILES - 1)) != 0 && board[TILES - 1].ports & E != 0,
        visited.count_ones() as u8,
    )
}

impl Whirligig {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let board = puzzle(seed, 0);
        Self {
            time: 0.,
            seed,
            tinkers: contestants
                .iter()
                .map(|c| Tinker {
                    seat: c.seat,
                    body_width: BodyProfile::new(c.body_width).width,
                    score: 0,
                    circuits: 0,
                    turns: 0,
                    board,
                    cooldown: 0.,
                    gone: false,
                    started: 0.,
                    seen: 0.,
                    next_bot: rng.random_range(0.8..2.0),
                    // Compress the lower skill gap; a small speed edge compounds across circuits.
                    skill: (0.5 + 0.5 * c.skill.clamp(0., 1.).powi(2)).clamp(0., 1.),
                    bot: c.is_bot,
                })
                .collect(),
            rng,
            places: None,
        }
    }

    fn finish(&mut self) {
        let mut ranked = self
            .tinkers
            .iter()
            .map(|p| {
                let (_, reach) = connected(&p.board);
                (p.seat, p.score, reach)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = vec![];
        let mut prior = None;
        for (seat, score, reach) in ranked {
            if prior == Some((score, reach)) {
                places.last_mut().unwrap().push(seat);
            } else {
                places.push(vec![seat]);
                prior = Some((score, reach));
            }
        }
        self.places = Some(places);
    }
}

impl Bout for Whirligig {
    fn key(&self) -> &'static str {
        KEY
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        for p in &mut self.tinkers {
            p.cooldown = (p.cooldown - dt).max(0.);
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone || p.cooldown > 0. {
                    continue;
                }
                let Some(index) = decode(input) else {
                    continue;
                };
                p.board[index].ports = rotated(p.board[index].ports);
                p.turns += 1;
                p.cooldown = turn_cooldown(p.body_width);
                p.next_bot = self.time
                    + (0.35 + (1. - p.skill) * 0.08 + self.rng.random_range(0.0..0.18))
                        / BodyProfile::new(p.body_width).speed;
                events.push(json!({"kind":"turn","seat":p.seat,"tile":index}));
                if connected(&p.board).0 {
                    let quick = (30. - (self.time - p.started) * 1.2).max(0.) as u32;
                    p.score += gust_points(p.body_width) + quick;
                    p.circuits += 1;
                    p.started = self.time;
                    p.board = puzzle(self.seed, p.circuits);
                    p.next_bot = self.time + self.rng.random_range(1.5..2.5);
                    events.push(json!({"kind":"whirl","seat":p.seat,"circuits":p.circuits}));
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
        let Some(p) = self.tinkers.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if !p.bot || p.gone || p.cooldown > 0. || self.time < p.next_bot {
            return Input::default();
        }
        let candidates = (0..TILES)
            .filter(|i| p.board[*i].target != 0 && p.board[*i].ports != p.board[*i].target)
            .collect::<Vec<_>>();
        let index = if candidates.is_empty() || self.rng.random::<f32>() < (1. - p.skill) * 0.01 {
            self.rng.random_range(0..TILES)
        } else {
            candidates[self.rng.random_range(0..candidates.len())]
        };
        Input {
            fire: true,
            yaw: turn_code(index),
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,
            "players":self.tinkers.iter().map(|p| {
                let (_, reach) = connected(&p.board);
                json!({"seat":p.seat,"score":p.score,"circuits":p.circuits,"turns":p.turns,
                    "cooldown":p.cooldown,"gone":p.gone,"reach":reach,
                    "body_width":p.body_width,"turn_cooldown":turn_cooldown(p.body_width),
                    "gust_points":gust_points(p.body_width),
                    "tiles":p.board.iter().map(|d|d.ports).collect::<Vec<_>>()})
            }).collect::<Vec<_>>()})
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
            yaw: turn_code(i),
            ..Default::default()
        }
    }
    #[test]
    fn body_width_trades_turn_speed_for_gust_score() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = Whirligig::new(&[small, broad], 8);
        for p in &mut game.tinkers {
            for duct in &mut p.board {
                if duct.target != 0 {
                    duct.ports = duct.target;
                }
            }
            for _ in 0..3 {
                p.board[0].ports = rotated(p.board[0].ports);
            }
        }
        game.step(&[(0, press(0)), (1, press(0))], 0.01, &mut Vec::new());
        assert_eq!(game.tinkers[0].circuits, 1);
        assert_eq!(game.tinkers[1].circuits, 1);
        assert!(game.tinkers[0].cooldown < game.tinkers[1].cooldown);
        assert!(game.tinkers[0].score < game.tinkers[1].score);
        assert_eq!(
            game.snapshot()["players"][1]["gust_points"].as_u64(),
            Some(108)
        );
    }
    #[test]
    fn generated_puzzles_are_solvable_and_not_already_complete() {
        for seed in 0..200 {
            let mut board = puzzle(seed, 0);
            assert!(!connected(&board).0);
            for duct in &mut board {
                if duct.target != 0 {
                    duct.ports = duct.target;
                }
            }
            assert!(connected(&board).0);
        }
    }
    #[test]
    fn turns_are_validated_and_cooldown_is_server_owned() {
        let mut game = Whirligig::new(&people(2), 3);
        let old = game.tinkers[0].board[2].ports;
        game.step(
            &[(0, press(2)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.tinkers[0].board[2].ports, rotated(old));
        game.step(
            &[(0, press(2)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.tinkers[0].board[2].ports, rotated(old));
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
                (1, Input::default()),
            ],
            0.2,
            &mut Vec::new(),
        );
        assert_eq!(game.tinkers[0].board[2].ports, rotated(old));
    }
    #[test]
    fn completed_circuit_scores_and_starts_fresh_board() {
        let mut game = Whirligig::new(&people(2), 8);
        for i in 0..TILES {
            if game.tinkers[0].board[i].target != 0 {
                game.tinkers[0].board[i].ports = game.tinkers[0].board[i].target;
            }
        }
        for _ in 0..3 {
            game.tinkers[0].board[0].ports = rotated(game.tinkers[0].board[0].ports);
        }
        game.step(
            &[(0, press(0)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.tinkers[0].circuits, 1);
        assert!(game.tinkers[0].score >= 100);
        assert!(!connected(&game.tinkers[0].board).0);
    }
    #[test]
    fn missing_player_and_ties_finish() {
        let mut game = Whirligig::new(&people(3), 9);
        while game.placements().is_none() {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.tinkers[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }
    #[test]
    fn bots_finish_multiple_circuits() {
        for seed in 0..12 {
            let seats = (0..4).map(|s| Contestant::bot(s, 0.7)).collect::<Vec<_>>();
            let mut game = Whirligig::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            assert!(game.tinkers.iter().all(|p| p.circuits >= 2));
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
            let mut game = Whirligig::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.tinkers {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.tinkers.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Whirligig mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 3000);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 15);
    }
    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1., 0.7), (0.7, 0.4), (0.7, 0.7)] {
            let mut wins = 0;
            let mut scores = 0i64;
            for seed in 0..1000 {
                let seats = (0..4)
                    .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&seats, seed + 32000, 1.);
                let mut game = Whirligig::new(&formed, seed + 32000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
                scores += game.tinkers[0].score as i64 - game.tinkers[1].score as i64;
            }
            println!(
                "Whirligig {favorite} vs {field}: {wins}/1000 wins, {:.1} score edge",
                scores as f32 / 1000.
            );
        }
    }
}
