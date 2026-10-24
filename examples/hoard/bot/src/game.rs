use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow, bail};
use game_framework::{GameEffect, GameLogic};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub const MAX_SEATS: usize = 4;
const HAND_SIZE: usize = 5;
const HAND_LIMIT: usize = 7;
const MARKET_SLOTS: usize = 3;
const TURN_SECONDS: u32 = 45;
const SET_BONUS: u32 = 5;
const EVENTS_KEPT: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Suit {
    Crown,
    Gem,
    Chalice,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Treasure { suit: Suit, value: u32 },
    Thief,
    Dragon,
    Scry,
    Shield,
    Trade,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Card {
    pub id: u32,
    #[serde(flatten)]
    pub kind: Kind,
}

impl Card {
    fn value(&self) -> u32 {
        match self.kind {
            Kind::Treasure { value, .. } => value,
            _ => 0,
        }
    }

    fn is_treasure(&self) -> bool {
        matches!(self.kind, Kind::Treasure { .. })
    }
}

fn new_deck(rng: &mut StdRng) -> Vec<Card> {
    let mut kinds = Vec::new();
    for suit in [Suit::Crown, Suit::Gem, Suit::Chalice] {
        for value in 1..=7 {
            kinds.push(Kind::Treasure { suit, value });
        }
    }
    kinds.extend([Kind::Thief; 4]);
    kinds.extend([Kind::Dragon; 3]);
    kinds.extend([Kind::Scry; 3]);
    kinds.extend([Kind::Shield; 3]);
    kinds.extend([Kind::Trade; 3]);
    let mut deck: Vec<Card> = kinds.into_iter().enumerate().map(|(i, kind)| Card { id: i as u32 + 1, kind }).collect();
    deck.shuffle(rng);
    deck
}

/// A place on the table, so the client can fly a card from one spot to another.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "at", rename_all = "snake_case")]
pub enum Spot {
    Deck,
    Discard,
    Market { slot: usize },
    Hand { seat: usize },
    Vault { seat: usize },
    Shield { seat: usize },
}

#[derive(Clone, Debug)]
struct Motion {
    from: Spot,
    to: Spot,
    card: Card,
    /// Seats allowed to see the card's face; `None` means everyone.
    seen_by: Option<Vec<usize>>,
}

#[derive(Clone, Debug)]
struct Event {
    seq: u64,
    text: String,
    /// What a seat reads instead of `text`, for things only it knows.
    private: Option<(usize, String)>,
    motions: Vec<Motion>,
}

#[derive(Deserialize)]
pub enum Action {
    View { name: String, place: String },
    Input(Input),
    Timeout,
}

/// What the client sends with `activity.send`.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    Sit,
    Stand,
    Start,
    Bank { card: u32 },
    Cast {
        card: u32,
        #[serde(default)]
        target: Option<usize>,
        #[serde(default)]
        slot: Option<usize>,
        #[serde(default)]
        give: Option<u32>,
    },
    Forage {
        #[serde(default)]
        slot: Option<usize>,
    },
    Keep { card: u32 },
    Pass,
    Rematch,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Seating,
    Playing,
    Over,
}

struct Seat {
    id: String,
    name: String,
    hand: Vec<Card>,
    vault: Vec<Card>,
    shield: Option<Card>,
}

impl Seat {
    fn score(&self) -> u32 {
        let base: u32 = self.vault.iter().map(Card::value).sum();
        let sets = [Suit::Crown, Suit::Gem, Suit::Chalice]
            .into_iter()
            .filter(|s| self.vault.iter().filter(|c| matches!(c.kind, Kind::Treasure { suit, .. } if suit == *s)).count() >= 3)
            .count() as u32;
        base + sets * SET_BONUS
    }
}

pub struct Standing {
    pub id: String,
    pub name: String,
    pub score: u32,
}

#[derive(Clone)]
pub struct HoardArgs {
    pub results: mpsc::UnboundedSender<(String, Vec<Standing>)>,
    pub seed: Option<u64>,
}

