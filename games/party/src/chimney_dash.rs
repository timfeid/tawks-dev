//! Climb on a draft and sweep the soot patches that pass the work line.
//! The same patch schedule is shared by all seats; each seat cleans its own copy.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input};

pub const KEY: &str = "chimneydash";
pub const LANES: usize = 4;
pub const LIMIT: f32 = 58.;
pub const HIT_WINDOW: f32 = 0.25;
pub const SWIPE_COOLDOWN: f32 = 0.28;
const CODE: f32 = 1130.;

#[derive(Clone, Copy)]
pub struct Patch {
    pub id: usize,
    pub lane: u8,
    pub due: f32,
    pub worth: u32,
    pub claimed: u8,
}

pub struct Sweep {
    pub seat: u8,
    pub body_width: f32,
    pub lane: u8,
    pub score: u32,
    pub hits: u32,
    pub misses: u32,
    pub combo: u8,
    pub cooldown: f32,
    pub gone: bool,
    seen: f32,
    bot_next: usize,
    bot_ready: f32,
    skill: f32,
    bot: bool,
}

pub struct ChimneyDash {
    pub time: f32,
    pub patches: Vec<Patch>,
    pub sweeps: Vec<Sweep>,
    expired: usize,
    rng: StdRng,
    places: Option<Vec<Vec<u8>>>,
}

pub fn swipe_code(lane: usize) -> f32 {
    CODE + lane as f32
}

fn decode(input: &Input) -> Option<u8> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    (0..LANES)
        .find(|lane| (input.yaw - swipe_code(*lane)).abs() < 0.01)
        .map(|lane| lane as u8)
}

fn next_ready(patch: &Patch, skill: f32, rng: &mut StdRng) -> f32 {
    // A sweep must read the passing patch; sharper bots vary less in timing.
    let spread = 0.28 + (1. - skill) * 0.22;
    patch.due + rng.random_range(-spread..spread)
}

fn hit_window(width: f32) -> f32 {
    HIT_WINDOW + (BodyProfile::new(width).width - 1.0) * 0.005
}

fn swipe_cooldown(width: f32) -> f32 {
    SWIPE_COOLDOWN / BodyProfile::new(width).speed
}

