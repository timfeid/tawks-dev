//! High Stakes Diner: five cuts sizzle on one grill, each cooking at its own
//! hidden pace. Before the buzzer every player picks the one that will come
//! off perfectly done and stakes chips on it. Sear colour, sizzle and smoke
//! all hint at the pace, and each carries its own fixed lie, so reading all
//! three beats trusting one. The odds on every cut fall as the grill gives
//! itself away, which is what makes an early lock worth the risk. Most chips
//! after the last round wins.
//!
//! A lock arrives as a press of fire with the pick and stake packed into
//! `yaw` (see [`lock_code`]), so the bout needs nothing new from the protocol,
//! and ordinary aim never lands in that range.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "diner";
pub const ROUNDS: usize = 5;
pub const CUTS: usize = 5;
pub const COOK_SECONDS: f32 = 10.0;
pub const REVEAL_SECONDS: f32 = 3.5;
pub const START_CHIPS: u32 = 100;
pub const MIN_BET: u32 = 10;
pub const BET_STEP: u32 = 10;
/// The doneness a perfect cut reaches at the buzzer.
pub const PERFECT: f32 = 1.0;
/// Opening odds lose this share of their profit by the buzzer.
pub const ODDS_DECAY: f32 = 0.55;
pub const DISHES: [&str; 3] = ["steak", "burger", "skewer"];

const CODE_BASE: f32 = 1000.0;
const CODE_SLOTS: u32 = 8;
const SEAR_BIAS: f32 = 0.14;
const SIZZLE_BIAS: f32 = 0.18;
const SMOKE_BIAS: f32 = 0.18;
const WOBBLE: f32 = 0.08;
const HOUSE_NOISE: f32 = 0.11;
/// The counter the diners stand behind, across the grill from the camera.
const ROW_Z: f32 = -3.6;
const ROW_GAP: f32 = 1.5;

/// The `yaw` a client sends with fire to lock `cut` for `bet` chips.
pub fn lock_code(cut: usize, bet: u32) -> f32 {
    CODE_BASE + (cut as u32 + CODE_SLOTS * (bet / BET_STEP)) as f32
}

fn decode(input: &Input) -> Option<(usize, u32)> {
    if !input.fire || !input.yaw.is_finite() || input.yaw < CODE_BASE {
        return None;
    }
    let raw = input.yaw - CODE_BASE;
    let whole = raw.round();
    if (raw - whole).abs() > 0.25 || whole > 1.0e7 {
        return None;
    }
    let whole = whole as u32;
    let cut = (whole % CODE_SLOTS) as usize;
    let units = whole / CODE_SLOTS;
    (cut < CUTS && units > 0).then_some((cut, units * BET_STEP))
}

/// The pace a cut must be on to be perfect at the buzzer, as a sear reading
/// at `progress`, which is what the client's reference swatch shows.
pub fn perfect_sear(progress: f32) -> f32 {
    PERFECT * progress
}

/// Odds as a total return per chip staked, falling with the round's progress.
pub fn odds_at(opening: f32, progress: f32) -> f32 {
    let now = 1.0 + (opening - 1.0) * (1.0 - ODDS_DECAY * progress.clamp(0.0, 1.0));
    (now * 10.0).round() / 10.0
}

fn normal(rng: &mut StdRng) -> f32 {
    let u: f32 = rng.random_range(f32::EPSILON..1.0);
    let v: f32 = rng.random_range(0.0..std::f32::consts::TAU);
    (-2.0 * u.ln()).sqrt() * v.cos()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temper {
    Bold,
    Steady,
    Cautious,
}

impl Temper {
    fn name(self) -> &'static str {
        match self {
            Temper::Bold => "bold",
            Temper::Steady => "steady",
            Temper::Cautious => "cautious",
        }
    }
    /// When in the round this bot likes to commit, as progress.
    fn lock_window(self) -> (f32, f32) {
        match self {
            Temper::Bold => (0.12, 0.4),
            Temper::Steady => (0.35, 0.65),
            Temper::Cautious => (0.55, 0.88),
        }
    }
    /// Share of the stack staked at no confidence and at full confidence.
    fn stake(self) -> (f32, f32) {
        match self {
            Temper::Bold => (0.2, 0.75),
            Temper::Steady => (0.1, 0.4),
            Temper::Cautious => (0.06, 0.25),
        }
    }
}

#[derive(Clone, Debug)]
struct Cut {
    done: f32,
    sear_bias: f32,
    sizzle_bias: f32,
    smoke_bias: f32,
    phase: [f32; 3],
    opening: f32,
}

