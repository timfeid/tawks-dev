//! Mugshot: a witness describes the suspect, everyone builds a face from their
//! own random kit of parts against the clock, then the lineup votes for the
//! best match. A face scores two points for every vote it gets and one for
//! every described trait it shows.
//!
//! Every action arrives as a press of fire with a code in `yaw`: a part pick
//! is `PART_CODE + slot * 16 + part`, a vote is `VOTE_CODE + seat`, and
//! `LOCK_CODE` / `UNLOCK_CODE` hand the face in or take it back. A pick names
//! the part outright rather than a step, so a repeated or late press is
//! harmless.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill};

pub const KEY: &str = "mugshot";
pub const WITNESS_SECONDS: f32 = 5.0;
pub const BUILD_SECONDS: f32 = 40.0;
pub const VOTE_SECONDS: f32 = 15.0;
pub const REVEAL_SECONDS: f32 = 9.0;
pub const TRAITS: usize = 4;
pub const VOTE_POINTS: u32 = 2;

pub const LOCK_CODE: u32 = 1;
pub const UNLOCK_CODE: u32 = 2;
pub const PART_CODE: u32 = 100;
pub const VOTE_CODE: u32 = 300;

pub const SLOTS: usize = 10;
pub const SLOT_NAMES: [&str; SLOTS] = [
    "Head", "Skin", "Hair", "Hat", "Brows", "Eyes", "Ears", "Nose", "Mouth", "Extra",
];
pub const HEAD: usize = 0;
pub const SKIN: usize = 1;
pub const HAIR: usize = 2;
pub const HAT: usize = 3;
pub const BROWS: usize = 4;
pub const EYES: usize = 5;
pub const EARS: usize = 6;
pub const NOSE: usize = 7;
pub const MOUTH: usize = 8;
pub const EXTRA: usize = 9;

pub struct Part {
    pub key: &'static str,
    pub name: &'static str,
    pub tags: &'static [&'static str],
}

const fn part(key: &'static str, name: &'static str, tags: &'static [&'static str]) -> Part {
    Part { key, name, tags }
}

pub const PARTS: [&[Part]; SLOTS] = [
    &[
        part("round", "Round", &["round_face"]),
        part("long", "Long", &["long_face"]),
        part("square", "Boxy", &["square_face"]),
        part("wide", "Wide", &["wide_face"]),
        part("pear", "Pear", &["pear_head"]),
    ],
    &[
        part("lime", "Lime", &["green"]),
        part("sky", "Sky", &["blue"]),
        part("grape", "Grape", &["purple"]),
        part("tangerine", "Tangerine", &["orange"]),
        part("lemon", "Lemon", &["yellow"]),
        part("stone", "Stone", &["grey"]),
        part("rose", "Rose", &["pink"]),
    ],
    &[
        part("black", "Black", &["dark_hair"]),
        part("ginger", "Ginger", &["red_hair"]),
        part("blond", "Blond", &["blond_hair"]),
        part("snow", "Snow", &["white_hair"]),
        part("teal", "Teal", &["blue_hair"]),
        part("bubblegum", "Bubblegum", &["pink_hair"]),
    ],
    &[
        part("none", "Bald", &["bald"]),
        part("tiny", "Tiny hat", &["tiny_hat"]),
        part("top", "Top hat", &["top_hat"]),
        part("cowboy", "Cowboy hat", &["cowboy_hat"]),
        part("beanie", "Beanie", &["beanie"]),
        part("crown", "Crown", &["crown"]),
        part("propeller", "Propeller cap", &["propeller"]),
        part("mohawk", "Mohawk", &["mohawk"]),
        part("messy", "Messy hair", &["messy_hair"]),
    ],
    &[
        part("plain", "Plain", &[]),
        part("none", "None", &["no_brows"]),
        part("unibrow", "Unibrow", &["one_eyebrow"]),
        part("angry", "Angry", &["angry"]),
        part("worried", "Worried", &["worried"]),
        part("bushy", "Bushy", &["bushy_brows"]),
        part("raised", "Raised", &["surprised"]),
    ],
    &[
        part("dots", "Beady", &["tiny_eyes"]),
        part("googly", "Googly", &["big_eyes"]),
        part("sleepy", "Sleepy", &["sleepy"]),
        part("cyclops", "Cyclops", &["one_eye"]),
        part("crossed", "Crossed", &["cross_eyed"]),
        part("shifty", "Shifty", &["shifty"]),
    ],
    &[
        part("plain", "Plain", &[]),
        part("big", "Big", &["big_ears"]),
        part("pointy", "Pointy", &["pointy_ears"]),
        part("lopsided", "Lopsided", &["lopsided_ears"]),
        part("none", "None", &["no_ears"]),
    ],
    &[
        part("button", "Button", &["small_nose"]),
        part("big", "Potato", &["big_nose"]),
        part("long", "Carrot", &["long_nose"]),
        part("clown", "Clown", &["red_nose"]),
        part("snout", "Snout", &["snout"]),
    ],
    &[
        part("smile", "Smile", &["happy"]),
        part("frown", "Frown", &["angry"]),
        part("grin", "Toothy grin", &["happy", "teeth"]),
        part("gasp", "Gasp", &["surprised"]),
        part("flat", "Flat", &["bored"]),
        part("tongue", "Tongue out", &["tongue"]),
    ],
    &[
        part("none", "Nothing", &[]),
        part("mustache", "Mustache", &["mustache"]),
        part("eyepatch", "Eyepatch", &["eyepatch"]),
        part("glasses", "Glasses", &["glasses"]),
        part("monocle", "Monocle", &["monocle"]),
        part("beard", "Beard", &["beard"]),
        part("scar", "Scar", &["scar"]),
        part("earring", "Earring", &["earring"]),
        part("bandage", "Bandage", &["bandage"]),
    ],
];

