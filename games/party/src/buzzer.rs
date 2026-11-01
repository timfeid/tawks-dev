//! Buzzer Round: quick questions everyone answers at once on their own screen.
//! The first right answer to each question scores a point. Everyone gets one
//! guess per question, so a wrong one sits you out until the next. Most points
//! after the last question wins.
//!
//! An answer arrives as a press of fire with the option's index in `yaw`, so
//! the bout needs nothing from the protocol beyond what every bout already has.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "buzzer";
pub const QUESTIONS: usize = 6;
pub const ANSWER_SECONDS: f32 = 12.0;
pub const REVEAL_SECONDS: f32 = 2.5;
/// The round stage the row of lecterns stands on, sized for four.
pub const STAGE_RADIUS: f32 = 7.5;

/// A question as written: the right answer first, then three wrong ones. The
/// order is shuffled when the question is asked.
type Written = (&'static str, &'static str, [&'static str; 3]);

const BANK: &[Written] = &[
    (
        "Which planet is closest to the Sun?",
        "Mercury",
        ["Venus", "Mars", "Earth"],
    ),
    ("How many sides does a hexagon have?", "6", ["5", "7", "8"]),
    (
        "What is the largest ocean on Earth?",
        "Pacific",
        ["Atlantic", "Indian", "Arctic"],
    ),
    (
        "Which gas do plants take in to make their food?",
        "Carbon dioxide",
        ["Oxygen", "Nitrogen", "Helium"],
    ),
    (
        "What is the capital of Canada?",
        "Ottawa",
        ["Toronto", "Vancouver", "Montreal"],
    ),
    (
        "How many players does a soccer team have on the field?",
        "11",
        ["9", "10", "12"],
    ),
    (
        "What is the hardest natural substance?",
        "Diamond",
        ["Granite", "Quartz", "Iron"],
    ),
    (
        "What is the largest animal alive today?",
        "Blue whale",
        ["African elephant", "Sperm whale", "Giraffe"],
    ),
    (
        "How many minutes are in a day?",
        "1,440",
        ["1,240", "1,600", "2,400"],
    ),
    (
        "Who painted the Mona Lisa?",
        "Leonardo da Vinci",
        ["Michelangelo", "Raphael", "Rembrandt"],
    ),
    (
        "What is the chemical symbol for gold?",
        "Au",
        ["Ag", "Gd", "Go"],
    ),
    (
        "Which country do kangaroos come from?",
        "Australia",
        ["New Zealand", "South Africa", "Brazil"],
    ),
    ("How many continents are there?", "7", ["5", "6", "8"]),
    (
        "What is the tallest mountain above sea level?",
        "Mount Everest",
        ["K2", "Kilimanjaro", "Denali"],
    ),
    (
        "Which instrument usually has 88 keys?",
        "Piano",
        ["Accordion", "Harpsichord", "Xylophone"],
    ),
    (
        "At sea level, water boils at how many degrees Celsius?",
        "100",
        ["90", "110", "212"],
    ),
    (
        "In which sport do you score a touchdown?",
        "American football",
        ["Baseball", "Basketball", "Golf"],
    ),
    (
        "Which planet is known as the Red Planet?",
        "Mars",
        ["Jupiter", "Venus", "Saturn"],
    ),
    (
        "What is the longest river in South America?",
        "Amazon",
        ["Orinoco", "Parana", "Magdalena"],
    ),
    ("How many legs does a spider have?", "8", ["6", "10", "12"]),
    (
        "Which language has the most native speakers?",
        "Mandarin Chinese",
        ["English", "Spanish", "Hindi"],
    ),
    (
        "Which metal is liquid at room temperature?",
        "Mercury",
        ["Lead", "Tin", "Aluminum"],
    ),
    ("What is the smallest prime number?", "2", ["0", "1", "3"]),
    (
        "Which organ pumps blood around the body?",
        "Heart",
        ["Liver", "Lungs", "Kidneys"],
    ),
    (
        "What is the capital of Japan?",
        "Tokyo",
        ["Kyoto", "Osaka", "Seoul"],
    ),
    (
        "How many strings does a standard guitar have?",
        "6",
        ["4", "5", "7"],
    ),
    (
        "Which bird is the usual symbol of peace?",
        "Dove",
        ["Eagle", "Swan", "Robin"],
    ),
    (
        "What is the largest planet in our solar system?",
        "Jupiter",
        ["Saturn", "Neptune", "Earth"],
    ),
    (
        "Who wrote Romeo and Juliet?",
        "William Shakespeare",
        ["Charles Dickens", "Jane Austen", "Mark Twain"],
    ),
    (
        "Mixing blue and yellow paint makes what?",
        "Green",
        ["Purple", "Orange", "Brown"],
    ),
    (
        "How many hours are in a week?",
        "168",
        ["148", "172", "186"],
    ),
    (
        "Which country gave the Statue of Liberty to the United States?",
        "France",
        ["England", "Spain", "Italy"],
    ),
    (
        "What is the main ingredient in guacamole?",
        "Avocado",
        ["Tomato", "Lime", "Pea"],
    ),
    (
        "How many degrees are in a right angle?",
        "90",
        ["45", "100", "180"],
    ),
    (
        "What is the fastest land animal?",
        "Cheetah",
        ["Lion", "Pronghorn", "Greyhound"],
    ),
    (
        "What is the smallest country in the world?",
        "Vatican City",
        ["Monaco", "San Marino", "Malta"],
    ),
    (
        "In what year did people first walk on the Moon?",
        "1969",
        ["1959", "1965", "1972"],
    ),
    (
        "How many cards are in a standard deck, leaving out the jokers?",
        "52",
        ["48", "54", "56"],
    ),
    (
        "Which ocean lies between Africa and Australia?",
        "Indian",
        ["Atlantic", "Pacific", "Southern"],
    ),
    (
        "Which planet is best known for its rings?",
        "Saturn",
        ["Uranus", "Jupiter", "Neptune"],
    ),
    (
        "How many periods are in a regulation NHL game?",
        "3",
        ["2", "4", "5"],
    ),
    (
        "What is the capital of Italy?",
        "Rome",
        ["Milan", "Venice", "Naples"],
    ),
    (
        "What do you call a shape with eight sides?",
        "Octagon",
        ["Hexagon", "Pentagon", "Decagon"],
    ),
    (
        "Which vitamin does your skin make in sunlight?",
        "Vitamin D",
        ["Vitamin C", "Vitamin A", "Vitamin B12"],
    ),
    (
        "How many teeth does an adult usually have?",
        "32",
        ["28", "30", "36"],
    ),
    (
        "Which element has the symbol O?",
        "Oxygen",
        ["Osmium", "Iron", "Oganesson"],
    ),
    (
        "How many bones are in the adult human body?",
        "206",
        ["186", "216", "256"],
    ),
    ("What is the square root of 81?", "9", ["7", "8", "11"]),
    (
        "Which city is home to the Eiffel Tower?",
        "Paris",
        ["Lyon", "Brussels", "Marseille"],
    ),
    (
        "How many sides does a stop sign have?",
        "8",
        ["6", "7", "10"],
    ),
];

struct Question {
    text: &'static str,
    options: [&'static str; 4],
    answer: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Asking,
    Reveal,
}

/// When a bot will answer the current question, and with what.
#[derive(Clone, Copy)]
struct Plan {
    round: usize,
    at: f32,
    choice: u8,
}

struct Player {
    seat: u8,
    skill: f32,
    points: u32,
    guess: Option<u8>,
    plan: Option<Plan>,
}

pub struct BuzzerRound {
    questions: Vec<Question>,
    players: Vec<Player>,
    round: usize,
    stage: Stage,
    /// Seconds into the current question, or into its reveal.
    clock: f32,
    /// Who got the current question right first; more than one when they
    /// answered on the same tick.
    scorers: Vec<u8>,
    /// The stage, wider than the one for four so a longer row of lecterns
    /// still stands on it.
    stage_radius: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

/// The option picked by a press, if there was one and it names an option.
fn choice(input: &Input) -> Option<u8> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let index = input.yaw.round();
    ((input.yaw - index).abs() < 0.25 && (0.0..=3.0).contains(&index)).then_some(index as u8)
}

impl BuzzerRound {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut picks: Vec<usize> = (0..BANK.len()).collect();
        picks.shuffle(&mut rng);
        let questions = picks
            .into_iter()
            .take(QUESTIONS)
            .map(|index| {
                let (text, right, wrong) = BANK[index];
                let mut options = [right, wrong[0], wrong[1], wrong[2]];
                options.shuffle(&mut rng);
                let answer = options
                    .iter()
                    .position(|option| *option == right)
                    .unwrap_or(0) as u8;
                Question {
                    text,
                    options,
                    answer,
                }
            })
            .collect();
        BuzzerRound {
            questions,
            players: contestants
                .iter()
                .map(|one| Player {
                    seat: one.seat,
                    skill: one.skill,
                    points: 0,
                    guess: None,
                    plan: None,
                })
                .collect(),
            round: 0,
            stage: Stage::Asking,
            clock: 0.0,
            scorers: Vec::new(),
            stage_radius: STAGE_RADIUS * crate::bout::crowd_scale(contestants.len()),
            rng,
            placed: None,
        }
    }

    #[cfg(test)]
    pub fn answer(&self) -> u8 {
        self.questions[self.round].answer
    }

    #[cfg(test)]
    pub fn round(&self) -> usize {
        self.round
    }

    #[cfg(test)]
    pub fn revealing(&self) -> bool {
        self.stage == Stage::Reveal
    }

    #[cfg(test)]
    pub fn bank() -> &'static [Written] {
        BANK
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, u32)> = self.players.iter().map(|p| (p.seat, p.points)).collect();
        order.sort_by(|a, b| b.1.cmp(&a.1));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<u32> = None;
        for (seat, points) in order {
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if previous == points => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(points);
        }
        self.placed = Some(groups);
    }

    fn reveal(&mut self, events: &mut Vec<Value>) {
        self.stage = Stage::Reveal;
        self.clock = 0.0;
        events.push(json!({
            "kind": "reveal",
            "answer": self.questions[self.round].answer,
            "scorers": self.scorers,
        }));
    }

    fn next_question(&mut self) {
        if self.round + 1 >= self.questions.len() {
            self.settle();
            return;
        }
        self.round += 1;
        self.stage = Stage::Asking;
        self.clock = 0.0;
        self.scorers.clear();
        for player in self.players.iter_mut() {
            player.guess = None;
        }
    }

    fn state(&self, player: &Player) -> &'static str {
        let answer = self.questions[self.round].answer;
        match (self.stage, player.guess) {
            (Stage::Asking, None) => "thinking",
            (Stage::Reveal, None) => "silent",
            (_, Some(guess)) if guess == answer => "right",
            (_, Some(_)) => "wrong",
        }
    }
}