impl ChimneyDash {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut patches = Vec::new();
        let mut due = 2.3;
        let mut last_lane = LANES as u8;
        while due < LIMIT - 0.6 {
            let mut lane = rng.random_range(0..LANES) as u8;
            if lane == last_lane {
                lane = (lane + rng.random_range(1..LANES) as u8) % LANES as u8;
            }
            patches.push(Patch {
                id: patches.len(),
                lane,
                due,
                worth: if rng.random_bool(0.22) { 20 } else { 10 },
                claimed: 0,
            });
            last_lane = lane;
            due += rng.random_range(1.65..2.05);
        }
        let sweeps = contestants
            .iter()
            .map(|c| {
                let skill = c.skill.clamp(0., 1.);
                Sweep {
                    seat: c.seat,
                    body_width: BodyProfile::new(c.body_width).width,
                    lane: 0,
                    score: 0,
                    hits: 0,
                    misses: 0,
                    combo: 0,
                    cooldown: 0.,
                    gone: false,
                    seen: 0.,
                    bot_next: 0,
                    bot_ready: next_ready(&patches[0], skill, &mut rng),
                    skill,
                    bot: c.is_bot,
                }
            })
            .collect();
        Self {
            time: 0.,
            patches,
            sweeps,
            expired: 0,
            rng,
            places: None,
        }
    }

    fn finish(&mut self) {
        let mut ranked = self
            .sweeps
            .iter()
            .map(|p| (p.seat, p.score))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut places: Vec<Vec<u8>> = vec![];
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

impl Bout for ChimneyDash {
    fn key(&self) -> &'static str {
        KEY
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.places.is_some() {
            return;
        }
        self.time = (self.time + dt.clamp(0., 0.1)).min(LIMIT);
        for p in &mut self.sweeps {
            p.cooldown = (p.cooldown - dt).max(0.);
            if let Some((_, input)) = inputs.iter().find(|(seat, _)| *seat == p.seat) {
                p.seen = self.time;
                if p.gone || p.cooldown > 0. {
                    continue;
                }
                let Some(lane) = decode(input) else {
                    continue;
                };
                p.lane = lane;
                p.cooldown = swipe_cooldown(p.body_width);
                let nearest = self
                    .patches
                    .iter()
                    .enumerate()
                    .filter(|(_, patch)| patch.lane == lane && patch.claimed & (1 << p.seat) == 0)
                    .filter(|(_, patch)| (patch.due - self.time).abs() <= hit_window(p.body_width))
                    .min_by(|a, b| {
                        (a.1.due - self.time)
                            .abs()
                            .total_cmp(&(b.1.due - self.time).abs())
                    })
                    .map(|(index, _)| index);
                if let Some(index) = nearest {
                    let patch = &mut self.patches[index];
                    patch.claimed |= 1 << p.seat;
                    p.combo = (p.combo + 1).min(5);
                    p.score += patch.worth + p.combo as u32 * 2;
                    p.hits += 1;
                    events.push(
                        json!({"kind":"sweep","seat":p.seat,"patch":patch.id,"score":p.score}),
                    );
                } else {
                    events.push(json!({"kind":"whiff","seat":p.seat,"lane":lane}));
                }
            } else if self.time - p.seen > 1. {
                p.gone = true;
            }
        }
        while self.expired < self.patches.len()
            && self.patches[self.expired].due + hit_window(1.4) < self.time
        {
            let claimed = self.patches[self.expired].claimed;
            for p in &mut self.sweeps {
                if claimed & (1 << p.seat) == 0 {
                    p.misses += 1;
                    p.combo = 0;
                }
            }
            self.expired += 1;
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.sweeps.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let p = &mut self.sweeps[index];
        if !p.bot
            || p.gone
            || p.cooldown > 0.
            || p.bot_next >= self.patches.len()
            || self.time < p.bot_ready
        {
            return Input::default();
        }
        let target = self.patches[p.bot_next];
        p.bot_next += 1;
        if let Some(next) = self.patches.get(p.bot_next) {
            p.bot_ready = next_ready(next, p.skill, &mut self.rng);
        }
        let lane = if self.rng.random::<f32>() < (1. - p.skill) * 0.07 {
            (target.lane + self.rng.random_range(1..LANES) as u8) % LANES as u8
        } else {
            target.lane
        };
        Input {
            fire: true,
            yaw: swipe_code(lane as usize),
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,
            "patches":self.patches.iter().filter(|p|p.due>=self.time-0.5&&p.due<=self.time+2.7)
                .map(|p|json!({"id":p.id,"lane":p.lane,"due":p.due,"worth":p.worth,"claimed":p.claimed})).collect::<Vec<_>>(),
            "players":self.sweeps.iter().map(|p|json!({"seat":p.seat,"lane":p.lane,"score":p.score,"hits":p.hits,
                "misses":p.misses,"combo":p.combo,"cooldown":p.cooldown,"gone":p.gone,
                "body_width":p.body_width,"hit_window":hit_window(p.body_width),
                "swipe_cooldown":swipe_cooldown(p.body_width)})).collect::<Vec<_>>()})
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
    fn swipe(lane: usize) -> Input {
        Input {
            fire: true,
            yaw: swipe_code(lane),
            ..Default::default()
        }
    }
    #[test]
    fn body_width_trades_timing_window_for_retry_speed() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = ChimneyDash::new(&[small, broad], 3);
        assert!(hit_window(game.sweeps[0].body_width) < hit_window(game.sweeps[1].body_width));
        assert!(
            swipe_cooldown(game.sweeps[0].body_width) < swipe_cooldown(game.sweeps[1].body_width)
        );
        let patch = game.patches[0];
        game.time = patch.due + 0.250;
        game.step(
            &[
                (0, swipe(patch.lane as usize)),
                (1, swipe(patch.lane as usize)),
            ],
            0.001,
            &mut Vec::new(),
        );
        assert_eq!((game.sweeps[0].hits, game.sweeps[1].hits), (0, 1));
        assert_eq!(
            game.snapshot()["players"][0]["body_width"]
                .as_f64()
                .unwrap() as f32,
            game.sweeps[0].body_width
        );
    }
    #[test]
    fn patch_schedule_is_bounded_and_changes_lanes() {
        for seed in 0..100 {
            let game = ChimneyDash::new(&people(2), seed);
            assert!(game.patches.len() >= 25 && game.patches.len() <= 35);
            assert!(
                game.patches
                    .windows(2)
                    .all(|w| w[0].due < w[1].due && w[0].lane != w[1].lane)
            );
        }
    }
    #[test]
    fn valid_sweep_scores_once_and_cooldown_blocks_spam() {
        let mut game = ChimneyDash::new(&people(2), 3);
        let patch = game.patches[0];
        while game.time < patch.due - 0.01 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        game.step(
            &[(0, swipe(patch.lane as usize)), (1, Input::default())],
            1. / 60.,
            &mut Vec::new(),
        );
        assert_eq!(game.sweeps[0].hits, 1);
        let score = game.sweeps[0].score;
        game.step(
            &[(0, swipe(patch.lane as usize)), (1, Input::default())],
            1. / 60.,
            &mut Vec::new(),
        );
        assert_eq!(game.sweeps[0].score, score);
        assert!(game.patches[0].claimed & 1 != 0);
    }
    #[test]
    fn invalid_inputs_and_expired_patches_do_not_score() {
        let mut game = ChimneyDash::new(&people(2), 5);
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
                (1, swipe(10)),
            ],
            0.01,
            &mut Vec::new(),
        );
        assert_eq!(game.sweeps[0].score, 0);
        assert_eq!(game.sweeps[1].score, 0);
        while game.time < game.patches[0].due + HIT_WINDOW + 0.05 {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert_eq!(game.sweeps[0].misses, 1);
        assert_eq!(game.sweeps[1].misses, 1);
    }
    #[test]
    fn leavers_and_ties_finish() {
        let mut game = ChimneyDash::new(&people(3), 7);
        while game.placements().is_none() {
            game.step(
                &[(0, Input::default()), (1, Input::default())],
                1. / 60.,
                &mut Vec::new(),
            );
        }
        assert!(game.sweeps[2].gone);
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }
    #[test]
    fn bots_score_before_time_runs_out() {
        for seed in 0..20 {
            let seats = (0..4).map(|s| Contestant::bot(s, 0.7)).collect::<Vec<_>>();
            let mut game = ChimneyDash::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut Vec::new());
            }
            assert!(game.sweeps.iter().all(|p| p.hits >= 10));
        }
    }
    #[test]
    #[ignore]
    fn balance_report() {
        for (favorite, field) in [(1., 0.7), (0.7, 0.4), (0.7, 0.7)] {
            let mut wins = 0;
            for seed in 0..1000 {
                let seats = (0..4)
                    .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                    .collect::<Vec<_>>();
                let formed = crate::bout::on_form_scaled(&seats, seed + 41000, 1.);
                let mut game = ChimneyDash::new(&formed, seed + 41000);
                while game.placements().is_none() {
                    let inputs = (0..4)
                        .map(|s| (s, game.bot_input(s, 1. / 60.)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1. / 60., &mut Vec::new());
                }
                wins += game.placements().unwrap()[0].contains(&0) as u32;
            }
            println!("Chimney Dash {favorite} vs {field}: {wins}/1000 favorite wins");
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
            let mut game = ChimneyDash::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for p in &game.sweeps {
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                scores[index] += p.score;
            }
            for &seat in &game.placements().unwrap()[0] {
                let p = game.sweeps.iter().find(|p| p.seat == seat).unwrap();
                let index = widths.iter().position(|w| *w == p.body_width).unwrap();
                wins[index] += 1;
            }
        }
        println!("Chimney Dash mixed builds: widths={widths:?} scores={scores:?} wins={wins:?}");
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 1800);
        assert!(*wins.iter().max().unwrap() - *wins.iter().min().unwrap() <= 20);
    }
}