impl Cut {
    fn wobble(&self, cue: usize, clock: f32, progress: f32) -> f32 {
        WOBBLE * (1.0 - 0.6 * progress) * (clock * (1.3 + cue as f32 * 0.7) + self.phase[cue]).sin()
    }
    fn sear(&self, clock: f32, progress: f32) -> f32 {
        (self.done * progress + self.sear_bias + self.wobble(0, clock, progress)).max(0.0)
    }
    fn sizzle(&self, clock: f32, progress: f32) -> f32 {
        (self.done + self.sizzle_bias + self.wobble(1, clock, progress)).max(0.0)
    }
    fn smoke(&self, clock: f32, progress: f32) -> f32 {
        (self.done + self.smoke_bias + self.wobble(2, clock, progress)).max(0.0)
    }
}

#[derive(Clone, Copy, Debug)]
struct Lock {
    cut: usize,
    bet: u32,
    odds: f32,
}

#[derive(Clone, Copy)]
struct Plan {
    round: usize,
    at: f32,
}

struct Player {
    seat: u8,
    is_bot: bool,
    skill: f32,
    temper: Temper,
    chips: u32,
    lock: Option<Lock>,
    /// Chips won or lost on the last reveal.
    swing: i64,
    comped: bool,
    plan: Option<Plan>,
    wins: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Cooking,
    Reveal,
}

pub struct HighStakesDiner {
    cuts: Vec<Cut>,
    players: Vec<Player>,
    round: usize,
    stage: Stage,
    clock: f32,
    perfect: usize,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

impl HighStakesDiner {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let players = contestants
            .iter()
            .map(|one| Player {
                seat: one.seat,
                is_bot: one.is_bot,
                skill: one.skill.clamp(0.0, 1.0),
                temper: match rng.random_range(0..3) {
                    0 => Temper::Bold,
                    1 => Temper::Steady,
                    _ => Temper::Cautious,
                },
                chips: START_CHIPS,
                lock: None,
                swing: 0,
                comped: false,
                plan: None,
                wins: 0,
            })
            .collect();
        let mut diner = HighStakesDiner {
            cuts: Vec::new(),
            players,
            round: 0,
            stage: Stage::Cooking,
            clock: 0.0,
            perfect: 0,
            rng,
            placed: None,
        };
        diner.fire_up();
        diner
    }

    fn progress(&self) -> f32 {
        if self.stage == Stage::Reveal {
            1.0
        } else {
            (self.clock / COOK_SECONDS).clamp(0.0, 1.0)
        }
    }

    /// Lay out a fresh grill: one cut lands on perfect, the rest miss by a
    /// margin that narrows as the rounds go on.
    fn fire_up(&mut self) {
        let rng = &mut self.rng;
        let narrowing = self.round as f32 / (ROUNDS - 1).max(1) as f32;
        let gap = 0.18 - 0.08 * narrowing;
        let perfect = rng.random_range(0..CUTS);
        let mut misses: Vec<f32> = Vec::new();
        while misses.len() < CUTS - 1 {
            let miss = rng.random_range(gap..0.6);
            let signed = if rng.random_bool(0.5) { -miss } else { miss };
            if misses.iter().all(|m| (m - signed).abs() > 0.06) {
                misses.push(signed);
            }
        }
        let mut misses = misses.into_iter();
        let mut cuts: Vec<Cut> = (0..CUTS)
            .map(|index| {
                let done = if index == perfect {
                    PERFECT + rng.random_range(-0.03..0.03)
                } else {
                    PERFECT + misses.next().unwrap_or(0.4)
                };
                Cut {
                    done,
                    sear_bias: normal(rng) * SEAR_BIAS,
                    sizzle_bias: normal(rng) * SIZZLE_BIAS,
                    smoke_bias: normal(rng) * SMOKE_BIAS,
                    phase: [
                        rng.random_range(0.0..std::f32::consts::TAU),
                        rng.random_range(0.0..std::f32::consts::TAU),
                        rng.random_range(0.0..std::f32::consts::TAU),
                    ],
                    opening: 0.0,
                }
            })
            .collect();
        // The house reads the grill too, just worse than a sharp eye.
        let weights: Vec<f32> = cuts
            .iter()
            .map(|cut| {
                let guess = cut.done + normal(rng) * HOUSE_NOISE;
                (-((guess - PERFECT) / 0.14).powi(2)).exp() + 0.04
            })
            .collect();
        let total: f32 = weights.iter().sum();
        for (cut, weight) in cuts.iter_mut().zip(weights) {
            let chance = weight / total;
            cut.opening = ((0.85 / chance).clamp(1.5, 7.0) * 2.0).round() / 2.0;
        }
        self.cuts = cuts;
        self.perfect = perfect;
        self.stage = Stage::Cooking;
        self.clock = 0.0;
        for player in self.players.iter_mut() {
            player.lock = None;
            player.comped = false;
            if player.chips < MIN_BET {
                player.chips = MIN_BET;
                player.comped = true;
            }
        }
    }