impl Bout for BuzzerRound {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rounds": self.questions.len(),
            "seconds": ANSWER_SECONDS,
            "stage": self.stage_radius,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        if self.stage == Stage::Reveal {
            if self.clock >= REVEAL_SECONDS {
                self.next_question();
            }
            return;
        }

        let answer = self.questions[self.round].answer;
        for (seat, input) in inputs {
            let Some(pick) = choice(input) else { continue };
            let Some(player) = self.players.iter_mut().find(|p| p.seat == *seat) else {
                continue;
            };
            if player.guess.is_some() {
                continue;
            }
            player.guess = Some(pick);
            let right = pick == answer;
            if right {
                player.points += 1;
                self.scorers.push(*seat);
            }
            events.push(json!({ "kind": "buzz", "seat": seat, "right": right }));
        }

        let everyone_guessed = self.players.iter().all(|p| p.guess.is_some());
        if !self.scorers.is_empty() || everyone_guessed || self.clock >= ANSWER_SECONDS {
            self.reveal(events);
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() || self.stage != Stage::Asking {
            return Input::default();
        }
        let round = self.round;
        let answer = self.questions[round].answer;
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.players[index].guess.is_some() {
            return Input::default();
        }
        let plan = match self.players[index].plan {
            Some(plan) if plan.round == round => plan,
            _ => {
                let skill = self.players[index].skill.clamp(0.0, 1.0);
                // Everyone needs a moment to read the question; a sharper bot
                // is quicker and more often right, but never instant or perfect.
                let at = 1.5 + (1.0 - skill) * 3.5 + self.rng.random_range(0.0..2.5);
                let knows = self.rng.random_bool((0.35 + 0.6 * skill) as f64);
                let choice = if knows {
                    answer
                } else {
                    (answer + self.rng.random_range(1..4u8)) % 4
                };
                let plan = Plan { round, at, choice };
                self.players[index].plan = Some(plan);
                plan
            }
        };
        if self.clock < plan.at {
            return Input::default();
        }
        Input {
            fire: true,
            yaw: plan.choice as f32,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let question = &self.questions[self.round];
        let revealing = self.stage == Stage::Reveal;
        json!({
            "game": KEY,
            "time": if revealing { ANSWER_SECONDS } else { self.clock.min(ANSWER_SECONDS) },
            "limit": ANSWER_SECONDS,
            "round": self.round,
            "rounds": self.questions.len(),
            "stage": if revealing { "reveal" } else { "asking" },
            "question": question.text,
            "options": question.options,
            // Held back until the reveal so it is not on the wire while it
            // can still be picked.
            "answer": revealing.then_some(question.answer),
            "scorers": self.scorers,
            "players": self
                .players
                .iter()
                .map(|player| json!({
                    "seat": player.seat,
                    "points": player.points,
                    "state": self.state(player),
                    "choice": if revealing { player.guess } else { None },
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_a_wider_stage() {
        let four = BuzzerRound::new(&players(4), 1);
        assert_eq!(four.setup()["stage"], json!(STAGE_RADIUS), "four play it as built");
        let eight = BuzzerRound::new(&players(8), 1);
        let stage = eight.setup()["stage"].as_f64().unwrap() as f32;
        assert!((stage / STAGE_RADIUS - 2f32.sqrt()).abs() < 1e-4, "eight get the room of two fours");
        // The client sets the lecterns 2.7 apart, each 1.7 wide and reaching
        // 1.1 forward, so the row of eight has to fit inside that.
        let end = (3.5f32 * 2.7 + 0.85).hypot(1.1);
        assert!(end > STAGE_RADIUS && end < stage, "eight would hang off the old stage but not this one");
        assert_eq!(eight.setup()["rounds"], json!(QUESTIONS), "and the quiz is as long");
    }
}