const MOODS: &[&str] = &["angry", "happy", "surprised", "worried", "sleepy", "bored"];

/// What a witness can say about a trait. The first phrase in each pair reads
/// in the middle of a list; both are lower case.
const PHRASES: &[(&str, [&str; 2])] = &[
    (
        "round_face",
        ["a big round face", "a face like a full moon"],
    ),
    (
        "long_face",
        ["a really long face", "a long face, like a horse"],
    ),
    (
        "square_face",
        ["a square head, like a brick", "a boxy head"],
    ),
    (
        "wide_face",
        ["a wide, squashed face", "a face as wide as a pancake"],
    ),
    (
        "pear_head",
        ["a head shaped like a pear", "a pear shaped head"],
    ),
    ("green", ["green skin, like pea soup", "a greenish face"]),
    ("blue", ["blue skin", "a face as blue as a popsicle"]),
    ("purple", ["purple skin, like a grape", "a purple face"]),
    ("orange", ["bright orange skin", "orange as a traffic cone"]),
    (
        "yellow",
        ["yellow skin, like a banana", "a lemon yellow face"],
    ),
    ("grey", ["grey skin, like a statue", "a stony grey face"]),
    ("pink", ["pink skin, like a ham", "a very pink face"]),
    ("dark_hair", ["jet black hair", "dark, dark hair"]),
    (
        "red_hair",
        ["fiery red hair", "hair as red as a fire truck"],
    ),
    ("blond_hair", ["blond hair", "golden hair"]),
    (
        "white_hair",
        ["snow white hair", "white hair, like a grandpa"],
    ),
    ("blue_hair", ["bright blue hair", "blue hair, dyed I think"]),
    ("pink_hair", ["bubblegum pink hair", "pink hair"]),
    ("bald", ["totally bald", "a shiny bald head"]),
    (
        "tiny_hat",
        ["a tiny hat", "a hat way too small for that head"],
    ),
    ("top_hat", ["a tall top hat", "a fancy top hat"]),
    ("cowboy_hat", ["a huge cowboy hat", "a cowboy hat"]),
    ("beanie", ["a woolly beanie with a bobble", "a beanie"]),
    ("crown", ["a crown, like actual royalty", "a gold crown"]),
    (
        "propeller",
        ["a spinning propeller cap", "a little propeller hat"],
    ),
    ("mohawk", ["a mohawk", "a tall spiky mohawk"]),
    (
        "messy_hair",
        ["messy hair everywhere", "bed hair, really messy"],
    ),
    ("no_brows", ["no eyebrows at all", "no eyebrows, weirdly"]),
    (
        "one_eyebrow",
        ["one big eyebrow", "just one eyebrow, all the way across"],
    ),
    (
        "bushy_brows",
        ["huge bushy eyebrows", "eyebrows like caterpillars"],
    ),
    ("tiny_eyes", ["tiny beady eyes", "little dot eyes"]),
    ("big_eyes", ["huge bulging eyes", "big googly eyes"]),
    (
        "one_eye",
        ["only one eye, right in the middle", "a single giant eye"],
    ),
    (
        "cross_eyed",
        ["crossed eyes", "eyes pointing at each other"],
    ),
    (
        "shifty",
        [
            "shifty eyes, looking sideways",
            "eyes that never looked at me",
        ],
    ),
    ("big_ears", ["big ears", "ears like satellite dishes"]),
    ("pointy_ears", ["pointy ears", "ears pointed like an elf"]),
    (
        "lopsided_ears",
        ["one ear way bigger than the other", "lopsided ears"],
    ),
    ("no_ears", ["no ears at all", "no ears, I swear"]),
    ("small_nose", ["a tiny button nose", "barely any nose"]),
    ("big_nose", ["a huge round nose", "a big potato nose"]),
    ("long_nose", ["a long pointy nose", "a nose like a carrot"]),
    ("red_nose", ["a red clown nose", "a bright red nose"]),
    ("snout", ["a pig snout", "a nose like a pig"]),
    ("teeth", ["lots of teeth", "a big toothy grin"]),
    (
        "tongue",
        ["tongue sticking out", "tongue out the whole time"],
    ),
    ("mustache", ["a big mustache", "a curly mustache"]),
    ("eyepatch", ["an eyepatch", "an eyepatch, like a pirate"]),
    ("glasses", ["round glasses", "big glasses"]),
    ("monocle", ["a monocle", "a fancy monocle"]),
    ("beard", ["a big beard", "a bushy beard"]),
    ("scar", ["a scar on the cheek", "a nasty scar"]),
    ("earring", ["a gold earring", "one shiny earring"]),
    (
        "bandage",
        ["a bandage on the forehead", "a plaster on the forehead"],
    ),
    (
        "angry",
        ["really angry", "furious, like someone cut in line"],
    ),
    (
        "happy",
        ["smiling way too much", "grinning like they won something"],
    ),
    ("surprised", ["totally shocked", "a surprised look"]),
    ("worried", ["worried, very worried", "a nervous look"]),
    ("sleepy", ["sleepy eyes, half closed", "half asleep"]),
    ("bored", ["bored out of their mind", "a flat, bored mouth"]),
];

const CRIMES: &[&str] = &[
    "Someone stole the mayor's rubber duck.",
    "The bake sale trophy is gone.",
    "A whole wheel of cheese vanished from the deli.",
    "Somebody painted a mustache on the town statue.",
    "The zoo's only penguin was taken for a joyride.",
    "Every left sock at the laundromat is missing.",
    "Someone swapped the museum dinosaur for a cardboard one.",
    "The bowling alley's lucky pin was snatched.",
    "Someone ate the display cake at the wedding fair.",
    "The prize pumpkin was rolled out of the county fair.",
    "The library's oldest book was returned. Chewed.",
    "Somebody let all the air out of the bouncy castle.",
];

