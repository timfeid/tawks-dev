//! A party room: seats, character picks, the host, practice bots and the
//! lobby, countdown, play and results cycle around one minigame at a time.
//! Pure state with no I/O so it can be tested tick by tick.

use std::collections::VecDeque;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::Serialize;
use serde_json::Value;

use crate::board::{Board, BoardError, BoardView, BotMove, Item, Map, Taunt, TurnStep, Upgrade};
use crate::table::{self, TABLE_GAMES, TableGame};
use crate::bout::{
    self, BOUTS, Bout, BoutSpec, Contestant, DEFAULT_SKILL, Input, Press, SKILLS, Skill, Stick,
};
use crate::roster::{ROSTER, character};

pub use crate::bout::{SNAPSHOT_EVERY_TICKS, TICK_DT, TICK_HZ};

#[cfg(test)]
#[path="candy_visit_room_tests.rs"]
mod candy_visit_tests;
#[cfg(test)]
#[path="raid_room_tests.rs"]
mod raid_tests;
#[cfg(test)]
#[path="round_settings_tests.rs"]
mod round_settings_tests;
#[cfg(test)]
#[path="city_heist_room_tests.rs"]
mod city_heist_tests;
#[cfg(test)]
#[path="map_finale_room_tests.rs"]
mod map_finale_tests;
pub const MAX_SEATS: usize = 8;
pub const NATIVE_BOUTS: &[&str] = &[
    "bumper", "race", "shooter", "coins", "titan", "stencil", "teeter", "glass", "lineup",
    "echo", "lastfloor", "fuse", crate::mugshot::KEY, "blueprints", crate::diner::KEY,
    "bridgecrew", crate::gallery::KEY, crate::tug::KEY,
    crate::lantern::KEY, crate::beacon::KEY, crate::cue_crowd::KEY, crate::flux_skip::KEY,
    crate::parcel_panic::KEY, crate::pigeon::KEY, crate::rime::KEY, crate::margin_run::KEY,
    crate::reef_bloom::KEY, crate::orbit::KEY, crate::crowd_current::KEY,
    crate::ravine_draft::KEY, crate::whirligig::KEY, crate::stackbreak_yard::KEY,
    crate::wake::KEY, crate::chimney_dash::KEY, crate::signal_sweep::KEY,
    crate::shadow_stamp::KEY, crate::garden_snapshot::KEY, crate::powder::KEY,
    crate::kite_net::KEY, crate::dockside_tangle::KEY, crate::mirth::KEY,
    crate::mixer_circuit::KEY,
    crate::pet_arcade::KEYS[0], crate::pet_arcade::KEYS[1], crate::pet_arcade::KEYS[2],
    crate::pet_arcade::KEYS[3], crate::pet_arcade::KEYS[4], crate::pet_arcade::KEYS[5],
    crate::pet_arcade::KEYS[6], crate::pet_arcade::KEYS[7], crate::pet_arcade::KEYS[8],
    crate::pet_arcade::KEYS[9],
    crate::pet_arena::KEYS[0], crate::pet_arena::KEYS[1],
];
const COUNTDOWN_TICKS: u32 = TICK_HZ * 3;
/// More taps than a hand can make in a few ticks are a flood, not play.
const MAX_QUEUED_PRESSES: usize = 8;
/// How long the practice before each bout lasts, unless everyone is ready sooner.
const PRACTICE_TICKS: u32 = TICK_HZ * 30;
const RESULTS_TICKS: u32 = TICK_HZ * 6;
const GAME_OVER_TICKS: u32 = TICK_HZ * 12;
/// The end of a Tawkathon: bonus trophies one at a time, then the podium.
const FINALE_TICKS: u32 = TICK_HZ * 24;
/// How long a practice bot thinks about each step of its board turn.
const BOT_THINK_TICKS: u32 = TICK_HZ * 3 / 4;
/// How long between one player's taunts.
const TAUNT_EVERY: u64 = TICK_HZ as u64 * 4;
/// A turn nobody takes plays itself, so one person cannot stall the party.
const TURN_LIMIT_TICKS: u32 = TICK_HZ * 45;
/// After a roll the board shows the die, then hops the token space by space;
/// the turn moves on once that has played out.
const DIE_TICKS: u32 = TICK_HZ * 13 / 5;
const HOP_TICKS: u32 = TICK_HZ * 32 / 100;
const LANDED_TICKS: u32 = TICK_HZ * 3 / 2;
pub const PARTY_ROUNDS: u32 = 10;
pub const MIN_PARTY_ROUNDS: u8 = 10;
pub const MAX_PARTY_ROUNDS: u8 = 30;
/// Rounds in the sandbox: enough that it never ends on its own.
pub const SANDBOX_ROUNDS: u32 = 999;
/// How long after landing a person has to take their vacuum out, in ticks.
const VACUUM_OFFER_TICKS: u32 = 4 * 60;

/// What the sandbox's tools do to whoever uses them.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DevAction {
    CandyVan,
    Crawler,
    Finale,
    Give { item: Item },
    Coins { coins: i32 },
    Heal,
    /// Hands a prize to you, or with `seat` to anyone, so stealing one can be tried.
    Prize {
        #[serde(default)]
        seat: Option<u8>,
    },
    /// Puts a token on a space: your own, or with `seat`, anyone's, like a bot dragged there.
    Teleport {
        space: u8,
        #[serde(default)]
        seat: Option<u8>,
    },
    /// The zombie catches you, or with `seat` anyone, and knocks them out, so a knockout can be seen.
    ZombieKill {
        #[serde(default)]
        seat: Option<u8>,
    },
    Exit,
    /// A city sandbox tool, such as spawning a dummy or starting an arrest.
    City { tool: crate::city_heist::CityTool },
    /// Leaves this sandbox for the free-roam city, as officer or crew.
    CitySandbox { police: bool },
    /// Leaves this sandbox for the board sandbox.
    BoardSandbox,
    /// Plays one bout straight away, with no ballot.
    Bout { key: String },
}

#[cfg(test)]
mod body_and_pet_bout_tests {
    use super::*;

    #[test]
    fn bout_contestants_freeze_validated_build_and_selected_pet() {
        let mut room = Room::new("build-room", 2);
        room.connect("alice", "Alice", "").unwrap();
        let mut costume = crate::roster::Costume::default();
        costume.body_width = Some(1.4);
        costume.pet.id = Some("pet-fizz".into());
        room.apply_costume("alice", costume);
        let contestant = room.contestants(true).remove(0);
        assert_eq!(contestant.body_width, 1.4);
        assert_eq!(contestant.pet, 3);
    }
    #[test]
    fn garden_snapshot_photo_survives_room_input_transport() {
        let mut room = Room::new("garden-photo-room", 4);
        room.connect("alice", "Alice", "").unwrap();
        room.add_bot("alice").unwrap();
        room.choose_game("alice", crate::garden_snapshot::KEY).unwrap();
        room.start("alice").unwrap();
        let shot = crate::garden_snapshot::shot_input(0.5, 0.5);
        room.queue_fire("alice", shot.yaw, shot.pitch);
        room.step();
        let seat = room.members.iter().find(|m| m.user_id == "alice").unwrap().seat;
        let game = room.snapshot().unwrap();
        let me = game["players"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["seat"].as_u64() == Some(u64::from(seat)))
            .unwrap();
        assert_eq!(me["photos"].as_u64(), Some(1));
    }
    #[test]
    fn crownfall_standalone_fills_both_teams_with_bots() {
        let mut room = Room::new("crownfall-room", 3);
        room.connect("alice", "Alice", "").unwrap();
        room.choose_game("alice", crate::pet_arena::KEYS[1]).unwrap();
        room.start("alice").unwrap();
        assert_eq!(room.members.len(),8);
        assert!(matches!(room.phase,Phase::Countdown { .. }));
        let game = room.snapshot().unwrap();
        let players = game["players"].as_array().unwrap();
        assert_eq!(players.len(),8);
    }
}
pub const COVEN_KEY: &str = crate::coven::KEY;
const LOBBY_DROP_TICKS: u64 = TICK_HZ as u64 * 20;
const VOTE_TICKS: u32 = TICK_HZ * 12;
/// Bots wait a moment before voting so people see the ballot fill up.
const BOT_VOTE_TICKS: u32 = TICK_HZ * 2;
const VOTE_OPTIONS: usize = 3;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "name", rename_all = "snake_case")]
pub enum Phase {
    Lobby,
    /// Taking turns on the Tawkathon board between bouts.
    Board,
    MapFinale,
    CityHeist,
    /// Everyone in the round picks the next bout from a few random ones.
    Vote {
        seconds: f32,
        options: Vec<&'static str>,
        votes: Vec<BoutVote>,
    },
    /// A card game at the table; its state travels per seat.
    Table,
    /// A free go at the bout before it counts: nothing is scored, and it
    /// starts over if it finishes. Ends when its time is up or everyone is ready.
    Practice {
        seconds: f32,
    },
    Countdown {
        seconds: f32,
    },
    Playing,
    Results {
        seconds: f32,
        placements: Vec<Vec<u8>>,
    },
    GameOver {
        seconds: f32,
        standings: Vec<Vec<u8>>,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BoutVote {
    pub seat: u8,
    pub key: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Member {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
    pub character: String,
    pub costume: Option<crate::roster::Costume>,
    pub seat: u8,
    pub is_bot: bool,
    pub connected: bool,
    pub wins: u32,
    #[serde(skip)]
    pub stick: Stick,
    #[serde(skip)]
    input_tick:u64,
    #[serde(skip)]
    pub yaw: f32,
    #[serde(skip)]
    pub pitch: f32,
    #[serde(skip)]
    pub jump: bool,
    #[serde(skip)]
    pub crouch: bool,
    /// A shot asked for since the last tick, taken on the next one.
    #[serde(skip)]
    pub fire: bool,
    /// Taps waiting for their tick, handed to the bout one a tick in order.
    #[serde(skip)]
    pub presses: VecDeque<Press>,
    #[serde(skip)]
    pub away_since: Option<u64>,
    /// Seated for the round in progress; people who join mid-round wait.
    pub in_round: bool,
    /// Done practising, and ready for the bout to count.
    pub ready: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct FeedbackAsk {
    pub number: u32,
    pub key: &'static str,
    pub name: &'static str,
    /// The stars already given with a tap, while a note can still follow.
    pub rated: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomView {
    pub city_mode:bool,
    pub city_config:crate::city_heist::CityConfig,
    pub city:Option<crate::city_heist::CityView>,
    pub match_id: String,
    pub minigame: &'static str,
    pub minigame_name: &'static str,
    /// Every bout the room can play.
    pub bouts: Vec<BoutSpec>,
    pub opening_minigame:bool,
    pub minigame_rewards:Vec<crate::board::MinigameReward>,
    pub table_games: &'static [(&'static str, &'static str)],
    /// How hard the bots play, and the names to pick between.
    pub bot_skill: f32,
    pub bot_skill_name: &'static str,
    pub skills: &'static [(&'static str, f32)],
    /// The board the next Tawkathon is played on, and the boards to pick from.
    pub map: Map,
    pub maps: Vec<(Map, &'static str)>,
    pub host_id: Option<String>,
    pub phase: Phase,
    pub members: Vec<Member>,
    /// Each seat's latest pet or poke.
    pub pet_reacts: Vec<crate::pets::PetReact>,
    pub max_seats: usize,
    /// Present while a party is running.
    pub board: Option<BoardView>,
    /// The bout this seat can still rate, if they played the last one.
    pub feedback: Option<FeedbackAsk>,
    /// Set up for a whole Tawkathon rather than a single bout.
    pub party_mode: bool,
    pub rounds: u8,
    /// In a lobby set up for a Tawkathon, the board it would be played on, to
    /// look at before starting.
    pub preview_board: Option<BoardView>,
    /// A dev stack, where the host can open the sandbox; and whether this is it.
    pub dev_tools: bool,
    pub sandbox: bool,
    /// Cartoon gore and visual effects, on by default.
    pub mature: bool,
    /// Played in pairs, and who is paired with whom once the Tawkathon starts.
    pub pairs_on: bool,
    pub pairs: Vec<TeamSeat>,
    /// The fixed layout of the minigame's arena, for the client to build.
    pub setup: Value,
    /// Sides for the bout being played, when it is played in teams.
    pub teams: Vec<TeamSeat>,
    pub recording: Option<Recording>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TeamSeat {
    pub seat: u8,
    pub team: u8,
}

/// A recording of the room asked for by a member and filmed elsewhere.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Recording {
    pub by: String,
    pub by_user_id: String,
    pub since: i64,
    /// False until the recorder has the room on screen.
    pub live: bool,
}

#[derive(Debug, PartialEq)]
pub enum RoomError {
    Full,
    NotHost,
    WrongPhase,
    Taken,
    UnknownCharacter,
    UnknownMinigame,
    NotEnoughPlayers,
    TooManyPlayers,
    Board(BoardError),
    Table(String),
    /// Feedback for a bout this person did not play, or has rated already.
    NoFeedback,
    /// The sandbox and its tools are only on a dev stack.
    NoDevTools,
    InvalidCostume(&'static str),
    InvalidRounds,
    NativeUnsupported,
}

impl std::fmt::Display for RoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RoomError::Full => "The room is full.",
            RoomError::NotHost => "Only the host can do that.",
            RoomError::WrongPhase => "Wait for this round to finish.",
            RoomError::Taken => "Someone already picked that character.",
            RoomError::UnknownCharacter => "Unknown character.",
            RoomError::UnknownMinigame => "There is no such bout.",
            RoomError::NotEnoughPlayers => "Add another player or a bot first.",
            RoomError::TooManyPlayers => "That game seats four; the rest would have to watch.",
            RoomError::Board(e) => return e.fmt(f),
            RoomError::Table(why) => why,
            RoomError::NoFeedback => "There is no bout of yours to rate.",
            RoomError::NoDevTools => "The sandbox is only on a dev stack.",
            RoomError::InvalidCostume(why) => why,
            RoomError::InvalidRounds => "Choose between 10 and 30 rounds.",
            RoomError::NativeUnsupported => "That game or board is not available in native Tawkathon yet.",
        })
    }
}

/// A Tawkathon bout that has just been played, kept until the next one ends
/// so the people in it can rate it from the results and after.
#[derive(Clone, Debug)]
pub struct PlayedBout {
    /// Counts the bouts of this party from 1.
    pub number: u32,
    pub key: &'static str,
    /// The board round, or 0 for a bout played on its own.
    pub round: u32,
    /// Who played it, and where each finished, from 1.
    pub players: Vec<(String, usize)>,
}

/// One person's rating of a bout, with what it needs to be filed.
#[derive(Clone, Debug)]
pub struct BoutFeedback {
    pub match_id: String,
    /// Names the Tawkathon this bout belonged to, uniquely even across
    /// restarts; each one files under its own ticket.
    pub party_id: String,
    pub bout: PlayedBout,
    pub user_id: String,
    pub name: String,
    pub place: usize,
    pub rating: u8,
    pub note: String,
    /// A note sent after the rating, which tops the reward up to a note's.
    pub follow_up: bool,
}

/// The longest note kept with a rating.
pub const FEEDBACK_NOTE_MAX: usize = 500;

pub struct Room {
    native_mode: bool,
    /// Who led each lone-role bout so far this party, oldest first.
    pub leads: Vec<u8>,
    pub city_mode:bool,
    pub city_config:crate::city_heist::CityConfig,
    pub city:Option<crate::city_heist::CityGame>,
    pub match_id: String,
    /// Set on a dev stack: the host can open a sandbox, a board with no bouts
    /// where bots stand still as targets and items can be given out freely.
    pub dev_tools: bool,
    pub sandbox: bool,
    /// A sandbox-only draft never becomes an account or normal-game outfit.
    sandbox_costume: Option<(String, String, Option<crate::roster::Costume>)>,
    finale_preview:bool,
    opening_minigame:bool,
    crawler_preview:bool,
    pub mature: bool,
    /// Played in pairs: the host's choice, and the pairs once the Tawkathon starts.
    pub pairs_on: bool,
    pairs: Vec<(u8, u8)>,
    /// How hard the practice bots play, for every game in this room.
    pub bot_skill: f32,
    pub map: Map,
    pub minigame: &'static str,
    pub members: Vec<Member>,
    pub host_id: Option<String>,
    /// Whoever owns the lobby; they host whenever they are in the room.
    owner_id: Option<String>,
    /// Whether the lobby is set up for a whole Tawkathon or a single bout,
    /// which decides what it previews.
    pub party_mode: bool,
    pub rounds: u8,
    /// Tawkathons started here, the last bout one finished, and who has rated it.
    parties: u32,
    party_id: String,
    bouts: u32,
    pub last_bout: Option<PlayedBout>,
    /// The stars each person gave the last bout, and who has added a note too.
    rated: std::collections::HashMap<String, u8>,
    noted: std::collections::HashSet<String>,
    pub phase: Phase,
    pub game: Option<Box<dyn Bout>>,
    pub board: Option<Board>,
    pub table: Option<Box<dyn TableGame>>,
    /// Room seat for each table seat, in table order.
    table_order: Vec<u8>,
    pub tick: u64,
    phase_ticks: u32,
    bot_think: u32,
    /// Sides for the bout being played, when it is played in teams.
    teams: Vec<(u8, u8)>,
    /// When each seat last shouted something, so it cannot be spammed.
    taunted: std::collections::HashMap<u8, u64>,
    pets: crate::pets::PetReacts,
    turn_idle: u32,
    /// Ticks since the current turn finished moving.
    turn_done: u32,
    /// How long the bomb in hand has been held, in ticks, for its fuse.
    fuse_ticks: u32,
    rng: StdRng,
    dirty: bool,
    recording: Option<Recording>,
    /// Every board move a bot made, and whether the board refused it, so a
    /// simulated party can tell a bot stuck repeating a refused move.
    #[cfg(test)]
    pub bot_log: Vec<(u8, BotMove, Result<(), BoardError>)>,
}

impl Room {
    pub fn new(match_id: impl Into<String>, seed: u64) -> Self {
        let mut room = Room {
            native_mode:false,
            city_mode:false,city_config:crate::city_heist::CityConfig{police:0,..Default::default()},city:None,
            match_id: match_id.into(),
            bot_skill: DEFAULT_SKILL,
            map: Map::Frozen,
            minigame: BOUTS[0].key,
            members: Vec::new(),
            host_id: None,
            owner_id: None,
            party_mode: true,
            rounds: PARTY_ROUNDS as u8,
            dev_tools: false,
            sandbox: false,
            sandbox_costume: None,
            finale_preview:false,
            opening_minigame:false,
            crawler_preview:false,
            mature: true,
            pairs_on: false,
            pairs: Vec::new(),
            parties: 0,
            party_id: String::new(),
            bouts: 0,
            last_bout: None,
            rated: std::collections::HashMap::new(),
            noted: std::collections::HashSet::new(),
            phase: Phase::Lobby,
            game: None,
            board: None,
            table: None,
            table_order: Vec::new(),
            tick: 0,
            phase_ticks: 0,
            bot_think: 0,
            teams: Vec::new(),
            taunted: std::collections::HashMap::new(),
            pets: crate::pets::PetReacts::default(),
            turn_idle: 0,
            turn_done: 0,
            fuse_ticks: 0,
            rng: StdRng::seed_from_u64(seed),
            dirty: true,
            recording: None,
            #[cfg(test)]
            bot_log: Vec::new(),
            leads: Vec::new(),
        };
        room.rebuild_preview();
        room
    }

