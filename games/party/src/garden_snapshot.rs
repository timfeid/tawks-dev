//! Photograph a featured butterfly in a living garden. Framing and wing timing
//! are checked by the server from the same moving scene every player sees.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "gardensnapshot";
pub const MOTHS: usize = 5;
pub const ROUNDS: u32 = 7;
pub const ROUND_SECONDS: f32 = 7.2;
pub const LIMIT: f32 = ROUNDS as f32 * ROUND_SECONDS;
pub const FRAME_RADIUS: f32 = 0.23;
const X_CODE: f32 = 1240.;
const Y_CODE: f32 = 1340.;

pub struct Photographer {
    pub seat: u8,
    pub score: u32,
    pub photos: u32,
    pub perfect: u32,
    pub last: u32,
    pub gone: bool,
    seen: f32,
    shot_round: u32,
    bot_ready: f32,
    skill: f32,
    bot: bool,
}

pub struct GardenSnapshot {
    pub time: f32,
    pub round: u32,
    pub target: [u8; ROUNDS as usize],
    pub photographers: Vec<Photographer>,
    seed: u64,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn shot_input(x: f32, y: f32) -> Input {
    Input {
        fire: true,
        yaw: X_CODE + x * 100.,
        pitch: Y_CODE + y * 100.,
        ..Default::default()
    }
}

fn decode(input: &Input) -> Option<(f32, f32)> {
    if !input.fire || !input.yaw.is_finite() || !input.pitch.is_finite() {
        return None;
    }
    let x = (input.yaw - X_CODE) / 100.;
    let y = (input.pitch - Y_CODE) / 100.;
    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return None;
    }
    Some((x, y))
}

pub fn position(id: usize, time: f32, round: u32) -> (f32, f32) {
    let phase = time * (0.77 + id as f32 * 0.06) + round as f32 * 0.37;
    let x = 0.5 + 0.34 * (phase + id as f32 * 1.71).sin();
    let y = 0.5 + 0.30 * (phase * 1.13 + id as f32 * 2.39).sin();
    (x, y)
}

pub fn wings_open(id: usize, time: f32) -> bool {
    (time * 4.8 + id as f32 * 1.27).sin() > 0.2
}

impl GardenSnapshot {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut target = [0; ROUNDS as usize];
        for round in 0..ROUNDS as usize {
            let mut pick = rng.random_range(0..MOTHS) as u8;
            if round > 0 && pick == target[round - 1] {
                pick = (pick + rng.random_range(1..MOTHS) as u8) % MOTHS as u8;
            }
            target[round] = pick;
        }
        let photographers = contestants
            .iter()
            .map(|c| Photographer {
                seat: c.seat,
                score: 0,
                photos: 0,
                perfect: 0,
                last: 0,
                gone: false,
                seen: 0.,
                shot_round: u32::MAX,
                bot_ready: rng.random_range(1.2..2.8),
                skill: c.skill.clamp(0., 1.),
                bot: c.is_bot,
            })
            .collect();
        Self {
            time: 0.,
            round: 0,
            target,
            photographers,
            seed,
            rng,
            places: None,
        }
    }

    fn finish(&mut self) {
        let mut ranked = self
            .photographers
            .iter()
            .map(|p| (p.seat, p.score))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = vec![];
        let mut prior = None;
        for (seat, score) in ranked {
            if prior == Some(score) {
                places.last_mut().unwrap().push(seat)
            } else {
                places.push(vec![seat]);
                prior = Some(score)
            }
        }
        self.places = Some(places);
    }
}