/// `^n` is trait n starting a sentence, `%n` is trait n inside one.
const STATEMENTS: &[&str] = &[
    "It happened so fast. ^0, %1, %2. And %3, I think.",
    "I only saw them for a second. ^0. ^1. ^2. Oh, and %3.",
    "Write this down. ^0, %1, %2 and %3. I will never forget that face.",
    "Short version? ^0, %1, %2. Also %3.",
    "Okay, okay. ^0. Definitely %1. ^2. And %3, for sure.",
    "Picture this: %0, %1, %2. Plus %3. Ugh, I can still see it.",
];

pub fn phrases(tag: &str) -> Option<[&'static str; 2]> {
    PHRASES.iter().find(|(t, _)| *t == tag).map(|(_, p)| *p)
}

pub fn part_has(slot: usize, part: u8, tag: &str) -> bool {
    PARTS
        .get(slot)
        .and_then(|parts| parts.get(part as usize))
        .is_some_and(|p| p.tags.contains(&tag))
}

/// Which of the described traits a face shows.
pub fn matches(face: &[u8; SLOTS], traits: &[&str]) -> Vec<bool> {
    traits
        .iter()
        .map(|tag| (0..SLOTS).any(|slot| part_has(slot, face[slot], tag)))
        .collect()
}

fn satisfiers(tag: &str) -> Vec<(usize, u8)> {
    (0..SLOTS)
        .flat_map(|slot| {
            PARTS[slot]
                .iter()
                .enumerate()
                .filter(move |(_, p)| p.tags.contains(&tag))
                .map(move |(i, _)| (slot, i as u8))
        })
        .collect()
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub struct Statement {
    pub crime: &'static str,
    pub text: String,
    pub traits: Vec<&'static str>,
    pub said: Vec<&'static str>,
    /// One part per trait that shows it, each in a slot of its own, so the
    /// whole description can be built at once.
    pub answer: Vec<(usize, u8)>,
}

impl Statement {
    pub fn draw(rng: &mut StdRng) -> Statement {
        let mut tags: Vec<&'static str> = PHRASES.iter().map(|(t, _)| *t).collect();
        tags.shuffle(rng);
        let mut traits = Vec::new();
        let mut answer: Vec<(usize, u8)> = Vec::new();
        for tag in tags {
            if traits.len() == TRAITS {
                break;
            }
            if MOODS.contains(&tag) && traits.iter().any(|t| MOODS.contains(t)) {
                continue;
            }
            let mut options: Vec<(usize, u8)> = satisfiers(tag)
                .into_iter()
                .filter(|(slot, _)| answer.iter().all(|(used, _)| used != slot))
                .collect();
            options.shuffle(rng);
            if let Some(pick) = options.first() {
                traits.push(tag);
                answer.push(*pick);
            }
        }
        let said: Vec<&'static str> = traits
            .iter()
            .map(|tag| phrases(tag).map_or(*tag, |p| p[rng.random_range(0..2)]))
            .collect();
        let template = STATEMENTS[rng.random_range(0..STATEMENTS.len())];
        let mut text = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            match (c, chars.peek().and_then(|d| d.to_digit(10))) {
                ('^' | '%', Some(index)) => {
                    chars.next();
                    let phrase = said[index as usize];
                    if c == '^' {
                        text.push_str(&capitalized(phrase));
                    } else {
                        text.push_str(phrase);
                    }
                }
                _ => text.push(c),
            }
        }
        Statement {
            crime: CRIMES[rng.random_range(0..CRIMES.len())],
            text,
            traits,
            said,
            answer,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Witness,
    Build,
    Lineup,
    Reveal,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Witness => "witness",
            Stage::Build => "build",
            Stage::Lineup => "lineup",
            Stage::Reveal => "reveal",
        }
    }

    fn limit(self) -> f32 {
        match self {
            Stage::Witness => WITNESS_SECONDS,
            Stage::Build => BUILD_SECONDS,
            Stage::Lineup => VOTE_SECONDS,
            Stage::Reveal => REVEAL_SECONDS,
        }
    }
}

/// How a bot means to build its face: the picks it will dial in, in order,
/// and when it next moves.
#[derive(Default)]
struct BotPlan {
    made: bool,
    picks: Vec<(usize, u8)>,
    /// A wrong part it will pass through on the way, like a wheel spun one
    /// notch too far.
    overshoot: Option<(usize, u8)>,
    next_at: f32,
    lock: bool,
    vote_at: Option<f32>,
    ballot: Option<u8>,
}

pub struct Suspect {
    pub seat: u8,
    skill: f32,
    pub kit: [Vec<u8>; SLOTS],
    pub face: [u8; SLOTS],
    pub locked: bool,
    pub vote: Option<u8>,
    pub votes: u32,
    pub matched: Vec<bool>,
    pub points: u32,
    edits: u32,
    plan: BotPlan,
}

impl Suspect {
    pub fn witness(&self) -> u32 {
        self.matched.iter().filter(|m| **m).count() as u32
    }
}

pub struct Mugshot {
    pub statement: Statement,
    pub suspects: Vec<Suspect>,
    pub stage: Stage,
    clock: f32,
    winners: Vec<u8>,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

pub enum Action {
    Part { slot: usize, part: u8 },
    Lock,
    Unlock,
    Vote(u8),
}

pub fn code(action: Action) -> f32 {
    (match action {
        Action::Part { slot, part } => PART_CODE + slot as u32 * 16 + part as u32,
        Action::Lock => LOCK_CODE,
        Action::Unlock => UNLOCK_CODE,
        Action::Vote(seat) => VOTE_CODE + seat as u32,
    }) as f32
}

pub fn decode(input: &Input) -> Option<Action> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let rounded = input.yaw.round();
    if (input.yaw - rounded).abs() > 0.01 || !(0.0..1000.0).contains(&rounded) {
        return None;
    }
    let value = rounded as u32;
    match value {
        LOCK_CODE => Some(Action::Lock),
        UNLOCK_CODE => Some(Action::Unlock),
        v if (PART_CODE..PART_CODE + SLOTS as u32 * 16).contains(&v) => Some(Action::Part {
            slot: ((v - PART_CODE) / 16) as usize,
            part: ((v - PART_CODE) % 16) as u8,
        }),
        v if (VOTE_CODE..VOTE_CODE + 64).contains(&v) => Some(Action::Vote((v - VOTE_CODE) as u8)),
        _ => None,
    }
}

