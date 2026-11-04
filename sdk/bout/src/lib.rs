//! The contract between a Tawkathon room and the bouts it runs.
//!
//! A room seats up to four players, counts down, then calls [`Bout::step`]
//! [`TICK_HZ`] times a second with everyone's [`Input`] until
//! [`Bout::placements`] returns the finishing order. Every
//! [`SNAPSHOT_EVERY_TICKS`] ticks it sends [`Bout::snapshot`] to each client,
//! together with the events the bout pushed since the last one.

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod form;
pub use form::{bout_form, bout_form_scaled, on_form, on_form_scaled, round_form};

pub const TICK_HZ: u32 = 60;
pub const TICK_DT: f32 = 1.0 / TICK_HZ as f32;
pub const SNAPSHOT_EVERY_TICKS: u64 = 3;

/// One analog stick, clamped to the unit disc. Keyboards send its corners.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Stick {
    pub x: f32,
    pub z: f32,
}

impl Stick {
    pub fn clamped(self) -> Stick {
        let (x, z) = (finite(self.x), finite(self.z));
        let len = (x * x + z * z).sqrt();
        if len > 1.0 {
            Stick {
                x: x / len,
                z: z / len,
            }
        } else {
            Stick { x, z }
        }
    }
}

fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// What one player is doing this tick: where they are pushing, where they are
/// looking, and whether they pulled the trigger or jumped since the last tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Input {
    pub stick: Stick,
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    pub jump: bool,
    pub crouch: bool,
    /// One tap made since the last tick, for bouts that count presses rather
    /// than read held keys. The room queues taps and hands them out one a
    /// tick in the order they came, so none are lost between ticks.
    #[serde(default)]
    pub press: Option<Press>,
}

/// Which hand a tap came from. Both is a single button, such as the action key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Press {
    Left,
    Right,
    Both,
}