    fn lock(&mut self, seat: u8, cut: usize, bet: u32, events: &mut Vec<Value>) -> bool {
        if self.stage != Stage::Cooking || cut >= CUTS {
            return false;
        }
        let odds = odds_at(self.cuts[cut].opening, self.progress());
        let Some(player) = self.players.iter_mut().find(|p| p.seat == seat) else {
            return false;
        };
        if player.lock.is_some() {
            return false;
        }
        let bet = if bet > player.chips && bet - player.chips < BET_STEP {
            player.chips
        } else {
            bet
        };
        if bet < MIN_BET.min(player.chips) || bet > player.chips || bet == 0 {
            return false;
        }
        player.lock = Some(Lock { cut, bet, odds });
        events.push(json!({ "kind": "lock", "seat": seat, "bet": bet }));
        true
    }

    fn buzzer(&mut self, events: &mut Vec<Value>) {
        self.stage = Stage::Reveal;
        self.clock = 0.0;
        let perfect = self.perfect;
        let mut payouts = Vec::new();
        for player in self.players.iter_mut() {
            player.swing = 0;
            let Some(lock) = player.lock else { continue };
            if lock.cut == perfect {
                let profit = (lock.bet as f32 * (lock.odds - 1.0)).round() as u32;
                player.chips += profit;
                player.swing = profit as i64;
                player.wins += 1;
            } else {
                player.chips -= lock.bet;
                player.swing = -(lock.bet as i64);
            }
            payouts.push(json!({ "seat": player.seat, "swing": player.swing, "pick": lock.cut }));
        }
        events.push(json!({ "kind": "buzzer", "perfect": perfect, "payouts": payouts }));
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, u32, u32)> = self
            .players
            .iter()
            .map(|p| (p.seat, p.chips, p.wins))
            .collect();
        order.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, u32)> = None;
        for (seat, chips, wins) in order {
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if previous == (chips, wins) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((chips, wins));
        }
        self.placed = Some(groups);
    }

    fn spot(&self, index: usize) -> (f32, f32) {
        let count = self.players.len().max(1) as f32;
        let x = (index as f32 - (count - 1.0) / 2.0) * ROW_GAP;
        (x, ROW_Z)
    }

    /// How a bot sees the grill right now: each cue read with an error that
    /// shrinks with skill, then weighed. The sear says little early on, when
    /// a fixed tint is a large share of a pale colour.
    fn read(&mut self, skill: f32) -> Vec<f32> {
        let progress = self.progress().max(0.05);
        let clock = self.clock;
        let eye = 0.03 + (1.0 - skill) * 0.18;
        let cuts = self.cuts.clone();
        cuts.iter()
            .map(|cut| {
                let sear = (cut.sear(clock, progress) + normal(&mut self.rng) * eye) / progress;
                let sizzle = cut.sizzle(clock, progress) + normal(&mut self.rng) * eye;
                let smoke = cut.smoke(clock, progress) + normal(&mut self.rng) * eye;
                let sear_weight = 2.5 * progress * progress;
                (sear * sear_weight + sizzle + smoke) / (sear_weight + 2.0)
            })
            .collect()
    }

    fn bot_decide(&mut self, index: usize) -> Option<(usize, u32)> {
        let skill = self.players[index].skill;
        let temper = self.players[index].temper;
        let chips = self.players[index].chips;
        let estimates = self.read(skill);
        let mut ranked: Vec<(usize, f32)> = estimates
            .iter()
            .enumerate()
            .map(|(i, e)| (i, (e - PERFECT).abs()))
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let margin = ranked[1].1 - ranked[0].1;
        let confidence = (margin / 0.15).clamp(0.0, 1.0);
        let slip = (1.0 - skill) as f64 * 0.18;
        let cut = if self.rng.random_bool(slip) {
            ranked[1].0
        } else {
            ranked[0].0
        };
        let (low, high) = temper.stake();
        let mut share = low + (high - low) * confidence;
        let leader = self.players.iter().map(|p| p.chips).max().unwrap_or(chips);
        let last = self.round + 1 == ROUNDS;
        if last && chips < leader {
            share = share.max(match temper {
                Temper::Bold => 1.0,
                Temper::Steady => 0.6,
                Temper::Cautious => 0.35,
            });
        }
        let wanted = (chips as f32 * share / BET_STEP as f32).round() as u32 * BET_STEP;
        let bet = wanted.clamp(MIN_BET.min(chips), chips);
        Some((cut, bet.div_ceil(BET_STEP) * BET_STEP))
    }

    fn seat_state(&self, player: &Player) -> &'static str {
        match (self.stage, player.lock) {
            (Stage::Cooking, None) => "eyeing",
            (Stage::Cooking, Some(_)) => "locked",
            (Stage::Reveal, None) => "sat_out",
            (Stage::Reveal, Some(lock)) if lock.cut == self.perfect => "won",
            (Stage::Reveal, Some(_)) => "lost",
        }
    }

    #[cfg(test)]
    pub fn perfect(&self) -> usize {
        self.perfect
    }
    #[cfg(test)]
    pub fn stage(&self) -> Stage {
        self.stage
    }
    #[cfg(test)]
    pub fn round(&self) -> usize {
        self.round
    }
    #[cfg(test)]
    pub fn chips(&self, seat: u8) -> u32 {
        self.players
            .iter()
            .find(|p| p.seat == seat)
            .map_or(0, |p| p.chips)
    }
    #[cfg(test)]
    pub fn set_chips(&mut self, seat: u8, chips: u32) {
        if let Some(p) = self.players.iter_mut().find(|p| p.seat == seat) {
            p.chips = chips;
        }
    }
    #[cfg(test)]
    pub fn temper(&self, seat: u8) -> Temper {
        self.players
            .iter()
            .find(|p| p.seat == seat)
            .map_or(Temper::Steady, |p| p.temper)
    }
    #[cfg(test)]
    pub fn true_doneness(&self) -> Vec<f32> {
        self.cuts.iter().map(|c| c.done).collect()
    }
}

