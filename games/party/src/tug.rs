//! Tug of War: two sides on facing docks with one rope over the water between
//! them. Alternating presses heave the knot your way, and the side it drags
//! over the gap goes in.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Press, Skill};

pub const KEY: &str = "tug";
/// Half the water gap the losing side is hauled over.
pub const PIT_HALF: f32 = 2.4;
/// How far the knot travels before the other side is over the water.
pub const BRINK: f32 = 4.5;
pub const DOCK_END: f32 = 11.0;
pub const DOCK_HALF_Z: f32 = 3.5;
pub const ROUND_SECONDS: f32 = 50.0;
/// Where the front pair of hands grips, and the gap back to the next pair.
/// The grip is inside the brink by more than the pit, so hauling the knot all
/// the way back leaves the other side standing over open water.
const GRIP: f32 = 3.0;
const SPACING: f32 = 1.2;
/// A heave counts when it alternates and keeps the rhythm; a longer gap
/// takes up the slack instead.
const MAX_GAP: f32 = 0.8;
/// Faster than this is a macro, not hands. A short burst over it still
/// counts, so taps the network bunches together are not lost.
const MAX_RATE: f32 = 20.0;
const BURST: f32 = 3.0;
/// How far one clean heave takes the knot, shared out across the side.
const STEP: f32 = 0.08;
const SPLASH_SECONDS: f32 = 1.5;
const FALL: f32 = 11.0;
const WATER_Y: f32 = -3.2;
const RULE: &str =
    "Alternate your presses to heave the rope. The side it hauls over the water goes in.";

/// Which way a side hauls: red stands at the negative end of the rope.
fn direction(team: u8) -> f32 {
    if team == 0 { -1.0 } else { 1.0 }
}

#[derive(Clone, Debug)]
pub struct Puller {
    pub seat: u8,
    pub team: u8,
    /// Place in the line back from the rope, nearest hands first.
    slot: usize,
    pub x: f32,
    pub y: f32,
    pub heaves: u32,
    /// How recently they hauled, for the lean the client draws.
    pull: f32,
    /// Which hand went last. The same one twice does nothing, so a side only
    /// moves the rope while it keeps alternating.
    last: Option<Press>,
    last_press: f32,
    /// Heaves in hand against the rate cap, refilled as time passes.
    budget: f32,
    refilled: f32,
}

impl Puller {
    fn z(&self) -> f32 {
        if self.slot % 2 == 0 { -0.45 } else { 0.45 }
    }

    /// Takes one press. It only pays when it alternates and keeps the rhythm;
    /// the first press of a round, and the first after dropping the rhythm,
    /// take up the slack instead. A press over the rate cap still counts as
    /// the hand that went last, so the next one in time alternates.
    fn take(&mut self, press: Press, now: f32) -> bool {
        if self.last == Some(press) {
            return false;
        }
        self.last = Some(press);
        self.budget = (self.budget + (now - self.refilled) * MAX_RATE).min(BURST);
        self.refilled = now;
        let since = now - self.last_press;
        self.last_press = now;
        if since > MAX_GAP || self.budget < 1.0 {
            return false;
        }
        self.budget -= 1.0;
        self.heaves += 1;
        self.pull = 1.0;
        true
    }
}

#[derive(Clone, Copy, Debug)]
struct BotBrain {
    side: f32,
    next: f32,
}

