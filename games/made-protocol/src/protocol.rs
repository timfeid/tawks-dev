//! What players and the relay say to each other over `/party/made/ws`.
//!
//! A client opens the socket and sends `hello` with the pass from the
//! `made_join` action (`responses.made.token`). The relay answers `welcome`,
//! then streams `frame`s at 20 Hz. The first frame after a welcome or a
//! restart is full (`base: null`); later ones are deltas the kit's
//! `applyFrame` rebuilds. A delta whose base the client does not hold is
//! dropped and the client sends `resync`, which makes the next frame full.
//!
//! Clients send `input` whenever it changes, at most 60 a second and at most
//! [`MAX_CLIENT_MSG_BYTES`] each; the relay drops the excess and closes the
//! socket of anyone far over. Every message is one JSON text frame.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::scene::FrameDelta;

pub const MAX_CLIENT_MSG_BYTES: usize = 512;
pub const MAX_INPUTS_PER_SEC: u32 = 60;
pub const MAX_LOG_TEXT: usize = 500;

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Vec2 {
    pub x: f32,
    pub z: f32,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Buttons {
    #[serde(default)]
    pub action: bool,
    #[serde(default)]
    pub alt: bool,
}

/// Running counts of presses, so a tap shorter than one tick is not lost.
#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Presses {
    #[serde(default)]
    pub action: u32,
    #[serde(default)]
    pub alt: u32,
}

/// One player's controls. `move` is the stick (keys or pad), at most length
/// 1, with +z towards the far side of the camera's frame and +x to the right.
/// `aim` is the floor point under the mouse (or the pad's right stick turned
/// into one) for `move_aim` games. `action` is Space, a left click or the
/// pad's south button; `alt` is Shift, a right click or the east button.
#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Input {
    #[serde(default)]
    pub seq: u32,
    #[serde(rename = "move", default)]
    pub stick: Vec2,
    #[serde(default)]
    pub aim: Option<Vec2>,
    #[serde(default)]
    pub held: Buttons,
    #[serde(default)]
    pub presses: Presses,
}

impl Input {
    /// Clamps the stick to length 1 and drops non-finite numbers.
    pub fn sanitized(mut self) -> Self {
        let (x, z) = (self.stick.x, self.stick.z);
        if !x.is_finite() || !z.is_finite() {
            self.stick = Vec2::default();
        } else {
            let l = (x * x + z * z).sqrt();
            if l > 1.0 {
                self.stick = Vec2 { x: x / l, z: z / l };
            }
        }
        if let Some(a) = self.aim {
            if !a.x.is_finite() || !a.z.is_finite() || a.x.abs() > 1000.0 || a.z.abs() > 1000.0 {
                self.aim = None;
            }
        }
        self
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Controls {
    /// Move only.
    Move,
    /// Move plus the action button.
    MoveAction,
    /// Move, aim with the mouse, and the action button.
    MoveAim,
}

/// A game's `meta`, as checked by the kit's `defineGame`.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GameMeta {
    pub name: String,
    pub rules: String,
    pub players: [u8; 2],
    pub controls: Controls,
    pub seconds: f32,
    /// The one line a player reads first: what to do.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    /// How a round is won.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub win: Option<String>,
    /// How a player is knocked out or loses, when that can happen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lose: Option<String>,
    /// "screen" (the default): each key moves that way on screen. "tank": up
    /// is throttle, down reverse, left and right turn the hull.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steer: Option<String>,
    /// Inputs the game ignores on purpose ("up", "down", "left", "right",
    /// "action"), so the controls gate does not expect them to do anything.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unused: Vec<String>,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Relaxed,
    #[default]
    Steady,
    Sharp,
    Ruthless,
}

impl Skill {
    /// The 0 to 1 level games read through `ctx.skill(seat)`.
    pub fn level(self) -> f32 {
        match self {
            Skill::Relaxed => 0.15,
            Skill::Steady => 0.4,
            Skill::Sharp => 0.7,
            Skill::Ruthless => 1.0,
        }
    }
}

/// One seat of a room. A bot seat is played by the game's `bot`.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RosterSeat {
    pub seat: u8,
    pub name: String,
    pub bot: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<Skill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// The party wardrobe's look for this person, as the party canvas reads it;
    /// null for bots and anyone without one, who get the seat's default look.
    #[serde(default)]
    pub costume: Option<serde_json::Value>,
    /// A human seat whose player has a socket open now.
    #[serde(default)]
    pub connected: bool,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

/// A line of the room's log: errors and slow frames from the game, and
/// joins, restarts and ends from the relay. The Play view's log strip and the
/// agent's `made_logs` read these.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LogLine {
    /// Unix milliseconds.
    pub at: f64,
    pub level: LogLevel,
    pub text: String,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Welcome {
    /// The seat this socket plays, or null when it only watches.
    pub seat: Option<u8>,
    pub user_id: String,
    /// The person who started the test lobby; only they may restart it.
    pub owner: bool,
    pub lobby_id: String,
    pub activity_id: String,
    pub version: i32,
    pub meta: GameMeta,
    pub roster: Vec<RosterSeat>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello {
        token: String,
    },
    Input {
        input: Input,
    },
    Ping {
        t: f64,
    },
    /// Asks for the next frame to be full.
    Resync,
    /// The owner asks for a new round with the seats the test lobby has now
    /// (after `activity_test_restart` changed them, or as they are).
    Restart,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// The game called `ctx.end`.
    Finished,
    /// The game ran past `meta.seconds` plus a grace period, so everyone tied.
    Timeout,
    /// The game threw, ran out of time or memory, or drew a bad frame.
    Crashed,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RelayMsg {
    Welcome {
        welcome: Welcome,
    },
    Roster {
        roster: Vec<RosterSeat>,
    },
    Frame {
        frame: FrameDelta,
    },
    /// The round is over. `placements` are tied groups of seats, best first.
    End {
        placements: Vec<Vec<u8>>,
        reason: EndReason,
    },
    /// A new round is starting; the next frame is full.
    Restarting {
        version: i32,
    },
    Log {
        line: LogLine,
    },
    /// With `fatal` the socket closes after this; fetch a new pass and rejoin.
    Error {
        message: String,
        fatal: bool,
    },
    Pong {
        t: f64,
        now: f64,
    },
}

/// The response the `made_join` action stores at `responses.made`, which the
/// made canvas display reads.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MadeJoin {
    pub ws_url: String,
    pub token: String,
    pub lobby_id: String,
    pub activity_id: String,
    pub version: i32,
    pub seat: Option<u8>,
}