pub struct Hoard {
    args: HoardArgs,
    place: Option<String>,
    rng: StdRng,
    phase: Phase,
    seats: Vec<Seat>,
    names: HashMap<String, String>,
    instances: HashMap<String, String>,
    deck: Vec<Card>,
    market: Vec<Option<Card>>,
    discard: Vec<Card>,
    turn: usize,
    scry: Option<Vec<Card>>,
    turns_left: Option<usize>,
    seconds_left: u32,
    deadline_ms: u64,
    events: Vec<Event>,
    seq: u64,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Hoard {
    fn seat_of(&self, player_id: &str) -> Option<usize> {
        self.seats.iter().position(|s| s.id == player_id)
    }

    fn push(&mut self, text: String, private: Option<(usize, String)>, motions: Vec<Motion>) {
        self.seq += 1;
        self.events.push(Event { seq: self.seq, text, private, motions });
        if self.events.len() > EVENTS_KEPT {
            self.events.remove(0);
        }
    }

    fn reset_clock(&mut self) {
        self.seconds_left = TURN_SECONDS;
        self.deadline_ms = now_ms() + TURN_SECONDS as u64 * 1000;
    }

    fn deal(&mut self) {
        self.deck = new_deck(&mut self.rng);
        self.discard.clear();
        self.scry = None;
        self.turns_left = None;
        self.events.clear();
        for seat in &mut self.seats {
            seat.vault.clear();
            seat.shield = None;
            seat.hand = self.deck.split_off(self.deck.len() - HAND_SIZE);
        }
        self.market = (0..MARKET_SLOTS).map(|_| self.deck.pop()).collect();
        self.turn = self.rng.random_range(0..self.seats.len());
        self.phase = Phase::Playing;
        self.reset_clock();
        let first = self.seats[self.turn].name.clone();
        self.push(format!("The hoard is dealt. {first} goes first."), None, Vec::new());
    }

    /// Draws for `seat`; the drawn card is only shown to that seat.
    fn draw(&mut self, seat: usize, n: usize) -> Vec<Motion> {
        let mut motions = Vec::new();
        for _ in 0..n {
            let Some(card) = self.deck.pop() else { break };
            self.seats[seat].hand.push(card);
            motions.push(Motion { from: Spot::Deck, to: Spot::Hand { seat }, card, seen_by: Some(vec![seat]) });
        }
        if self.deck.is_empty() && self.turns_left.is_none() {
            self.turns_left = Some(self.seats.len());
        }
        motions
    }

    fn refill_market(&mut self) -> Vec<Motion> {
        let mut motions = Vec::new();
        for slot in 0..MARKET_SLOTS {
            if self.market[slot].is_none() {
                if let Some(card) = self.deck.pop() {
                    self.market[slot] = Some(card);
                    motions.push(Motion { from: Spot::Deck, to: Spot::Market { slot }, card, seen_by: None });
                }
            }
        }
        if self.deck.is_empty() && self.turns_left.is_none() {
            self.turns_left = Some(self.seats.len());
        }
        motions
    }

    fn take_from_hand(&mut self, seat: usize, card: u32) -> Result<Card> {
        let hand = &mut self.seats[seat].hand;
        let i = hand.iter().position(|c| c.id == card).ok_or_else(|| anyhow!("That card is not in your hand."))?;
        Ok(hand.remove(i))
    }

    fn end_turn(&mut self) {
        if let Some(left) = self.turns_left.as_mut() {
            *left = left.saturating_sub(1);
            if *left == 0 {
                return self.finish();
            }
        }
        self.turn = (self.turn + 1) % self.seats.len();
        self.reset_clock();
    }

    fn finish(&mut self) {
        self.phase = Phase::Over;
        let winners = self.winners();
        let names: Vec<String> = winners.iter().map(|&w| self.seats[w].name.clone()).collect();
        self.push(format!("The hoard is counted. {} wins.", names.join(" and ")), None, Vec::new());
        let mut standings: Vec<Standing> = self
            .seats
            .iter()
            .map(|s| Standing { id: s.id.clone(), name: s.name.clone(), score: s.score() })
            .collect();
        standings.sort_by(|a, b| b.score.cmp(&a.score));
        if let Some(place) = self.place.clone() {
            let _ = self.args.results.send((place, standings));
        }
    }

    fn winners(&self) -> Vec<usize> {
        let best = self.seats.iter().map(|s| (s.score(), s.vault.len())).max().unwrap_or((0, 0));
        (0..self.seats.len()).filter(|&i| (self.seats[i].score(), self.seats[i].vault.len()) == best).collect()
    }

    fn require_turn(&self, player_id: &str) -> Result<usize> {
        let seat = self.seat_of(player_id).ok_or_else(|| anyhow!("You are watching this game."))?;
        if self.phase != Phase::Playing {
            bail!("The game is not running.");
        }
        if seat != self.turn {
            bail!("It's {}'s turn.", self.seats[self.turn].name);
        }
        Ok(seat)
    }

    fn bank(&mut self, seat: usize, card: u32) -> Result<()> {
        if self.scry.is_some() {
            bail!("Pick a card from your scrying first.");
        }
        let held = self.seats[seat].hand.iter().find(|c| c.id == card).copied().ok_or_else(|| anyhow!("That card is not in your hand."))?;
        if !held.is_treasure() {
            bail!("Only treasure can go in your vault.");
        }
        let card = self.take_from_hand(seat, card)?;
        self.seats[seat].vault.push(card);
        let mut motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Vault { seat }, card, seen_by: None }];
        motions.extend(self.draw(seat, 1));
        let name = self.seats[seat].name.clone();
        self.push(format!("{name} banks {}.", describe(&card)), None, motions);
        self.end_turn();
        Ok(())
    }

    fn forage(&mut self, seat: usize, slot: Option<usize>) -> Result<()> {
        if self.scry.is_some() {
            bail!("Pick a card from your scrying first.");
        }
        let name = self.seats[seat].name.clone();
        let hand = self.seats[seat].hand.len();
        match slot {
            Some(slot) => {
                if hand >= HAND_LIMIT {
                    bail!("Your hand is full.");
                }
                let card = self.market.get_mut(slot).and_then(Option::take).ok_or_else(|| anyhow!("That market stall is empty."))?;
                self.seats[seat].hand.push(card);
                let mut motions = vec![Motion { from: Spot::Market { slot }, to: Spot::Hand { seat }, card, seen_by: None }];
                motions.extend(self.refill_market());
                self.push(format!("{name} takes {} from the market.", describe(&card)), None, motions);
            }
            None => {
                if self.deck.is_empty() {
                    bail!("The deck is empty.");
                }
                if hand + 2 > HAND_LIMIT {
                    bail!("You can hold at most {HAND_LIMIT} cards.");
                }
                let motions = self.draw(seat, 2);
                let drawn: Vec<String> = motions.iter().map(|m| describe(&m.card)).collect();
                let private = Some((seat, format!("You draw {}.", drawn.join(" and "))));
                self.push(format!("{name} draws {} cards.", motions.len()), private, motions);
            }
        }
        self.end_turn();
        Ok(())
    }

    fn cast(&mut self, seat: usize, card: u32, target: Option<usize>, slot: Option<usize>, give: Option<u32>) -> Result<()> {
        if self.scry.is_some() {
            bail!("Pick a card from your scrying first.");
        }
        let spell = self.seats[seat].hand.iter().find(|c| c.id == card).copied().ok_or_else(|| anyhow!("That card is not in your hand."))?;
        let name = self.seats[seat].name.clone();
        match spell.kind {
            Kind::Treasure { .. } => bail!("Treasure is banked, not cast."),
            Kind::Shield => {
                if self.seats[seat].shield.is_some() {
                    bail!("You already have a shield up.");
                }
                self.take_from_hand(seat, card)?;
                self.seats[seat].shield = Some(spell);
                let mut motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Shield { seat }, card: spell, seen_by: None }];
                motions.extend(self.draw(seat, 1));
                self.push(format!("{name} raises a Shield."), None, motions);
            }
            Kind::Thief => {
                let target = target.ok_or_else(|| anyhow!("Choose whose vault to rob."))?;
                if target == seat || target >= self.seats.len() {
                    bail!("Pick another player's vault.");
                }
                if self.seats[target].vault.is_empty() {
                    bail!("{}'s vault is empty.", self.seats[target].name);
                }
                self.take_from_hand(seat, card)?;
                self.discard.push(spell);
                let victim = self.seats[target].name.clone();
                let mut motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Discard, card: spell, seen_by: None }];
                let text = if let Some(shield) = self.seats[target].shield.take() {
                    self.discard.push(shield);
                    motions.push(Motion { from: Spot::Shield { seat: target }, to: Spot::Discard, card: shield, seen_by: None });
                    format!("{name} sends a Thief at {victim}, but the Shield stops it.")
                } else {
                    let loot = self.seats[target].vault.pop().expect("checked above");
                    self.seats[seat].vault.push(loot);
                    motions.push(Motion { from: Spot::Vault { seat: target }, to: Spot::Vault { seat }, card: loot, seen_by: None });
                    format!("{name} steals {} from {victim}.", describe(&loot))
                };
                motions.extend(self.draw(seat, 1));
                self.push(text, None, motions);
            }
            Kind::Dragon => {
                self.take_from_hand(seat, card)?;
                self.discard.push(spell);
                let mut motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Discard, card: spell, seen_by: None }];
                let mut burned = Vec::new();
                for other in (0..self.seats.len()).filter(|&o| o != seat) {
                    if let Some(shield) = self.seats[other].shield.take() {
                        self.discard.push(shield);
                        motions.push(Motion { from: Spot::Shield { seat: other }, to: Spot::Discard, card: shield, seen_by: None });
                        burned.push(format!("{} shields", self.seats[other].name));
                    } else if !self.seats[other].hand.is_empty() {
                        let i = self.rng.random_range(0..self.seats[other].hand.len());
                        let lost = self.seats[other].hand.remove(i);
                        self.discard.push(lost);
                        motions.push(Motion { from: Spot::Hand { seat: other }, to: Spot::Discard, card: lost, seen_by: None });
                        burned.push(format!("{} loses {}", self.seats[other].name, describe(&lost)));
                    }
                }
                motions.extend(self.draw(seat, 1));
                let detail = if burned.is_empty() { "nobody is hurt".to_string() } else { burned.join(", ") };
                self.push(format!("{name} wakes the Dragon: {detail}."), None, motions);
            }
            Kind::Trade => {
                let slot = slot.ok_or_else(|| anyhow!("Choose a market card to trade for."))?;
                let give = give.ok_or_else(|| anyhow!("Choose a card from your hand to give."))?;
                if give == card {
                    bail!("Trade away a different card.");
                }
                let wanted = self.market.get(slot).copied().flatten().ok_or_else(|| anyhow!("That market stall is empty."))?;
                self.take_from_hand(seat, card)?;
                let given = self.take_from_hand(seat, give).inspect_err(|_| self.seats[seat].hand.push(spell))?;
                self.discard.push(spell);
                self.market[slot] = Some(given);
                self.seats[seat].hand.push(wanted);
                let mut motions = vec![
                    Motion { from: Spot::Hand { seat }, to: Spot::Discard, card: spell, seen_by: None },
                    Motion { from: Spot::Hand { seat }, to: Spot::Market { slot }, card: given, seen_by: None },
                    Motion { from: Spot::Market { slot }, to: Spot::Hand { seat }, card: wanted, seen_by: None },
                ];
                motions.extend(self.draw(seat, 1));
                self.push(format!("{name} trades {} for {}.", describe(&given), describe(&wanted)), None, motions);
            }
            Kind::Scry => {
                if self.deck.is_empty() {
                    bail!("There is nothing left to scry.");
                }
                self.take_from_hand(seat, card)?;
                self.discard.push(spell);
                let n = self.deck.len().min(3);
                let peek = self.deck.split_off(self.deck.len() - n);
                self.scry = Some(peek);
                let motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Discard, card: spell, seen_by: None }];
                self.push(format!("{name} scries the top {n} cards."), None, motions);
                return Ok(());
            }
        }
        self.end_turn();
        Ok(())
    }

    fn keep(&mut self, seat: usize, card: u32) -> Result<()> {
        let mut peek = self.scry.take().ok_or_else(|| anyhow!("You are not scrying."))?;
        let Some(i) = peek.iter().position(|c| c.id == card) else {
            self.scry = Some(peek);
            bail!("Pick one of the scried cards.");
        };
        let kept = peek.remove(i);
        self.seats[seat].hand.push(kept);
        for rest in peek {
            self.deck.insert(0, rest);
        }
        let name = self.seats[seat].name.clone();
        let motions = vec![Motion { from: Spot::Deck, to: Spot::Hand { seat }, card: kept, seen_by: Some(vec![seat]) }];
        self.push(
            format!("{name} keeps one card and buries the rest."),
            Some((seat, format!("You keep {}.", describe(&kept)))),
            motions,
        );
        self.end_turn();
        Ok(())
    }

    fn timeout(&mut self) {
        let seat = self.turn;
        let name = self.seats[seat].name.clone();
        if let Some(first) = self.scry.as_ref().and_then(|p| p.first()).map(|c| c.id) {
            let _ = self.keep(seat, first);
            return;
        }
        let lowest = self.seats[seat].hand.iter().filter(|c| c.is_treasure()).min_by_key(|c| c.value()).map(|c| c.id);
        if !self.deck.is_empty() && self.seats[seat].hand.len() < HAND_LIMIT {
            let motions = self.draw(seat, 1);
            self.push(format!("{name} runs out of time and draws a card."), None, motions);
        } else if let Some(card) = lowest {
            let _ = self.bank(seat, card);
            return;
        } else if !self.seats[seat].hand.is_empty() {
            let card = self.seats[seat].hand.remove(0);
            self.discard.push(card);
            let motions = vec![Motion { from: Spot::Hand { seat }, to: Spot::Discard, card, seen_by: None }];
            self.push(format!("{name} runs out of time and drops {}.", describe(&card)), None, motions);
        } else {
            self.push(format!("{name} runs out of time."), None, Vec::new());
        }
        self.end_turn();
    }

    fn can_pass(&self, seat: usize) -> bool {
        let hand = &self.seats[seat].hand;
        let market_open = self.market.iter().any(Option::is_some) && hand.len() < HAND_LIMIT;
        let deck_open = !self.deck.is_empty() && hand.len() + 2 <= HAND_LIMIT;
        self.deck.is_empty() || (hand.is_empty() && !market_open && !deck_open)
    }
}