impl Bout for GardenSnapshot {
    fn key(&self) -> &'static str {
        KEY
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        let next = ((self.time / ROUND_SECONDS).floor() as u32).min(ROUNDS - 1);
        if next != self.round {
            self.round = next;
            for p in &mut self.photographers {
                p.bot_ready = self.round as f32 * ROUND_SECONDS + self.rng.random_range(1.2..2.8);
            }
            events.push(json!({"kind":"new_subject","round":self.round,"target":self.target[self.round as usize]}));
        }
        let target = self.target[self.round as usize] as usize;
        for p in &mut self.photographers {
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone || p.shot_round == self.round {
                    continue;
                }
                let Some((x, y)) = decode(input) else {
                    continue;
                };
                p.shot_round = self.round;
                p.photos += 1;
                let nearest = (0..MOTHS)
                    .map(|id| {
                        let (mx, my) = position(id, self.time, self.round);
                        (id, ((mx - x).powi(2) + (my - y).powi(2)).sqrt())
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap();
                let points = if nearest.0 == target && nearest.1 <= FRAME_RADIUS {
                    ((1. - nearest.1 / FRAME_RADIUS) * 65.).round() as u32
                        + if wings_open(target, self.time) { 35 } else { 0 }
                } else {
                    0
                };
                p.last = points;
                p.score += points;
                if points >= 85 {
                    p.perfect += 1;
                }
                events.push(json!({"kind":"photo","seat":p.seat,"round":self.round,"points":points,"x":x,"y":y}));
            } else if self.time - p.seen > 1. {
                p.gone = true;
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.photographers.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let p = &self.photographers[index];
        if !p.bot || p.gone || p.shot_round == self.round || self.time < p.bot_ready {
            return Input::default();
        }
        let target = self.target[self.round as usize] as usize;
        if !wings_open(target, self.time)
            && self.time < (self.round + 1) as f32 * ROUND_SECONDS - 0.9
        {
            return Input::default();
        }
        let skill = crate::bout::round_form(p.skill, self.seed, seat, self.round);
        let id = if self.rng.random::<f32>() < (1. - skill) * 0.06 {
            self.rng.random_range(0..MOTHS)
        } else {
            target
        };
        let (mx, my) = position(id, self.time, self.round);
        let spread = 0.025 + (1. - skill) * 0.18;
        shot_input(
            (mx + self.rng.random_range(-spread..spread)).clamp(0., 1.),
            (my + self.rng.random_range(-spread..spread)).clamp(0., 1.),
        )
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"round":self.round,"rounds":ROUNDS,
            "target":self.target[self.round as usize],
            "butterflies":(0..MOTHS).map(|id|{let (x,y)=position(id,self.time,self.round);json!({"id":id,"x":x,"y":y,"open":wings_open(id,self.time)})}).collect::<Vec<_>>(),
            "players":self.photographers.iter().map(|p|json!({"seat":p.seat,"score":p.score,"photos":p.photos,
                "perfect":p.perfect,"last":p.last,"shot":p.shot_round==self.round,"gone":p.gone})).collect::<Vec<_>>()})
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
    #[test]
    fn targets_change_and_moths_stay_on_screen() {
        for seed in 0..100 {
            let game = GardenSnapshot::new(&people(2), seed);
            assert!(game.target.windows(2).all(|w| w[0] != w[1]));
            for id in 0..MOTHS {
                for time in [0., 1., 2., 6., 20., 45.] {
                    let (x, y) = position(id, time, 0);
                    assert!((0.1..0.9).contains(&x) && (0.1..0.9).contains(&y));
                }
            }
        }
    }
    #[test]
    fn accurate_photo_scores_once_and_bad_inputs_do_not_consume_shutter() {
        let mut game = GardenSnapshot::new(&people(2), 3);
        game.step(
            &[(0, shot_input(-1., 0.5)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.photographers[0].photos, 0);
        let target = game.target[0] as usize;
        let (x, y) = position(target, game.time + 0.01, 0);
        game.step(
            &[(0, shot_input(x, y)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.photographers[0].photos, 1);
        assert!(game.photographers[0].score >= 60);
        let score = game.photographers[0].score;
        game.step(
            &[(0, shot_input(x, y)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.photographers[0].score, score);
    }
    #[test]
    fn new_round_reopens_shutter() {
        let mut game = GardenSnapshot::new(&people(2), 5);
        game.step(
            &[(0, shot_input(0.5, 0.5)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        while game.round == 0 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(!game.snapshot()["players"][0]["shot"].as_bool().unwrap());
        game.step(
            &[(0, shot_input(0.5, 0.5)), (1, Input::default())],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.photographers[0].photos, 2);
    }
    #[test]
    fn leavers_and_ties_finish() {
        let mut game = GardenSnapshot::new(&people(3), 7);
        while game.placements().is_none() {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.photographers[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }
    #[test]
    fn bots_take_photos_and_finish() {
        for seed in 0..20 {
            let people = (0..4).map(|s| Contestant::bot(s, 0.7)).collect::<Vec<_>>();
            let mut game = GardenSnapshot::new(&people, seed);
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            assert!(game.photographers.iter().all(|p| p.photos == ROUNDS));
        }
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
                let formed = crate::bout::on_form_scaled(&people, seed + 50000, 1.);
                let mut game = GardenSnapshot::new(&formed, seed + 50000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
            }
            println!("Garden Snapshot {favorite} vs {field}: {wins}/1000 favorite wins");
        }
    }
}
