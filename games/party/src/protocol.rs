use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::board::{Item, Map, Taunt, Upgrade};
use crate::room::RoomView;
use crate::roster::Character;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    RefreshCostume,
    Face { yaw: f32 },
    PositionInput { x: f32, z: f32 },
    ActivateItem,
    CancelItem,
    RaidInput { x:f32, z:f32, yaw:f32 },
    RaidSlash,
    CrawlerInput {x:f32,z:f32},
    RaidFire {on:bool,yaw:f32},
    BuyCandy,
    JoinCandyVisit,
    CandyVisitInput{x:f32,z:f32},
    DriveInput {throttle:f32,steer:f32,#[serde(default)] horn:bool},
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
    /// One tap for a bout that counts presses, sent the moment it happens.
    Press {
        hand: crate::bout::Press,
    },
    ChooseGame {
        key: String,
    },
    /// A vote for the next bout of a Tawkathon round.
    Vote {
        key: String,
    },
    /// A 1 to 5 rating of the bout just played, with an optional note.
    Feedback {
        rating: u8,
        #[serde(default)]
        note: String,
    },
    ChooseSkill {
        level: String,
    },
    ChooseMap {
        map: Map,
    },
    /// A whole Tawkathon, or a single bout.
    ChooseMode {
        party: bool,
    },
    ChooseRounds { rounds:u8 },
    ChooseCity {police:u8,jobs:u8,#[serde(default)]host_police:bool},
    StartCity {#[serde(default)]watch:bool},
    CityAction {held:bool,#[serde(default)]suction:bool},
    CityTool {tool:crate::city_heist::Tool},
    CityAutomate {on:bool},
    CityReady,
    CityVehicle,
    Pick {
        character: String,
    },
    Start,
    StartParty,
    /// Done practising; the bout counts in once everyone is.
    Ready,
    /// The host turning pairs on or off.
    ChoosePairs {
        on: bool,
    },
    /// The host turning the mature game on or off.
    ChooseMature {
        on: bool,
    },
    /// A dev stack's sandbox, and its tools.
    StartSandbox {
        #[serde(default)]
        costume: Option<crate::roster::Costume>,
    },
    Dev {
        tool: crate::room::DevAction,
    },
    UseItem {
        item: Item,
        target: Option<u8>,
        #[serde(default)]
        number: Option<u8>,
    },
    PassItems,
    Roll,
    /// Buy an item or a vacuum upgrade at the shop, or neither to walk on.
    Shop {
        #[serde(default)]
        item: Option<Item>,
        #[serde(default)]
        upgrade: Option<Upgrade>,
    },
    FireCannon {
        power: u8,
    },
    /// Throw the bomb: `yaw` the way (0 is +z, a quarter turn +x), `pitch`
    /// how steeply in radians, `power` from 0 to 1.
    Lob {
        yaw: f32,
        pitch: f32,
        power: f32,
    },
    /// Pull in the coins and a crate near you, once a turn.
    Suck,
    /// Point the vacuum (0 is +z, a quarter turn +x), turn it on or off, and
    /// suck or blow.
    Vacuum {
        yaw: f32,
        on: bool,
        #[serde(default)]
        blow: bool,
    },
    /// No target id or reward claim: the server checks the held aim while walking.
    WalkVacuum {
        yaw: f32,
        pitch: f32,
        on: bool,
        #[serde(default)]
        blow: bool,
    },
    StowVacuum,
    /// Hold on against someone else's vacuum while it drains you, or let go.
    Brace {
        on: bool,
    },
    /// Throw the crate you are holding.
    ThrowCrate,
    /// Throw the snowball you are holding.
    ThrowSnowball,
    /// Stop the skill check of the item in hand at `stop` (0 to 1, best at 0.5).
    Check {
        stop: f32,
    },
    /// Swing the bat toward `yaw`, the timing needle stopped at `stop` (0 to 1, best at 0.5).
    Swing {
        yaw: f32,
        stop: f32,
    },
    Bet {
        coins: i32,
    },
    Taunt {
        taunt: Taunt,
    },
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

impl ClientMsg{
    pub fn allowed_while_candy_paused(&self)->bool{matches!(self,Self::JoinCandyVisit|Self::CandyVisitInput{..}|Self::BuyCandy|Self::Ping{..}|Self::Feedback{..}|Self::Dev{tool:crate::room::DevAction::Exit})}
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
    FeedbackResult{number:u32,with_note:bool,saved:bool,rewarded:bool,#[serde(skip_serializing_if="Option::is_none")]message:Option<String>},
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<&'a str>,
    },
}