#[derive(Clone, Serialize)]
pub struct CardView {
    id: u32,
    #[serde(flatten)]
    kind: Kind,
}

impl From<&Card> for CardView {
    fn from(card: &Card) -> Self {
        CardView { id: card.id, kind: card.kind }
    }
}

#[derive(Clone, Serialize)]
pub struct MotionView {
    from: Spot,
    to: Spot,
    card_id: u32,
    /// The card's face, or absent when this viewer may not see it.
    card: Option<CardView>,
}

#[derive(Clone, Serialize)]
pub struct EventView {
    seq: u64,
    text: String,
    motions: Vec<MotionView>,
}

#[derive(Clone, Serialize)]
pub struct SeatView {
    name: String,
    hand_count: usize,
    vault: Vec<CardView>,
    shield: bool,
    score: u32,
}

#[derive(Clone, Serialize)]
pub struct View {
    phase: Phase,
    seat: Option<usize>,
    seats: Vec<SeatView>,
    watchers: Vec<String>,
    hand: Vec<CardView>,
    deck_count: usize,
    market: Vec<Option<CardView>>,
    discard_top: Option<CardView>,
    turn: usize,
    deadline_ms: u64,
    scry: Option<Vec<CardView>>,
    turns_left: Option<usize>,
    can_pass: bool,
    events: Vec<EventView>,
    winners: Vec<usize>,
}

