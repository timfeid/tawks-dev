//! Blueprints: an everyday object turns up a bit at a time, first as an
//! extreme close-up in bare lines, then from a strange angle, then as a
//! blueprint, then in colour. Type what it is. The first right answer scores
//! most, the next ones less; a wrong guess locks you out for two seconds.
//! Several objects make a bout.
//!
//! A guess arrives as a press of fire with a vocabulary index in `yaw`, the
//! way Buzzer Round sends its option. The client turns typed text into that
//! index with [`resolve`], the same matcher the tests use, so typos and
//! synonyms land on the right word; `UNKNOWN` is a word that is not in the
//! vocabulary, and still costs the lockout. `pitch` at `PICKED` marks a pick
//! from the controller's multiple choice, which only counts once the choices
//! are open. Whether a guess is right is only ever decided here, and the
//! object's name stays off the wire until it is revealed.

use std::sync::OnceLock;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::blueprints_catalog::{CATALOG, Part};
use crate::bout::{Bout, Contestant, Input, Skill, round_form};

pub const KEY: &str = "blueprints";
pub const OBJECTS: usize = 5;
pub const SHOT_SECONDS: f32 = 4.0;
pub const LOOK_SECONDS: f32 = SHOT_SECONDS * 4.0;
/// Once someone has it, everyone else gets this long to catch up.
pub const AFTER_FIRST_SECONDS: f32 = 5.0;
pub const REVEAL_SECONDS: f32 = 3.5;
pub const LOCKOUT_SECONDS: f32 = 2.0;
/// When the controller's four choices open: the start of the blueprint shot.
pub const CHOICES_AT: f32 = SHOT_SECONDS * 2.0;
/// Points by the order of right answers; everyone after the last gets the last.
pub const POINTS: [u32; 4] = [5, 3, 2, 1];
pub const UNKNOWN: f32 = -1.0;
pub const PICKED: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Closeup,
    Angle,
    Blueprint,
    Full,
    Reveal,
}

impl Shot {
    pub fn name(self) -> &'static str {
        match self {
            Shot::Closeup => "closeup",
            Shot::Angle => "angle",
            Shot::Blueprint => "blueprint",
            Shot::Full => "full",
            Shot::Reveal => "reveal",
        }
    }

    fn at(clock: f32) -> Shot {
        match (clock / SHOT_SECONDS) as usize {
            0 => Shot::Closeup,
            1 => Shot::Angle,
            2 => Shot::Blueprint,
            _ => Shot::Full,
        }
    }
}

pub struct Term {
    pub word: &'static str,
    pub object: usize,
    plain: String,
}

/// Every word that names an object, in a fixed order shared by client and
/// server: a guess travels as its index.
pub fn vocabulary() -> &'static [Term] {
    static TERMS: OnceLock<Vec<Term>> = OnceLock::new();
    TERMS.get_or_init(|| {
        CATALOG
            .iter()
            .enumerate()
            .flat_map(|(object, entry)| {
                entry.terms.iter().map(move |word| Term {
                    word,
                    object,
                    plain: normalize(word),
                })
            })
            .collect()
    })
}

/// Lower case letters and digits, single spaces, and no leading article.
pub fn normalize(text: &str) -> String {
    let lowered: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    let mut words: Vec<&str> = lowered.split_whitespace().collect();
    if words.len() > 1 && matches!(words[0], "a" | "an" | "the" | "its" | "it" | "is") {
        words.remove(0);
    }
    if words.len() > 1 && matches!(words[0], "a" | "an" | "the") {
        words.remove(0);
    }
    words.join(" ")
}

