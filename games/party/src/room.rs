//! A party room: seats, character picks, the host, practice bots and the
//! lobby, countdown, play and results cycle around one minigame at a time.
//! Pure state with no I/O so it can be tested tick by tick.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::Serialize;
use serde_json::Value;

use crate::board::{Board, BoardError, BoardView, BotMove, Item, TurnStep};
use crate::table::{self, TABLE_GAMES, TableGame};
use crate::bout::{
    self, BOUTS, Bout, BoutSpec, Contestant, DEFAULT_SKILL, Input, SKILLS, Skill, Stick,
};
use crate::roster::{ROSTER, character};

pub use crate::bout::{SNAPSHOT_EVERY_TICKS, TICK_DT, TICK_HZ};
pub const MAX_SEATS: usize = 4;
const COUNTDOWN_TICKS: u32 = TICK_HZ * 3;
const RESULTS_TICKS: u32 = TICK_HZ * 6;
const GAME_OVER_TICKS: u32 = TICK_HZ * 12;
/// How long a practice bot thinks about each step of its board turn.
const BOT_THINK_TICKS: u32 = TICK_HZ * 3 / 4;
/// A turn nobody takes plays itself, so one person cannot stall the party.
const TURN_LIMIT_TICKS: u32 = TICK_HZ * 45;
pub const PARTY_ROUNDS: u32 = 5;
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
    /// Everyone in the round picks the next bout from a few random ones.
    Vote {
        seconds: f32,
        options: Vec<&'static str>,
        votes: Vec<BoutVote>,
    },
    /// A card game at the table; its state travels per seat.
    Table,
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
    pub seat: u8,
    pub is_bot: bool,
    pub connected: bool,
    pub wins: u32,
    #[serde(skip)]
    pub stick: Stick,
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
    #[serde(skip)]
    pub away_since: Option<u64>,
    /// Seated for the round in progress; people who join mid-round wait.
    pub in_round: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RoomView {
    pub match_id: String,
    pub minigame: &'static str,
    pub minigame_name: &'static str,
    /// Every bout the room can play.
    pub bouts: &'static [BoutSpec],
    pub table_games: &'static [(&'static str, &'static str)],
    /// How hard the bots play, and the names to pick between.
    pub bot_skill: f32,
    pub bot_skill_name: &'static str,
    pub skills: &'static [(&'static str, f32)],
    pub host_id: Option<String>,
    pub phase: Phase,
    pub members: Vec<Member>,
    pub max_seats: usize,
    /// Present while a party is running.
    pub board: Option<BoardView>,
    /// The fixed layout of the minigame's arena, for the client to build.
    pub setup: Value,
    pub recording: Option<Recording>,
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
        })
    }
}

pub struct Room {
    pub match_id: String,
    /// How hard the practice bots play, for every game in this room.
    pub bot_skill: f32,
    pub minigame: &'static str,
    pub members: Vec<Member>,
    pub host_id: Option<String>,
    pub phase: Phase,
    pub game: Option<Box<dyn Bout>>,
    pub board: Option<Board>,
    pub table: Option<Box<dyn TableGame>>,
    /// Room seat for each table seat, in table order.
    table_order: Vec<u8>,
    pub tick: u64,
    phase_ticks: u32,
    bot_think: u32,
    turn_idle: u32,
    rng: StdRng,
    dirty: bool,
    recording: Option<Recording>,
}