pub struct TugOfWar {
    pub pullers: Vec<Puller>,
    /// Where the knot sits. Negative is red's way, positive is blue's.
    pub rope: f32,
    pub time: f32,
    sides: [usize; 2],
    /// How far each dock runs out, longer than the docks for four so a
    /// bigger side's line still stands on planks when it is hauled back.
    pub half: f32,
    /// The side going in the water, once one is.
    pub dunked: Option<u8>,
    splash: f32,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

impl TugOfWar {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        // The board picks the sides; with none given, alternate down the list.
        let given = contestants.iter().any(|c| c.team != contestants[0].team);
        let mut sides = [0usize; 2];
        let mut pullers = Vec::new();
        for (index, contestant) in contestants.iter().enumerate() {
            let team = if given {
                contestant.team.min(1)
            } else {
                (index % 2) as u8
            };
            let slot = sides[team as usize];
            sides[team as usize] += 1;
            pullers.push(Puller {
                seat: contestant.seat,
                team,
                slot,
                x: 0.0,
                y: 0.0,
                heaves: 0,
                pull: 0.0,
                last: None,
                last_press: -10.0,
                budget: BURST,
                refilled: 0.0,
            });
        }
        let mut game = TugOfWar {
            pullers,
            rope: 0.0,
            time: 0.0,
            sides,
            half: DOCK_END * crate::bout::crowd_scale(contestants.len()),
            dunked: None,
            splash: 0.0,
            brains: HashMap::new(),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            rng: StdRng::seed_from_u64(seed),
            placed: None,
        };
        game.line_up();
        game
    }

    pub fn team_of(&self, seat: u8) -> Option<u8> {
        self.pullers.iter().find(|p| p.seat == seat).map(|p| p.team)
    }

    /// Everyone stands where their grip on the rope has dragged them.
    fn line_up(&mut self) {
        let rope = self.rope;
        for puller in &mut self.pullers {
            puller.x = rope + direction(puller.team) * (GRIP + puller.slot as f32 * SPACING);
        }
    }

    fn heaves(&self) -> [u32; 2] {
        let mut total = [0u32; 2];
        for puller in &self.pullers {
            total[puller.team as usize] += puller.heaves;
        }
        total
    }

    /// Which side is losing when the clock runs out: the rope first, then
    /// whoever did less of the hauling.
    fn behind(&self) -> u8 {
        if self.rope.abs() > 0.05 {
            return if self.rope < 0.0 { 1 } else { 0 };
        }
        let heaves = self.heaves();
        if heaves[0] >= heaves[1] { 1 } else { 0 }
    }

    fn dunk(&mut self, loser: u8, events: &mut Vec<Value>) {
        self.dunked = Some(loser);
        self.splash = SPLASH_SECONDS;
        for puller in self.pullers.iter().filter(|p| p.team == loser) {
            events.push(
                json!({ "kind": "splash", "seat": puller.seat, "x": puller.x, "z": puller.z() }),
            );
        }
        events.push(json!({ "kind": "dunked", "team": loser }));
    }

    /// The splash: the rope snaps over and the losing side drops in.
    fn settle(&mut self, dt: f32) {
        let Some(loser) = self.dunked else {
            return;
        };
        self.splash -= dt;
        let target = direction(1 - loser) * BRINK;
        self.rope += (target - self.rope).clamp(-10.0 * dt, 10.0 * dt);
        self.line_up();
        for puller in &mut self.pullers {
            puller.pull = 0.0;
            if puller.team == loser {
                puller.y = (puller.y - FALL * dt).max(WATER_Y);
            }
        }
        if self.splash <= 0.0 {
            self.placed = Some(
                [1 - loser, loser]
                    .into_iter()
                    .map(|team| {
                        self.pullers
                            .iter()
                            .filter(|p| p.team == team)
                            .map(|p| p.seat)
                            .collect::<Vec<u8>>()
                    })
                    .filter(|group| !group.is_empty())
                    .collect(),
            );
        }
    }
}

