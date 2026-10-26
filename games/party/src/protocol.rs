use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::board::Item;
use crate::room::RoomView;
use crate::roster::Character;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello {
        token: String,
    },
    Input {
        x: f32,
        z: f32,
        #[serde(default)]
        yaw: f32,
        #[serde(default)]
        pitch: f32,
        #[serde(default)]
        jump: bool,
        #[serde(default)]
        crouch: bool,
    },
    Fire {
        #[serde(default)]
        yaw: f32,
        #[serde(default)]
        pitch: f32,
    },
    ChooseGame {
        key: String,
    },
    /// A vote for the next bout of a Tawkathon round.
    Vote {
        key: String,
    },
    ChooseSkill {
        level: String,
    },
    Pick {
        character: String,
    },
    Start,
    StartParty,
    UseItem {
        item: Item,
        target: Option<u8>,
    },
    PassItems,
    Roll,
    ChooseBranch {
        node: u8,
    },
    EndTurn,
    AddBot,
    RemoveBot {
        seat: u8,
    },
    /// A move in the table game, in that game's own shape.
    Table {
        action: Value,
    },
    /// Start or stop filming the room for the channel's files.
    Record {
        on: bool,
    },
    Ping {
        t: f64,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg<'a> {
    Welcome {
        user_id: &'a str,
        match_id: &'a str,
        roster: &'a [Character],
        tick_hz: u32,
        snapshot_hz: u32,
        guest: bool,
    },
    Room {
        room: RoomView,
        spectating: bool,
    },
    Snap {
        tick: u64,
        game: Value,
        events: Vec<Value>,
    },
    Pong {
        t: f64,
    },
    /// One seat's view of the card table; hands and face-down cards differ per seat.
    Table {
        game: &'static str,
        view: Value,
    },
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<&'a str>,
    },
}
