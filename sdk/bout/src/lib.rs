//! The contract between a Tawkathon room and the bouts it runs.
//!
//! A room seats up to four players, counts down, then calls [`Bout::step`]
//! [`TICK_HZ`] times a second with everyone's [`Input`] until
//! [`Bout::placements`] returns the finishing order. Every
//! [`SNAPSHOT_EVERY_TICKS`] ticks it sends [`Bout::snapshot`] to each client,
//! together with the events the bout pushed since the last one.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
}

impl Contestant {
    pub fn person(seat: u8) -> Self {
        Contestant {
            seat,
            is_bot: false,
            skill: 1.0,
        }
    }

    pub fn bot(seat: u8, skill: f32) -> Self {
        Contestant {
            seat,
            is_bot: true,
            skill: skill.clamp(0.0, 1.0),
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