    pub fn recording(&self) -> Option<&Recording> {
        self.recording.as_ref()
    }

    pub fn set_recording(&mut self, recording: Option<Recording>) {
        if self.recording != recording {
            self.recording = recording;
            self.dirty = true;
        }
    }

    /// True once since the last call when the room view changed.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    pub fn view(&self) -> RoomView {
        self.view_for(None)
    }

    /// The room as one seat sees it, which is how traps stay hidden.
    pub fn view_for(&self, seat: Option<u8>) -> RoomView {
        RoomView {
            city_mode:self.city_mode,city_config:self.city_config.clone(),city:self.city.as_ref().map(|city|city.view_for(seat)),
            match_id: self.match_id.clone(),
            minigame: self.minigame,
            minigame_name: if self.city_mode&&matches!(self.phase,Phase::Lobby|Phase::CityHeist){"City Heist"}else{self.board.as_ref().and_then(Board::finale_title).or_else(||bout::find(self.minigame)
                .map(|spec| spec.name)
                .or_else(|| TABLE_GAMES.iter().find(|(k, _)| *k == self.minigame).map(|(_, n)| *n)))
                .unwrap_or("Bout")},
            bouts: BOUTS.iter().filter(|spec| !self.native_mode || NATIVE_BOUTS.contains(&spec.key)).cloned().collect(),
            opening_minigame:self.opening_minigame,
            minigame_rewards:if self.city_mode{Vec::new()}else{match &self.phase {Phase::Results{placements,..}=>self.board.as_ref().map(|b|b.minigame_rewards(placements,self.opening_minigame)).unwrap_or_default(),_=>Vec::new()}},
            table_games: if self.native_mode { &[] } else { TABLE_GAMES },
            bot_skill: self.bot_skill,
            bot_skill_name: bout::skill_name(self.bot_skill),
            skills: SKILLS,
            map: self.map,
            maps: Map::ALL.iter().filter(|map| !self.native_mode || **map==Map::Frozen).map(|map| (*map, map.label())).collect(),
            host_id: self.host_id.clone(),
            phase: self.phase.clone(),
            members: self.members.clone(),
            pet_reacts: self.pets.view(),
            max_seats: self.seat_cap(),
            board: self.board.as_ref().map(|board| board.view_for(seat)),
            feedback: self.feedback_for(seat),
            party_mode: self.party_mode,
            rounds: self.rounds,
            preview_board: self.preview_board(),
            dev_tools: self.dev_tools || (self.native_mode && seat.is_some_and(|seat| {
                self.members.iter().any(|member| member.seat == seat
                    && self.host_id.as_deref() == Some(member.user_id.as_str()))
            })),
            sandbox: self.sandbox,
            mature: self.mature,
            pairs_on: self.pairs_on,
            pairs: self.pairs.iter().flat_map(|&(a, b)| [TeamSeat { seat: a, team: self.pair_index(a) }, TeamSeat { seat: b, team: self.pair_index(a) }]).collect(),
            setup: self.game.as_ref().map(|g| g.setup()).unwrap_or(Value::Null),
            teams: self.teams.iter().map(|(seat, team)| TeamSeat { seat: *seat, team: *team }).collect(),
            recording: self.recording.clone(),
        }
    }

    fn feedback_for(&self, seat: Option<u8>) -> Option<FeedbackAsk> {
        let user = &self.members.iter().find(|m| Some(m.seat) == seat)?.user_id;
        let bout = self.last_bout.as_ref()?;
        if self.noted.contains(user) || !bout.players.iter().any(|(player, _)| player == user) {
            return None;
        }
        Some(FeedbackAsk {
            number: bout.number,
            key: bout.key,
            name: bout::find(bout.key).map(|spec| spec.name).unwrap_or("Bout"),
            rated: self.rated.get(user).copied(),
        })
    }

    pub fn snapshot(&self) -> Option<Value> {
        self.game.as_ref().map(|g| g.snapshot())
    }

    /// The bout as one seat may see it, when it hides something from them.
    pub fn private_snapshot(&self, seat: Option<u8>) -> Option<Value> {
        self.game.as_ref().and_then(|g| g.snapshot_for(seat))
    }

    pub fn snapshot_for(&self, seat: Option<u8>) -> Option<Value> {
        self.private_snapshot(seat).or_else(|| self.snapshot())
    }

    fn seat_cap(&self) -> usize {
        table::spec(self.minigame).map(|s| s.max_players).unwrap_or(MAX_SEATS)
    }