impl GameLogic for Hoard {
    type Action = Action;
    type View = View;
    type Effect = GameEffect;
    type InitArgs = HoardArgs;

    fn new(args: HoardArgs) -> Self {
        let seed = args.seed.unwrap_or_else(now_ms);
        Self {
            args,
            place: None,
            rng: StdRng::seed_from_u64(seed),
            phase: Phase::Seating,
            seats: Vec::new(),
            names: HashMap::new(),
            instances: HashMap::new(),
            deck: Vec::new(),
            market: vec![None; MARKET_SLOTS],
            discard: Vec::new(),
            turn: 0,
            scry: None,
            turns_left: None,
            seconds_left: TURN_SECONDS,
            deadline_ms: 0,
            events: Vec::new(),
            seq: 0,
        }
    }

    fn record_instance_id(&mut self, player_id: &str, instance_id: &str) {
        if player_id != "system" {
            self.instances.insert(player_id.to_string(), instance_id.to_string());
        }
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.instances.get(player_id).cloned()
    }

    fn get_player_ids(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }

    fn update(&mut self, player_id: &str, action: Action) -> Result<Vec<GameEffect>> {
        match action {
            Action::View { name, place } => {
                self.place.get_or_insert(place);
                self.names.insert(player_id.to_string(), name);
            }
            Action::Timeout => self.timeout(),
            Action::Input(input) => match input {
                Input::Sit => {
                    if self.phase != Phase::Seating {
                        bail!("The game has started; you can watch.");
                    }
                    if self.seat_of(player_id).is_none() {
                        if self.seats.len() >= MAX_SEATS {
                            bail!("The table is full.");
                        }
                        let name = self.names.get(player_id).cloned().unwrap_or_else(|| player_id.to_string());
                        self.seats.push(Seat { id: player_id.to_string(), name, hand: Vec::new(), vault: Vec::new(), shield: None });
                    }
                }
                Input::Stand => {
                    if self.phase == Phase::Playing {
                        bail!("You can't leave in the middle of a game.");
                    }
                    self.seats.retain(|s| s.id != player_id);
                    self.phase = Phase::Seating;
                }
                Input::Start | Input::Rematch => {
                    if self.seat_of(player_id).is_none() {
                        bail!("Take a seat first.");
                    }
                    if self.phase == Phase::Playing {
                        bail!("The game is already running.");
                    }
                    if self.seats.len() < 2 {
                        bail!("You need at least two players.");
                    }
                    self.deal();
                }
                Input::Bank { card } => {
                    let seat = self.require_turn(player_id)?;
                    self.bank(seat, card)?;
                }
                Input::Cast { card, target, slot, give } => {
                    let seat = self.require_turn(player_id)?;
                    self.cast(seat, card, target, slot, give)?;
                }
                Input::Forage { slot } => {
                    let seat = self.require_turn(player_id)?;
                    self.forage(seat, slot)?;
                }
                Input::Keep { card } => {
                    let seat = self.require_turn(player_id)?;
                    self.keep(seat, card)?;
                }
                Input::Pass => {
                    let seat = self.require_turn(player_id)?;
                    if !self.can_pass(seat) {
                        bail!("You still have a move to make.");
                    }
                    let name = self.seats[seat].name.clone();
                    self.push(format!("{name} passes."), None, Vec::new());
                    self.end_turn();
                }
            },
        }
        Ok(Vec::new())
    }