/// Optimal string alignment distance: edits, with a swap of two neighbours
/// counted as one.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut rows = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=b.len() {
        rows[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

/// How many typos a word of this length forgives.
fn slack(word: &str) -> usize {
    match word.chars().filter(|c| *c != ' ').count() {
        0..=3 => 0,
        4..=5 => 1,
        _ => 2,
    }
}

fn closest(text: &str) -> Option<usize> {
    if text.is_empty() {
        return None;
    }
    let squashed = text.replace(' ', "");
    let mut forms = vec![text.to_string()];
    for suffix in ["es", "s"] {
        if let Some(single) = text.strip_suffix(suffix).filter(|s| s.len() >= 3) {
            forms.push(single.to_string());
        }
    }
    let terms = vocabulary();
    let mut best: Option<(usize, usize)> = None;
    let mut tied_objects = false;
    for (index, term) in terms.iter().enumerate() {
        let plain_squashed = term.plain.replace(' ', "");
        let gap = forms
            .iter()
            .map(|form| distance(form, &term.plain))
            .chain(std::iter::once(distance(&squashed, &plain_squashed)))
            .min()
            .unwrap_or(usize::MAX);
        if gap > slack(&term.plain) {
            continue;
        }
        match best {
            Some((_, held)) if gap > held => {}
            Some((kept, held)) if gap == held => {
                if terms[kept].object != term.object {
                    tied_objects = true;
                }
            }
            _ => {
                best = Some((index, gap));
                tied_objects = false;
            }
        }
    }
    if tied_objects {
        return None;
    }
    best.map(|(index, _)| index)
}

/// The vocabulary index typed text means, forgiving typos, plurals, an
/// article in front and extra words before the noun ("a red car").
pub fn resolve(text: &str) -> Option<usize> {
    let plain = normalize(text);
    if let Some(found) = closest(&plain) {
        return Some(found);
    }
    let words: Vec<&str> = plain.split(' ').collect();
    (1..words.len().min(3))
        .rev()
        .find_map(|keep| closest(&words[words.len() - keep..].join(" ")))
}

fn round3(v: f32) -> f64 {
    ((v as f64) * 1000.0).round() / 1000.0
}

fn v3(v: [f32; 3]) -> Value {
    json!([round3(v[0]), round3(v[1]), round3(v[2])])
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn gap(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// A unit direction from yaw (0 looks back along +z at the object's front)
/// and pitch (up is positive).
fn heading(yaw: f32, pitch: f32) -> [f32; 3] {
    [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: [f32; 3],
    pub at: [f32; 3],
    /// Vertical field of view in degrees.
    pub fov: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Odd {
    Below,
    Above,
    Grazing,
}

struct Piece {
    catalog: usize,
    parts: Vec<Part>,
    center: [f32; 3],
    radius: f32,
    focus: usize,
    odd: Odd,
    yaw: f32,
    /// Vocabulary indices of the four controller choices, the right one
    /// among them.
    choices: [usize; 4],
}

impl Piece {
    fn new(catalog: usize, rng: &mut StdRng) -> Piece {
        let mut parts = (CATALOG[catalog].build)();
        parts.shuffle(rng);
        let count = parts.len() as f32;
        let mut center = [0.0; 3];
        for part in &parts {
            center = add(center, scale(part.at, 1.0 / count));
        }
        let radius = parts
            .iter()
            .map(|part| gap(part.at, center) + part.reach())
            .fold(0.5f32, f32::max);
        // A close-up of a small, off-centre piece says the least.
        let mut candidates: Vec<usize> = (0..parts.len())
            .filter(|&i| parts[i].reach() < radius * 0.5 && gap(parts[i].at, center) > radius * 0.2)
            .collect();
        if candidates.is_empty() {
            candidates = (0..parts.len()).collect();
        }
        let focus = candidates[rng.random_range(0..candidates.len())];
        let odd = [Odd::Below, Odd::Above, Odd::Grazing][rng.random_range(0..3)];
        let yaw = rng.random_range(-0.9..0.9f32);
        let answer = first_term(catalog);
        let kind = CATALOG[catalog].kind;
        let mut decoys: Vec<usize> = (0..CATALOG.len())
            .filter(|&other| other != catalog && CATALOG[other].kind == kind)
            .collect();
        decoys.shuffle(rng);
        let mut rest: Vec<usize> = (0..CATALOG.len())
            .filter(|&other| other != catalog && CATALOG[other].kind != kind)
            .collect();
        rest.shuffle(rng);
        decoys.extend(rest);
        let mut choices = [
            answer,
            first_term(decoys[0]),
            first_term(decoys[1]),
            first_term(decoys[2]),
        ];
        choices.shuffle(rng);
        Piece {
            catalog,
            parts,
            center,
            radius,
            focus,
            odd,
            yaw,
            choices,
        }
    }

    fn camera(&self, shot: Shot, t: f32) -> Camera {
        let r = self.radius;
        let c = self.center;
        let (at, direction, distance, fov) = match shot {
            Shot::Closeup => {
                let focus = &self.parts[self.focus];
                (
                    focus.at,
                    heading(self.yaw + 2.2 + t * 0.2, 0.3),
                    (focus.reach() * 2.2).max(r * 0.42),
                    30.0,
                )
            }
            Shot::Angle => match self.odd {
                Odd::Below => (c, heading(self.yaw + 1.4 + t * 0.12, -1.2), r * 1.9, 52.0),
                Odd::Above => (c, heading(self.yaw - 1.2 + t * 0.12, 1.42), r * 1.9, 50.0),
                Odd::Grazing => (
                    add(c, [0.0, -0.35 * r, 0.0]),
                    heading(self.yaw + 2.9 + t * 0.1, 0.03),
                    r * 1.35,
                    62.0,
                ),
            },
            Shot::Blueprint => (c, heading(self.yaw + 0.5 + t * 0.08, 0.32), r * 2.75, 36.0),
            Shot::Full => (c, heading(self.yaw + 0.82 + t * 0.12, 0.3), r * 2.6, 38.0),
            Shot::Reveal => (c, heading(self.yaw + 1.3 + t * 0.5, 0.28), r * 2.6, 38.0),
        };
        Camera {
            eye: add(at, scale(direction, distance)),
            at,
            fov,
        }
    }

    /// The parts a shot shows: only those near the focus in the close-up.
    fn shown(&self, shot: Shot) -> Vec<&Part> {
        if shot != Shot::Closeup {
            return self.parts.iter().collect();
        }
        let focus = self.parts[self.focus].at;
        let near = self.radius * 0.45;
        self.parts
            .iter()
            .filter(|part| gap(part.at, focus) - part.reach() < near)
            .collect()
    }
}

fn first_term(catalog: usize) -> usize {
    vocabulary()
        .iter()
        .position(|term| term.object == catalog)
        .unwrap_or(0)
}

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Looking,
    Reveal,
}

#[derive(Clone, Copy)]
struct Plan {
    round: usize,
    /// When the right answer goes in, in seconds into the object.
    answer_at: f32,
    /// A wrong word and when it goes in, before the right one.
    blunder: Option<(f32, usize)>,
}

struct Player {
    seat: u8,
    skill: f32,
    points: u32,
    /// Where in the order of right answers this one came, for this object.
    rank: Option<usize>,
    locked_until: f32,
    misses: u32,
    firsts: u32,
    plan: Option<Plan>,
}

pub struct Blueprints {
    pieces: Vec<Piece>,
    players: Vec<Player>,
    round: usize,
    stage: Stage,
    clock: f32,
    deadline: f32,
    solvers: Vec<u8>,
    seed: u64,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

/// What a press of fire asks: a vocabulary index or `None` for a word the
/// vocabulary does not hold, and whether it was a pick from the choices.
fn guess(input: &Input) -> Option<(Option<usize>, bool)> {
    if !input.fire || !input.yaw.is_finite() {
        return None;
    }
    let index = input.yaw.round();
    if (input.yaw - index).abs() > 0.25 {
        return None;
    }
    let picked = input.pitch.is_finite() && input.pitch >= PICKED * 0.5;
    if index == UNKNOWN {
        return Some((None, picked));
    }
    if index < 0.0 || index as usize >= vocabulary().len() {
        return None;
    }
    Some((Some(index as usize), picked))
}

impl Blueprints {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut order: Vec<usize> = (0..CATALOG.len()).collect();
        order.shuffle(&mut rng);
        let pieces = order
            .into_iter()
            .take(OBJECTS)
            .map(|catalog| Piece::new(catalog, &mut rng))
            .collect();
        Blueprints {
            pieces,
            players: contestants
                .iter()
                .map(|one| Player {
                    seat: one.seat,
                    skill: one.skill,
                    points: 0,
                    rank: None,
                    locked_until: 0.0,
                    misses: 0,
                    firsts: 0,
                    plan: None,
                })
                .collect(),
            round: 0,
            stage: Stage::Looking,
            clock: 0.0,
            deadline: LOOK_SECONDS,
            solvers: Vec::new(),
            seed,
            rng,
            placed: None,
        }
    }

    fn piece(&self) -> &Piece {
        &self.pieces[self.round]
    }

    fn shot(&self) -> Shot {
        match self.stage {
            Stage::Reveal => Shot::Reveal,
            Stage::Looking => Shot::at(self.clock),
        }
    }

    fn shot_time(&self) -> f32 {
        match self.stage {
            Stage::Reveal => self.clock,
            Stage::Looking => {
                self.clock - SHOT_SECONDS * (self.clock / SHOT_SECONDS).floor().min(3.0)
            }
        }
    }

    pub fn camera(&self) -> Camera {
        self.piece().camera(self.shot(), self.shot_time())
    }

    #[cfg(test)]
    pub fn answer_term(&self) -> usize {
        first_term(self.piece().catalog)
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
    pub fn clock(&self) -> f32 {
        self.clock
    }

    #[cfg(test)]
    pub fn points(&self, seat: u8) -> u32 {
        self.players
            .iter()
            .find(|p| p.seat == seat)
            .map_or(0, |p| p.points)
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
        let object = &CATALOG[self.piece().catalog];
        events.push(json!({
            "kind": "revealed",
            "name": object.name,
            "solvers": self.solvers,
        }));
    }

    fn next_object(&mut self, events: &mut Vec<Value>) {
        if self.round + 1 >= self.pieces.len() {
            self.settle();
            events.push(json!({ "kind": "finished" }));
            return;
        }
        self.round += 1;
        self.stage = Stage::Looking;
        self.clock = 0.0;
        self.deadline = LOOK_SECONDS;
        self.solvers.clear();
        for player in self.players.iter_mut() {
            player.rank = None;
            player.locked_until = 0.0;
        }
        events.push(json!({ "kind": "object", "round": self.round }));
    }

    fn take(&mut self, seat: u8, word: Option<usize>, picked: bool, events: &mut Vec<Value>) {
        if picked && self.clock < CHOICES_AT {
            return;
        }
        let clock = self.clock;
        let answer = self.piece().catalog;
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return;
        };
        let player = &self.players[index];
        if player.rank.is_some() || clock < player.locked_until {
            return;
        }
        let right = word.is_some_and(|w| vocabulary()[w].object == answer);
        if right {
            let rank = self.solvers.len();
            let points = POINTS[rank.min(POINTS.len() - 1)];
            let player = &mut self.players[index];
            player.rank = Some(rank);
            player.points += points;
            if rank == 0 {
                player.firsts += 1;
                self.deadline = self.deadline.min(clock + AFTER_FIRST_SECONDS);
            }
            self.solvers.push(seat);
            events.push(json!({ "kind": "solved", "seat": seat, "rank": rank, "points": points }));
        } else {
            let player = &mut self.players[index];
            player.locked_until = clock + LOCKOUT_SECONDS;
            player.misses += 1;
            events.push(json!({
                "kind": "miss",
                "seat": seat,
                "word": word.map(|w| vocabulary()[w].word),
            }));
        }
    }

    fn plan_for(&mut self, index: usize) -> Plan {
        let round = self.round;
        if let Some(plan) = self.players[index].plan.filter(|plan| plan.round == round) {
            return plan;
        }
        let player = &self.players[index];
        let skill = Skill(round_form(player.skill, self.seed, player.seat, round as u32));
        let level = skill.level();
        let piece = &self.pieces[round];
        let object = &CATALOG[piece.catalog];
        let wobble: f32 = (0..3).map(|_| self.rng.random_range(-0.15..0.15f32)).sum();
        let seen = (object.difficulty + 0.08 + (0.5 - level) * 0.3 + wobble).clamp(0.12, 1.15);
        let typing = 0.35 + object.name.len() as f32 * 0.07 * (1.6 - level);
        let answer_at = (seen * LOOK_SECONDS + skill.reaction() + typing).max(2.4);
        let blunder = if self
            .rng
            .random_bool((0.12 + (1.0 - level) as f64 * 0.3).min(0.9))
            && answer_at > 3.0
        {
            let decoy = piece.choices[self.rng.random_range(0..4)];
            let decoy = if vocabulary()[decoy].object == piece.catalog {
                piece.choices[(piece.choices.iter().position(|&c| c == decoy).unwrap_or(0) + 1) % 4]
            } else {
                decoy
            };
            Some((
                self.rng.random_range(1.5..(answer_at - 0.5).max(1.6)),
                decoy,
            ))
        } else {
            None
        };
        let plan = Plan {
            round,
            answer_at,
            blunder,
        };
        self.players[index].plan = Some(plan);
        plan
    }

    fn standing(&self, player: &Player) -> &'static str {
        match (player.rank, self.stage) {
            (Some(_), _) => "solved",
            (None, Stage::Reveal) => "missed",
            (None, Stage::Looking) if self.clock < player.locked_until => "locked",
            (None, Stage::Looking) => "guessing",
        }
    }
}

fn part_json(part: &Part, colour: bool) -> Value {
    json!({
        "s": part.shape.name(),
        "p": v3(part.at),
        "z": v3(part.size),
        "r": v3(part.turn),
        "c": colour.then(|| format!("#{:06x}", part.color)),
    })
}

impl Bout for Blueprints {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rounds": self.pieces.len(),
            "look": LOOK_SECONDS,
            "shot": SHOT_SECONDS,
            "lockout": LOCKOUT_SECONDS,
            "choices_at": CHOICES_AT,
            "terms": vocabulary().len(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.clock += dt;
        if self.stage == Stage::Reveal {
            if self.clock >= REVEAL_SECONDS {
                self.next_object(events);
            }
            return;
        }
        for (seat, input) in inputs {
            if let Some((word, picked)) = guess(input) {
                self.take(*seat, word, picked, events);
            }
        }
        let everyone = self.players.iter().all(|p| p.rank.is_some());
        if everyone || self.clock >= self.deadline {
            self.reveal(events);
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.placed.is_some() || self.stage != Stage::Looking {
            return Input::default();
        }
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.players[index].rank.is_some() || self.clock < self.players[index].locked_until {
            return Input::default();
        }
        let mut plan = self.plan_for(index);
        let fire = |word: usize| Input {
            fire: true,
            yaw: word as f32,
            ..Default::default()
        };
        if let Some((at, word)) = plan.blunder {
            if self.clock >= at {
                plan.blunder = None;
                self.players[index].plan = Some(plan);
                return fire(word);
            }
        }
        if self.clock >= plan.answer_at {
            return fire(first_term(self.piece().catalog));
        }
        Input::default()
    }

    fn snapshot(&self) -> Value {
        let piece = self.piece();
        let shot = self.shot();
        let revealing = self.stage == Stage::Reveal;
        let colour = matches!(shot, Shot::Full | Shot::Reveal);
        let camera = self.camera();
        let choices_open = revealing || self.clock >= CHOICES_AT;
        let object = &CATALOG[piece.catalog];
        json!({
            "game": KEY,
            "time": if revealing { self.deadline } else { self.clock },
            "limit": self.deadline,
            "round": self.round,
            "rounds": self.pieces.len(),
            "stage": if revealing { "reveal" } else { "looking" },
            "shot": shot.name(),
            "shot_time": round3(self.shot_time()),
            "clock": round3(self.clock),
            "deadline": round3(self.deadline),
            "parts": piece.shown(shot).into_iter().map(|part| part_json(part, colour)).collect::<Vec<_>>(),
            "camera": { "eye": v3(camera.eye), "at": v3(camera.at), "fov": round3(camera.fov) },
            "choices": choices_open.then(|| piece.choices.iter().map(|&term| json!({
                "term": term,
                "word": CATALOG[vocabulary()[term].object].name,
            })).collect::<Vec<_>>()),
            "answer": revealing.then(|| json!({ "name": object.name, "term": first_term(piece.catalog) })),
            "solvers": self.solvers,
            "seats": self
                .players
                .iter()
                .map(|player| json!({
                    "seat": player.seat,
                    "points": player.points,
                    "state": self.standing(player),
                    "rank": player.rank,
                    "locked": round3((player.locked_until - self.clock).max(0.0) * f32::from(!revealing)),
                    "misses": player.misses,
                    "firsts": player.firsts,
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
#[path = "blueprints_tests.rs"]
mod tests;