    fn table_spec(&self) -> Option<&'static table::TableSpec> {
        table::spec(self.minigame)
    }

    fn free_seat(&self) -> Option<u8> {
        (0..self.seat_cap() as u8).find(|s| !self.members.iter().any(|m| m.seat == *s))
    }

    fn free_character(&mut self) -> &'static str {
        let open: Vec<&'static str> = ROSTER
            .iter()
            .map(|c| c.id)
            .filter(|id| !self.members.iter().any(|m| m.character == *id))
            .collect();
        if open.is_empty() {
            ROSTER[self.rng.random_range(0..ROSTER.len())].id
        } else {
            open[self.rng.random_range(0..open.len())]
        }
    }

    fn is_host(&self, user_id: &str) -> bool {
        self.host_id.as_deref() == Some(user_id)
    }

    /// A host who drops keeps the crown for as long as the lobby would keep
    /// their seat, so a reconnect does not hand it to someone else.
    fn host_holds(&self) -> bool {
        let tick = self.tick;
        self.host_id.as_ref().is_some_and(|h| {
            self.members.iter().any(|m| {
                &m.user_id == h
                    && (m.connected
                        || m.away_since.is_some_and(|t| tick.saturating_sub(t) < LOBBY_DROP_TICKS))
            })
        })
    }

    /// Marks the lobby's owner, who takes the host from whoever opened the
    /// room first.
    pub fn set_owner(&mut self, user_id: &str) {
        self.owner_id = Some(user_id.to_string());
        self.pick_host();
    }

    fn pick_host(&mut self) {
        let owner = self
            .owner_id
            .as_ref()
            .filter(|o| self.members.iter().any(|m| &m.user_id == *o && m.connected && !m.is_bot))
            .cloned();
        if let Some(owner) = owner {
            if self.host_id.as_ref() != Some(&owner) {
                self.host_id = Some(owner);
                self.dirty = true;
            }
            return;
        }
        if !self.host_holds() {
            let next = self
                .members
                .iter()
                .find(|m| !m.is_bot && m.connected)
                .map(|m| m.user_id.clone());
            if next != self.host_id {
                self.host_id = next;
                self.dirty = true;
            }
        }
    }

    /// A person opened the room. Returns their seat. A full lobby gives up a
    /// practice bot's seat to a person.
    pub fn connect(
        &mut self,
        user_id: &str,
        name: &str,
        avatar_url: &str,
    ) -> Result<u8, RoomError> {
        self.dirty = true;
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            member.connected = true;
            member.away_since = None;
            member.name = name.to_string();
            member.avatar_url = avatar_url.to_string();
            let seat = member.seat;
            self.pick_host();
            return Ok(seat);
        }
        if self.free_seat().is_none() && self.phase == Phase::Lobby {
            if let Some(bot) = self.members.iter().rposition(|m| m.is_bot) {
                self.members.remove(bot);
            }
        }
        let seat = self.free_seat().ok_or(RoomError::Full)?;
        let character = self.free_character().to_string();
        self.members.push(Member {
            user_id: user_id.to_string(),
            name: name.to_string(),
            avatar_url: avatar_url.to_string(),
            character,
            costume: None,
            seat,
            is_bot: false,
            connected: true,
            wins: 0,
            stick: Stick::default(),
            input_tick:0,
            yaw: 0.0,
            pitch: 0.0,
            jump: false,
            crouch: false,
            fire: false,
            presses: VecDeque::new(),
            ready: false,
            away_since: None,
            in_round: false,
        });
        self.members.sort_by_key(|m| m.seat);
        self.pick_host();
        self.rebuild_preview();
        Ok(seat)
    }

    pub fn disconnect(&mut self, user_id: &str) {
        let tick = self.tick;
        if !self.board_paused(){if let Some(seat) = self.seat_of(user_id) {if let Some(board)=self.board.as_mut() {board.abort_position(seat);board.end_raid(seat,"disconnect");}}}
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            member.connected = false;
            member.stick = Stick::default();
            member.fire = false;
            member.jump = false;
            member.crouch = false;
            member.away_since = Some(tick);
            self.dirty = true;
        }
        self.pick_host();
        let eligible:Vec<_>=self.members.iter().filter(|member|member.connected).map(|member|member.seat).collect();
        let bots:Vec<_>=self.members.iter().filter(|member|member.connected&&member.is_bot).map(|member|member.seat).collect();
        if !self.board_paused(){if let Some(board)=self.board.as_mut(){board.crawler_control_presence(&eligible,&bots);}}
    }

    pub fn set_input(
        &mut self,
        user_id: &str,
        stick: Stick,
        yaw: f32,
        pitch: f32,
        jump: bool,
        crouch: bool,
    ) {
        if self.board_paused(){return;}

        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            member.stick = stick.clamped();
            member.input_tick=self.tick;
            // A shot still waiting for its tick keeps the aim it was fired
            // with; bouts that read `yaw` as a choice depend on it.
            if !member.fire {
                if yaw.is_finite() {
                    member.yaw = yaw;
                }
                if pitch.is_finite() {
                    member.pitch = pitch.clamp(-1.4, 1.4);
                }
            }
            member.jump = jump;
            member.crouch = crouch;
        }
    }

    /// A shot, taken on the next tick with the aim it was fired with.
    pub fn queue_fire(&mut self, user_id: &str, yaw: f32, pitch: f32) {
        if self.board_paused(){return;}

        let garden_shot = self.minigame == crate::garden_snapshot::KEY
            && matches!(self.phase, Phase::Practice { .. } | Phase::Playing);
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            if yaw.is_finite() {
                member.yaw = yaw;
            }
            if pitch.is_finite() {
                member.pitch = if garden_shot { pitch } else { pitch.clamp(-1.4, 1.4) };
            }
            member.fire = true;
        }
    }

    /// A tap for a bout that counts presses, taken on its own tick so a quick
    /// run of them is never merged between ticks.
    pub fn queue_press(&mut self, user_id: &str, press: Press) {
        if self.board_paused(){return;}

        let live = matches!(self.phase, Phase::Practice { .. } | Phase::Playing);
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            if live && member.presses.len() < MAX_QUEUED_PRESSES {
                member.presses.push_back(press);
            }
        }
    }

    /// The host sets how hard the practice bots play.
    pub fn choose_skill(&mut self, user_id: &str, level: &str) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        let found = SKILLS
            .iter()
            .find(|(name, _)| *name == level)
            .ok_or(RoomError::UnknownMinigame)?;
        self.bot_skill = found.1;
        self.rebuild_preview();
        self.dirty = true;
        Ok(())
    }

    /// The host picks the board the next Tawkathon is played on.
    pub fn choose_map(&mut self, user_id: &str, map: Map) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if self.native_mode && map != Map::Frozen { return Err(RoomError::NativeUnsupported); }
        self.map = map;
        self.dirty = true;
        Ok(())
    }

    /// The host sets the lobby up for a whole Tawkathon or a single bout.
    /// The host's choice of playing in pairs.
    pub fn choose_pairs(&mut self, user_id: &str, on: bool) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        self.pairs_on = on;
        self.dirty = true;
        Ok(())
    }

    /// Which pair a seat is in, from 0, by the order they were dealt.
    fn pair_index(&self, seat: u8) -> u8 {
        self.pairs.iter().position(|&(a, b)| a == seat || b == seat).unwrap_or(0) as u8
    }

    /// Deals the seats into pairs at random. An odd one out plays alone.
    fn deal_pairs(&mut self, seats: &[u8]) {
        let mut dealt = seats.to_vec();
        for i in (1..dealt.len()).rev() {
            let j = self.rng.random_range(0..=i);
            dealt.swap(i, j);
        }
        self.pairs = dealt.chunks(2).filter(|pair| pair.len() == 2).map(|pair| (pair[0], pair[1])).collect();
    }

    /// A bout's places, shared by partners: each pair takes the better of its
    /// two, and the order is otherwise kept.
    fn share_places(&self, placements: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        if self.pairs.is_empty() {
            return placements;
        }
        let mut shared: Vec<Vec<u8>> = Vec::new();
        let mut placed: Vec<u8> = Vec::new();
        for tier in placements {
            let mut group = Vec::new();
            for seat in tier {
                if placed.contains(&seat) {
                    continue;
                }
                group.push(seat);
                placed.push(seat);
                if let Some(&(a, b)) = self.pairs.iter().find(|&&(a, b)| a == seat || b == seat) {
                    let partner = if a == seat { b } else { a };
                    if !placed.contains(&partner) {
                        group.push(partner);
                        placed.push(partner);
                    }
                }
            }
            if !group.is_empty() {
                shared.push(group);
            }
        }
        shared
    }

    #[cfg(test)]
    pub fn share_places_for_test(&self, placements: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        self.share_places(placements)
    }

    /// The host's choice of cartoon gore and visual effects.
    pub fn choose_mature(&mut self, user_id: &str, on: bool) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        // In the lobby, or any time in the sandbox, to try it out.
        if self.phase != Phase::Lobby && !self.sandbox {
            return Err(RoomError::WrongPhase);
        }
        self.mature = on;
        if let Some(board) = self.board.as_mut() {
            board.mature = on;
        }
        self.dirty = true;
        Ok(())
    }

    pub fn choose_mode(&mut self, user_id: &str, party: bool) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        self.city_mode=false;self.city=None;self.board=None;
        self.party_mode = party;
        // A Tawkathon is bouts; a card game left picked gives way to the first.
        if party && self.table_spec().is_some() {
            self.minigame = BOUTS[0].key;
            self.rebuild_preview();
        }
        self.dirty = true;
        Ok(())
    }

    pub fn choose_rounds(&mut self,user_id:&str,rounds:u8)->Result<(),RoomError> {
        if !self.is_host(user_id) {return Err(RoomError::NotHost);}
        if self.phase!=Phase::Lobby {return Err(RoomError::WrongPhase);}
        if !(MIN_PARTY_ROUNDS..=MAX_PARTY_ROUNDS).contains(&rounds) {return Err(RoomError::InvalidRounds);}
        self.rounds=rounds;
        self.dirty=true;
        Ok(())
    }

    /// The board a Tawkathon started now would be played on, with everyone at
    /// the start. A fixed seed keeps it from shuffling as people come and go.
    fn preview_board(&self) -> Option<BoardView> {
        if self.phase != Phase::Lobby || (!self.party_mode&&!self.city_mode) {
            return None;
        }
        let seats: Vec<u8> = self.members.iter().map(|m| m.seat).collect();
        Some(Board::for_room(self.map, &seats, u32::from(self.rounds), 0).view_for(None))
    }

    /// A lobby entry made for one game sets the room to it before anyone sits
    /// down, as long as nothing is being played.
    pub fn prefer_game(&mut self, key: &str) {
        if self.phase != Phase::Lobby || self.minigame == key {
            return;
        }
        let found = TABLE_GAMES
            .iter()
            .map(|(k, _)| *k)
            .chain(BOUTS.iter().map(|spec| spec.key))
            .find(|k| *k == key);
        let Some(found) = found else {
            return;
        };
        if bout::find(found).is_some() && self.members.len() > MAX_SEATS {
            return;
        }
        self.minigame = found;
        self.game = None;
        self.rebuild_preview();
        self.dirty = true;
    }

    /// The host picks which bout the lobby plays on its own.
    pub fn choose_game(&mut self, user_id: &str, key: &str) -> Result<(), RoomError> {
        if self.native_mode && !NATIVE_BOUTS.contains(&key) { return Err(RoomError::NativeUnsupported); }
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if let Some(found) = TABLE_GAMES.iter().find(|(k, _)| *k == key) {
            self.minigame = found.0;
            self.game = None;
            // A card game is played on its own, never as a Tawkathon.
            self.party_mode = false;
            self.dirty = true;
            return Ok(());
        }
        let found = bout::find(key).ok_or(RoomError::UnknownMinigame)?;
        if self.members.len() > found.max_players {
            return Err(RoomError::TooManyPlayers);
        }
        self.minigame = found.key;
        self.rebuild_preview();
        self.dirty = true;
        Ok(())
    }

    pub fn pick(&mut self, user_id: &str, character_id: &str) -> Result<(), RoomError> {
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if character(character_id).is_none() {
            return Err(RoomError::UnknownCharacter);
        }
        if self
            .members
            .iter()
            .any(|m| m.character == character_id && m.user_id != user_id)
        {
            return Err(RoomError::Taken);
        }
        let member = self
            .members
            .iter_mut()
            .find(|m| m.user_id == user_id)
            .ok_or(RoomError::WrongPhase)?;
        member.character = character_id.to_string();
        if let Some(costume)=member.costume.as_mut(){costume.color=character_id.to_string();}
        self.dirty = true;
        Ok(())
    }

    /// Only server-validated account state or a validated dev sandbox preview
    /// reaches this method; ordinary socket messages cannot apply outfits.
    pub fn apply_costume(&mut self, user_id: &str, mut costume: crate::roster::Costume) {
        if self.phase != Phase::Lobby { return; }
        let _ = self.pick(user_id, &costume.color);
        if let Some(member)=self.members.iter_mut().find(|m|m.user_id==user_id) {
            costume.color=member.character.clone();
            member.costume=Some(costume);
            self.dirty=true;
        }
    }

    pub fn add_bot(&mut self, user_id: &str) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        let seat = self.free_seat().ok_or(RoomError::Full)?;
        let character = self.free_character();
        let name = crate::roster::character(character)
            .map(|c| c.name)
            .unwrap_or("Bot");
        self.members.push(Member {
            user_id: format!("bot-{seat}-{}", self.tick),
            name: name.to_string(),
            avatar_url: String::new(),
            character: character.to_string(),
            costume: None,
            seat,
            is_bot: true,
            connected: true,
            wins: 0,
            stick: Stick::default(),
            input_tick:0,
            yaw: 0.0,
            pitch: 0.0,
            jump: false,
            crouch: false,
            fire: false,
            presses: VecDeque::new(),
            ready: false,
            away_since: None,
            in_round: false,
        });
        self.members.sort_by_key(|m| m.seat);
        self.rebuild_preview();
        self.dirty = true;
        Ok(())
    }

    pub fn remove_bot(&mut self, user_id: &str, seat: u8) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        self.members.retain(|m| !(m.is_bot && m.seat == seat));
        self.rebuild_preview();
        self.dirty = true;
        Ok(())
    }

    pub fn start(&mut self, user_id: &str) -> Result<(), RoomError> {
        if self.city_mode{return self.start_city(user_id,false);}
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if self.table_spec().is_some() {
            return self.start_table();
        }
        if self.minigame == crate::pet_arena::KEYS[1] {
            while self.members.len() < 8 {
                self.add_bot(user_id)?;
            }
        }
        if self.members.len() < 2 {
            return Err(RoomError::NotEnoughPlayers);
        }
        for member in self.members.iter_mut() {
            member.in_round = true;
        }
        self.rebuild_game();
        self.start_practice();
        Ok(())
    }

    /// A free go at the bout before it counts.
    fn start_practice(&mut self) {
        if self.minigame == crate::pet_arena::KEYS[1] {
            self.phase_ticks = COUNTDOWN_TICKS;
            self.phase = Phase::Countdown {
                seconds: COUNTDOWN_TICKS as f32 / TICK_HZ as f32,
            };
            self.dirty = true;
            return;
        }
        for member in self.members.iter_mut() {
            member.ready = false;
        }
        self.phase_ticks = PRACTICE_TICKS;
        self.phase = Phase::Practice {
            seconds: PRACTICE_TICKS as f32 / TICK_HZ as f32,
        };
        self.dirty = true;
    }

    /// Done practising: once everyone in the round is, the bout counts in.
    pub fn ready(&mut self, user_id: &str) -> Result<(), RoomError> {
        if !matches!(self.phase, Phase::Practice { .. }) {
            return Err(RoomError::WrongPhase);
        }
        let member = self.members.iter_mut().find(|m| m.user_id == user_id).ok_or(RoomError::WrongPhase)?;
        member.ready = true;
        self.dirty = true;
        Ok(())
    }

    /// Everyone's input for a tick of the bout: bots think, people send theirs.
    fn inputs(&mut self) -> Vec<(u8, Input)> {
        let Some(game) = self.game.as_mut() else {
            return Vec::new();
        };
        let mut inputs: Vec<(u8, Input)> = Vec::new();
        for member in self.members.iter_mut().filter(|m| m.in_round) {
            let input = if member.is_bot {
                game.bot_input(member.seat, TICK_DT)
            } else if member.connected {
                Input {
                    stick: member.stick,
                    yaw: member.yaw,
                    pitch: member.pitch,
                    fire: member.fire,
                    jump: member.jump,
                    crouch: member.crouch,
                    press: member.presses.pop_front(),
                }
            } else {
                Input::default()
            };
            member.fire = false;
            inputs.push((member.seat, input));
        }
        inputs
    }

    /// Deals the chosen table game to everyone seated. Bots fill seats and
    /// play as well as the game lets them.
    fn start_table(&mut self) -> Result<(), RoomError> {
        let spec = self.table_spec().ok_or(RoomError::UnknownMinigame)?;
        let mut order: Vec<u8> = self.members.iter().map(|m| m.seat).collect();
        order.sort_unstable();
        if order.len() < spec.min_players {
            return Err(RoomError::NotEnoughPlayers);
        }
        order.truncate(spec.max_players);
        for member in self.members.iter_mut() {
            member.in_round = order.contains(&member.seat);
        }
        self.table = Some((spec.create)(order.len(), self.rng.random()));
        self.table_order = order;
        self.phase = Phase::Table;
        self.bot_think = BOT_THINK_TICKS;
        self.dirty = true;
        Ok(())
    }

    pub fn table_seat_of(&self, user_id: &str) -> Option<u8> {
        let seat = self.seat_of(user_id)?;
        self.table_order.iter().position(|s| *s == seat).map(|i| i as u8)
    }

    pub fn table_act(&mut self, user_id: &str, action: Value) -> Result<(), RoomError> {
        if self.phase != Phase::Table {
            return Err(RoomError::WrongPhase);
        }
        let seat = self.table_seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let game = self.table.as_mut().ok_or(RoomError::WrongPhase)?;
        game.act(seat, action).map_err(RoomError::Table)?;
        self.dirty = true;
        Ok(())
    }

    /// The table as one seat sees it, with who sits where. `None` is a spectator.
    pub fn table_view(&self, seat: Option<u8>) -> Option<(&'static str, Value)> {
        let game = self.table.as_ref()?;
        let mut view = game.view_for(seat);
        let seats: Vec<Value> = self
            .table_order
            .iter()
            .map(|room_seat| {
                let member = self.members.iter().find(|m| m.seat == *room_seat);
                serde_json::json!({
                    "user_id": member.map(|m| m.user_id.clone()),
                    "name": member.map(|m| m.name.clone()).unwrap_or_else(|| "Gone".into()),
                    "avatar_url": member.map(|m| m.avatar_url.clone()),
                    "character": member.map(|m| m.character.clone()),
                    "connected": member.is_some_and(|m| m.connected),
                })
            })
            .collect();
        view["seats"] = Value::Array(seats);
        view["me"] = seat.map(Value::from).unwrap_or(Value::Null);
        Some((game.key(), view))
    }

    fn table_seat_unattended(&self, seat: u8) -> bool {
        self.table_order
            .get(seat as usize)
            .and_then(|room_seat| self.members.iter().find(|m| m.seat == *room_seat))
            .is_none_or(|m| m.is_bot || !m.connected)
    }

    fn step_table(&mut self) {
        let Some(game) = self.table.as_mut() else {
            self.phase = Phase::Lobby;
            return;
        };
        let before = game.log_len();
        game.tick(TICK_DT);
        let pending = game.pending_actors();
        let stalled_by_bots = !pending.is_empty() && pending.iter().all(|s| self.table_seat_unattended(*s));
        let game = self.table.as_mut().expect("checked above");
        if stalled_by_bots {
            self.bot_think = self.bot_think.saturating_sub(1);
            if self.bot_think == 0 {
                game.bot_act();
                self.bot_think = BOT_THINK_TICKS;
            }
        } else {
            self.bot_think = BOT_THINK_TICKS;
        }
        if game.log_len() != before {
            self.dirty = true;
        }
        let Some(winning_seats) = game.winners() else { return };
        let winners: Vec<u8> = winning_seats.iter().map(|s| self.table_order[*s as usize]).collect();
        let losers: Vec<u8> = self.table_order.iter().copied().filter(|s| !winners.contains(s)).collect();
        for member in self.members.iter_mut() {
            if winners.contains(&member.seat) {
                member.wins += 1;
            }
        }
        self.phase_ticks = GAME_OVER_TICKS;
        self.phase = Phase::GameOver {
            seconds: GAME_OVER_TICKS as f32 / TICK_HZ as f32,
            standings: vec![winners, losers],
        };
        self.dirty = true;
    }

    /// Start a Tawkathon: rounds of board turns with a bout after each one.
    pub fn start_party(&mut self, user_id: &str) -> Result<(), RoomError> {
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if self.members.len() < 2 {
            return Err(RoomError::NotEnoughPlayers);
        }
        self.finale_preview=false;
        self.crawler_preview=false;
        for member in self.members.iter_mut() {
            member.in_round = true;
        }
        let seats: Vec<u8> = self.members.iter().map(|m| m.seat).collect();
        let mut board = Board::for_room(self.map, &seats, u32::from(self.rounds), self.rng.random());
        board.timed_items = true;
        board.timed_walk = true;
        board.mature = self.mature;
        board.prepare_opening();
        self.pairs.clear();
        if self.pairs_on && seats.len() >= 4 {
            self.deal_pairs(&seats);
            board.set_partners(&self.pairs);
        }
        self.board = Some(board);
        self.parties += 1;
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        self.party_id = format!("{}-{}-{started}", self.match_id, self.parties);
        self.bouts = 0;
        self.last_bout = None;
        self.rated.clear();
        self.noted.clear();
        self.opening_minigame=true;
        self.open_vote();
        self.bot_think = BOT_THINK_TICKS;
        self.dirty = true;
        Ok(())
    }

    /// Rates the last bout played, once per person who played it.
    /// A rating is one tap; a note can come with it or once after it.
    pub fn prepare_feedback(&self, user_id: &str, rating: u8, note: &str) -> Result<BoutFeedback, RoomError> {
        let note: String = note.trim().chars().take(FEEDBACK_NOTE_MAX).collect();
        let bout = self.last_bout.clone().ok_or(RoomError::NoFeedback)?;
        let place = bout
            .players
            .iter()
            .find(|(player, _)| player == user_id)
            .map(|(_, place)| *place)
            .ok_or(RoomError::NoFeedback)?;
        let (rating, follow_up) = match self.rated.get(user_id) {
            None if (1..=5).contains(&rating) => (rating, false),
            Some(&given) if !note.is_empty() && !self.noted.contains(user_id) => (given, true),
            _ => return Err(RoomError::NoFeedback),
        };
        let name = self
            .members
            .iter()
            .find(|m| m.user_id == user_id)
            .map(|m| m.name.clone())
            .unwrap_or_default();
        Ok(BoutFeedback {
            match_id: self.match_id.clone(),
            party_id: self.party_id.clone(),
            bout,
            user_id: user_id.to_string(),
            name,
            place,
            rating,
            note,
            follow_up,
        })
    }

    /// Called only after the API confirms durable storage. Old in-flight
    /// receipts must never mark a newer bout or a restarted party as rated.
    pub fn commit_feedback(&mut self,feedback:&BoutFeedback){
        if self.party_id!=feedback.party_id||self.last_bout.as_ref().is_none_or(|bout|bout.number!=feedback.bout.number){return;}
        self.rated.insert(feedback.user_id.clone(),feedback.rating);
        if !feedback.note.is_empty(){self.noted.insert(feedback.user_id.clone());}
        self.dirty=true;
    }
    /// Synchronous internal callers/tests; socket submissions use prepare then
    /// commit only after their asynchronous persistence result returns.
    pub fn feedback(&mut self,user_id:&str,rating:u8,note:&str)->Result<BoutFeedback,RoomError>{
        let feedback=self.prepare_feedback(user_id,rating,note)?;self.commit_feedback(&feedback);Ok(feedback)
    }

    /// The round a bout that just finished is rated under. Bouts played on
    /// their own share one ticket until the lobby next starts a Tawkathon.
    fn bout_round(&mut self) -> Option<u32> {
        if let Some(board) = self.board.as_ref() {
            return Some(board.round);
        }
        if self.city_mode {
            return None;
        }
        let prefix = format!("{}:bouts:", self.match_id);
        if !self.party_id.starts_with(&prefix) {
            self.parties += 1;
            self.party_id = format!("{prefix}{}:{}", self.parties, self.rng.random::<u64>());
            self.bouts = 0;
        }
        Some(0)
    }

    /// Whether this person has rated the last bout, so the prompt can go away.
    pub fn has_rated(&self, user_id: &str) -> bool {
        self.rated.contains_key(user_id)
    }

    pub fn seat_of(&self, user_id: &str) -> Option<u8> {
        self.members
            .iter()
            .find(|m| m.user_id == user_id)
            .map(|m| m.seat)
    }

    pub fn board_paused(&self)->bool{self.phase==Phase::Board&&self.board.as_ref().is_some_and(Board::candy_paused)}
    pub fn join_candy_visit(&mut self,user_id:&str)->Result<(),RoomError>{if self.phase!=Phase::Board{return Err(RoomError::WrongPhase);}let seat=self.members.iter().find(|m|m.user_id==user_id&&m.connected).map(|m|m.seat).ok_or(RoomError::WrongPhase)?;self.board.as_mut().ok_or(RoomError::WrongPhase)?.join_candy_visit(seat).map_err(RoomError::Board)?;self.dirty=true;Ok(())}
    pub fn candy_visit_input(&mut self,user_id:&str,x:f32,z:f32)->Result<(),RoomError>{if !self.board_paused(){return Ok(());}let seat=self.members.iter().find(|m|m.user_id==user_id&&m.connected).map(|m|m.seat).ok_or(RoomError::WrongPhase)?;let board=self.board.as_mut().ok_or(RoomError::WrongPhase)?;match board.candy_visit_input(seat,x,z){Ok(())=>{self.dirty=true;Ok(())},Err(BoardError::WrongStep)=>Ok(()),Err(error)=>Err(RoomError::Board(error))}}

    fn board_turn(&mut self, user_id: &str) -> Result<(&mut Board, u8), RoomError> {
        if self.phase != Phase::Board || self.board_paused() {
            return Err(RoomError::WrongPhase);
        }
        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        self.turn_idle = 0;
        let board = self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        Ok((board, seat))
    }

    pub fn use_item(
        &mut self,
        user_id: &str,
        item: Item,
        target: Option<u8>,
        number: Option<u8>,
    ) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        // An off-turn actor must not reset the current player's idle clock.
        if matches!(item,Item::SlashRush|Item::DriveBy) {
            if self.phase!=Phase::Board {return Err(RoomError::WrongPhase);}
            let seat=self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
            self.board.as_mut().ok_or(RoomError::WrongPhase)?.start_raid_item(seat,item).map_err(RoomError::Board)?;
            self.dirty=true;
            return Ok(());
        }
        let (board, seat) = self.board_turn(user_id)?;
        board
            .use_item_with(seat, item, target, number)
            .map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    pub fn position_input(&mut self, user_id: &str, x: f32, z: f32) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board {return Ok(());}
        let Some(seat)=self.seat_of(user_id) else {return Ok(());};
        let Some(board)=self.board.as_mut() else {return Ok(());};
        if board.turn_seat()!=seat || !matches!(board.step(),TurnStep::Position{..}) {return Ok(());}
        board.position_input(seat,x,z).map_err(RoomError::Board)?;
        self.turn_idle=0;
        self.dirty=true;
        Ok(())
    }

    pub fn activate_item(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board,seat)=self.board_turn(user_id)?;
        board.activate_item(seat).map_err(RoomError::Board)?;
        self.fuse_ticks=0;
        self.dirty=true;
        Ok(())
    }

    pub fn cancel_item(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board,seat)=self.board_turn(user_id)?;
        board.cancel_item(seat).map_err(RoomError::Board)?;
        self.dirty=true;
        Ok(())
    }

    pub fn drive_input(&mut self,user_id:&str,throttle:f32,steer:f32,horn:bool)->Result<(),RoomError>{
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board{return Ok(());}let Some(seat)=self.seat_of(user_id)else{return Ok(());};let Some(board)=self.board.as_mut()else{return Ok(());};if !board.has_driver_for(seat){return Ok(());}board.drive_input(seat,throttle,steer,horn).map_err(RoomError::Board)?;self.dirty=true;Ok(())
    }

    pub fn buy_candy(&mut self,user_id:&str)->Result<(),RoomError>{if self.phase!=Phase::Board{return Err(RoomError::WrongPhase);}let seat=self.members.iter().find(|m|m.user_id==user_id&&m.connected).map(|m|m.seat).ok_or(RoomError::WrongPhase)?;self.board.as_mut().ok_or(RoomError::WrongPhase)?.buy_candy(seat).map_err(RoomError::Board)?;self.dirty=true;Ok(())}

    pub fn crawler_input(&mut self,user_id:&str,x:f32,z:f32)->Result<(),RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board {return Ok(());}
        let Some(seat)=self.seat_of(user_id) else{return Ok(());};
        let Some(board)=self.board.as_mut() else{return Ok(());};
        if board.crawler_view().controller!=Some(seat){return Ok(());}
        match board.crawler_input(seat,x,z){Ok(())=>{},Err(BoardError::WrongStep)=>return Ok(()),Err(error)=>return Err(RoomError::Board(error))}
        self.dirty=true;Ok(())
    }

    pub fn raid_fire(&mut self,user_id:&str,on:bool,yaw:f32)->Result<(),RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board{return Ok(());}
        let Some(seat)=self.seat_of(user_id)else{return Ok(());};
        let Some(board)=self.board.as_mut()else{return Ok(());};
        if !board.has_raid_for(seat){return Ok(());}
        match board.raid_fire(seat,on,yaw){Ok(())=>{},Err(BoardError::WrongStep)=>return Ok(()),Err(error)=>return Err(RoomError::Board(error))}
        Ok(())
    }

    pub fn raid_input(&mut self,user_id:&str,x:f32,z:f32,yaw:f32)->Result<(),RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board {return Ok(());}
        let Some(seat)=self.seat_of(user_id) else {return Ok(());};
        let Some(board)=self.board.as_mut() else {return Ok(());};
        if !board.has_raid_for(seat) {return Ok(());}
        board.raid_input(seat,x,z,yaw).map_err(RoomError::Board)?;
        self.dirty=true;
        Ok(())
    }

    pub fn raid_slash(&mut self,user_id:&str)->Result<(),RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase!=Phase::Board {return Ok(());}
        let Some(seat)=self.seat_of(user_id) else {return Ok(());};
        let eligible:Vec<_>=self.members.iter().filter(|member|member.connected).map(|member|member.seat).collect();
        let Some(board)=self.board.as_mut() else {return Ok(());};
        if !board.has_raid_for(seat) {return Ok(());}
        match board.raid_slash(seat,&eligible) {Ok(())=>{},Err(BoardError::WrongStep)=>return Ok(()),Err(error)=>return Err(RoomError::Board(error))}
        self.dirty=true;
        Ok(())
    }

    pub fn pass_items(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.pass_items(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    pub fn roll(&mut self, user_id: &str) -> Result<(), RoomError> {
        if self.phase!=Phase::Board {return Err(RoomError::WrongPhase);}
        let seat=self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let board=self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        board.roll(seat).map_err(RoomError::Board)?;
        self.turn_idle=0;
        self.dirty = true;
        Ok(())
    }

    /// Fire the cannon you are standing in.
    pub fn fire(&mut self, user_id: &str, power: u8, stop: Option<f32>) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        match stop {
            Some(stop) => board.fire_timed(seat, power, stop),
            None => board.fire(seat, power),
        }
        .map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Pull in the coins and a crate lying near you, once a turn.
    pub fn suck(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.suck(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Point the vacuum, turn it on or off, and suck or blow.
    pub fn aim_vacuum(&mut self, user_id: &str, yaw: f32, on: bool, blow: bool) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        let seat=self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        if self.phase!=Phase::Board{return Ok(());}
        let board=self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        if !matches!(board.step(),TurnStep::Vacuum{..}){return Ok(());}
        board.aim_vacuum_with(seat,yaw,on,blow).map_err(RoomError::Board)?;
        self.turn_idle=0;
        Ok(())
    }

    pub fn aim_walk_vacuum(&mut self, user_id: &str, yaw: f32, pitch: f32, on: bool) -> Result<(), RoomError> {
        self.aim_walk_vacuum_with(user_id,yaw,pitch,on,false)
    }

    pub fn aim_walk_vacuum_with(&mut self, user_id: &str, yaw: f32, pitch: f32, on: bool, blow: bool) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        let seat=self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        if self.phase != Phase::Board { return Ok(()); }
        let board=self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        // Aim packets can already be in flight when a walk lands or reaches
        // a junction. Ignore those without resetting the next decision's clock.
        if !matches!(board.step(),TurnStep::Walking) { return Ok(()); }
        board.aim_walk_vacuum_with(seat,yaw,pitch,on,blow).map_err(RoomError::Board)?;
        self.turn_idle=0;
        Ok(())
    }

    /// Cosmetic facing is tied to the authenticated member, with no target
    /// seat or effect on the turn's idle timer.
    pub fn face(&mut self, user_id: &str, yaw: f32) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        // Cosmetic/held input may arrive after a sandbox exit or a phase change.
        if self.phase != Phase::Board { return Ok(()); }
        self.board.as_mut().ok_or(RoomError::WrongPhase)?.face(seat, yaw).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Hold on against the vacuum draining you. Bracing is not your turn, so
    /// it leaves the turn's idle timer alone.
    pub fn brace(&mut self, user_id: &str, on: bool) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        if self.phase != Phase::Board {
            return if on {Err(RoomError::WrongPhase)}else{Ok(())};
        }
        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let board = self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        match board.brace(seat,on) {Err(BoardError::WrongStep) if !on=>Ok(()),result=>result.map_err(RoomError::Board)}
    }

    /// Put the vacuum away.
    pub fn stow_vacuum(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.stow_vacuum(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Throw the snowball you are holding.
    pub fn throw_snowball(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.throw_snowball(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Throw the crate you are holding, to make a barricade.
    pub fn throw_crate(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.throw_crate(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Stop the skill check of the item in hand.
    pub fn check(&mut self, user_id: &str, stop: f32) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.check(seat, stop).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Swing the bat in hand.
    pub fn swing(&mut self, user_id: &str, yaw: f32, stop: f32) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.swing(seat, yaw, stop).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Throw whatever is in hand.
    pub fn lob(&mut self, user_id: &str, yaw: f32, pitch: f32, power: f32) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.lob(seat, yaw, pitch, power).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Buy one of the shop's items or vacuum upgrades, or walk on when neither is given.
    pub fn shop(&mut self, user_id: &str, item: Option<Item>, upgrade: Option<Upgrade>) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.shop_with(seat, item, upgrade).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// Stake coins on yourself while the ballot is open.
    pub fn bet(&mut self, user_id: &str, coins: i32) -> Result<(), RoomError> {
        if self.city_mode{return Err(RoomError::WrongPhase);}
        if !matches!(self.phase, Phase::Vote { .. }) {
            return Err(RoomError::WrongPhase);
        }
        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let board = self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        board.bet(seat, coins).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    /// True when the pet reacts; false while its last reaction is still playing.
    pub fn pet_react(&mut self, user_id: &str, action: crate::pets::PetAction) -> Result<bool, RoomError> {
        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let fresh = self.pets.react(seat, action, self.tick);
        self.dirty |= fresh;
        Ok(fresh)
    }

    /// A shout from the sidelines. Anyone in the party can, whoever's turn
    /// it is, as long as they have not just shouted.
    pub fn taunt(&mut self, user_id: &str, taunt: Taunt) -> Result<(), RoomError> {
        if self.board_paused(){return Ok(());}

        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        if let Some(last) = self.taunted.get(&seat) {
            if self.tick.saturating_sub(*last) < TAUNT_EVERY {
                return Ok(());
            }
        }
        let Some(board) = self.board.as_mut() else {
            return Ok(());
        };
        board.taunt(seat, taunt);
        self.taunted.insert(seat, self.tick);
        self.dirty = true;
        Ok(())
    }

    pub fn choose_branch(&mut self, user_id: &str, node: u8) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.choose_branch(seat, node).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    pub fn end_turn(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        if board.turn_seat() != seat {
            return Err(RoomError::Board(BoardError::NotYourTurn));
        }
        if *board.step() != TurnStep::Done {
            return Err(RoomError::Board(BoardError::WrongStep));
        }
        self.finish_board_turn();
        Ok(())
    }

    /// Hand the turn on, and put the round's bout to a vote once everyone has gone.
    fn finish_board_turn(&mut self) {
        let round_over = self
            .board
            .as_mut()
            .map(|board| board.end_turn())
            .unwrap_or(false);
        self.bot_think = BOT_THINK_TICKS;
        self.turn_idle = 0;
        self.turn_done = 0;
        self.dirty = true;
        if round_over && self.sandbox {
            // No bouts in the sandbox: straight on to the next round.
            if let Some(board) = self.board.as_mut() {
                board.finish_round(&[]);
            }
        } else if round_over {
            if self.board.as_ref().is_some_and(|board|board.round>=board.rounds) {self.open_map_finale(false);}else{self.open_vote();}
        }
    }

    /// The sandbox: the host and a few bots on the chosen board, for as many
    /// rounds as they like, with no bouts.
    pub fn start_sandbox(&mut self, user_id: &str) -> Result<(), RoomError> {
        self.check_start_sandbox(user_id)?;
        self.finale_preview=false;
        self.opening_minigame=false;
        self.crawler_preview=false;
        while self.members.len() < 4 && self.add_bot(user_id).is_ok() {}
        for member in self.members.iter_mut() {
            member.in_round = true;
        }
        let seats: Vec<u8> = self.members.iter().map(|m| m.seat).collect();
        let mut board = Board::for_room(self.map, &seats, SANDBOX_ROUNDS, self.rng.random());
        board.timed_items = true;
        board.timed_walk = true;
        board.mature = self.mature;
        board.scatter_starts();
        board.endless_vacuum = true;
        board.use_manual_events();
        if let Some(seat) = self.seat_of(user_id) {
            board.dev_first(seat);
        }
        self.board = Some(board);
        self.sandbox = true;
        self.phase = Phase::Board;
        self.turn_idle = 0;
        self.turn_done = 0;
        self.dirty = true;
        Ok(())
    }

    pub fn check_start_sandbox(&self, user_id: &str) -> Result<(), RoomError> {
        if !self.dev_tools && !self.native_mode {
            return Err(RoomError::NoDevTools);
        }
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        Ok(())
    }

    pub fn start_sandbox_with_preview(&mut self, user_id: &str, costume: crate::roster::Costume, wardrobe: Option<&crate::roster::Wardrobe>) -> Result<(), RoomError> {
        self.check_start_sandbox(user_id)?;
        crate::roster::validate_sandbox_costume(&costume, wardrobe).map_err(RoomError::InvalidCostume)?;
        let mut costume = costume;
        costume.jersey = wardrobe.and_then(|w| w.jersey.clone()).filter(|_| costume.shirt.as_deref() == Some(crate::roster::PUCKLAB_JERSEY));
        (costume.pet.name, costume.pet.stage) = (None, None);
        let member = self.members.iter().find(|member| member.user_id == user_id).ok_or(RoomError::WrongPhase)?;
        let original = (user_id.to_string(), member.character.clone(), member.costume.clone());
        self.apply_costume(user_id, costume);
        self.start_sandbox(user_id)?;
        self.sandbox_costume = Some(original);
        Ok(())
    }

    /// Free-roam Cops & Robbers for the host: no night clock, no winner,
    /// and the city tools in `dev`. Gated like the board sandbox.
    pub fn start_city_sandbox(&mut self, user_id: &str, police: bool) -> Result<(), RoomError> {
        self.check_start_sandbox(user_id)?;
        self.choose_city(user_id, 0, 3, police)?;
        while self.members.len() < 4 && self.add_bot(user_id).is_ok() {}
        self.start_city(user_id, false)?;
        if let Some(city) = self.city.as_mut() {
            city.begin_sandbox();
        }
        self.sandbox = true;
        self.dirty = true;
        Ok(())
    }

    /// Back to the lobby from any sandbox, keeping the seats and the sandbox
    /// costume, so another sandbox can start at once.
    fn leave_sandbox_for_lobby(&mut self) {
        self.city = None;
        self.city_mode = false;
        self.board = None;
        self.game = None;
        self.table = None;
        self.finale_preview = false;
        self.crawler_preview = false;
        self.opening_minigame = false;
        self.phase = Phase::Lobby;
        self.members.retain(|m| !m.is_bot || m.seat < 4);
        self.dirty = true;
    }

    /// A seat for a sandbox dummy, with a bot member so it has a name.
    fn sandbox_bot_seat(&mut self) -> Option<u8> {
        let seat = self.free_seat()?;
        let character = self.free_character();
        let name = crate::roster::character(character).map(|c| c.name).unwrap_or("Bot");
        self.members.push(Member {
            user_id: format!("bot-{seat}-{}", self.tick),
            name: name.to_string(),
            avatar_url: String::new(),
            character: character.to_string(),
            costume: None,
            seat,
            is_bot: true,
            connected: true,
            wins: 0,
            stick: Stick::default(),
            input_tick: 0,
            yaw: 0.0,
            pitch: 0.0,
            jump: false,
            crouch: false,
            fire: false,
            presses: VecDeque::new(),
            ready: false,
            away_since: None,
            in_round: true,
        });
        self.members.sort_by_key(|m| m.seat);
        Some(seat)
    }

    fn city_sandbox_tool(&mut self, seat: u8, tool: crate::city_heist::CityTool) -> Result<(), RoomError> {
        use crate::city_heist::CityTool;
        if self.phase != Phase::CityHeist {
            return Err(RoomError::WrongPhase);
        }
        let done = match tool {
            CityTool::Spawn { police, behavior } => {
                let new = self.sandbox_bot_seat().ok_or(RoomError::Full)?;
                let city = self.city.as_mut().ok_or(RoomError::WrongPhase)?;
                city.sandbox_spawn(new, seat, police, behavior)
            }
            CityTool::Despawn => {
                let city = self.city.as_mut().ok_or(RoomError::WrongPhase)?;
                let Some(last) = city.sandbox_spawned().last().copied() else {
                    return Ok(());
                };
                city.sandbox_despawn(last);
                self.members.retain(|m| !(m.is_bot && m.seat == last));
                true
            }
            CityTool::Traffic { level } => {
                self.board.as_mut().ok_or(RoomError::WrongPhase)?.set_traffic_level(level);
                true
            }
            tool => {
                let (Some(city), Some(board)) = (self.city.as_mut(), self.board.as_mut()) else {
                    return Err(RoomError::WrongPhase);
                };
                match tool {
                    CityTool::Role { police } => city.sandbox_role(seat, police),
                    CityTool::Behave { behavior } => {
                        city.sandbox_behave(behavior);
                        true
                    }
                    CityTool::Alarm { on } => {
                        city.set_alarm(on);
                        true
                    }
                    CityTool::Teleport { place } => city.sandbox_teleport(seat, place, board),
                    CityTool::ResetCar => city.sandbox_reset_car(seat, board),
                    CityTool::Arrest => city.sandbox_arrest(seat),
                    CityTool::Station { station } => city.sandbox_station(seat, station, board),
                    CityTool::Give { item } => city.sandbox_give(seat, item),
                    CityTool::Flat => city.sandbox_flat(seat),
                    CityTool::Skill { level } => {
                        city.set_bot_skill(level.value());
                        true
                    }
                    CityTool::Spawn { .. } | CityTool::Despawn | CityTool::Traffic { .. } => false,
                }
            }
        };
        if done {
            self.dirty = true;
        }
        Ok(())
    }

    /// The sandbox's tools, for whoever is in it.
    pub fn dev(&mut self, user_id: &str, action: DevAction) -> Result<(), RoomError> {
        if self.board_paused()&&action!=DevAction::Exit{return Err(RoomError::WrongPhase);}

        if !self.sandbox {
            return Err(RoomError::NoDevTools);
        }
        let sandbox_access = self.dev_tools || (self.native_mode && self.is_host(user_id));
        match &action {
            DevAction::City { tool } => {
                if !sandbox_access { return Err(RoomError::NoDevTools); }
                let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
                return self.city_sandbox_tool(seat, *tool);
            }
            DevAction::CitySandbox { police } => {
                if !sandbox_access { return Err(RoomError::NoDevTools); }
                if !self.is_host(user_id) { return Err(RoomError::NotHost); }
                let police = *police;
                self.leave_sandbox_for_lobby();
                return self.start_city_sandbox(user_id, police);
            }
            DevAction::BoardSandbox => {
                if !sandbox_access { return Err(RoomError::NoDevTools); }
                if !self.is_host(user_id) { return Err(RoomError::NotHost); }
                self.leave_sandbox_for_lobby();
                return self.start_sandbox(user_id);
            }
            DevAction::Bout { key } => {
                if !sandbox_access { return Err(RoomError::NoDevTools); }
                if !self.is_host(user_id) { return Err(RoomError::NotHost); }
                let key = BOUTS.iter().find(|spec| spec.key == key.as_str()).map(|spec| spec.key).ok_or(RoomError::NativeUnsupported)?;
                self.leave_sandbox_for_lobby();
                self.choose_game(user_id, key)?;
                while self.members.len() < 4 && self.add_bot(user_id).is_ok() {}
                self.start(user_id)?;
                self.sandbox = true;
                return Ok(());
            }
            _ => {}
        }
        if action == DevAction::CandyVan {
            if !sandbox_access { return Err(RoomError::NoDevTools); }
            if !self.is_host(user_id) { return Err(RoomError::NotHost); }
            if self.phase != Phase::Board { return Err(RoomError::WrongPhase); }
            self.board.as_mut().ok_or(RoomError::WrongPhase)?.summon_candy_van().map_err(RoomError::Board)?;
            self.dirty = true;
            return Ok(());
        }
        if action==DevAction::Crawler {
            if !sandbox_access{return Err(RoomError::NoDevTools);}
            if !self.is_host(user_id){return Err(RoomError::NotHost);}
            if self.phase!=Phase::Board{return Err(RoomError::WrongPhase);}
            let seat=self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
            let board=self.board.as_mut().ok_or(RoomError::WrongPhase)?;
            if self.crawler_preview&&board.crawler_view().controller==Some(seat){board.end_crawler_preview(seat);self.crawler_preview=false;}
            else {board.dev_crawler_controller(seat);self.crawler_preview=true;}
            self.dirty=true;return Ok(());
        }
        if action==DevAction::Finale {
            if !sandbox_access {return Err(RoomError::NoDevTools);}
            if !self.is_host(user_id) {return Err(RoomError::NotHost);}
            if self.phase!=Phase::Board {return Err(RoomError::WrongPhase);}
            self.open_map_finale(true);
            return Ok(());
        }
        if action == DevAction::Exit {
            if !self.is_host(user_id) {
                return Err(RoomError::NotHost);
            }
            self.sandbox = false;
            self.opening_minigame=false;
            self.finale_preview=false;
            self.crawler_preview=false;
            self.board = None;
            self.phase = Phase::Lobby;
            if let Some((user_id, character, costume)) = self.sandbox_costume.take() {
                if let Some(member) = self.members.iter_mut().find(|member| member.user_id == user_id) {
                    member.character = character;
                    member.costume = costume;
                }
            }
            self.dirty = true;
            return Ok(());
        }
        let seat = self.seat_of(user_id).ok_or(RoomError::WrongPhase)?;
        let board = self.board.as_mut().ok_or(RoomError::WrongPhase)?;
        match action {
            DevAction::Give { item } => board.dev_give(seat, item),
            DevAction::Coins { coins } => board.dev_coins(seat, coins),
            DevAction::Heal => board.dev_heal(seat),
            DevAction::ZombieKill { seat: who } => board.dev_zombie_kill(who.unwrap_or(seat)),
            DevAction::Prize { seat: who } => board.dev_prize(who.unwrap_or(seat)),
            // Your own jump lands you there as a walk would; a token dragged about only moves.
            DevAction::Teleport { space, seat: who } => board.dev_place(who.unwrap_or(seat), space, who.is_none()),
            DevAction::Exit => {}
            DevAction::Finale => {}
            DevAction::Crawler => {}
            DevAction::CandyVan => {}
            DevAction::City { .. } | DevAction::CitySandbox { .. } | DevAction::BoardSandbox | DevAction::Bout { .. } => {}
        }
        self.dirty = true;
        Ok(())
    }

    /// Practice bots take their board turns a step at a time, and so does a
    /// seat whose player has wandered off or run out of time.
    fn step_board_bots(&mut self) {
        if self.phase != Phase::Board {
            return;
        }
        let Some(board) = self.board.as_ref() else {
            return;
        };
        let seat = board.turn_seat();
        if matches!(board.step(),TurnStep::Raiding) {self.fuse_ticks=0;return;}
        if matches!(board.step(),TurnStep::Driving) {let automatic=self.members.iter().any(|m|m.seat==seat&&(m.is_bot||!m.connected));if automatic {if let Some(board)=self.board.as_mut(){board.bot_drive_tick();}}return;}
        if matches!(board.step(),TurnStep::Position {..}) {
            self.fuse_ticks=0;
            let automatic=self.members.iter().any(|member|member.seat==seat && (member.is_bot || !member.connected)) || self.turn_idle >= TURN_LIMIT_TICKS;
            if automatic {if let Some(board)=self.board.as_mut() {board.bot_position_tick();}}
            return;
        }
        if matches!(board.step(),TurnStep::Walking) {
            let bot=self.members.iter().any(|m|m.seat==seat && (m.is_bot || !m.connected));
            if bot && self.tick%3==0 {
                if let Some(board)=self.board.as_mut() {board.bot_walk_aim(self.bot_skill);}
            }
            return;
        }
        if *board.step() == TurnStep::Done {
            // Nothing left to decide, so the turn hands itself on.
            // A person who has not had their vacuum out gets a moment to
            // take it out once they have landed.
            let person = self.members.iter().any(|m| m.seat == seat && !m.is_bot && m.connected);
            let offer = if person && !board.view().sucked { VACUUM_OFFER_TICKS } else { 0 };
            let wait = if board.timed_walk { LANDED_TICKS + offer } else { DIE_TICKS + HOP_TICKS * board.last_roll().unwrap_or(0) as u32 + LANDED_TICKS + offer };
            self.turn_done += 1;
            if self.turn_done >= wait {
                self.finish_board_turn();
            }
            return;
        }
        // A bomb held too long goes off in the hand, in the sandbox too.
        if matches!(board.step(), TurnStep::Lob { .. } | TurnStep::Swing { .. } | TurnStep::Check { .. }) {
            self.fuse_ticks += 1;
            if self.fuse_ticks as f32 * TICK_DT >= crate::board::BOMB_FUSE {
                self.fuse_ticks = 0;
                if let Some(board) = self.board.as_mut() {
                    board.fuse_out(seat);
                }
                self.dirty = true;
                return;
            }
        } else {
            self.fuse_ticks = 0;
        }
        let is_bot = self.members.iter().any(|m| m.seat == seat && m.is_bot);
        if self.sandbox&&self.crawler_preview&&is_bot&&board.crawler_view().controller.is_some(){return;}
        if self.sandbox {
            // Bots stand where they are, as targets, and a person takes as
            // long as they like.
            if is_bot {
                self.finish_board_turn();
            }
            return;
        }
        let away = self
            .members
            .iter()
            .any(|m| m.seat == seat && !m.is_bot && !m.connected);
        if !(is_bot || away) {
            self.turn_idle += 1;
            if self.turn_idle < TURN_LIMIT_TICKS {
                return;
            }
        }
        if self.bot_think > 0 {
            self.bot_think -= 1;
            return;
        }
        self.bot_think = BOT_THINK_TICKS;
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let action = board.bot_move();
        #[cfg(test)]
        let logged = action.clone();
        let result = match action {
            BotMove::Walk => Ok(()),
            BotMove::Item {
                item,
                target,
                number,
            } => board.use_item_with(seat, item, target, number),
            BotMove::Pass => board.pass_items(seat),
            BotMove::Roll => board.roll(seat).map(|_| ()),
            BotMove::Branch { node } => board.choose_branch(seat, node),
            BotMove::Shop { item, upgrade } => board.shop_with(seat, item, upgrade),
            BotMove::Fire { power } => board.fire(seat, power),
            BotMove::Lob { yaw, pitch, power } => board.lob(seat, yaw, pitch, power),
            BotMove::Swing { yaw, stop } => board.swing(seat, yaw, stop),
            BotMove::Check { stop } => board.check(seat, stop),
            BotMove::Suck => board.suck(seat),
            BotMove::Vacuum { yaw, on, blow } => board.aim_vacuum_with(seat, yaw, on, blow),
            BotMove::StowVacuum => board.stow_vacuum(seat),
            BotMove::ThrowCrate => board.throw_crate(seat),
            BotMove::ThrowSnowball => board.throw_snowball(seat),
            BotMove::EndTurn => {
                #[cfg(test)]
                self.bot_log.push((seat, logged, Ok(())));
                self.finish_board_turn();
                return;
            }
        };
        #[cfg(test)]
        self.bot_log.push((seat, logged, result));
        #[cfg(not(test))]
        let _ = result;
        self.dirty = true;
    }

    fn contestants(&self, all: bool) -> Vec<Contestant> {
        self.members
            .iter()
            .filter(|m| all || m.in_round)
            .enumerate()
            .map(|(index, m)| Contestant {
                seat: m.seat,
                is_bot: m.is_bot,
                skill: if m.is_bot { self.bot_skill } else { 1.0 },
                // The board picks the sides for a team bout; without one, fall
                // back to alternating, which is what the bouts used to do.
                team: self
                    .teams
                    .iter()
                    .find(|(seat, _)| *seat == m.seat)
                    .map(|(_, team)| *team)
                    .unwrap_or((index % 2) as u8),
                // In pairs, partners are allies even in a free-for-all.
                ally: self.pairs.iter().position(|&(a, b)| a == m.seat || b == m.seat).map(|index| index as u8),
                body_width: m.costume.as_ref().and_then(|costume| costume.body_width)
                    .filter(|width| width.is_finite() && (0.7..=1.4).contains(width))
                    .unwrap_or(1.0),
                pet: crate::pets::species_for(&m.user_id, m.costume.as_ref().and_then(|costume| costume.pet.id.as_deref())),
            })
            .collect()
    }

    /// Sides for the bout about to start, when it is played in teams.
    fn pick_teams(&mut self) {
        self.teams.clear();
        let Some(spec) = BOUTS.iter().find(|spec| spec.key == self.minigame) else {
            return;
        };
        if spec.format != bout::Format::Teams {
            return;
        }
        let seats: Vec<u8> = self.members.iter().filter(|m| m.in_round).map(|m| m.seat).collect();
        if seats.len() < 2 {
            return;
        }
        if let Some(board) = self.board.as_mut() {
            self.teams = board.team_split(&seats);
        }
    }

    /// In the lobby the arena shows everyone on their starting spots.
    fn rebuild_preview(&mut self) {
        if self.table_spec().is_some() {
            self.game = None;
            return;
        }
        if self.phase == Phase::Lobby {
            let contestants = self.contestants(true);
            self.game = bout::create(self.minigame, &contestants, self.rng.random());
            if let Some(game) = self.game.as_mut() { game.rotate_lead(&self.leads); }
        }
    }

    fn rebuild_game(&mut self) {
        self.pick_teams();
        let contestants = self.contestants(false);
        self.game = bout::create(self.minigame, &contestants, self.rng.random());
        if let Some(game) = self.game.as_mut() { game.rotate_lead(&self.leads); }
        for member in &mut self.members {
            member.presses.clear();
        }
    }

    /// The ballot holds a few random bouts that fit the round, leaving out the
    /// one just played when there are enough others.
    fn open_vote(&mut self) {
        let players = self.members.iter().filter(|m| m.in_round).count();
        let mut pool: Vec<&'static str> = BOUTS
            .iter()
            .filter(|spec| spec.fits(players) && (!self.native_mode || NATIVE_BOUTS.contains(&spec.key)))
            .map(|spec| spec.key)
            .collect();
        if pool.len() > VOTE_OPTIONS {
            pool.retain(|key| *key != self.minigame);
        }
        let mut options = Vec::new();
        while options.len() < VOTE_OPTIONS && !pool.is_empty() {
            options.push(pool.swap_remove(self.rng.random_range(0..pool.len())));
        }
        if options.is_empty() {
            options.push(self.minigame);
        }
        self.phase_ticks = VOTE_TICKS;
        self.phase = Phase::Vote {
            seconds: VOTE_TICKS as f32 / TICK_HZ as f32,
            options,
            votes: Vec::new(),
        };
        self.dirty = true;
    }

    fn open_map_finale(&mut self,preview:bool) {
        let seats:Vec<_>=self.members.iter().filter(|member|member.in_round).map(|member|member.seat).collect();
        let eligible:Vec<_>=self.members.iter().filter(|member|member.in_round&&member.connected).map(|member|member.seat).collect();
        if let Some(board)=self.board.as_mut() {board.start_map_finale(&seats,&eligible);}
        for member in &mut self.members {member.stick=Stick::default();member.fire=false;member.input_tick=0;}
        self.finale_preview=preview;self.game=None;self.last_bout=None;self.phase=Phase::MapFinale;self.dirty=true;
    }

    fn step_map_finale(&mut self) {
        let eligible:Vec<_>=self.members.iter().filter(|member|member.connected&&member.in_round).map(|member|member.seat).collect();
        let mut inputs:Vec<_>=self.members.iter().filter(|member|!member.is_bot&&member.connected&&member.in_round).map(|member|(member.seat,if self.tick.saturating_sub(member.input_tick)<=TICK_HZ as u64*35/100 {member.stick}else{Stick::default()})).collect();
        let bots:Vec<_>=self.members.iter().filter(|member|member.is_bot&&member.connected&&member.in_round).map(|member|member.seat).collect();
        let Some(board)=self.board.as_mut() else{return;};
        board.traffic_tick(TICK_DT);board.pedestrians_tick(TICK_DT);
        for seat in bots {inputs.push((seat,board.finale_bot_input(seat)));}
        let result=board.finale_tick(TICK_DT,&inputs,&eligible);
        if self.tick%3==0 {self.dirty=true;}
        if let Some(placements)=result {
            if !self.finale_preview {
                if let Some(winners)=placements.first() {for member in &mut self.members {if winners.contains(&member.seat) {member.wins+=1;}}}
            }
            self.phase_ticks=RESULTS_TICKS;
            self.phase=Phase::Results{seconds:RESULTS_TICKS as f32/TICK_HZ as f32,placements};
            self.dirty=true;
        }
    }

    pub fn vote(&mut self, user_id: &str, key: &str) -> Result<(), RoomError> {
        let seat = self
            .members
            .iter()
            .find(|m| m.user_id == user_id && m.in_round)
            .map(|m| m.seat)
            .ok_or(RoomError::WrongPhase)?;
        self.cast_vote(seat, key)
    }

    fn cast_vote(&mut self, seat: u8, key: &str) -> Result<(), RoomError> {
        let Phase::Vote { options, votes, .. } = &mut self.phase else {
            return Err(RoomError::WrongPhase);
        };
        let key = *options
            .iter()
            .find(|option| **option == key)
            .ok_or(RoomError::UnknownMinigame)?;
        votes.retain(|vote| vote.seat != seat);
        votes.push(BoutVote { seat, key });
        self.dirty = true;
        Ok(())
    }

    fn step_vote(&mut self) {
        self.phase_ticks = self.phase_ticks.saturating_sub(1);
        if self.phase_ticks == VOTE_TICKS - BOT_VOTE_TICKS {
            self.cast_bot_votes();
        }
        let Phase::Vote { options, votes, .. } = self.phase.clone() else {
            return;
        };
        let everyone_in = self
            .members
            .iter()
            .filter(|m| m.in_round && (m.is_bot || m.connected))
            .all(|m| votes.iter().any(|v| v.seat == m.seat));
        if self.phase_ticks == 0 || everyone_in {
            self.minigame = self.tally(&options, &votes);
            self.rebuild_game();
            self.start_practice();
        } else if self.phase_ticks % TICK_HZ == 0 {
            self.phase = Phase::Vote {
                seconds: self.phase_ticks as f32 / TICK_HZ as f32,
                options,
                votes,
            };
            self.dirty = true;
        }
    }

    fn cast_bot_votes(&mut self) {
        let Phase::Vote { options, votes, .. } = &self.phase else {
            return;
        };
        let options = options.clone();
        let bots: Vec<u8> = self
            .members
            .iter()
            .filter(|m| m.in_round && m.is_bot && !votes.iter().any(|v| v.seat == m.seat))
            .map(|m| m.seat)
            .collect();
        for seat in bots {
            let key = options[self.rng.random_range(0..options.len())];
            let _ = self.cast_vote(seat, key);
        }
    }

    /// Most votes wins; a tie, or no votes at all, is settled at random.
    fn tally(&mut self, options: &[&'static str], votes: &[BoutVote]) -> &'static str {
        let count = |key: &str| votes.iter().filter(|v| v.key == key).count();
        let best = options.iter().map(|k| count(k)).max().unwrap_or(0);
        let leaders: Vec<&'static str> = options.iter().copied().filter(|k| count(k) == best).collect();
        leaders[self.rng.random_range(0..leaders.len())]
    }

    /// Advance one tick. Returns the minigame events from this tick.
    pub fn step(&mut self) -> Vec<Value> {
        self.tick += 1;
        let mut events = Vec::new();
        self.pick_host();

        if self.phase == Phase::Lobby {
            let cutoff = self.tick.saturating_sub(LOBBY_DROP_TICKS);
            let before = self.members.len();
            self.members
                .retain(|m| m.is_bot || m.away_since.is_none_or(|t| t > cutoff));
            if self.members.len() != before {
                self.pick_host();
                // Bots stay only while a person is around to play with them.
                if !self.members.iter().any(|m| !m.is_bot) {
                    self.members.clear();
                }
                self.rebuild_preview();
                self.dirty = true;
            }
        }

        if self.board_paused(){
            let eligible:Vec<_>=self.members.iter().filter(|m|m.connected).map(|m|m.seat).collect();
            if let Some(board)=self.board.as_mut(){let old=board.candy_van_view().map(|v|v.phase);let changed=board.candy_visit_tick(TICK_DT,&eligible);if self.tick%3==0||!board.candy_paused()||old!=board.candy_van_view().map(|v|v.phase){self.dirty=true;}let _=changed;}
            return events;
        }
        match self.phase.clone() {
            Phase::Lobby => {}
            Phase::MapFinale => self.step_map_finale(),
            Phase::CityHeist => self.step_city(),
            Phase::Board => {
                // The vacuum runs every tick; the room is sent a few times a
                // second while it does, so the pull shows.
                if let Some(board) = self.board.as_mut() {
                    board.begin_crawler_tick();
                    if board.pedestrians_tick(TICK_DT) && self.tick%3==0 {self.dirty=true;}
                    let eligible:Vec<_>=self.members.iter().filter(|member|member.connected).map(|member|member.seat).collect();
                    let bots:Vec<_>=self.members.iter().filter(|member|member.connected && member.is_bot).map(|member|member.seat).collect();
                    board.crawler_control_presence(&eligible,&bots);
                    if board.traffic_tick(TICK_DT) || (board.traffic_view().is_some() && self.tick%6==0) {self.dirty=true;}
                    if board.drive_tick(TICK_DT,&eligible) || (!board.driving_view().is_empty() && self.tick%3==0){self.dirty=true;}
                    let raiding=board.has_raids();
                    let raided=board.raids_tick(TICK_DT,&eligible);
                    if self.tick%3==0 {board.bot_raids_tick(&bots,&eligible);}
                    if raided || (raiding && self.tick%3==0) {self.dirty=true;}
                    let positioning=matches!(board.step(),TurnStep::Position {..});
                    let positioned=board.position_tick(TICK_DT);
                    if positioned || (positioning && self.tick%3==0) {self.dirty=true;}
                    let walking=matches!(board.step(),TurnStep::Walking);
                    let walked=board.walk_tick(TICK_DT);
                    if walked || (walking && self.tick%3==0) {self.dirty=true;}
                    if board.crawler_tick(TICK_DT) || self.tick%6==0 {self.dirty=true;}
                    let changed = board.vacuum_tick(TICK_DT);
                    if changed || (matches!(board.step(), TurnStep::Vacuum { .. }) && self.tick % 6 == 0) {
                        self.dirty = true;
                    }
                    // A practice bot being drained braces, the better it plays the sooner.
                    if self.tick % 10 == 0 && matches!(board.step(), TurnStep::Vacuum { .. }) {
                        let drained = board.view().vacuum.map(|v| v.siphoning).unwrap_or_default();
                        for member in self.members.iter().filter(|m| m.is_bot && drained.contains(&m.seat)) {
                            if self.rng.random_bool(f64::from(self.bot_skill.clamp(0.1, 0.9))) {
                                let _ = board.brace(member.seat, true);
                            }
                        }
                    }
                }
                self.step_board_bots();
            }
            Phase::Vote { .. } => self.step_vote(),
            Phase::Table => self.step_table(),
            Phase::GameOver { .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                if self.phase_ticks == 0 {
                    self.board = None;self.city=None;
                    self.table = None;
                    self.table_order.clear();
                    for member in self.members.iter_mut() {
                        member.in_round = false;
                    }
                    self.phase = Phase::Lobby;
                    self.rebuild_preview();
                    self.dirty = true;
                } else if self.phase_ticks % TICK_HZ == 0 {
                    if let Phase::GameOver { standings, .. } = self.phase.clone() {
                        self.phase = Phase::GameOver {
                            seconds: self.phase_ticks as f32 / TICK_HZ as f32,
                            standings,
                        };
                        self.dirty = true;
                    }
                }
            }
            Phase::Practice { .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                let inputs = self.inputs();
                let mut over = false;
                if let Some(game) = self.game.as_mut() {
                    game.step(&inputs, TICK_DT, &mut events);
                    over = game.placements().is_some();
                }
                // Finished in practice: it starts over, still not counting.
                if over {
                    self.rebuild_game();
                    self.dirty = true;
                }
                let everyone_ready = self
                    .members
                    .iter()
                    .filter(|m| m.in_round && !m.is_bot && m.connected && !self.city.as_ref().is_some_and(|city|city.automated(m.seat)))
                    .all(|m| m.ready);
                if self.phase_ticks == 0 || everyone_ready {
                    // A fresh start for the bout that counts.
                    self.rebuild_game();
                    self.phase_ticks = COUNTDOWN_TICKS;
                    self.phase = Phase::Countdown {
                        seconds: COUNTDOWN_TICKS as f32 / TICK_HZ as f32,
                    };
                    self.dirty = true;
                } else if self.phase_ticks % TICK_HZ == 0 {
                    self.phase = Phase::Practice {
                        seconds: self.phase_ticks as f32 / TICK_HZ as f32,
                    };
                    self.dirty = true;
                }
            }
            Phase::Countdown { .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                if self.phase_ticks == 0 {
                    self.leads.extend(self.game.as_ref().and_then(|g| g.lead()));
                    self.phase = Phase::Playing;
                    self.dirty = true;
                } else if self.phase_ticks % TICK_HZ == 0 {
                    self.phase = Phase::Countdown {
                        seconds: self.phase_ticks as f32 / TICK_HZ as f32,
                    };
                    self.dirty = true;
                }
            }
            Phase::Playing => {
                if let Some(game) = self.game.as_mut() {
                    let mut inputs: Vec<(u8, Input)> = Vec::new();
                    for member in self.members.iter_mut().filter(|m| m.in_round) {
                        let input = if member.is_bot || self.city.as_ref().is_some_and(|city|city.automated(member.seat)) {
                            game.bot_input(member.seat, TICK_DT)
                        } else if member.connected {
                            Input {
                                stick: member.stick,
                                yaw: member.yaw,
                                pitch: member.pitch,
                                fire: member.fire,
                                jump: member.jump,
                                crouch: member.crouch,
                                press: member.presses.pop_front(),
                            }
                        } else {
                            Input::default()
                        };
                        member.fire = false;
                        inputs.push((member.seat, input));
                    }
                    game.step(&inputs, TICK_DT, &mut events);
                    let placements = game.placements();
                    if let Some(placements) = placements.map(|placements| self.share_places(placements)) {
                        if let Some(winners) = placements.first() {
                            for member in self.members.iter_mut() {
                                if winners.contains(&member.seat) {
                                    member.wins += 1;
                                }
                            }
                        }
                        if let Some(round) = self.bout_round() {
                            self.bouts += 1;
                            let place = |seat: u8| placements.iter().position(|tier| tier.contains(&seat)).map_or(0, |i| i + 1);
                            self.last_bout = Some(PlayedBout {
                                number: self.bouts,
                                key: self.minigame,
                                round,
                                players: self
                                    .members
                                    .iter()
                                    .filter(|m| m.in_round && !m.is_bot)
                                    .map(|m| (m.user_id.clone(), place(m.seat)))
                                    .collect(),
                            });
                            self.rated.clear();
        self.noted.clear();
                        }
                        self.phase_ticks = RESULTS_TICKS;
                        self.phase = Phase::Results {
                            seconds: RESULTS_TICKS as f32 / TICK_HZ as f32,
                            placements,
                        };
                        self.dirty = true;
                    }
                }
            }
            Phase::Results { placements, .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                if self.phase_ticks == 0 {
                    if self.city_mode && self.city.is_some() {
                        let city=self.city.as_mut().unwrap();
                        if city.round>=city.config.jobs {
                            let standings=city.standings();self.phase_ticks=GAME_OVER_TICKS;
                            self.phase=Phase::GameOver{seconds:GAME_OVER_TICKS as f32/TICK_HZ as f32,standings};
                        }else{city.next_job(&placements);if let Some(board)=self.board.as_mut(){board.round=city.round as u32;}self.game=None;self.phase=Phase::CityHeist;}
                        self.dirty=true;return events;
                    }
                    if self.finale_preview {
                        self.finale_preview=false;
                        if let Some(board)=self.board.as_mut() {board.end_finale_preview();}
                        self.phase=Phase::Board;self.dirty=true;
                        return events;
                    }
                    if self.opening_minigame {
                        self.opening_minigame=false;
                        if let Some(board)=self.board.as_mut(){board.finish_opening(&placements);}
                        self.phase=Phase::Board;self.bot_think=BOT_THINK_TICKS;self.dirty=true;
                        return events;
                    }
                    // In a party the minigame pays coins and the board carries on.
                    if let Some(board) = self.board.as_mut() {
                        if board.finish_round(&placements) {
                            self.phase = Phase::Board;
                            self.bot_think = BOT_THINK_TICKS;
                        } else {
                            let standings = board.placements();
                            // Partners finish together, at the better one's place.
                            let standings = self.share_places(standings);
                            self.phase_ticks = FINALE_TICKS;
                            self.phase = Phase::GameOver {
                                seconds: FINALE_TICKS as f32 / TICK_HZ as f32,
                                standings,
                            };
                        }
                        self.dirty = true;
                        return events;
                    }
                    for member in self.members.iter_mut() {
                        member.in_round = false;
                    }
                    self.phase = Phase::Lobby;
                    self.rebuild_preview();
                    self.dirty = true;
                } else if self.phase_ticks % TICK_HZ == 0 {
                    self.phase = Phase::Results {
                        seconds: self.phase_ticks as f32 / TICK_HZ as f32,
                        placements,
                    };
                    self.dirty = true;
                }
            }
        }
        events
    }

    #[cfg(test)]
    pub(crate) fn finish_candy_offer_for_test(&mut self){let mut ticks=0;while self.board_paused()&&ticks<TICK_HZ*21{self.step();ticks+=1;}assert!(!self.board_paused(),"test setup must finish opt-in interruption");}

    #[cfg(test)]
    pub(crate) fn finish_opening_for_test(&mut self) {
        if !self.opening_minigame{return;}
        self.phase=Phase::Results{seconds:0.,placements:self.members.iter().map(|m|vec![m.seat]).collect()};
        self.phase_ticks=1;self.step();self.finish_candy_offer_for_test();
    }

    #[cfg(test)]
    pub fn max_seats_for_test(&self) -> usize {
        self.seat_cap()
    }

    #[cfg(test)]
    pub fn table_order_for_test(&self) -> &[u8] {
        &self.table_order
    }

    pub fn has_people(&self) -> bool {
        self.members.iter().any(|m| !m.is_bot && m.connected)
    }
}

impl Room {
    /// Offline native practice starts directly at the host's board turn.
    /// Normal online parties still earn order/coins in the opening minigame.
    pub fn start_native_board_practice(&mut self, user:&str) -> Result<(),RoomError> {
        if !self.is_host(user) { return Err(RoomError::NotHost); }
        if self.phase != Phase::Lobby { return Err(RoomError::WrongPhase); }
        self.enable_native_mode();
        self.start_party(user)?;
        let mut seats:Vec<_>=self.members.iter().map(|m| m.seat).collect();
        let host=self.seat_of(user);
        seats.sort_by_key(|seat|(Some(*seat)!=host,*seat));
        let placements:Vec<_>=seats.into_iter().map(|seat|vec![seat]).collect();
        if let Some(board)=self.board.as_mut() { board.finish_opening(&placements); }
        self.opening_minigame=false;self.game=None;self.phase=Phase::Board;
        self.bot_think=BOT_THINK_TICKS;self.dirty=true;Ok(())
    }

    /// Once a native player joins, every future ballot stays within the
    /// shared, actually rendered game set. Browser-only rooms keep all games.
    pub fn enable_native_mode(&mut self) {
        self.native_mode = true;
        if self.phase == Phase::Lobby {
            self.map = Map::Frozen;
            if !NATIVE_BOUTS.contains(&self.minigame) { self.minigame="bumper"; }
            self.rebuild_preview();
        }
        self.dirty=true;
    }
    pub fn native_compatible(&self) -> bool {
        self.phase==Phase::Lobby || (self.map==Map::Frozen &&
            (!matches!(self.phase,Phase::Practice{..}|Phase::Countdown{..}|Phase::Playing) || NATIVE_BOUTS.contains(&self.minigame)))
    }
    pub fn choose_city(&mut self,user:&str,police:u8,jobs:u8,host_police:bool)->Result<(),RoomError>{
        if !self.is_host(user){return Err(RoomError::NotHost);}if self.phase!=Phase::Lobby{return Err(RoomError::WrongPhase);}
        if police>2||![1,3,5,7].contains(&jobs){return Err(RoomError::InvalidRounds);}
        self.city_mode=true;self.party_mode=false;self.pairs_on=false;self.city=None;self.board=None;self.map=Map::Frozen;self.minigame=BOUTS[0].key;
        self.city_config=crate::city_heist::CityConfig{police,jobs,host_police,..Default::default()};self.rebuild_preview();self.dirty=true;Ok(())
    }
    pub fn start_city(&mut self,user:&str,watch:bool)->Result<(),RoomError>{
        if !self.is_host(user){return Err(RoomError::NotHost);}if self.phase!=Phase::Lobby||!self.city_mode{return Err(RoomError::WrongPhase);}
        if watch&&self.members.iter().filter(|m|!m.is_bot).count()>1{return Err(RoomError::WrongPhase);}
        if self.members.len()==1{for _ in 0..3{self.add_bot(user)?;}}
        let players=self.members.len();
        let police=match self.city_config.police{0=>crate::city_heist::formats::police_for(players),n=>n};
        if !crate::city_heist::formats::allowed(police,players){return Err(RoomError::NotEnoughPlayers);}
        let mut config=self.city_config.clone();config.police=police;
        config.skill.get_or_insert(self.bot_skill);
        let host=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        let mut seats:Vec<_>=self.members.iter().map(|m|(m.seat,m.is_bot)).collect();
        seats.sort_by_key(|(seat,_)|if *seat==host{if self.city_config.host_police{0}else{2}}else{1});
        for member in &mut self.members{member.in_round=true;member.stick=Stick::default();member.fire=false;member.input_tick=0;}
        self.parties+=1;self.party_id=format!("{}:city:{}:{}",self.match_id,self.parties,self.rng.random::<u64>());self.bouts=0;self.last_bout=None;
        let all:Vec<_>=seats.iter().map(|s|s.0).collect();let mut board=Board::for_room(Map::Frozen,&all,999,self.rng.random());board.use_manual_events();
        self.board=Some(board);self.game=None;self.table=None;self.sandbox=false;
        self.city=Some(crate::city_heist::CityGame::seeded(&seats,config,watch,self.rng.random()));self.phase=Phase::CityHeist;self.dirty=true;Ok(())
    }
    pub fn city_action(&mut self,user:&str,held:bool,suction:bool)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.control(seat,held,suction);}Ok(())
    }
    pub fn city_trace(&mut self,user:&str,x:f32,y:f32)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.trace(seat,x,y);}Ok(())
    }
    pub fn city_struggle(&mut self,user:&str)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.struggle(seat){self.dirty=true;}}Ok(())
    }
    pub fn city_item(&mut self,user:&str,act:crate::city_heist::ItemAct,yaw:f32)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.use_item(seat,act,yaw){self.dirty=true;}}Ok(())
    }
    pub fn city_dodge(&mut self,user:&str,yaw:f32)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.dodge(seat,yaw){self.dirty=true;}}Ok(())
    }
    pub fn city_takedown(&mut self,user:&str,yaw:f32)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.takedown(seat,yaw){self.dirty=true;}}Ok(())
    }
    pub fn city_callout(&mut self,user:&str,yaw:f32)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.callout(seat,yaw){self.dirty=true;}}Ok(())
    }
    pub fn city_clarity(&mut self,user:&str,held:bool)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.clarity(seat,held){self.dirty=true;}}Ok(())
    }
    pub fn city_check(&mut self,user:&str,input:crate::city_heist::CheckInput)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.check_input(seat,input){self.dirty=true;}}Ok(())
    }
    pub fn city_slip(&mut self,user:&str,input:crate::city_slip::SlipInput)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){if city.slip_input(seat,input){self.dirty=true;}}Ok(())
    }
    pub fn city_tool(&mut self,user:&str,tool:crate::city_heist::Tool)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Ok(());}let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.use_tool(seat,tool);}self.dirty=true;Ok(())
    }
    pub fn city_automate(&mut self,user:&str,on:bool)->Result<(),RoomError>{
        let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.automate(seat,on);}self.dirty=true;Ok(())
    }
    pub fn city_ready(&mut self,user:&str)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Err(RoomError::WrongPhase);}
        let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.ready(seat);}self.dirty=true;Ok(())
    }
    /// Puts a stuck patrol car back on a clear lane.
    pub fn city_reset_car(&mut self,user:&str)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Err(RoomError::WrongPhase);}
        let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let (Some(city),Some(board))=(self.city.as_mut(),self.board.as_mut()){if city.reset_car(seat,board){self.dirty=true;}}
        Ok(())
    }
    pub fn city_vehicle(&mut self,user:&str)->Result<(),RoomError>{
        if self.phase!=Phase::CityHeist{return Err(RoomError::WrongPhase);}
        let seat=self.seat_of(user).ok_or(RoomError::WrongPhase)?;
        if let Some(city)=self.city.as_mut(){city.toggle_car(seat);}self.dirty=true;Ok(())
    }
    fn step_city(&mut self){
        let movement:Vec<_>=self.members.iter().filter(|m|m.connected&&self.tick.saturating_sub(m.input_tick)<=15).map(|m|(m.seat,m.stick,m.yaw)).collect();
        let eligible:Vec<_>=self.members.iter().filter(|m|m.connected).map(|m|m.seat).collect();
        if let (Some(city),Some(board))=(self.city.as_mut(),self.board.as_mut()){
            if city.resolved.is_none(){let world=TICK_DT*city.time_scale();board.traffic_tick_city(world,&city.patrol_traffic_obstacles(),&city.patrol_sirens());board.pedestrians_tick(world);}
            city.tick_streets(TICK_DT,&movement,&eligible,board);
            if city.resolved_for>=4.{self.open_vote();}
        }
        if self.tick%3==0{self.dirty=true;}
    }
}