    fn should_fanout(&self, action: &Action) -> bool {
        !matches!(action, Action::View { .. })
    }

    fn view_for(&self, player_id: &str) -> View {
        let me = self.seat_of(player_id);
        let events = self
            .events
            .iter()
            .map(|e| EventView {
                seq: e.seq,
                text: match (&e.private, me) {
                    (Some((seat, text)), Some(m)) if *seat == m => text.clone(),
                    _ => e.text.clone(),
                },
                motions: e
                    .motions
                    .iter()
                    .map(|m| {
                        let visible = match &m.seen_by {
                            None => true,
                            Some(seats) => me.is_some_and(|s| seats.contains(&s)),
                        };
                        MotionView {
                            from: m.from.clone(),
                            to: m.to.clone(),
                            card_id: m.card.id,
                            card: visible.then(|| (&m.card).into()),
                        }
                    })
                    .collect(),
            })
            .collect();
        let seated: Vec<&String> = self.seats.iter().map(|s| &s.id).collect();
        let mut watchers: Vec<String> = self
            .instances
            .keys()
            .filter(|id| !seated.contains(id))
            .map(|id| self.names.get(id).cloned().unwrap_or_else(|| id.clone()))
            .collect();
        watchers.sort();
        View {
            phase: self.phase,
            seat: me,
            seats: self
                .seats
                .iter()
                .map(|s| SeatView {
                    name: s.name.clone(),
                    hand_count: s.hand.len(),
                    vault: s.vault.iter().map(Into::into).collect(),
                    shield: s.shield.is_some(),
                    score: s.score(),
                })
                .collect(),
            watchers,
            hand: me.map(|s| self.seats[s].hand.iter().map(Into::into).collect()).unwrap_or_default(),
            deck_count: self.deck.len(),
            market: self.market.iter().map(|c| c.as_ref().map(Into::into)).collect(),
            discard_top: self.discard.last().map(Into::into),
            turn: self.turn,
            deadline_ms: if self.phase == Phase::Playing { self.deadline_ms } else { 0 },
            scry: match (me, &self.scry) {
                (Some(s), Some(peek)) if s == self.turn => Some(peek.iter().map(Into::into).collect()),
                _ => None,
            },
            turns_left: self.turns_left,
            can_pass: me.is_some_and(|s| self.phase == Phase::Playing && s == self.turn && self.can_pass(s)),
            events,
            winners: if self.phase == Phase::Over { self.winners() } else { Vec::new() },
        }
    }