impl Room {
    pub fn new(match_id: impl Into<String>, seed: u64) -> Self {
        let mut room = Room {
            match_id: match_id.into(),
            bot_skill: DEFAULT_SKILL,
            minigame: BOUTS[0].key,
            members: Vec::new(),
            host_id: None,
            phase: Phase::Lobby,
            game: None,
            board: None,
            table: None,
            table_order: Vec::new(),
            tick: 0,
            phase_ticks: 0,
            bot_think: 0,
            turn_idle: 0,
            rng: StdRng::seed_from_u64(seed),
            dirty: true,
            recording: None,
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
        RoomView {
            match_id: self.match_id.clone(),
            minigame: self.minigame,
            minigame_name: bout::find(self.minigame)
                .map(|spec| spec.name)
                .or_else(|| TABLE_GAMES.iter().find(|(k, _)| *k == self.minigame).map(|(_, n)| *n))
                .unwrap_or("Bout"),
            bouts: BOUTS,
            table_games: TABLE_GAMES,
            bot_skill: self.bot_skill,
            bot_skill_name: bout::skill_name(self.bot_skill),
            skills: SKILLS,
            host_id: self.host_id.clone(),
            phase: self.phase.clone(),
            members: self.members.clone(),
            max_seats: self.seat_cap(),
            board: self.board.as_ref().map(|board| board.view()),
            setup: self.game.as_ref().map(|g| g.setup()).unwrap_or(Value::Null),
            recording: self.recording.clone(),
        }
    }

    pub fn snapshot(&self) -> Option<Value> {
        self.game.as_ref().map(|g| g.snapshot())
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

    fn pick_host(&mut self) {
        let current_ok = self
            .host_id
            .as_ref()
            .is_some_and(|h| self.members.iter().any(|m| &m.user_id == h && m.connected));
        if !current_ok {
            self.host_id = self
                .members
                .iter()
                .find(|m| !m.is_bot && m.connected)
                .map(|m| m.user_id.clone());
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
            seat,
            is_bot: false,
            connected: true,
            wins: 0,
            stick: Stick::default(),
            yaw: 0.0,
            pitch: 0.0,
            jump: false,
            crouch: false,
            fire: false,
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
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            member.stick = stick.clamped();
            if yaw.is_finite() {
                member.yaw = yaw;
            }
            if pitch.is_finite() {
                member.pitch = pitch.clamp(-1.4, 1.4);
            }
            member.jump = jump;
            member.crouch = crouch;
        }
    }

    /// A shot, taken on the next tick with the aim it was fired with.
    pub fn queue_fire(&mut self, user_id: &str, yaw: f32, pitch: f32) {
        if let Some(member) = self.members.iter_mut().find(|m| m.user_id == user_id) {
            if yaw.is_finite() {
                member.yaw = yaw;
            }
            if pitch.is_finite() {
                member.pitch = pitch.clamp(-1.4, 1.4);
            }
            member.fire = true;
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
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if let Some(found) = TABLE_GAMES.iter().find(|(k, _)| *k == key) {
            self.minigame = found.0;
            self.game = None;
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
        self.dirty = true;
        Ok(())
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
            seat,
            is_bot: true,
            connected: true,
            wins: 0,
            stick: Stick::default(),
            yaw: 0.0,
            pitch: 0.0,
            jump: false,
            crouch: false,
            fire: false,
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
        if !self.is_host(user_id) {
            return Err(RoomError::NotHost);
        }
        if self.phase != Phase::Lobby {
            return Err(RoomError::WrongPhase);
        }
        if self.table_spec().is_some() {
            return self.start_table();
        }
        if self.members.len() < 2 {
            return Err(RoomError::NotEnoughPlayers);
        }
        for member in self.members.iter_mut() {
            member.in_round = true;
        }
        self.rebuild_game();
        self.phase_ticks = COUNTDOWN_TICKS;
        self.phase = Phase::Countdown {
            seconds: COUNTDOWN_TICKS as f32 / TICK_HZ as f32,
        };
        self.dirty = true;
        Ok(())
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
        for member in self.members.iter_mut() {
            member.in_round = true;
        }
        let seats: Vec<u8> = self.members.iter().map(|m| m.seat).collect();
        self.board = Some(Board::new(&seats, PARTY_ROUNDS, self.rng.random()));
        self.phase = Phase::Board;
        self.bot_think = BOT_THINK_TICKS;
        self.dirty = true;
        Ok(())
    }

    fn seat_of(&self, user_id: &str) -> Option<u8> {
        self.members
            .iter()
            .find(|m| m.user_id == user_id)
            .map(|m| m.seat)
    }

    fn board_turn(&mut self, user_id: &str) -> Result<(&mut Board, u8), RoomError> {
        if self.phase != Phase::Board {
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
    ) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board
            .use_item(seat, item, target)
            .map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    pub fn pass_items(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.pass_items(seat).map_err(RoomError::Board)?;
        self.dirty = true;
        Ok(())
    }

    pub fn roll(&mut self, user_id: &str) -> Result<(), RoomError> {
        let (board, seat) = self.board_turn(user_id)?;
        board.roll(seat).map_err(RoomError::Board)?;
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
        self.dirty = true;
        if round_over {
            self.open_vote();
        }
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
        let is_bot = self.members.iter().any(|m| m.seat == seat && m.is_bot);
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
        let _ = match action {
            BotMove::Item { item, target } => board.use_item(seat, item, target),
            BotMove::Pass => board.pass_items(seat),
            BotMove::Roll => board.roll(seat).map(|_| ()),
            BotMove::Branch { node } => board.choose_branch(seat, node),
            BotMove::EndTurn => {
                self.finish_board_turn();
                return;
            }
        };
        self.dirty = true;
    }

    fn contestants(&self, all: bool) -> Vec<Contestant> {
        self.members
            .iter()
            .filter(|m| all || m.in_round)
            .map(|m| Contestant {
                seat: m.seat,
                is_bot: m.is_bot,
                skill: if m.is_bot { self.bot_skill } else { 1.0 },
            })
            .collect()
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
        }
    }

    fn rebuild_game(&mut self) {
        let contestants = self.contestants(false);
        self.game = bout::create(self.minigame, &contestants, self.rng.random());
    }

    /// The ballot holds a few random bouts that fit the round, leaving out the
    /// one just played when there are enough others.
    fn open_vote(&mut self) {
        let players = self.members.iter().filter(|m| m.in_round).count();
        let mut pool: Vec<&'static str> = BOUTS
            .iter()
            .filter(|spec| spec.fits(players))
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
            self.phase_ticks = COUNTDOWN_TICKS;
            self.phase = Phase::Countdown {
                seconds: COUNTDOWN_TICKS as f32 / TICK_HZ as f32,
            };
            self.dirty = true;
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

        match self.phase.clone() {
            Phase::Lobby => {}
            Phase::Board => self.step_board_bots(),
            Phase::Vote { .. } => self.step_vote(),
            Phase::Table => self.step_table(),
            Phase::GameOver { .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                if self.phase_ticks == 0 {
                    self.board = None;
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
            Phase::Countdown { .. } => {
                self.phase_ticks = self.phase_ticks.saturating_sub(1);
                if self.phase_ticks == 0 {
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
                            }
                        } else {
                            Input::default()
                        };
                        member.fire = false;
                        inputs.push((member.seat, input));
                    }
                    game.step(&inputs, TICK_DT, &mut events);
                    if let Some(placements) = game.placements() {
                        if let Some(winners) = placements.first() {
                            for member in self.members.iter_mut() {
                                if winners.contains(&member.seat) {
                                    member.wins += 1;
                                }
                            }
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
                    // In a party the minigame pays coins and the board carries on.
                    if let Some(board) = self.board.as_mut() {
                        if board.finish_round(&placements) {
                            self.phase = Phase::Board;
                            self.bot_think = BOT_THINK_TICKS;
                        } else {
                            let standings = board.placements();
                            self.phase_ticks = GAME_OVER_TICKS;
                            self.phase = Phase::GameOver {
                                seconds: GAME_OVER_TICKS as f32 / TICK_HZ as f32,
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