/// A part a few steps along the kit from `from`, the way a wheel would land.
pub fn cycle(kit: &[u8], from: u8, steps: i32) -> u8 {
    if kit.is_empty() {
        return from;
    }
    let at = kit.iter().position(|p| *p == from).unwrap_or(0) as i32;
    kit[(at + steps).rem_euclid(kit.len() as i32) as usize]
}

impl Mugshot {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let statement = Statement::draw(&mut rng);
        let suspects = contestants
            .iter()
            .map(|one| {
                let kit: [Vec<u8>; SLOTS] = std::array::from_fn(|slot| {
                    let all = PARTS[slot].len();
                    let size = rng.random_range(4..=5).min(all);
                    let mut pool: Vec<u8> = (0..all as u8).collect();
                    pool.shuffle(&mut rng);
                    let mut kit: Vec<u8> = statement
                        .answer
                        .iter()
                        .filter(|(s, _)| *s == slot)
                        .map(|(_, p)| *p)
                        .collect();
                    for part in pool {
                        if kit.len() >= size {
                            break;
                        }
                        if !kit.contains(&part) {
                            kit.push(part);
                        }
                    }
                    kit.shuffle(&mut rng);
                    kit
                });
                let face =
                    std::array::from_fn(|slot| kit[slot][rng.random_range(0..kit[slot].len())]);
                Suspect {
                    seat: one.seat,
                    skill: if one.is_bot {
                        one.skill.clamp(0.0, 1.0)
                    } else {
                        1.0
                    },
                    kit,
                    face,
                    locked: false,
                    vote: None,
                    votes: 0,
                    matched: Vec::new(),
                    points: 0,
                    edits: 0,
                    plan: BotPlan::default(),
                }
            })
            .collect();
        Mugshot {
            statement,
            suspects,
            stage: Stage::Witness,
            clock: 0.0,
            winners: Vec::new(),
            rng,
            placed: None,
        }
    }

    pub fn suspect(&self, seat: u8) -> Option<&Suspect> {
        self.suspects.iter().find(|s| s.seat == seat)
    }

    fn enter(&mut self, stage: Stage, events: &mut Vec<Value>) {
        self.stage = stage;
        self.clock = 0.0;
        events.push(json!({ "kind": "stage", "stage": stage.name() }));
        if stage == Stage::Reveal {
            self.score(events);
        }
    }

    fn score(&mut self, events: &mut Vec<Value>) {
        let ballots: Vec<u8> = self.suspects.iter().filter_map(|s| s.vote).collect();
        let traits = self.statement.traits.clone();
        for suspect in self.suspects.iter_mut() {
            suspect.matched = matches(&suspect.face, &traits);
            suspect.votes = ballots.iter().filter(|b| **b == suspect.seat).count() as u32;
            suspect.points = suspect.votes * VOTE_POINTS + suspect.witness();
        }
        let best = self.suspects.iter().map(|s| s.points).max().unwrap_or(0);
        self.winners = self
            .suspects
            .iter()
            .filter(|s| s.points == best)
            .map(|s| s.seat)
            .collect();
        events.push(json!({ "kind": "reveal", "winners": self.winners }));
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, u32)> = self.suspects.iter().map(|s| (s.seat, s.points)).collect();
        order.sort_by(|a, b| b.1.cmp(&a.1));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, points) in order {
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if previous == points => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(points);
        }
        self.placed = Some(groups);
    }

    fn apply(&mut self, seat: u8, action: Action, events: &mut Vec<Value>) {
        let stage = self.stage;
        let seats: Vec<u8> = self.suspects.iter().map(|s| s.seat).collect();
        let Some(suspect) = self.suspects.iter_mut().find(|s| s.seat == seat) else {
            return;
        };
        match (stage, action) {
            (Stage::Build, Action::Part { slot, part }) => {
                if suspect.locked || slot >= SLOTS || !suspect.kit[slot].contains(&part) {
                    return;
                }
                if suspect.face[slot] != part {
                    suspect.face[slot] = part;
                    suspect.edits += 1;
                    events.push(json!({ "kind": "part", "seat": seat, "slot": slot }));
                }
            }
            (Stage::Build, Action::Lock) if !suspect.locked => {
                suspect.locked = true;
                events.push(json!({ "kind": "lock", "seat": seat }));
            }
            (Stage::Build, Action::Unlock) if suspect.locked => {
                suspect.locked = false;
                events.push(json!({ "kind": "unlock", "seat": seat }));
            }
            (Stage::Lineup, Action::Vote(target)) => {
                if target == seat || !seats.contains(&target) || suspect.vote == Some(target) {
                    return;
                }
                let first = suspect.vote.is_none();
                suspect.vote = Some(target);
                events.push(json!({ "kind": "vote", "seat": seat, "first": first }));
            }
            _ => {}
        }
    }

    fn plan_build(&mut self, index: usize) {
        let skill = Skill(self.suspects[index].skill);
        let recall = 0.45 + 0.5 * skill.level() as f64;
        let mut picks: Vec<(usize, u8)> = Vec::new();
        for (t, tag) in self.statement.traits.clone().into_iter().enumerate() {
            let suspect = &self.suspects[index];
            let have: Vec<(usize, u8)> = (0..SLOTS)
                .flat_map(|slot| suspect.kit[slot].iter().map(move |p| (slot, *p)))
                .filter(|(slot, p)| part_has(*slot, *p, tag))
                .collect();
            if self.rng.random_bool(recall) && !have.is_empty() {
                picks.push(have[self.rng.random_range(0..have.len())]);
            } else if self.rng.random_bool(0.7) {
                // Misremembered: the right feature, the wrong part.
                let slot = self.statement.answer[t].0;
                let kit = &suspect.kit[slot];
                picks.push((slot, kit[self.rng.random_range(0..kit.len())]));
            }
        }
        let flair = self.rng.random_range(0..=2);
        for _ in 0..flair {
            let slot = self.rng.random_range(0..SLOTS);
            if picks.iter().any(|(s, _)| *s == slot) {
                continue;
            }
            let kit = &self.suspects[index].kit[slot];
            let part = kit[self.rng.random_range(0..kit.len())];
            let at = self.rng.random_range(0..=picks.len());
            picks.insert(at, (slot, part));
        }
        let start = 1.2 + skill.reaction() * 1.5 + self.rng.random_range(0.0..1.5);
        let lock = self.rng.random_bool(0.55 + 0.35 * skill.level() as f64);
        let plan = &mut self.suspects[index].plan;
        plan.made = true;
        plan.picks = picks;
        plan.next_at = start;
        plan.lock = lock;
    }

    fn bot_build(&mut self, index: usize) -> Input {
        if !self.suspects[index].plan.made {
            self.plan_build(index);
        }
        if self.suspects[index].locked || self.clock < self.suspects[index].plan.next_at {
            return Input::default();
        }
        let skill = Skill(self.suspects[index].skill);
        let step_gap = 0.22 + (1.0 - skill.level()) * 0.4 + self.rng.random_range(0.0..0.25);
        let think_gap = skill.reaction() + self.rng.random_range(0.3..1.6);
        let suspect = &self.suspects[index];
        if let Some((slot, part)) = suspect.plan.overshoot {
            let plan = &mut self.suspects[index].plan;
            plan.overshoot = None;
            plan.next_at = self.clock + step_gap + 0.25;
            return fire(code(Action::Part { slot, part }));
        }
        let Some(&(slot, target)) = suspect.plan.picks.first() else {
            let lock = suspect.plan.lock;
            self.suspects[index].plan.next_at = f32::INFINITY;
            return if lock {
                fire(code(Action::Lock))
            } else {
                Input::default()
            };
        };
        let kit = &suspect.kit[slot];
        let at = kit
            .iter()
            .position(|p| *p == suspect.face[slot])
            .unwrap_or(0) as i32;
        let goal = kit.iter().position(|p| *p == target).unwrap_or(0) as i32;
        let len = kit.len() as i32;
        let forward = (goal - at).rem_euclid(len);
        if forward == 0 {
            let plan = &mut self.suspects[index].plan;
            plan.picks.remove(0);
            plan.next_at = self.clock + think_gap;
            return Input::default();
        }
        let direction = if forward <= len - forward { 1 } else { -1 };
        let next = cycle(kit, suspect.face[slot], direction);
        if next == target && self.rng.random_bool(skill.hesitation() * 0.35) {
            let past = cycle(kit, target, direction);
            if past != suspect.face[slot] {
                let plan = &mut self.suspects[index].plan;
                plan.overshoot = Some((slot, target));
                plan.next_at = self.clock + step_gap;
                return fire(code(Action::Part { slot, part: past }));
            }
        }
        self.suspects[index].plan.next_at = self.clock + step_gap;
        fire(code(Action::Part { slot, part: next }))
    }

    fn bot_vote(&mut self, index: usize) -> Input {
        let skill = Skill(self.suspects[index].skill);
        if self.suspects[index].plan.vote_at.is_none() {
            let at = 2.0 + (1.0 - skill.level()) * 3.0 + self.rng.random_range(0.0..4.0);
            let traits = self.statement.traits.clone();
            let seat = self.suspects[index].seat;
            let wobble = 0.5 + (1.0 - skill.level()) * 1.8;
            let mut best: Option<(f32, u8)> = None;
            for other in self.suspects.iter().filter(|s| s.seat != seat) {
                let shown = matches(&other.face, &traits).iter().filter(|m| **m).count() as f32;
                let taste = self.rng.random_range(-wobble..wobble);
                let effort = (other.edits.min(12) as f32) * 0.03;
                let score = shown + taste + effort;
                if best.is_none_or(|(top, _)| score > top) {
                    best = Some((score, other.seat));
                }
            }
            let plan = &mut self.suspects[index].plan;
            plan.vote_at = Some(at);
            plan.ballot = best.map(|(_, seat)| seat);
        }
        let plan = &self.suspects[index].plan;
        match (plan.vote_at, plan.ballot, self.suspects[index].vote) {
            (Some(at), Some(seat), None) if self.clock >= at => fire(code(Action::Vote(seat))),
            _ => Input::default(),
        }
    }
}