impl Bout for TugOfWar {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rule": RULE,
            "half": self.half,
            "half_z": DOCK_HALF_Z,
            "pit": PIT_HALF,
            "brink": BRINK,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        if self.dunked.is_some() {
            self.settle(dt);
            return;
        }
        self.time += dt;
        let now = self.time;
        for index in 0..self.pullers.len() {
            let seat = self.pullers[index].seat;
            let Some(press) = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .and_then(|(_, input)| input.press)
            else {
                continue;
            };
            let puller = &mut self.pullers[index];
            if !puller.take(press, now) {
                continue;
            }
            let team = puller.team;
            // Every side hauls at the pace of one pair of hands, so being
            // outnumbered still leaves you a rope to pull.
            let share = STEP / self.sides[team as usize].max(1) as f32;
            self.rope = (self.rope + direction(team) * share).clamp(-BRINK, BRINK);
            events.push(json!({ "kind": "heave", "seat": seat, "team": team }));
        }
        for puller in &mut self.pullers {
            puller.pull = (puller.pull - dt * 3.0).max(0.0);
        }
        self.line_up();
        if self.rope.abs() >= BRINK {
            let loser = if self.rope < 0.0 { 1 } else { 0 };
            self.dunk(loser, events);
        } else if self.time >= ROUND_SECONDS {
            let loser = self.behind();
            self.dunk(loser, events);
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        if self.dunked.is_some() {
            return Input::default();
        }
        let brain = self.brains.get(&seat).copied().unwrap_or(BotBrain {
            side: -1.0,
            next: 0.05,
        });
        let mut side = brain.side;
        let mut next = brain.next - dt;
        let mut press = None;
        if next <= 0.0 {
            let skill = self.skills.get(&seat).copied().unwrap_or_default();
            side = -side;
            // A fumbled beat drops out of the rhythm window, which costs the
            // heave, so a clumsy bot pulls at a believable stutter.
            let fumbled = self.rng.random_bool(skill.hesitation() * 0.35);
            next = self.rng.random_range(0.14..0.22)
                + (1.0 - skill.level()) * 0.16
                + if fumbled { 0.55 } else { 0.0 };
            press = Some(if side > 0.0 {
                Press::Right
            } else {
                Press::Left
            });
        }
        self.brains.insert(seat, BotBrain { side, next });
        Input {
            press,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "rope": round(self.rope),
            "brink": BRINK,
            "dunked": self.dunked,
            "players": self.pullers.iter().map(|p| json!({
                "seat": p.seat,
                "team": p.team,
                "x": round(p.x),
                "z": p.z(),
                "y": round(p.y),
                "heaves": p.heaves,
                "pull": round(p.pull),
                "out": self.dunked == Some(p.team),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::{Stick, TICK_DT};

    fn pullers(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    /// How far out the back of the longest line stands with the knot hauled
    /// all the way to the other side.
    fn furthest(game: &TugOfWar) -> f32 {
        let back = game.pullers.iter().map(|p| p.slot).max().unwrap_or(0);
        BRINK + GRIP + back as f32 * SPACING
    }

    fn press(hand: Press) -> Input {
        Input {
            press: Some(hand),
            ..Default::default()
        }
    }

    fn alternate(hand: Press) -> Press {
        if hand == Press::Left {
            Press::Right
        } else {
            Press::Left
        }
    }

    /// Taps at `rate` a second with a human wobble on each beat, as the
    /// times they reach the room.
    fn mashing(rate: f32, seconds: f32, rng: &mut StdRng) -> Vec<f32> {
        let mut times = Vec::new();
        let mut at = 0.02;
        while at < seconds {
            times.push(at);
            at += (1.0 / rate + rng.random_range(-0.015..0.015)).max(0.03);
        }
        times
    }

    /// Plays a round the way the room feeds it: each tap waits in its seat's
    /// queue and is handed over on the first tick it can be, one a tick.
    fn play(game: &mut TugOfWar, taps: &[Vec<f32>]) {
        let mut queued = vec![0usize; taps.len()];
        let mut given = vec![0usize; taps.len()];
        let mut hands = vec![Press::Right; taps.len()];
        let mut events = Vec::new();
        let mut tick = 0;
        while game.placements().is_none() {
            tick += 1;
            let now = tick as f32 * TICK_DT;
            let mut inputs = Vec::new();
            for (seat, times) in taps.iter().enumerate() {
                while queued[seat] < times.len() && times[queued[seat]] <= now {
                    queued[seat] += 1;
                }
                if given[seat] < queued[seat] {
                    given[seat] += 1;
                    hands[seat] = alternate(hands[seat]);
                    inputs.push((seat as u8, press(hands[seat])));
                }
            }
            game.step(&inputs, TICK_DT, &mut events);
        }
    }

    #[test]
    fn the_faster_masher_wins_and_every_press_pays() {
        let mut rng = StdRng::seed_from_u64(5);
        for (fast, slow) in [(12.0, 8.0), (14.0, 10.0), (13.0, 12.0)] {
            let seconds = ROUND_SECONDS + 5.0;
            let taps = vec![
                mashing(fast, seconds, &mut rng),
                mashing(slow, seconds, &mut rng),
            ];
            let mut game = TugOfWar::new(&[Contestant::person(0), Contestant::person(1)], 1);
            play(&mut game, &taps);
            assert_eq!(game.dunked, Some(1), "{fast}/s hauls {slow}/s in");
            let elapsed = game.time;
            for (puller, rate) in game.pullers.iter().zip([fast, slow]) {
                let pressed = taps[puller.seat as usize]
                    .iter()
                    .filter(|t| **t <= elapsed)
                    .count() as u32;
                assert!(
                    puller.heaves + 2 >= pressed,
                    "at {rate}/s every press after the first counts: {} of {pressed}",
                    puller.heaves
                );
            }
        }
    }

    #[test]
    fn a_press_over_the_cap_does_not_cost_the_next_one() {
        let mut game = TugOfWar::new(&pullers(2), 1);
        let puller = &mut game.pullers[0];
        let mut hand = Press::Left;
        let mut now = 1.0;
        let mut counted = 0;
        for _ in 0..100 {
            counted += puller.take(hand, now) as u32;
            hand = alternate(hand);
            now += 0.01;
        }
        assert!(
            (20..=25).contains(&counted),
            "a hundred a second is held to the cap: {counted}"
        );
        assert!(!puller.take(hand, now), "this one is over the cap");
        hand = alternate(hand);
        now += 1.0 / 12.0;
        assert!(
            puller.take(hand, now),
            "the next press in time still alternates and counts"
        );
    }

    #[test]
    fn holding_or_repeating_one_press_does_nothing() {
        let mut game = TugOfWar::new(&pullers(2), 1);
        let mut events = Vec::new();
        game.step(&[(0, press(Press::Left))], TICK_DT, &mut events);
        for tick in 0..240 {
            let held = Input {
                stick: Stick { x: 1.0, z: 0.0 },
                fire: true,
                press: (tick % 6 == 0).then_some(Press::Left),
                ..Default::default()
            };
            game.step(&[(0, held)], TICK_DT, &mut events);
        }
        assert_eq!(game.pullers[0].heaves, 0);
        assert_eq!(game.rope, 0.0, "the rope has not moved");
    }

    #[test]
    fn a_bigger_field_gets_longer_docks() {
        let four = TugOfWar::new(&pullers(4), 1);
        assert_eq!(four.half, DOCK_END, "four play it as built");
        let eight = TugOfWar::new(&pullers(8), 1);
        assert!(
            (eight.half / DOCK_END - 2f32.sqrt()).abs() < 1e-4,
            "eight get the room of two fours"
        );
        assert!(
            furthest(&eight) > DOCK_END,
            "four a side would hang off the old docks"
        );
        assert!(furthest(&eight) < eight.half, "but stand on the new ones");
        assert_eq!(
            eight.setup()["half"],
            json!(eight.half),
            "and the client is told"
        );
        assert_eq!(
            eight.setup()["brink"],
            json!(BRINK),
            "the rope still has as far to go"
        );
    }
}
