//! Rules: seats, the train card market, route claims, tickets, the final
//! round and scoring. Pure state, driven by `Action`s; no engine here.

use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{anyhow, Result};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

use crate::map::{self, Route, Ticket, TrackColor, ROUTES, TRAINS_PER_PLAYER};

pub const MAX_PLAYERS: usize = 5;
pub const MIN_PLAYERS: usize = 2;
pub const STARTING_HAND: usize = 4;
pub const FACE_UP_SLOTS: usize = 5;
pub const LONGEST_PATH_BONUS: i32 = 10;
/// A player who ends a turn with this many trains or fewer starts the last round.
pub const FINAL_ROUND_TRAINS: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Card {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Black,
    White,
    /// Wild: stands in for any color.
    Locomotive,
}

impl Card {
    pub const ALL: [Card; 9] = [
        Card::Red,
        Card::Orange,
        Card::Yellow,
        Card::Green,
        Card::Blue,
        Card::Purple,
        Card::Black,
        Card::White,
        Card::Locomotive,
    ];

    pub fn matches(self, color: TrackColor) -> bool {
        match (self, color) {
            (Card::Locomotive, _) => true,
            (Card::Red, TrackColor::Red)
            | (Card::Orange, TrackColor::Orange)
            | (Card::Yellow, TrackColor::Yellow)
            | (Card::Green, TrackColor::Green)
            | (Card::Blue, TrackColor::Blue)
            | (Card::Purple, TrackColor::Purple)
            | (Card::Black, TrackColor::Black)
            | (Card::White, TrackColor::White) => true,
            _ => false,
        }
    }

    pub fn as_color(self) -> Option<TrackColor> {
        match self {
            Card::Red => Some(TrackColor::Red),
            Card::Orange => Some(TrackColor::Orange),
            Card::Yellow => Some(TrackColor::Yellow),
            Card::Green => Some(TrackColor::Green),
            Card::Blue => Some(TrackColor::Blue),
            Card::Purple => Some(TrackColor::Purple),
            Card::Black => Some(TrackColor::Black),
            Card::White => Some(TrackColor::White),
            Card::Locomotive => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Lobby,
    Playing,
    Finished,
}

/// Where the active player is inside their turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnPhase {
    /// Nothing done yet: draw, claim or take tickets.
    Start,
    /// One card drawn; only a second draw (not a face-up locomotive) remains.
    DrewOne,
    /// Tickets were drawn and must be kept or returned.
    ChoosingTickets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub is_ai: bool,
    pub trains: u8,
    pub hand: Vec<Card>,
    pub tickets: Vec<Ticket>,
    /// Tickets offered but not yet chosen from.
    pub ticket_offer: Vec<Ticket>,
    /// The first offer must keep at least two; later offers at least one.
    pub first_ticket_pick_done: bool,
    pub claimed: Vec<u16>,
    pub score: i32,
}

impl Player {
    fn new(id: String, name: String, avatar_url: String, is_ai: bool) -> Self {
        Self {
            id,
            name,
            avatar_url,
            is_ai,
            trains: TRAINS_PER_PLAYER,
            hand: Vec::new(),
            tickets: Vec::new(),
            ticket_offer: Vec::new(),
            first_ticket_pick_done: false,
            claimed: Vec::new(),
            score: 0,
        }
    }