impl Bout for HighStakesDiner {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rounds": ROUNDS,
            "cuts": CUTS,
            "cook": COOK_SECONDS,
            "radius": 14.0,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        match self.stage {
            Stage::Reveal => {
                if self.clock >= REVEAL_SECONDS {
                    if self.round + 1 >= ROUNDS {
                        self.settle();
                    } else {
                        self.round += 1;
                        self.fire_up();
                        events.push(json!({ "kind": "round", "round": self.round }));
                    }
                }
            }
            Stage::Cooking => {
                for (seat, input) in inputs {
                    if let Some((cut, bet)) = decode(input) {
                        self.lock(*seat, cut, bet, events);
                    }
                }
                if self.clock >= COOK_SECONDS {
                    self.buzzer(events);
                }
            }
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() || self.stage != Stage::Cooking {
            return Input::default();
        }
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.players[index].lock.is_some() {
            return Input::default();
        }
        let round = self.round;
        let plan = match self.players[index].plan {
            Some(plan) if plan.round == round => plan,
            _ => {
                let player = &self.players[index];
                let (early, late) = player.temper.lock_window();
                let reaction = crate::bout::Skill(player.skill).reaction();
                let at = self.rng.random_range(early..late) * COOK_SECONDS + reaction;
                let plan = Plan {
                    round,
                    at: at.min(COOK_SECONDS - 0.3),
                };
                self.players[index].plan = Some(plan);
                plan
            }
        };
        if self.clock < plan.at {
            return Input::default();
        }
        let Some((cut, bet)) = self.bot_decide(index) else {
            return Input::default();
        };
        Input {
            fire: true,
            yaw: lock_code(cut, bet),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let revealing = self.stage == Stage::Reveal;
        let progress = self.progress();
        let clock = if revealing { COOK_SECONDS } else { self.clock };
        json!({
            "game": KEY,
            "time": clock.min(COOK_SECONDS),
            "limit": COOK_SECONDS,
            "round": self.round,
            "rounds": ROUNDS,
            "stage": if revealing { "reveal" } else { "cooking" },
            "reveal_time": if revealing { self.clock } else { 0.0 },
            "progress": progress,
            "dish": DISHES[self.round % DISHES.len()],
            "perfect_sear": perfect_sear(progress),
            "min_bet": MIN_BET,
            "bet_step": BET_STEP,
            "over": self.placed.is_some(),
            "perfect": revealing.then_some(self.perfect),
            "cuts": self.cuts.iter().map(|cut| json!({
                "sear": cut.sear(clock, progress),
                "sizzle": cut.sizzle(clock, progress),
                "smoke": cut.smoke(clock, progress),
                "opening": cut.opening,
                "odds": odds_at(cut.opening, progress),
                // Held back until the buzzer so it is never on the wire while
                // it can still be bet on.
                "done": revealing.then_some(cut.done),
            })).collect::<Vec<_>>(),
            "players": self.players.iter().enumerate().map(|(index, player)| {
                let (x, z) = self.spot(index);
                json!({
                    "seat": player.seat,
                    "x": x,
                    "z": z,
                    "yaw": 0.0,
                    "chips": player.chips,
                    "wins": player.wins,
                    "state": self.seat_state(player),
                    "bet": player.lock.map(|l| l.bet),
                    "odds": player.lock.map(|l| l.odds),
                    "pick": if revealing { player.lock.map(|l| l.cut) } else { None },
                    "swing": if revealing { Some(player.swing) } else { None },
                    "comped": player.comped,
                    "bot": player.is_bot,
                    "temper": player.is_bot.then(|| player.temper.name()),
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::TICK_DT;

    fn table(count: u8, skill: f32) -> Vec<Contestant> {
        (0..count)
            .map(|seat| Contestant::bot(seat, skill))
            .collect()
    }

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn lock_input(cut: usize, bet: u32) -> Input {
        Input {
            fire: true,
            yaw: lock_code(cut, bet),
            ..Default::default()
        }
    }

    fn idle(game: &mut HighStakesDiner, seats: u8, seconds: f32, events: &mut Vec<Value>) {
        for _ in 0..(seconds / TICK_DT).round() as usize {
            let inputs: Vec<(u8, Input)> = (0..seats).map(|s| (s, Input::default())).collect();
            game.step(&inputs, TICK_DT, events);
        }
    }

    fn until_reveal(game: &mut HighStakesDiner, seats: u8, events: &mut Vec<Value>) {
        while game.stage() == Stage::Cooking {
            idle(game, seats, TICK_DT, events);
        }
    }

    fn until_next_round(game: &mut HighStakesDiner, seats: u8, events: &mut Vec<Value>) {
        until_reveal(game, seats, events);
        while game.stage() == Stage::Reveal && game.placements().is_none() {
            idle(game, seats, TICK_DT, events);
        }
    }

    #[test]
    fn a_lock_code_carries_the_cut_and_stake_and_ordinary_aim_never_reads_as_one() {
        for cut in 0..CUTS {
            for bet in [10, 50, 1230, 98_000] {
                assert_eq!(decode(&lock_input(cut, bet)), Some((cut, bet)));
            }
        }
        for yaw in [0.0, 3.1, -3.1, 12.0, 999.0, f32::NAN, f32::INFINITY] {
            let aim = Input {
                fire: true,
                yaw,
                ..Default::default()
            };
            assert_eq!(decode(&aim), None, "aim {yaw} is not a lock");
        }
        let held = Input {
            fire: false,
            yaw: lock_code(1, 20),
            ..Default::default()
        };
        assert_eq!(decode(&held), None, "only a press locks");
        let off = Input {
            fire: true,
            yaw: CODE_BASE + 5.0 + 8.0,
            ..Default::default()
        };
        assert_eq!(decode(&off), None, "there is no sixth cut");
        let free = Input {
            fire: true,
            yaw: CODE_BASE + 2.0,
            ..Default::default()
        };
        assert_eq!(decode(&free), None, "a lock stakes something");
    }

    #[test]
    fn every_grill_has_exactly_one_perfect_cut_and_the_misses_are_clear_of_it() {
        for seed in 0..200 {
            let mut game = HighStakesDiner::new(&table(4, 0.5), seed);
            let mut events = Vec::new();
            for round in 0..ROUNDS {
                let done = game.true_doneness();
                let perfect = game.perfect();
                let gap = 0.18 - 0.08 * round as f32 / (ROUNDS - 1) as f32;
                assert!((done[perfect] - PERFECT).abs() <= 0.03);
                for (index, value) in done.iter().enumerate() {
                    if index != perfect {
                        assert!(
                            (value - PERFECT).abs() >= gap - 1e-4,
                            "seed {seed} round {round}"
                        );
                    }
                }
                for cut in &game.cuts {
                    assert!((1.5..=7.0).contains(&cut.opening));
                }
                until_next_round(&mut game, 4, &mut events);
            }
        }
    }

    #[test]
    fn the_perfect_pick_pays_its_locked_odds_and_a_miss_loses_the_stake() {
        let mut game = HighStakesDiner::new(&people(3), 11);
        let mut events = Vec::new();
        let perfect = game.perfect();
        let wrong = (perfect + 1) % CUTS;
        let opening = game.cuts[perfect].opening;
        game.step(
            &[(0, lock_input(perfect, 40)), (1, lock_input(wrong, 30))],
            TICK_DT,
            &mut events,
        );
        assert!(events.iter().any(|e| e["kind"] == "lock" && e["seat"] == 0));
        until_reveal(&mut game, 3, &mut events);
        let odds = odds_at(opening, TICK_DT / COOK_SECONDS);
        let profit = (40.0 * (odds - 1.0)).round() as u32;
        assert_eq!(game.chips(0), START_CHIPS + profit);
        assert_eq!(game.chips(1), START_CHIPS - 30);
        assert_eq!(game.chips(2), START_CHIPS, "sitting out costs nothing");
        let snap = game.snapshot();
        assert_eq!(snap["players"][0]["state"], "won");
        assert_eq!(snap["players"][1]["state"], "lost");
        assert_eq!(snap["players"][2]["state"], "sat_out");
        assert_eq!(snap["players"][1]["pick"], wrong);
        assert_eq!(snap["perfect"], perfect);
    }

    #[test]
    fn odds_fall_as_the_grill_reveals_itself_so_an_early_lock_pays_more() {
        assert!(odds_at(6.0, 0.0) > odds_at(6.0, 0.5));
        assert!(odds_at(6.0, 0.5) > odds_at(6.0, 1.0));
        assert!(odds_at(1.5, 1.0) > 1.0, "a favourite always pays something");
        let mut early = HighStakesDiner::new(&people(2), 4);
        let mut late = HighStakesDiner::new(&people(2), 4);
        let perfect = early.perfect();
        let mut events = Vec::new();
        early.step(&[(0, lock_input(perfect, 50))], TICK_DT, &mut events);
        idle(&mut late, 2, COOK_SECONDS * 0.9, &mut events);
        late.step(&[(0, lock_input(perfect, 50))], TICK_DT, &mut events);
        until_reveal(&mut early, 2, &mut events);
        until_reveal(&mut late, 2, &mut events);
        assert!(early.chips(0) > late.chips(0));
    }

    #[test]
    fn the_server_refuses_second_locks_overdrafts_short_stakes_and_late_locks() {
        let mut game = HighStakesDiner::new(&people(3), 8);
        let mut events = Vec::new();
        game.step(
            &[
                (0, lock_input(0, START_CHIPS + 50)),
                (1, lock_input(0, 5)),
                (2, lock_input(1, 20)),
            ],
            TICK_DT,
            &mut events,
        );
        let snap = game.snapshot();
        assert!(
            snap["players"][0]["bet"].is_null(),
            "no betting chips you do not have"
        );
        assert!(
            snap["players"][1]["bet"].is_null(),
            "under the table minimum"
        );
        assert_eq!(snap["players"][2]["bet"], 20);
        game.step(&[(2, lock_input(2, 90))], TICK_DT, &mut events);
        assert_eq!(game.snapshot()["players"][2]["bet"], 20, "a lock is final");
        game.step(&[(9, lock_input(2, 20))], TICK_DT, &mut events);
        until_reveal(&mut game, 3, &mut events);
        game.step(&[(0, lock_input(0, 20))], TICK_DT, &mut events);
        assert!(
            game.snapshot()["players"][0]["bet"].is_null(),
            "bets close at the buzzer"
        );
    }

    #[test]
    fn an_odd_stack_can_still_go_all_in() {
        let mut game = HighStakesDiner::new(&people(2), 3);
        game.set_chips(0, 37);
        let mut events = Vec::new();
        game.step(&[(0, lock_input(0, 40))], TICK_DT, &mut events);
        assert_eq!(game.snapshot()["players"][0]["bet"], 37);
    }

    #[test]
    fn nothing_that_decides_the_round_is_on_the_wire_before_the_buzzer() {
        let mut game = HighStakesDiner::new(&people(2), 21);
        let mut events = Vec::new();
        game.step(&[(0, lock_input(2, 30))], TICK_DT, &mut events);
        let snap = game.snapshot();
        assert!(snap["perfect"].is_null());
        assert!(
            snap["cuts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["done"].is_null())
        );
        assert!(
            snap["players"][0]["pick"].is_null(),
            "rivals cannot copy a lock"
        );
        assert_eq!(snap["players"][0]["state"], "locked");
        until_reveal(&mut game, 2, &mut events);
        let snap = game.snapshot();
        assert!(
            snap["cuts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["done"].is_number())
        );
        assert_eq!(snap["players"][0]["pick"], 2);
    }

    #[test]
    fn a_busted_player_is_comped_back_to_the_minimum_for_the_next_round() {
        let mut game = HighStakesDiner::new(&people(2), 5);
        let mut events = Vec::new();
        let wrong = (game.perfect() + 2) % CUTS;
        game.step(&[(0, lock_input(wrong, START_CHIPS))], TICK_DT, &mut events);
        until_reveal(&mut game, 2, &mut events);
        assert_eq!(game.chips(0), 0);
        until_next_round(&mut game, 2, &mut events);
        assert_eq!(game.round(), 1);
        assert_eq!(game.chips(0), MIN_BET);
        assert_eq!(game.snapshot()["players"][0]["comped"], true);
    }

    #[test]
    fn standings_rank_by_chips_and_keep_true_ties_together() {
        let mut game = HighStakesDiner::new(&people(4), 9);
        let mut events = Vec::new();
        let perfect = game.perfect();
        game.step(&[(2, lock_input(perfect, 50))], TICK_DT, &mut events);
        while game.placements().is_none() {
            idle(&mut game, 4, TICK_DT, &mut events);
        }
        let placed = game.placements().unwrap();
        assert_eq!(placed[0], vec![2]);
        assert_eq!(placed[1], vec![0, 1, 3], "the ones who never bet tie");
    }

    #[test]
    fn a_player_who_leaves_keeps_their_stack_and_is_still_ranked() {
        let mut game = HighStakesDiner::new(&table(4, 0.6), 31);
        let mut events = Vec::new();
        let mut ticks = 0;
        while game.placements().is_none() && ticks < 6000 {
            let inputs: Vec<(u8, Input)> = (0..4u8)
                .map(|s| {
                    if s == 3 && ticks > 200 {
                        (s, Input::default())
                    } else {
                        (s, game.bot_input(s, TICK_DT))
                    }
                })
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            ticks += 1;
        }
        let placed = game.placements().expect("the bout finishes without them");
        assert_eq!(placed.iter().flatten().count(), 4);
        assert!(placed.iter().flatten().any(|s| *s == 3));
    }

    #[test]
    fn diners_stand_in_a_row_across_the_grill_for_any_table_size() {
        for count in 2..=8u8 {
            let game = HighStakesDiner::new(&table(count, 0.5), 1);
            let snap = game.snapshot();
            let xs: Vec<f64> = snap["players"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["x"].as_f64().unwrap())
                .collect();
            assert!(xs.windows(2).all(|w| w[1] - w[0] > 1.0), "nobody overlaps");
            assert!(
                (xs.iter().sum::<f64>()).abs() < 1e-3,
                "centred on the grill"
            );
        }
    }

    #[test]
    fn a_lock_sent_as_a_fire_message_reaches_the_bout_through_a_real_room() {
        use crate::room::{Phase, Room};
        let mut room = Room::new("diner-room", 12);
        room.enable_native_mode();
        room.connect("host", "Host", "").unwrap();
        for _ in 0..3 {
            room.add_bot("host").unwrap();
        }
        room.choose_game("host", KEY).unwrap();
        room.start("host").unwrap();
        room.ready("host").unwrap();
        for _ in 0..600 {
            room.step();
            if room.phase == Phase::Playing {
                break;
            }
        }
        assert_eq!(room.phase, Phase::Playing);
        room.set_input("host", Default::default(), 0.4, 0.0, false, false);
        room.queue_fire("host", lock_code(4, 30), 0.0);
        room.set_input("host", Default::default(), 0.9, 0.0, false, false);
        room.step();
        let snap = room.snapshot().unwrap();
        let host = snap["players"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["bot"] == false)
            .unwrap()
            .clone();
        assert_eq!(
            host["state"], "locked",
            "a movement update between press and tick keeps the lock"
        );
        assert_eq!(host["bet"], 30);
        for _ in 0..(60 * 90) {
            room.step();
            if matches!(room.phase, Phase::Results { .. }) {
                break;
            }
        }
        assert!(
            matches!(room.phase, Phase::Results { .. }),
            "the bout ends on its own"
        );
    }

    struct Tally {
        bouts: u32,
        ticks: u64,
        locks: u32,
        right: [u32; 4],
        picks: [u32; 4],
        sat_out: u32,
        lock_progress: f32,
        wins_by_skill: [f32; 4],
        wins_by_temper: [f32; 3],
        seats_by_temper: [u32; 3],
        final_chips: Vec<u32>,
        comps: u32,
    }

    fn simulate(bouts: u64, seed: u64) -> Tally {
        let skills = [0.15, 0.4, 0.7, 1.0];
        let mut t = Tally {
            bouts: 0,
            ticks: 0,
            locks: 0,
            right: [0; 4],
            picks: [0; 4],
            sat_out: 0,
            lock_progress: 0.0,
            wins_by_skill: [0.0; 4],
            wins_by_temper: [0.0; 3],
            seats_by_temper: [0; 3],
            final_chips: Vec::new(),
            comps: 0,
        };
        for bout in 0..bouts {
            let count = 2 + (bout % 7) as u8;
            let seats: Vec<Contestant> = (0..count)
                .map(|s| Contestant::bot(s, skills[s as usize % 4]))
                .collect();
            let mut game = HighStakesDiner::new(&seats, seed + bout);
            let mut events = Vec::new();
            let mut ticks = 0u64;
            while game.placements().is_none() {
                let inputs: Vec<(u8, Input)> = (0..count)
                    .map(|s| (s, game.bot_input(s, TICK_DT)))
                    .collect();
                let cooking = game.stage() == Stage::Cooking;
                let progress = game.progress();
                events.clear();
                game.step(&inputs, TICK_DT, &mut events);
                for event in &events {
                    if event["kind"] == "lock" {
                        t.locks += 1;
                        if cooking {
                            t.lock_progress += progress;
                        }
                    }
                    if event["kind"] == "buzzer" {
                        let perfect = event["perfect"].as_u64().unwrap();
                        let payouts = event["payouts"].as_array().unwrap();
                        t.sat_out += count as u32 - payouts.len() as u32;
                        for pay in payouts {
                            let seat = pay["seat"].as_u64().unwrap() as usize;
                            t.picks[seat % 4] += 1;
                            if pay["pick"].as_u64() == Some(perfect) {
                                t.right[seat % 4] += 1;
                            }
                        }
                    }
                    if event["kind"] == "round" {
                        t.comps += game.players.iter().filter(|p| p.comped).count() as u32;
                    }
                }
                ticks += 1;
                assert!(
                    ticks < 6000,
                    "a bout of bots finishes inside the room's budget"
                );
            }
            t.ticks += ticks;
            t.bouts += 1;
            let placed = game.placements().unwrap();
            let share = 1.0 / placed[0].len() as f32;
            for seat in &placed[0] {
                t.wins_by_skill[*seat as usize % 4] += share;
                t.wins_by_temper[game.temper(*seat) as usize] += share;
            }
            for s in 0..count {
                t.seats_by_temper[game.temper(s) as usize] += 1;
                t.final_chips.push(game.chips(s));
            }
        }
        t
    }

    #[test]
    fn sharper_bots_read_the_grill_better_and_every_bout_fits_the_clock() {
        let t = simulate(140, 500);
        let accuracy: Vec<f32> = (0..4)
            .map(|i| t.right[i] as f32 / t.picks[i].max(1) as f32)
            .collect();
        assert!(
            accuracy[3] > accuracy[0] + 0.15,
            "ruthless beats relaxed: {accuracy:?}"
        );
        assert!(
            accuracy[0] > 1.0 / CUTS as f32,
            "even relaxed beats a blind guess: {accuracy:?}"
        );
        assert!(accuracy[3] < 0.95, "the cues stay noisy: {accuracy:?}");
        let seconds = t.ticks as f32 / t.bouts as f32 * TICK_DT;
        assert!((45.0..=90.0).contains(&seconds), "a bout runs {seconds}s");
        assert_eq!(t.sat_out, 0, "bots always get a bet down");
    }

    /// Prints the balance table; run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn balance_report() {
        let t = simulate(2000, 90_000);
        let names = ["relaxed", "steady", "sharp", "ruthless"];
        println!(
            "bouts {} average {:.1}s",
            t.bouts,
            t.ticks as f32 / t.bouts as f32 * TICK_DT
        );
        for i in 0..4 {
            println!(
                "{:9} pick accuracy {:.2}  bout wins {:.0}",
                names[i],
                t.right[i] as f32 / t.picks[i].max(1) as f32,
                t.wins_by_skill[i]
            );
        }
        for (i, name) in ["bold", "steady", "cautious"].iter().enumerate() {
            println!(
                "{name:9} seats {:5}  win share per seat {:.3}",
                t.seats_by_temper[i],
                t.wins_by_temper[i] / t.seats_by_temper[i].max(1) as f32
            );
        }
        let mut chips = t.final_chips.clone();
        chips.sort();
        println!(
            "final chips p10 {} median {} p90 {} max {}  comps {}  mean lock progress {:.2}",
            chips[chips.len() / 10],
            chips[chips.len() / 2],
            chips[chips.len() * 9 / 10],
            chips[chips.len() - 1],
            t.comps,
            t.lock_progress / t.locks.max(1) as f32
        );
    }
}