    fn tick(&mut self) -> Vec<Action> {
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        self.seconds_left = self.seconds_left.saturating_sub(1);
        if self.seconds_left == 0 { vec![Action::Timeout] } else { Vec::new() }
    }
}

pub fn describe(card: &Card) -> String {
    match card.kind {
        Kind::Treasure { suit, value } => {
            let suit = match suit {
                Suit::Crown => "Crown",
                Suit::Gem => "Gem",
                Suit::Chalice => "Chalice",
            };
            format!("{suit} {value}")
        }
        Kind::Thief => "a Thief".into(),
        Kind::Dragon => "a Dragon".into(),
        Kind::Scry => "a Scry".into(),
        Kind::Shield => "a Shield".into(),
        Kind::Trade => "a Trade".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(players: usize) -> Hoard {
        let (results, _) = mpsc::unbounded_channel();
        let mut g = Hoard::new(HoardArgs { results, seed: Some(7) });
        for i in 0..players {
            let id = format!("p{i}");
            g.update(&id, Action::View { name: format!("P{i}"), place: "l".into() }).unwrap();
            g.update(&id, Action::Input(Input::Sit)).unwrap();
        }
        g.update("p0", Action::Input(Input::Start)).unwrap();
        g
    }

    #[test]
    fn hands_stay_private() {
        let g = game(3);
        let v1 = serde_json::to_value(g.view_for("p1")).unwrap();
        let ids: Vec<u64> = v1["hand"].as_array().unwrap().iter().map(|c| c["id"].as_u64().unwrap()).collect();
        let v0 = serde_json::to_string(&g.view_for("p0")).unwrap();
        for id in ids {
            assert!(!v0.contains(&format!("\"id\":{id},")), "p0 can see p1's card {id}");
        }
        assert_eq!(v1["seats"][0]["hand_count"], 5);
    }

    #[test]
    fn plays_to_the_end_on_timeouts() {
        let mut g = game(2);
        for _ in 0..500 {
            if g.phase == Phase::Over {
                break;
            }
            g.update("system", Action::Timeout).unwrap();
        }
        assert!(g.phase == Phase::Over);
    }

    #[test]
    fn out_of_turn_is_refused() {
        let mut g = game(2);
        let other = 1 - g.turn;
        let id = g.seats[other].id.clone();
        assert!(g.update(&id, Action::Input(Input::Forage { slot: None })).is_err());
    }
}
