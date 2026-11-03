//! Pigeon Post: watch four marked letters reach colored mailboxes, then name
//! the box that received the called letter after the birds clear the yard.
use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "pigeon";
pub const ROUNDS: usize = 6;
pub const STUDY: f32 = 6.0;
pub const GUESS: f32 = 5.0;
pub const REVEAL: f32 = 1.0;
pub const LIMIT: f32 = ROUNDS as f32 * (STUDY + GUESS + REVEAL);
const MARKS: [&str; 9] = [
    "crown", "moon", "fish", "star", "shoe", "leaf", "cup", "key", "heart",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Study,
    Guess,
    Reveal,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Study => "study",
            Self::Guess => "guess",
            Self::Reveal => "reveal",
        }
    }
}

#[derive(Clone)]
struct Letter {
    mark: &'static str,
    box_id: u8,
}
#[derive(Clone)]
struct Sorter {
    seat: u8,
    score: u32,
    right: u8,
    guess: Option<u8>,
    plan: f32,
    skill: f32,
}

pub struct PigeonPost {
    players: Vec<Sorter>,
    letters: Vec<Letter>,
    called: usize,
    round: usize,
    stage: Stage,
    clock: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
impl PigeonPost {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut game = Self {
            players: contestants
                .iter()
                .map(|c| Sorter {
                    seat: c.seat,
                    score: 0,
                    right: 0,
                    guess: None,
                    plan: 0.0,
                    skill: c.skill.clamp(0.0, 1.0),
                })
                .collect(),
            letters: Vec::new(),
            called: 0,
            round: 0,
            stage: Stage::Study,
            clock: 0.0,
            rng: StdRng::seed_from_u64(seed),
            result: None,
        };
        game.deal();
        game
    }
    fn deal(&mut self) {
        let mut marks = MARKS.to_vec();
        marks.shuffle(&mut self.rng);
        self.letters = marks
            .into_iter()
            .take(4)
            .map(|mark| Letter {
                mark,
                box_id: self.rng.random_range(0..3),
            })
            .collect();
        self.called = self.rng.random_range(0..self.letters.len());
        for p in &mut self.players {
            p.guess = None;
            p.plan = 0.7 + (1.0 - p.skill) * 1.0 + self.rng.random_range(0.0..1.8);
        }
    }
    fn advance(&mut self) {
        self.clock = 0.0;
        match self.stage {
            Stage::Study => self.stage = Stage::Guess,
            Stage::Guess => self.stage = Stage::Reveal,
            Stage::Reveal => {
                self.round += 1;
                if self.round == ROUNDS {
                    self.finish();
                } else {
                    self.stage = Stage::Study;
                    self.deal();
                }
            }
        }
    }
    fn finish(&mut self) {
        let mut scores: Vec<_> = self.players.iter().map(|p| (p.seat, p.score)).collect();
        scores.sort_by(|a, b| b.1.cmp(&a.1));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score) in scores {
            if prior != Some(score) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some(score);
        }
        self.result = Some(places);
    }
    fn answer(&self) -> u8 {
        self.letters[self.called].box_id
    }
}
fn choice(input: Input) -> Option<u8> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let index = input.yaw.round();
    if (input.yaw - index).abs() > 0.01 || !(0.0..=2.0).contains(&index) {
        return None;
    }
    Some(index as u8)
}
impl Bout for PigeonPost {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"rounds":ROUNDS,"seconds":LIMIT})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        self.clock += dt.clamp(0.0, 0.1);
        if self.stage == Stage::Guess {
            let answer = self.answer();
            for (seat, input) in inputs {
                let Some(pick) = choice(*input) else {
                    continue;
                };
                let Some(player) = self.players.iter_mut().find(|p| p.seat == *seat) else {
                    continue;
                };
                if player.guess.is_some() {
                    continue;
                }
                player.guess = Some(pick);
                if pick == answer {
                    player.score += 2;
                    player.right += 1;
                }
                events.push(json!({"kind":"pigeon_guess","seat":seat,"right":pick==answer}));
            }
        }
        let duration = match self.stage {
            Stage::Study => STUDY,
            Stage::Guess => GUESS,
            Stage::Reveal => REVEAL,
        };
        if self.clock >= duration {
            self.advance();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.stage != Stage::Guess || self.result.is_some() {
            return Input::default();
        }
        let answer = self.answer();
        let Some(player) = self.players.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if player.guess.is_some() || self.clock < player.plan {
            return Input::default();
        }
        let correct = self.rng.random_bool((0.42 + player.skill * 0.43) as f64);
        let pick = if correct {
            answer
        } else {
            (answer + self.rng.random_range(1..3)) % 3
        };
        Input {
            fire: true,
            yaw: pick as f32,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        let show_routes = self.stage != Stage::Guess;
        let center = (self.players.len() as f32 - 1.0).max(0.0) * 0.5;
        json!({"game":KEY,"time":self.round as f32 * (STUDY+GUESS+REVEAL)+self.clock,
            "limit":LIMIT,"round":(self.round+1).min(ROUNDS),"rounds":ROUNDS,"stage":self.stage.name(),"clock":self.clock,
            "called":if self.stage==Stage::Study { None } else { Some(self.letters[self.called].mark) },
            "letters":self.letters.iter().map(|l| json!({"mark":l.mark,"box":if show_routes { Some(l.box_id) } else { None }})).collect::<Vec<_>>(),
            "answer":if self.stage==Stage::Reveal { Some(self.answer()) } else { None },
            "players":self.players.iter().enumerate().map(|(index,p)| json!({"seat":p.seat,"x":(index as f32-center)*2.0,"z":-4.8,"yaw":0.0,"score":p.score,"right":p.right,
                "guess":if self.stage==Stage::Reveal { p.guess } else { None }})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn people() -> Vec<Contestant> {
        vec![Contestant::person(0), Contestant::person(1)]
    }
    #[test]
    fn secret_answer_is_not_sent_during_guess() {
        let mut game = PigeonPost::new(&people(), 3);
        for player in game.snapshot()["players"].as_array().unwrap() {
            assert!(player["x"].is_number());
            assert_eq!(player["z"], -4.8);
        }
        assert!(game.snapshot()["letters"][0]["box"].is_number());
        assert!(game.snapshot()["called"].is_null());
        for _ in 0..360 {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.stage, Stage::Guess);
        assert!(game.snapshot()["answer"].is_null());
        assert!(game.snapshot()["letters"][0]["box"].is_null());
    }
    #[test]
    fn bad_and_duplicate_guesses_do_not_score() {
        let mut game = PigeonPost::new(&people(), 3);
        game.stage = Stage::Guess;
        let answer = game.answer();
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
                (
                    99,
                    Input {
                        fire: true,
                        yaw: answer as f32,
                        ..Default::default()
                    },
                ),
            ],
            1.0 / 60.0,
            &mut vec![],
        );
        assert_eq!(game.players[0].score, 0);
        let good = Input {
            fire: true,
            yaw: answer as f32,
            ..Default::default()
        };
        game.step(&[(0, good)], 1.0 / 60.0, &mut vec![]);
        game.step(&[(0, good)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 2);
    }
    #[test]
    fn bots_and_missing_player_finish_with_all_seats() {
        let players = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = PigeonPost::new(&players, 9);
        for _ in 0..(LIMIT as usize * 60 + 120) {
            let inputs = (0..7)
                .map(|s| (s, game.bot_input(s, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.score > 0));
    }
}