impl Input {
    pub fn from_stick(stick: Stick) -> Self {
        Input {
            stick,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Contestant {
    pub seat: u8,
    pub is_bot: bool,
    /// How sharp this bot plays, from 0 (clumsy) to 1 (ruthless).
    pub skill: f32,
    /// Which side they are on in a bout with `Format::Teams`. Free-for-all
    /// bouts ignore it.
    pub team: u8,
    /// Played in pairs: contestants with the same ally are partners in a
    /// free-for-all bout too, and a bout that lets players hurt each other
    /// skips their partner. None plays alone.
    pub ally: Option<u8>,
    /// Frozen character build for this bout. The room validates 0.7..=1.4.
    pub body_width: f32,
    /// Server-selected pet species: ember, puddle, sprout, fizz, mochi.
    pub pet: u8,
}

/// Whether two contestants are partners, by their allies.
pub fn allied(a: Option<u8>, b: Option<u8>) -> bool {
    a.is_some() && a == b
}

impl Contestant {
    pub fn body(&self) -> BodyProfile {
        BodyProfile::new(self.body_width)
    }

    /// The same contestant on a given side.
    pub fn on_team(self, team: u8) -> Self {
        Contestant { team, ..self }
    }

    pub fn person(seat: u8) -> Self {
        Contestant {
            seat,
            is_bot: false,
            skill: 1.0,
            team: 0,
            ally: None,
            body_width: 1.0,
            pet: 4,
        }
    }

    pub fn bot(seat: u8, skill: f32) -> Self {
        Contestant {
            seat,
            is_bot: true,
            skill: skill.clamp(0.0, 1.0),
            team: 0,
            ally: None,
            body_width: 1.0,
            pet: 4,
        }
    }
}

/// Small builds accelerate and run a little faster; broad builds occupy more
/// space, resist contact and recover sooner. Both extremes have a tradeoff.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyProfile {
    pub width: f32,
    pub speed: f32,
    pub acceleration: f32,
    pub radius: f32,
    pub mass: f32,
    pub stun: f32,
    pub reach: f32,
}

impl BodyProfile {
    pub fn new(width: f32) -> Self {
        let width = if width.is_finite() { width.clamp(0.7, 1.4) } else { 1.0 };
        let delta = width - 1.0;
        Self {
            width,
            speed: 1.0 - delta * 0.18,
            acceleration: 1.0 - delta * 0.22,
            radius: 1.0 + delta * 0.22,
            mass: 1.0 + delta * 0.50,
            stun: 1.0 - delta * 0.27,
            reach: 1.0 + delta * 0.12,
        }
    }
}

/// The named settings the host picks between.
pub const SKILLS: &[(&str, f32)] = &[
    ("relaxed", 0.15),
    ("steady", 0.4),
    ("sharp", 0.7),
    ("ruthless", 1.0),
];

pub const DEFAULT_SKILL: f32 = 0.4;

pub fn skill_name(skill: f32) -> &'static str {
    SKILLS
        .iter()
        .min_by(|a, b| {
            (a.1 - skill)
                .abs()
                .partial_cmp(&(b.1 - skill).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(name, _)| *name)
        .unwrap_or("steady")
}

/// How well a bot plays, and the handful of knobs every game reads off it.
#[derive(Clone, Copy, Debug)]
pub struct Skill(pub f32);

impl Default for Skill {
    fn default() -> Self {
        Skill(DEFAULT_SKILL)
    }
}

impl Skill {
    pub fn level(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// Radians of wobble on an aimed shot.
    pub fn aim_error(self) -> f32 {
        0.03 + (1.0 - self.level()) * 0.42
    }

    /// Seconds between seeing something and acting on it.
    pub fn reaction(self) -> f32 {
        0.12 + (1.0 - self.level()) * 0.85
    }

    /// How often a bot simply does not take the shot it had.
    pub fn hesitation(self) -> f64 {
        ((1.0 - self.level()) * 0.55) as f64
    }

    /// Movement pace, so a clumsy bot is also a slower one.
    pub fn pace(self) -> f32 {
        0.62 + self.level() * 0.38
    }

    /// How tightly a bot tracks a moving mark.
    pub fn tracking(self) -> f32 {
        2.0 + self.level() * 6.0
    }
}

/// A real-time bout run by a Tawkathon room. The room owns seats, inputs and
/// phases; the bout owns the simulation. Placements list tied groups of seats,
/// best first.
pub trait Bout: Send {
    fn key(&self) -> &'static str;
    /// The fixed layout the client needs to build the arena, sent with the
    /// room rather than in every snapshot.
    fn setup(&self) -> Value {
        Value::Null
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>);
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input;
    /// An object with `game` (the bout's key), and `time` and `limit` in
    /// seconds for the clock, plus whatever the renderer draws.
    fn snapshot(&self) -> Value;
    /// What one seat may see, for a bout that hides part of itself from some
    /// players; `None` for a spectator. Returning `None` sends that seat the
    /// shared [`Bout::snapshot`].
    fn snapshot_for(&self, _seat: Option<u8>) -> Option<Value> {
        None
    }
    /// The seat playing the bout's lone role, one against everyone, so a
    /// party can pass the role around.
    fn lead(&self) -> Option<u8> {
        None
    }
    /// Seats that led this bout earlier in the party, oldest first, handed
    /// over right after it is made.
    fn rotate_lead(&mut self, _earlier: &[u8]) {}
    fn placements(&self) -> Option<Vec<Vec<u8>>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Controls {
    /// The stick only.
    Move,
    /// The stick and one button.
    MoveAction,
    /// The stick, the mouse to aim, and a button to fire.
    MoveAim,
    /// Mouse look, the stick to walk, fire, jump and crouch.
    FirstPerson,
}

/// How much wider an arena laid out for four should be for `players`, so each
/// player keeps about the same room: the area grows with the head count, the
/// width with its square root. Four or fewer play the arena as built.
pub fn crowd_scale(players: usize) -> f32 {
    (players.max(4) as f32 / 4.0).sqrt()
}

/// How many of something laid out for four there should be for `players`:
/// pickups, hazards, coins. Never fewer than built.
pub fn crowd_count(base: usize, players: usize) -> usize {
    (base * players.max(4)).div_ceil(4)
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    FreeForAll,
    Teams,
}

/// Everything the room and the client need to know about a bout before it
/// starts.
#[derive(Clone, Copy, Serialize)]
pub struct BoutSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub rules: &'static str,
    pub controls: Controls,
    pub format: Format,
    pub min_players: usize,
    pub max_players: usize,
    #[serde(skip)]
    pub create: fn(&[Contestant], u64) -> Box<dyn Bout>,
}

impl std::fmt::Debug for BoutSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.key)
    }
}

impl BoutSpec {
    pub fn fits(&self, players: usize) -> bool {
        (self.min_players..=self.max_players).contains(&players)
    }
}
