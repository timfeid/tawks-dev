//! What the relay and the API say to made-simd over its local socket: one JSON
//! object per line in each direction. The relay keeps one connection open and
//! multiplexes its rooms on it by `room`; the API opens a connection per
//! playout or render and reads one answer.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::protocol::{GameMeta, Input, LogLine, RosterSeat};
use crate::scene::SceneFrame;

/// A seat as the sandbox sees it. `skill` is the 0 to 1 level.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimSeat {
    pub seat: u8,
    pub name: String,
    pub bot: bool,
    pub skill: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SimdRequest {
    /// Starts (or restarts) a room's round from `source`.
    Open {
        room: String,
        source: String,
        seats: Vec<SimSeat>,
        seed: u32,
    },
    Input {
        room: String,
        seat: u8,
        input: Input,
    },
    Close {
        room: String,
    },
    /// Runs the gates headless and answers with one `playout_done`.
    Playout {
        source: String,
        #[serde(default)]
        spec: PlayoutSpec,
    },
    Ping,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SimdEvent {
    Opened {
        room: String,
        meta: GameMeta,
    },
    Frame {
        room: String,
        frame: SceneFrame,
    },
    End {
        room: String,
        placements: Vec<Vec<u8>>,
        reason: crate::protocol::EndReason,
    },
    Log {
        room: String,
        line: LogLine,
    },
    /// The round stopped for good: a throw, a budget overrun, a bad frame.
    Failed {
        room: String,
        message: String,
    },
    PlayoutDone {
        report: Box<PlayoutReport>,
    },
    Pong,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    /// Every seat is a bot at the given skill.
    Bots,
    /// Random sticks and presses.
    Monkey,
    /// Nobody touches anything, so the clock has to end it.
    Idle,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayoutSpec {
    /// Seat counts to play; empty means every count from min to max.
    #[serde(default)]
    pub seats: Vec<u8>,
    #[serde(default = "default_seeds")]
    pub seeds: u32,
    #[serde(default = "default_inputs")]
    pub inputs: Vec<InputKind>,
    /// Record frames for screenshots (at 1 s, 5 s and the end of one game at the most seats).
    #[serde(default = "yes")]
    pub shots: bool,
}

fn default_seeds() -> u32 {
    3
}
fn default_inputs() -> Vec<InputKind> {
    vec![InputKind::Bots, InputKind::Monkey, InputKind::Idle]
}
fn yes() -> bool {
    true
}

impl Default for PlayoutSpec {
    fn default() -> Self {
        PlayoutSpec {
            seats: Vec::new(),
            seeds: default_seeds(),
            inputs: default_inputs(),
            shots: true,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Gate {
    /// finishes, no_exceptions, placements_valid, tick_budget, state_ok, view_sane, controls_respond,
    /// bots_play, player_can_tell, idle_never_wins, round_length.
    pub name: String,
    pub ok: bool,
    /// A failing warning does not block.
    pub warning: bool,
    pub detail: String,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Shot {
    /// "1s", "5s" or "end".
    pub label: String,
    pub frame: SceneFrame,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayoutReport {
    /// Every gate that is not a warning passed.
    pub ok: bool,
    pub meta: Option<GameMeta>,
    pub gates: Vec<Gate>,
    pub games: u32,
    pub longest_s: f32,
    pub p99_frame_ms: f32,
    pub max_state_bytes: u32,
    /// The first error with its stack, for the agent.
    pub first_error: Option<String>,
    /// One line for the card, e.g. "36 games at 2 to 8 seats, longest 94 s, p99 frame 0.6 ms".
    pub summary: String,
    pub roster: Vec<RosterSeat>,
    pub shots: Vec<Shot>,
}

/// What the render service is asked to draw: frames with the roster their
/// players come from, at `width` by `height`.
///
/// The render page contract (the kit renderer, built by the client side): the
/// service serves the built renderer folder (`MADE_RENDER_DIR`, holding
/// `index.html`) on a loopback port and opens
/// `index.html?shot=/shots/<n>.json` in headless Chrome at `width` by
/// `height`. The page fetches that [`RenderShot`], draws its one frame as a
/// still (no interpolation; effects may be ignored), and then sets
/// `window.__madeReady = true`, or `window.__madeError = "<why>"` when it
/// cannot. The service screenshots the page once ready, within 60 seconds.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RenderRequest {
    pub roster: Vec<RosterSeat>,
    pub shots: Vec<Shot>,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
}

fn default_width() -> u32 {
    960
}
fn default_height() -> u32 {
    540
}

/// One still for the render page to draw.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RenderShot {
    pub roster: Vec<RosterSeat>,
    pub frame: SceneFrame,
    pub width: u32,
    pub height: u32,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RenderedShot {
    pub label: String,
    /// Base64 PNG, or null when it could not be drawn.
    pub png: Option<String>,
    /// Blank or nearly one colour.
    pub blank: bool,
    pub problem: Option<String>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RenderResponse {
    pub shots: Vec<RenderedShot>,
}