fn fire(yaw: f32) -> Input {
    Input {
        fire: true,
        yaw,
        ..Default::default()
    }
}

impl Bout for Mugshot {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "radius": 0.01,
            "slots": (0..SLOTS)
                .map(|slot| json!({
                    "name": SLOT_NAMES[slot],
                    "parts": PARTS[slot].iter().map(|p| json!({ "key": p.key, "name": p.name })).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        for (seat, input) in inputs {
            if let Some(action) = decode(input) {
                self.apply(*seat, action, events);
            }
        }
        let limit = self.stage.limit();
        match self.stage {
            Stage::Witness if self.clock >= limit => self.enter(Stage::Build, events),
            Stage::Build if self.clock >= limit || self.suspects.iter().all(|s| s.locked) => {
                self.enter(Stage::Lineup, events)
            }
            Stage::Lineup
                if self.clock >= limit || self.suspects.iter().all(|s| s.vote.is_some()) =>
            {
                self.enter(Stage::Reveal, events)
            }
            Stage::Reveal if self.clock >= limit => self.settle(),
            _ => {}
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() {
            return Input::default();
        }
        let Some(index) = self.suspects.iter().position(|s| s.seat == seat) else {
            return Input::default();
        };
        match self.stage {
            Stage::Build => self.bot_build(index),
            Stage::Lineup => self.bot_vote(index),
            _ => Input::default(),
        }
    }

    fn snapshot(&self) -> Value {
        let revealing = self.stage == Stage::Reveal;
        json!({
            "game": KEY,
            "time": self.clock.min(self.stage.limit()),
            "limit": self.stage.limit(),
            "stage": self.stage.name(),
            "crime": self.statement.crime,
            "statement": self.statement.text,
            "traits": self.statement.traits.iter().zip(&self.statement.said)
                .map(|(tag, said)| json!({ "tag": tag, "said": said }))
                .collect::<Vec<_>>(),
            "winners": self.winners,
            "suspects": self.suspects.iter().map(|s| json!({
                "seat": s.seat,
                "kit": s.kit,
                "face": s.face,
                "locked": s.locked,
                "voted": s.vote.is_some(),
                "vote": if revealing { s.vote } else { None },
                "votes": revealing.then_some(s.votes),
                "matched": revealing.then(|| s.matched.clone()),
                "points": revealing.then_some(s.points),
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
    use crate::bout::TICK_DT;

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn press(action: Action) -> Input {
        fire(code(action))
    }

    fn run(game: &mut Mugshot, seconds: f32, events: &mut Vec<Value>) {
        for _ in 0..(seconds / TICK_DT).ceil() as usize {
            if game.placements().is_some() {
                return;
            }
            game.step(&[], TICK_DT, events);
        }
    }

    fn to_build(game: &mut Mugshot, events: &mut Vec<Value>) {
        run(game, WITNESS_SECONDS + 0.05, events);
        assert_eq!(game.stage, Stage::Build);
    }

    #[test]
    fn every_statement_can_be_built_from_every_kit() {
        for seed in 0..400 {
            let game = Mugshot::new(&people(8), seed);
            let statement = &game.statement;
            assert_eq!(statement.traits.len(), TRAITS);
            assert!(
                statement
                    .traits
                    .iter()
                    .filter(|t| MOODS.contains(t))
                    .count()
                    <= 1
            );
            let mut slots: Vec<usize> = statement.answer.iter().map(|(s, _)| *s).collect();
            slots.sort();
            slots.dedup();
            assert_eq!(slots.len(), TRAITS, "each trait has a slot of its own");
            for suspect in &game.suspects {
                let mut face = suspect.face;
                for (slot, part) in &statement.answer {
                    assert!(
                        suspect.kit[*slot].contains(part),
                        "seed {seed}: kit misses a described part"
                    );
                    face[*slot] = *part;
                }
                assert!(matches(&face, &statement.traits).iter().all(|m| *m));
                for slot in 0..SLOTS {
                    assert!(suspect.kit[slot].len() >= 4 && suspect.kit[slot].len() <= 5);
                    assert!(suspect.kit[slot].contains(&suspect.face[slot]));
                }
            }
            for said in &statement.said {
                assert!(
                    statement.text.to_lowercase().contains(&said.to_lowercase()),
                    "{}",
                    statement.text
                );
            }
            assert!(!statement.text.contains('%') && !statement.text.contains('^'));
            assert!(statement.text.chars().next().unwrap().is_uppercase());
        }
    }

    #[test]
    fn the_copy_is_plain_and_every_tag_has_words() {
        for parts in PARTS {
            for part in parts {
                for tag in part.tags {
                    assert!(phrases(tag).is_some(), "{tag} has no phrases");
                }
            }
        }
        let all: Vec<&str> = PHRASES
            .iter()
            .flat_map(|(_, p)| p.iter().copied())
            .chain(CRIMES.iter().copied())
            .chain(STATEMENTS.iter().copied())
            .collect();
        for text in all {
            assert!(text.is_ascii(), "{text} is plain ASCII copy");
            assert!(!text.contains('\u{2014}'));
        }
        let one = Mugshot::new(&people(2), 1).statement.text;
        let other = Mugshot::new(&people(2), 2).statement.text;
        assert_ne!(one, other, "the seed picks the witness");
    }

    #[test]
    fn codes_round_trip_and_junk_is_ignored() {
        for slot in 0..SLOTS {
            for part in 0..PARTS[slot].len() as u8 {
                match decode(&press(Action::Part { slot, part })) {
                    Some(Action::Part { slot: s, part: p }) => assert_eq!((s, p), (slot, part)),
                    _ => panic!("part code lost"),
                }
            }
        }
        assert!(matches!(
            decode(&press(Action::Vote(7))),
            Some(Action::Vote(7))
        ));
        assert!(matches!(decode(&press(Action::Lock)), Some(Action::Lock)));
        for yaw in [
            0.0,
            3.14159,
            100.5,
            -101.0,
            f32::NAN,
            f32::INFINITY,
            99.0,
            999.0,
        ] {
            assert!(decode(&fire(yaw)).is_none(), "{yaw} is not an action");
        }
        let quiet = Input {
            yaw: 101.0,
            ..Default::default()
        };
        assert!(decode(&quiet).is_none(), "no fire, no action");
    }

    #[test]
    fn parts_outside_your_kit_or_stage_are_refused() {
        let mut game = Mugshot::new(&people(2), 3);
        let mut events = Vec::new();
        let kit = game.suspects[0].kit[HAT].clone();
        let foreign = (0..PARTS[HAT].len() as u8)
            .find(|p| !kit.contains(p))
            .unwrap();
        let mine = cycle(&kit, game.suspects[0].face[HAT], 1);
        game.step(
            &[(
                0,
                press(Action::Part {
                    slot: HAT,
                    part: mine,
                }),
            )],
            TICK_DT,
            &mut events,
        );
        assert_ne!(
            game.suspects[0].face[HAT], mine,
            "nobody builds while the witness talks"
        );
        to_build(&mut game, &mut events);
        game.step(
            &[(
                0,
                press(Action::Part {
                    slot: HAT,
                    part: foreign,
                }),
            )],
            TICK_DT,
            &mut events,
        );
        assert_ne!(
            game.suspects[0].face[HAT], foreign,
            "a part from outside the kit"
        );
        game.step(
            &[(
                0,
                press(Action::Part {
                    slot: HAT,
                    part: mine,
                }),
            )],
            TICK_DT,
            &mut events,
        );
        assert_eq!(game.suspects[0].face[HAT], mine);
        game.step(
            &[(0, fire(PART_CODE as f32 + SLOTS as f32 * 16.0))],
            TICK_DT,
            &mut events,
        );
        game.step(&[(0, press(Action::Lock))], TICK_DT, &mut events);
        let back = cycle(&kit, mine, 1);
        game.step(
            &[(
                0,
                press(Action::Part {
                    slot: HAT,
                    part: back,
                }),
            )],
            TICK_DT,
            &mut events,
        );
        assert_eq!(
            game.suspects[0].face[HAT], mine,
            "a handed in face is frozen"
        );
        game.step(&[(0, press(Action::Unlock))], TICK_DT, &mut events);
        game.step(
            &[(
                0,
                press(Action::Part {
                    slot: HAT,
                    part: back,
                }),
            )],
            TICK_DT,
            &mut events,
        );
        assert_eq!(
            game.suspects[0].face[HAT], back,
            "taking it back reopens it"
        );
        game.step(&[(9, press(Action::Lock))], TICK_DT, &mut events);
        assert!(
            events.iter().all(|e| e["seat"] != 9),
            "a stranger's press does nothing"
        );
    }

    #[test]
    fn everyone_locking_in_skips_to_the_lineup_and_votes_skip_to_the_reveal() {
        let mut game = Mugshot::new(&people(3), 4);
        let mut events = Vec::new();
        to_build(&mut game, &mut events);
        game.step(
            &[(0, press(Action::Lock)), (1, press(Action::Lock))],
            TICK_DT,
            &mut events,
        );
        assert_eq!(game.stage, Stage::Build);
        game.step(&[(2, press(Action::Lock))], TICK_DT, &mut events);
        assert_eq!(game.stage, Stage::Lineup);
        game.step(&[(0, press(Action::Vote(0)))], TICK_DT, &mut events);
        assert_eq!(game.suspects[0].vote, None, "you cannot vote for yourself");
        game.step(&[(0, press(Action::Vote(6)))], TICK_DT, &mut events);
        assert_eq!(game.suspects[0].vote, None, "nor for an empty seat");
        assert!(game.snapshot()["suspects"][0]["vote"].is_null());
        game.step(
            &[(0, press(Action::Vote(1))), (1, press(Action::Vote(2)))],
            TICK_DT,
            &mut events,
        );
        game.step(&[(0, press(Action::Vote(2)))], TICK_DT, &mut events);
        assert_eq!(
            game.suspects[0].vote,
            Some(2),
            "a vote can change until the lineup closes"
        );
        assert_eq!(game.snapshot()["suspects"][0]["voted"], true);
        assert!(
            game.snapshot()["suspects"][0]["vote"].is_null(),
            "ballots stay secret"
        );
        game.step(&[(2, press(Action::Vote(0)))], TICK_DT, &mut events);
        assert_eq!(game.stage, Stage::Reveal);
        assert_eq!(game.snapshot()["suspects"][0]["vote"], 2);
        assert_eq!(game.suspect(2).unwrap().votes, 2);
    }

    #[test]
    fn votes_and_witness_matches_decide_the_places() {
        let mut game = Mugshot::new(&people(3), 5);
        let mut events = Vec::new();
        to_build(&mut game, &mut events);
        let answer = game.statement.answer.clone();
        let traits = game.statement.traits.clone();
        // Seat 0 builds the description exactly; seat 1 is made to miss all of it.
        for (slot, part) in &answer {
            game.step(
                &[(
                    0,
                    press(Action::Part {
                        slot: *slot,
                        part: *part,
                    }),
                )],
                TICK_DT,
                &mut events,
            );
        }
        for slot in 0..SLOTS {
            let wrong = game.suspects[1].kit[slot]
                .iter()
                .copied()
                .find(|p| traits.iter().all(|t| !part_has(slot, *p, t)));
            if let Some(part) = wrong {
                game.step(
                    &[(1, press(Action::Part { slot, part }))],
                    TICK_DT,
                    &mut events,
                );
            }
        }
        run(&mut game, BUILD_SECONDS, &mut events);
        assert_eq!(game.stage, Stage::Lineup, "the clock closes the build");
        game.step(
            &[(1, press(Action::Vote(2))), (2, press(Action::Vote(1)))],
            TICK_DT,
            &mut events,
        );
        run(&mut game, VOTE_SECONDS, &mut events);
        assert_eq!(
            game.stage,
            Stage::Reveal,
            "the clock closes the lineup without seat 0's vote"
        );
        let perfect = game.suspect(0).unwrap();
        assert_eq!(perfect.witness(), TRAITS as u32);
        assert_eq!(perfect.points, TRAITS as u32);
        let missed = game.suspect(1).unwrap();
        assert_eq!(missed.points, missed.witness() + VOTE_POINTS);
        let snap = game.snapshot();
        assert_eq!(
            snap["suspects"][0]["matched"],
            json!([true, true, true, true])
        );
        assert!(game.placements().is_none(), "the reveal plays out first");
        run(&mut game, REVEAL_SECONDS + 0.1, &mut events);
        let placed = game.placements().unwrap();
        assert_eq!(placed[0], vec![0], "four traits beat one vote");
    }

    #[test]
    fn level_points_share_a_place() {
        let mut game = Mugshot::new(&people(2), 6);
        let mut events = Vec::new();
        to_build(&mut game, &mut events);
        game.suspects[1].face = game.suspects[0].face;
        game.step(
            &[(0, press(Action::Lock)), (1, press(Action::Lock))],
            TICK_DT,
            &mut events,
        );
        game.step(
            &[(0, press(Action::Vote(1))), (1, press(Action::Vote(0)))],
            TICK_DT,
            &mut events,
        );
        run(&mut game, REVEAL_SECONDS + 0.1, &mut events);
        assert_eq!(game.placements().unwrap(), vec![vec![0, 1]]);
        assert_eq!(game.snapshot()["winners"], json!([0, 1]));
    }

    #[test]
    fn a_leaver_keeps_their_face_in_the_lineup_and_the_clock_carries_on() {
        let seats = vec![
            Contestant::person(0),
            Contestant::bot(1, 0.5),
            Contestant::bot(2, 0.5),
        ];
        let mut game = Mugshot::new(&seats, 7);
        let mut events = Vec::new();
        let mut ticks = 0;
        while game.placements().is_none() && ticks < 60 * 200 {
            let inputs: Vec<(u8, Input)> = [1, 2]
                .iter()
                .map(|s| (*s, game.bot_input(*s, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            ticks += 1;
        }
        let placed = game.placements().expect("finishes without the leaver");
        assert_eq!(
            placed.iter().flatten().count(),
            3,
            "the leaver is still placed"
        );
        let total = (ticks as f32) * TICK_DT;
        assert!(total <= WITNESS_SECONDS + BUILD_SECONDS + VOTE_SECONDS + REVEAL_SECONDS + 0.5);
        assert!(game.suspect(0).unwrap().vote.is_none());
    }

    #[test]
    fn bots_take_human_time_and_never_vote_for_themselves() {
        let seats: Vec<Contestant> = (0..6).map(|s| Contestant::bot(s, 1.0)).collect();
        let mut game = Mugshot::new(&seats, 8);
        let mut events = Vec::new();
        let mut first_edit = None;
        let mut time = 0.0;
        while game.placements().is_none() {
            let inputs: Vec<(u8, Input)> =
                (0..6).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
            events.clear();
            game.step(&inputs, TICK_DT, &mut events);
            time += TICK_DT;
            if first_edit.is_none() && events.iter().any(|e| e["kind"] == "part") {
                first_edit = Some(time);
            }
        }
        assert!(
            first_edit.unwrap() > WITNESS_SECONDS + 1.0,
            "even a ruthless bot reads first"
        );
        for suspect in &game.suspects {
            assert_ne!(suspect.vote, Some(suspect.seat));
            assert!(suspect.vote.is_some());
        }
    }

    /// Bot-only bouts across skills and table sizes: they all finish, sharper
    /// bots match more of the description, and nobody is a lock to win.
    #[test]
    fn seeded_bot_simulation() {
        let mut matched_by_skill = Vec::new();
        for skill in [0.15f32, 0.4, 0.7, 1.0] {
            let (mut shown, mut faces, mut seconds, mut runs) = (0u32, 0u32, 0.0f32, 0u32);
            for seed in 0..120u64 {
                let count = 2 + (seed % 7) as u8;
                let seats: Vec<Contestant> =
                    (0..count).map(|s| Contestant::bot(s, skill)).collect();
                let mut game = Mugshot::new(&seats, seed * 31 + 1);
                let mut events = Vec::new();
                let mut ticks = 0u32;
                while game.placements().is_none() {
                    let inputs: Vec<(u8, Input)> = (0..count)
                        .map(|s| (s, game.bot_input(s, TICK_DT)))
                        .collect();
                    game.step(&inputs, TICK_DT, &mut events);
                    ticks += 1;
                    assert!(ticks < 60 * 120, "seed {seed} never finished");
                }
                for suspect in &game.suspects {
                    shown += suspect.witness();
                    faces += 1;
                }
                seconds += ticks as f32 * TICK_DT;
                runs += 1;
            }
            let rate = shown as f32 / (faces * TRAITS as u32) as f32;
            println!(
                "skill {skill}: {:.0}% of traits shown, {:.1}s a bout",
                rate * 100.0,
                seconds / runs as f32
            );
            matched_by_skill.push(rate);
        }
        assert!(
            matched_by_skill[0] < matched_by_skill[3],
            "{matched_by_skill:?}"
        );
        assert!(
            matched_by_skill[3] < 0.97,
            "even ruthless bots slip: {matched_by_skill:?}"
        );
        assert!(
            matched_by_skill[0] > 0.35,
            "relaxed bots still try: {matched_by_skill:?}"
        );

        let mut wins = [0u32; 4];
        for seed in 0..300u64 {
            let seats: Vec<Contestant> = [0.15f32, 0.4, 0.7, 1.0]
                .iter()
                .enumerate()
                .map(|(s, k)| Contestant::bot(s as u8, *k))
                .collect();
            let mut game = Mugshot::new(&seats, seed + 9000);
            let mut events = Vec::new();
            while game.placements().is_none() {
                let inputs: Vec<(u8, Input)> =
                    (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
                game.step(&inputs, TICK_DT, &mut events);
            }
            for seat in &game.placements().unwrap()[0] {
                wins[*seat as usize] += 1;
            }
        }
        println!("mixed table wins by skill (relaxed, steady, sharp, ruthless): {wins:?}");
        assert!(wins[3] > wins[0], "skill shows: {wins:?}");
        assert!(wins[0] > 15, "a relaxed bot still wins sometimes: {wins:?}");
    }
}