    pub fn hand_count(&self, card: Card) -> usize {
        self.hand.iter().filter(|c| **c == card).count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LogEntry {
    Joined { player: String },
    Drew { player: String, from_deck: bool, card: Option<Card> },
    Claimed { player: String, route: u16, points: u16 },
    Tickets { player: String, kept: usize },
    FinalRound { player: String },
    Finished { winner: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub stage: Stage,
    pub host_id: Option<String>,
    pub players: Vec<Player>,
    pub active: usize,
    pub phase: TurnPhase,
    pub deck: Vec<Card>,
    pub discard: Vec<Card>,
    pub face_up: Vec<Card>,
    pub ticket_deck: VecDeque<Ticket>,
    pub owners: HashMap<u16, String>,
    /// Who triggered the final round; everyone else gets one more turn.
    pub final_round_by: Option<String>,
    pub turns_left: Option<usize>,
    pub round: u32,
    pub seed: u64,
    pub log: Vec<LogEntry>,
    pub winner_id: Option<String>,
    pub longest_path: Option<(String, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Action {
    Join { name: String, avatar_url: String },
    AddAi,
    Start,
    /// Draw from the deck (`face_up` None) or a face-up slot.
    DrawCard { face_up: Option<usize> },
    /// Claim a route. `color` picks the card color to spend on a grey route,
    /// or on any route when the hand has several usable colors.
    ClaimRoute { route: u16, color: Option<Card> },
    DrawTickets,
    /// Ticket ids to keep from the current offer.
    KeepTickets { keep: Vec<u16> },
    Reset,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Self {
            stage: Stage::Lobby,
            host_id: None,
            players: Vec::new(),
            active: 0,
            phase: TurnPhase::Start,
            deck: Vec::new(),
            discard: Vec::new(),
            face_up: Vec::new(),
            ticket_deck: VecDeque::new(),
            owners: HashMap::new(),
            final_round_by: None,
            turns_left: None,
            round: 0,
            seed,
            log: Vec::new(),
            winner_id: None,
            longest_path: None,
        }
    }

    fn rng(&mut self) -> ChaCha8Rng {
        // Derive from the seed and how far the game has gone so replays are stable.
        let salt = self.deck.len() as u64 + self.discard.len() as u64 * 131 + self.round as u64 * 17;
        ChaCha8Rng::seed_from_u64(self.seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    pub fn player(&self, id: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.id == id)
    }

    fn player_mut(&mut self, id: &str) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.id == id)
    }

    pub fn active_player(&self) -> Option<&Player> {
        self.players.get(self.active)
    }

    pub fn is_active(&self, id: &str) -> bool {
        self.stage == Stage::Playing && self.active_player().map(|p| p.id == id).unwrap_or(false)
    }

    pub fn apply(&mut self, player_id: &str, action: Action) -> Result<()> {
        match action {
            Action::Join { name, avatar_url } => self.join(player_id, name, avatar_url, false),
            Action::AddAi => self.add_ai(player_id),
            Action::Start => self.start(player_id),
            Action::Reset => self.reset(player_id),
            Action::DrawCard { face_up } => self.draw_card(player_id, face_up),
            Action::ClaimRoute { route, color } => self.claim_route(player_id, route, color),
            Action::DrawTickets => self.draw_tickets(player_id),
            Action::KeepTickets { keep } => self.keep_tickets(player_id, &keep),
        }
    }

    // ---- lobby -----------------------------------------------------------

    fn join(&mut self, id: &str, name: String, avatar_url: String, is_ai: bool) -> Result<()> {
        if self.stage != Stage::Lobby {
            return Err(anyhow!("The game has already started"));
        }
        if self.players.iter().any(|p| p.id == id) {
            return Ok(());
        }
        if self.players.len() >= MAX_PLAYERS {
            return Err(anyhow!("The table is full"));
        }
        if self.host_id.is_none() {
            self.host_id = Some(id.to_string());
        }
        self.players.push(Player::new(id.to_string(), name.clone(), avatar_url, is_ai));
        self.log.push(LogEntry::Joined { player: name });
        Ok(())
    }

    fn add_ai(&mut self, by: &str) -> Result<()> {
        if self.host_id.as_deref() != Some(by) {
            return Err(anyhow!("Only the host can add an AI"));
        }
        let n = self.players.iter().filter(|p| p.is_ai).count() + 1;
        self.join(&format!("ai-{n}"), format!("Practice AI {n}"), String::new(), true)
    }

    fn reset(&mut self, by: &str) -> Result<()> {
        let host = self.host_id.as_deref() == Some(by);
        if self.stage == Stage::Playing && !host {
            return Err(anyhow!("Only the host can reset a game in progress"));
        }
        *self = Game::new(self.seed.wrapping_add(1));
        Ok(())
    }

    fn start(&mut self, by: &str) -> Result<()> {
        if self.stage != Stage::Lobby {
            return Err(anyhow!("The game has already started"));
        }
        if self.host_id.as_deref() != Some(by) {
            return Err(anyhow!("Only the host can start the game"));
        }
        if self.players.len() < MIN_PLAYERS {
            return Err(anyhow!("Need at least two players"));
        }
        let mut rng = self.rng();
        let mut deck: Vec<Card> = Vec::with_capacity(110);
        for card in Card::ALL {
            let copies = if card == Card::Locomotive { 14 } else { 12 };
            deck.extend(std::iter::repeat(card).take(copies));
        }
        deck.shuffle(&mut rng);
        let mut tickets: Vec<Ticket> = map::tickets();
        tickets.shuffle(&mut rng);
        self.players.shuffle(&mut rng);
        self.deck = deck;
        self.ticket_deck = tickets.into();
        for i in 0..self.players.len() {
            for _ in 0..STARTING_HAND {
                if let Some(card) = self.deck.pop() {
                    self.players[i].hand.push(card);
                }
            }
            let offer = self.take_tickets(3);
            self.players[i].ticket_offer = offer;
        }
        self.refill_face_up();
        self.stage = Stage::Playing;
        self.active = 0;
        self.round = 1;
        // Everyone picks their opening tickets before the first turn; the
        // active player goes through ChoosingTickets like any later draw.
        self.phase = TurnPhase::ChoosingTickets;
        Ok(())
    }

    // ---- cards -----------------------------------------------------------

    fn take_tickets(&mut self, n: usize) -> Vec<Ticket> {
        let mut out = Vec::new();
        for _ in 0..n {
            if let Some(t) = self.ticket_deck.pop_front() {
                out.push(t);
            }
        }
        out
    }

    fn draw_from_deck(&mut self) -> Option<Card> {
        if self.deck.is_empty() && !self.discard.is_empty() {
            let mut rng = self.rng();
            self.deck = std::mem::take(&mut self.discard);
            self.deck.shuffle(&mut rng);
        }
        self.deck.pop()
    }

    fn refill_face_up(&mut self) {
        while self.face_up.len() < FACE_UP_SLOTS {
            match self.draw_from_deck() {
                Some(card) => self.face_up.push(card),
                None => break,
            }
        }
        // Three or more face-up locomotives wipe the market.
        let locomotives = self.face_up.iter().filter(|c| **c == Card::Locomotive).count();
        let can_refill = self.deck.len() + self.discard.len() >= FACE_UP_SLOTS;
        if locomotives >= 3 && can_refill {
            self.discard.append(&mut self.face_up);
            self.refill_face_up();
        }
    }

    fn draw_card(&mut self, player_id: &str, face_up: Option<usize>) -> Result<()> {
        self.require_turn(player_id)?;
        if self.phase == TurnPhase::ChoosingTickets {
            return Err(anyhow!("Choose your tickets first"));
        }
        let card = match face_up {
            Some(slot) => {
                let card = *self.face_up.get(slot).ok_or_else(|| anyhow!("No card in that slot"))?;
                if card == Card::Locomotive && self.phase == TurnPhase::DrewOne {
                    return Err(anyhow!("A face-up locomotive must be your only draw"));
                }
                self.face_up.remove(slot);
                self.refill_face_up();
                card
            }
            None => self.draw_from_deck().ok_or_else(|| anyhow!("The deck is empty"))?,
        };
        let name = self.active_player().map(|p| p.name.clone()).unwrap_or_default();
        self.player_mut(player_id).unwrap().hand.push(card);
        self.log.push(LogEntry::Drew {
            player: name,
            from_deck: face_up.is_none(),
            card: face_up.map(|_| card),
        });
        let turn_over = matches!(face_up, Some(_)) && card == Card::Locomotive
            || self.phase == TurnPhase::DrewOne
            || (self.deck.is_empty() && self.discard.is_empty() && self.face_up.is_empty());
        if turn_over {
            self.end_turn();
        } else {
            self.phase = TurnPhase::DrewOne;
        }
        Ok(())
    }

    // ---- routes ------------------------------------------------------------

    /// The colors a player could spend on a route, each paired with how many
    /// locomotives would have to fill in.
    pub fn claim_options(&self, player_id: &str, route: &Route) -> Vec<(Card, usize)> {
        let Some(player) = self.player(player_id) else { return Vec::new() };
        if player.trains < route.length {
            return Vec::new();
        }
        let locos = player.hand_count(Card::Locomotive);
        let need = route.length as usize;
        let colors: Vec<Card> = match route.color {
            TrackColor::Any => Card::ALL.iter().copied().filter(|c| *c != Card::Locomotive).collect(),
            c => Card::ALL.iter().copied().filter(|card| card.matches(c) && *card != Card::Locomotive).collect(),
        };
        let mut out = Vec::new();
        for color in colors {
            let have = player.hand_count(color);
            if have + locos >= need {
                out.push((color, need.saturating_sub(have)));
            }
        }
        if locos >= need && route.color == TrackColor::Any {
            out.push((Card::Locomotive, need));
        } else if locos >= need && out.is_empty() {
            out.push((Card::Locomotive, need));
        }
        out
    }

    pub fn route_is_claimable(&self, player_id: &str, route: &Route) -> bool {
        if self.owners.contains_key(&route.id) {
            return false;
        }
        if let Some(twin) = route.twin {
            if let Some(owner) = self.owners.get(&twin) {
                // Two-track routes: one player never takes both, and with
                // fewer than four players the second track stays closed.
                if owner == player_id || self.players.len() < 4 {
                    return false;
                }
            }
        }
        !self.claim_options(player_id, route).is_empty()
    }

    fn claim_route(&mut self, player_id: &str, route_id: u16, color: Option<Card>) -> Result<()> {
        self.require_turn(player_id)?;
        if self.phase != TurnPhase::Start {
            return Err(anyhow!("You cannot claim a route after drawing"));
        }
        let route = map::route_by_id(route_id).ok_or_else(|| anyhow!("Unknown route"))?;
        if !self.route_is_claimable(player_id, route) {
            return Err(anyhow!("That route cannot be claimed"));
        }
        let options = self.claim_options(player_id, route);
        let (color, locos) = match color {
            Some(c) => *options.iter().find(|(o, _)| *o == c).ok_or_else(|| anyhow!("You do not hold the cards for that"))?,
            None => options[0],
        };
        let need = route.length as usize;
        let player = self.player_mut(player_id).unwrap();
        let mut spent = Vec::new();
        let colored = need - locos;
        let mut take = |hand: &mut Vec<Card>, card: Card, n: usize| {
            for _ in 0..n {
                if let Some(i) = hand.iter().position(|c| *c == card) {
                    spent.push(hand.remove(i));
                }
            }
        };
        if color != Card::Locomotive {
            take(&mut player.hand, color, colored);
        }
        take(&mut player.hand, Card::Locomotive, if color == Card::Locomotive { need } else { locos });
        player.trains -= route.length;
        player.claimed.push(route.id);
        let points = map::route_points(route.length);
        player.score += points as i32;
        let name = player.name.clone();
        self.discard.extend(spent);
        self.owners.insert(route.id, player_id.to_string());
        self.log.push(LogEntry::Claimed { player: name, route: route.id, points });
        self.end_turn();
        Ok(())
    }

    // ---- tickets -----------------------------------------------------------

    fn draw_tickets(&mut self, player_id: &str) -> Result<()> {
        self.require_turn(player_id)?;
        if self.phase != TurnPhase::Start {
            return Err(anyhow!("You cannot draw tickets after drawing cards"));
        }
        let offer = self.take_tickets(3);
        if offer.is_empty() {
            return Err(anyhow!("No tickets left"));
        }
        self.player_mut(player_id).unwrap().ticket_offer = offer;
        self.phase = TurnPhase::ChoosingTickets;
        Ok(())
    }

    fn keep_tickets(&mut self, player_id: &str, keep: &[u16]) -> Result<()> {
        if self.stage != Stage::Playing {
            return Err(anyhow!("The game is not running"));
        }
        let opening = !self.player(player_id).map(|p| p.first_ticket_pick_done).unwrap_or(true);
        if !opening {
            self.require_turn(player_id)?;
            if self.phase != TurnPhase::ChoosingTickets {
                return Err(anyhow!("No tickets to choose from"));
            }
        }
        let player = self.player_mut(player_id).ok_or_else(|| anyhow!("Not at the table"))?;
        if player.ticket_offer.is_empty() {
            return Err(anyhow!("No tickets to choose from"));
        }
        let minimum = if opening { 2.min(player.ticket_offer.len()) } else { 1 };
        let chosen: Vec<Ticket> = player.ticket_offer.iter().filter(|t| keep.contains(&t.id)).cloned().collect();
        if chosen.len() < minimum {
            return Err(anyhow!("Keep at least {minimum}"));
        }
        let returned: Vec<Ticket> = player.ticket_offer.drain(..).filter(|t| !keep.contains(&t.id)).collect();
        let kept = chosen.len();
        player.tickets.extend(chosen);
        player.first_ticket_pick_done = true;
        let name = player.name.clone();
        self.ticket_deck.extend(returned);
        self.log.push(LogEntry::Tickets { player: name, kept });
        if self.is_active(player_id) && self.phase == TurnPhase::ChoosingTickets {
            self.end_turn();
        }
        Ok(())
    }

    // ---- turns -------------------------------------------------------------

    fn require_turn(&self, player_id: &str) -> Result<()> {
        if self.stage != Stage::Playing {
            return Err(anyhow!("The game is not running"));
        }
        if !self.is_active(player_id) {
            return Err(anyhow!("It is not your turn"));
        }
        Ok(())
    }

    fn end_turn(&mut self) {
        let just_played = self.active_player().cloned();
        if let Some(p) = &just_played {
            if p.trains <= FINAL_ROUND_TRAINS && self.final_round_by.is_none() {
                self.final_round_by = Some(p.id.clone());
                self.turns_left = Some(self.players.len() - 1);
                self.log.push(LogEntry::FinalRound { player: p.name.clone() });
            } else if let Some(left) = self.turns_left.as_mut() {
                if *left == 0 {
                    self.finish();
                    return;
                }
                *left -= 1;
                if *left == 0 && self.final_round_by.is_some() {
                    // The player after this one is the last to move.
                }
            }
        }
        if let Some(0) = self.turns_left {
            // Everyone else has had their extra turn once the counter hits zero
            // *after* their turn ended; handled on the next end_turn.
        }
        self.active = (self.active + 1) % self.players.len();
        if self.active == 0 {
            self.round += 1;
        }
        self.phase = if self.players[self.active].first_ticket_pick_done {
            TurnPhase::Start
        } else {
            TurnPhase::ChoosingTickets
        };
        if self.turns_left == Some(0) && self.final_round_by.is_some() {
            // Counter reached zero on the previous end_turn: the game is over.
            self.finish();
        }
    }

    fn finish(&mut self) {
        if self.stage == Stage::Finished {
            return;
        }
        for i in 0..self.players.len() {
            let id = self.players[i].id.clone();
            let connected = self.connected_cities(&id);
            let mut delta = 0i32;
            for t in &self.players[i].tickets {
                if connected.get(t.from.as_str()).map(|set| set.contains(t.to.as_str())).unwrap_or(false) {
                    delta += t.points as i32;
                } else {
                    delta -= t.points as i32;
                }
            }
            self.players[i].score += delta;
        }
        let mut best: Option<(String, u32)> = None;
        for p in &self.players {
            let len = self.longest_path_for(&p.id);
            match &best {
                Some((_, b)) if *b >= len => {}
                _ => best = Some((p.id.clone(), len)),
            }
        }
        if let Some((id, len)) = &best {
            if *len > 0 {
                if let Some(p) = self.player_mut(id) {
                    p.score += LONGEST_PATH_BONUS;
                }
            }
        }
        self.longest_path = best;
        let winner = self
            .players
            .iter()
            .max_by_key(|p| (p.score, p.tickets.len()))
            .map(|p| p.id.clone());
        if let Some(w) = &winner {
            let name = self.player(w).map(|p| p.name.clone()).unwrap_or_default();
            self.log.push(LogEntry::Finished { winner: name });
        }
        self.winner_id = winner;
        self.stage = Stage::Finished;
    }

    // ---- network ---------------------------------------------------------

    fn owned_routes(&self, player_id: &str) -> Vec<&'static Route> {
        ROUTES
            .iter()
            .filter(|r| self.owners.get(&r.id).map(|o| o == player_id).unwrap_or(false))
            .collect()
    }

    /// Every city reachable from every city over the player's own routes.
    pub fn connected_cities(&self, player_id: &str) -> HashMap<&'static str, HashSet<&'static str>> {
        let routes = self.owned_routes(player_id);
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
        for r in &routes {
            adjacency.entry(r.from).or_default().push(r.to);
            adjacency.entry(r.to).or_default().push(r.from);
        }
        let mut out: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
        let mut seen: HashSet<&str> = HashSet::new();
        for start in adjacency.keys().copied() {
            if seen.contains(start) {
                continue;
            }
            let mut component: HashSet<&'static str> = HashSet::new();
            let mut stack = vec![start];
            while let Some(c) = stack.pop() {
                if !component.insert(c) {
                    continue;
                }
                for next in adjacency.get(c).into_iter().flatten() {
                    stack.push(next);
                }
            }
            for c in &component {
                seen.insert(c);
                out.insert(c, component.clone());
            }
        }
        out
    }

    /// Longest trail of track pieces without reusing a route.
    pub fn longest_path_for(&self, player_id: &str) -> u32 {
        let routes = self.owned_routes(player_id);
        let mut best = 0u32;
        let mut used = vec![false; routes.len()];
        let cities: HashSet<&str> = routes.iter().flat_map(|r| [r.from, r.to]).collect();
        fn walk(at: &str, routes: &[&Route], used: &mut Vec<bool>, len: u32, best: &mut u32) {
            *best = (*best).max(len);
            for (i, r) in routes.iter().enumerate() {
                if used[i] {
                    continue;
                }
                let next = if r.from == at { r.to } else if r.to == at { r.from } else { continue };
                used[i] = true;
                walk(next, routes, used, len + r.length as u32, best);
                used[i] = false;
            }
        }
        for c in cities {
            walk(c, &routes, &mut used, 0, &mut best);
        }
        best
    }
}
