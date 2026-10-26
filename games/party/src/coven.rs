//! Coven: a hidden-role card game around a lobby table. Everyone holds a row
//! of face-down verdict cards; whoever holds a Witch is a witch. Players pile
//! accusations on each other until a verdict card is turned over, and when
//! Nightfall is drawn the witches kill while the constable protects. The town
//! wins once every Witch card is face up; the witches win once only witches
//! are left standing.

use std::collections::{BTreeMap, BTreeSet};

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const KEY: &str = "coven";
pub const MIN_PLAYERS: usize = 4;
pub const MAX_PLAYERS: usize = 12;
pub const ACCUSATIONS_TO_REVEAL: u8 = 7;
pub const HAND_AT_START: usize = 3;
pub const DRAWS_PER_TURN: u8 = 2;
pub const VOUCHING_STRENGTH: u8 = 3;
pub const TURN_SECONDS: f32 = 60.0;
pub const DAWN_SECONDS: f32 = 30.0;
pub const NIGHT_SECONDS: f32 = 30.0;
pub const CONFESS_SECONDS: f32 = 30.0;
pub const CHOICE_SECONDS: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Innocent,
    Witch,
    Constable,
}

/// Face-down cards each player starts with and how many Witch cards are dealt.
pub fn deal_plan(players: usize) -> (usize, usize) {
    match players {
        0..=7 => (5, if players <= 5 { 1 } else { 2 }),
        8..=9 => (4, 2),
        _ => (3, 2),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Red,
    Green,
    Blue,
    Black,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Card {
    Rumor,
    Testimony,
    SpectralEvidence,
    Vouching,
    BarnFire,
    Hex,
    Pickpocket,
    Deflection,
    Sanctuary,
    GoodStanding,
    Pillory,
    Betrothal,
    Familiar,
    Nightfall,
    Conspiracy,
}

impl Card {
    pub fn color(self) -> Color {
        match self {
            Card::Rumor | Card::Testimony | Card::SpectralEvidence => Color::Red,
            Card::Vouching | Card::BarnFire | Card::Hex | Card::Pickpocket | Card::Deflection => Color::Green,
            Card::Sanctuary | Card::GoodStanding | Card::Pillory | Card::Betrothal | Card::Familiar => Color::Blue,
            Card::Nightfall | Card::Conspiracy => Color::Black,
        }
    }

    pub fn accusations(self) -> u8 {
        match self {
            Card::Rumor | Card::Familiar => 1,
            Card::Testimony => 3,
            Card::SpectralEvidence => ACCUSATIONS_TO_REVEAL,
            _ => 0,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Card::Rumor => "Rumor",
            Card::Testimony => "Testimony",
            Card::SpectralEvidence => "Spectral Evidence",
            Card::Vouching => "Vouching",
            Card::BarnFire => "Barn Fire",
            Card::Hex => "Hex",
            Card::Pickpocket => "Pickpocket",
            Card::Deflection => "Deflection",
            Card::Sanctuary => "Sanctuary",
            Card::GoodStanding => "Good Standing",
            Card::Pillory => "Pillory",
            Card::Betrothal => "Betrothal",
            Card::Familiar => "Familiar",
            Card::Nightfall => "Nightfall",
            Card::Conspiracy => "Conspiracy",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Card::Rumor => "One accusation against a player.",
            Card::Testimony => "Three accusations against a player.",
            Card::SpectralEvidence => "Seven accusations at once: the accused turns over a verdict card now.",
            Card::Vouching => "Clear up to three accusations from a player.",
            Card::BarnFire => "Burn every blue card in front of a player.",
            Card::Hex => "Discard one blue card in front of a player.",
            Card::Pickpocket => "Take a player's whole hand and give it to another player.",
            Card::Deflection => "Move every red and blue card from one player to another.",
            Card::Sanctuary => "This player cannot be killed at night.",
            Card::GoodStanding => "No red cards may be played on this player.",
            Card::Pillory => "This player skips their next turn.",
            Card::Betrothal => "Bound to whoever holds the other Betrothal: if one is killed at night, both die.",
            Card::Familiar => "The witches' cat. Counts as one accusation and follows the blue cards.",
            Card::Nightfall => "Night falls. The witches choose a victim and the constable chooses whom to guard.",
            Card::Conspiracy => "Everyone takes a face-down verdict card from the player on their left.",
        }
    }

    /// The playing deck before Familiar, Nightfall and Conspiracy are placed.
    pub fn deck() -> Vec<Card> {
        let mut deck = Vec::new();
        let counts = [
            (Card::Rumor, 32),
            (Card::Testimony, 3),
            (Card::SpectralEvidence, 1),
            (Card::Vouching, 3),
            (Card::BarnFire, 2),
            (Card::Hex, 2),
            (Card::Pickpocket, 2),
            (Card::Deflection, 2),
            (Card::Sanctuary, 2),
            (Card::GoodStanding, 2),
            (Card::Pillory, 3),
            (Card::Betrothal, 2),
        ];
        for (card, n) in counts {
            deck.extend(std::iter::repeat_n(card, n));
        }
        deck
    }
}

/// The townsperson each player is for the game and the one edge it gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Character {
    Minister,
    Magistrate,
    Farmer,
    Midwife,
    Innkeeper,
    Blacksmith,
    Widow,
    Servant,
    Trapper,
    Schoolmaster,
    SeaCaptain,
    Physician,
}

pub const CHARACTERS: [Character; 12] = [
    Character::Minister,
    Character::Magistrate,
    Character::Farmer,
    Character::Midwife,
    Character::Innkeeper,
    Character::Blacksmith,
    Character::Widow,
    Character::Servant,
    Character::Trapper,
    Character::Schoolmaster,
    Character::SeaCaptain,
    Character::Physician,
];

impl Character {
    pub fn title(self) -> &'static str {
        match self {
            Character::Minister => "Minister",
            Character::Magistrate => "Magistrate",
            Character::Farmer => "Farmer",
            Character::Midwife => "Midwife",
            Character::Innkeeper => "Innkeeper",
            Character::Blacksmith => "Blacksmith",
            Character::Widow => "Widow",
            Character::Servant => "Servant",
            Character::Trapper => "Trapper",
            Character::Schoolmaster => "Schoolmaster",
            Character::SeaCaptain => "Sea Captain",
            Character::Physician => "Physician",
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            Character::Minister => "Once per game, on your turn, look at one face-down verdict card of any player.",
            Character::Magistrate => "When you place the final accusation, six are enough instead of seven.",
            Character::Farmer => "When you draw, draw three cards instead of two.",
            Character::Midwife => "A Betrothal cannot drag you to your death.",
            Character::Innkeeper => "You may play Vouching on yourself.",
            Character::Blacksmith => "It takes nine accusations to make you turn over a verdict card.",
            Character::Widow => "When the player on your left dies, their hand becomes yours.",
            Character::Servant => "Pickpocket and Deflection cannot be played on you.",
            Character::Trapper => "A Familiar placed in front of you is thrown out at once.",
            Character::Schoolmaster => "You sit out every Conspiracy: you neither give nor take a verdict card.",
            Character::SeaCaptain => "Once per game, on your turn, bring Nightfall to the top of the deck.",
            Character::Physician => "Once per game, on your turn, clear every accusation from any player, yourself included.",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct VerdictSlot {
    pub card: Verdict,
    pub revealed: bool,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub seat: u8,
    pub character: Character,
    pub alive: bool,
    pub ever_witch: bool,
    pub verdicts: Vec<VerdictSlot>,
    pub hand: Vec<Card>,
    pub table: Vec<Card>,
    pub ability_used: bool,
    /// Verdict cards this player has seen through the Minister, as (seat, index).
    pub peeked: Vec<(u8, usize)>,
}

impl Player {
    fn hidden(&self) -> Vec<usize> {
        self.verdicts.iter().enumerate().filter(|(_, v)| !v.revealed).map(|(i, _)| i).collect()
    }

    fn holds(&self, verdict: Verdict) -> bool {
        self.verdicts.iter().any(|v| !v.revealed && v.card == verdict)
    }

    pub fn accusations(&self) -> u8 {
        self.table.iter().map(|c| c.accusations()).sum()
    }

    fn has(&self, card: Card) -> bool {
        self.table.contains(&card)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// The witches agree where the Familiar starts.
    Dawn { votes: BTreeMap<u8, u8>, left: f32 },
    Turn { seat: u8, played: bool, draws_left: u8, left: f32 },
    /// `chooser` picks which face-down card of `accused` turns over.
    Reveal { chooser: u8, accused: u8, left: f32 },
    Conspiracy { picks: BTreeMap<u8, usize>, left: f32 },
    Night { kill_votes: BTreeMap<u8, u8>, guarded: Option<u8>, guard_done: bool, left: f32 },
    Confess { victim: Option<u8>, guarded: Option<u8>, confessed: BTreeSet<u8>, left: f32 },
    Over { winner: Team },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Town,
    Witches,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    PlaceFamiliar { target: u8 },
    Draw,
    Play {
        card: usize,
        #[serde(default)]
        target: Option<u8>,
        /// Where Pickpocket and Deflection deliver, or which blue card Hex burns.
        #[serde(default)]
        second: Option<u8>,
    },
    EndTurn,
    Reveal { index: usize },
    ConspiracyPick { index: usize },
    Kill { target: u8 },
    Guard { target: u8 },
    Confess { index: usize },
    Peek { target: u8, index: usize },
    Tide,
    Cure { target: u8 },
}

#[derive(Debug, PartialEq, Eq)]
pub enum CovenError {
    NotYourTurn,
    WrongPhase,
    NoSuchCard,
    NoSuchPlayer,
    Dead,
    NeedsTarget,
    OnYourself,
    Blocked(&'static str),
    AlreadyPlayed,
    AbilityUsed,
    NotYourCharacter,
    NotAWitch,
    NotTheConstable,
}

impl std::fmt::Display for CovenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CovenError::NotYourTurn => "It is not your turn.",
            CovenError::WrongPhase => "Not right now.",
            CovenError::NoSuchCard => "That card is not there.",
            CovenError::NoSuchPlayer | CovenError::Dead => "That player is out of the game.",
            CovenError::NeedsTarget => "Pick who that is for.",
            CovenError::OnYourself => "You cannot play that on yourself.",
            CovenError::Blocked(why) => why,
            CovenError::AlreadyPlayed => "You already did that this turn.",
            CovenError::AbilityUsed => "You have used your ability.",
            CovenError::NotYourCharacter => "That is not your ability.",
            CovenError::NotAWitch => "Only witches do that.",
            CovenError::NotTheConstable => "Only the constable does that.",
        })
    }
}

impl crate::table::TableGame for Coven {
    fn key(&self) -> &'static str {
        KEY
    }

    fn act(&mut self, seat: u8, action: Value) -> Result<(), String> {
        let action: Action = serde_json::from_value(action).map_err(|_| "That is not a move in this game.".to_string())?;
        Coven::act(self, seat, action).map_err(|e| e.to_string())
    }

    fn tick(&mut self, dt: f32) {
        Coven::tick(self, dt)
    }

    fn hurry(&mut self) {
        Coven::hurry(self)
    }

    fn pending_actors(&self) -> Vec<u8> {
        Coven::pending_actors(self)
    }

    fn view_for(&self, seat: Option<u8>) -> Value {
        Coven::view_for(self, seat)
    }

    fn log_len(&self) -> usize {
        self.log.len()
    }

    fn winners(&self) -> Option<Vec<u8>> {
        let winner = self.winner()?;
        Some(
            self.players
                .iter()
                .filter(|p| match winner {
                    Team::Witches => p.ever_witch,
                    Team::Town => !p.ever_witch,
                })
                .map(|p| p.seat)
                .collect(),
        )
    }
}

/// A turn interrupted by a reveal, a conspiracy or a night resumes here.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TurnState {
    seat: u8,
    played: bool,
    draws_left: u8,
}

#[derive(Clone, Debug)]
pub struct Coven {
    rng: StdRng,
    pub players: Vec<Player>,
    pub deck: Vec<Card>,
    pub discard: Vec<Card>,
    pub phase: Phase,
    resume: Option<TurnState>,
    /// A reveal during a conspiracy goes back to the conspiracy, not the turn.
    conspiracy_after_reveal: bool,
    pub witch_cards: usize,
    pub round: u32,
    pub log: Vec<Value>,
}

impl Coven {
    pub fn new(players: usize, seed: u64) -> Coven {
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&players));
        let mut rng = StdRng::seed_from_u64(seed);
        let (per_player, witches) = deal_plan(players);
        let mut verdicts = vec![Verdict::Witch; witches];
        verdicts.push(Verdict::Constable);
        verdicts.resize(per_player * players, Verdict::Innocent);
        verdicts.shuffle(&mut rng);
        let mut characters = CHARACTERS.to_vec();
        characters.shuffle(&mut rng);
        let mut deck = Card::deck();
        deck.shuffle(&mut rng);
        let mut seats: Vec<Player> = (0..players as u8)
            .map(|seat| Player {
                seat,
                character: characters[seat as usize],
                alive: true,
                ever_witch: false,
                verdicts: verdicts
                    .drain(..per_player)
                    .map(|card| VerdictSlot { card, revealed: false })
                    .collect(),
                hand: deck.drain(..HAND_AT_START).collect(),
                table: Vec::new(),
                ability_used: false,
                peeked: Vec::new(),
            })
            .collect();
        for player in &mut seats {
            player.ever_witch = player.holds(Verdict::Witch);
        }
        deck.push(Card::Conspiracy);
        deck.shuffle(&mut rng);
        deck.insert(0, Card::Nightfall);
        Coven {
            rng,
            players: seats,
            deck,
            discard: Vec::new(),
            phase: Phase::Dawn { votes: BTreeMap::new(), left: DAWN_SECONDS },
            resume: None,
            conspiracy_after_reveal: false,
            witch_cards: witches,
            round: 0,
            log: Vec::new(),
        }
    }

    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Over { .. })
    }

    pub fn winner(&self) -> Option<Team> {
        match self.phase {
            Phase::Over { winner } => Some(winner),
            _ => None,
        }
    }

    pub fn witches(&self) -> Vec<u8> {
        self.players.iter().filter(|p| p.alive && p.ever_witch).map(|p| p.seat).collect()
    }

    pub fn constable(&self) -> Option<u8> {
        self.players.iter().find(|p| p.alive && p.holds(Verdict::Constable)).map(|p| p.seat)
    }

    pub fn familiar_owner(&self) -> Option<u8> {
        self.players.iter().find(|p| p.alive && p.has(Card::Familiar)).map(|p| p.seat)
    }

    fn player(&self, seat: u8) -> Result<&Player, CovenError> {
        self.players.get(seat as usize).ok_or(CovenError::NoSuchPlayer)
    }

    fn living(&self, seat: u8) -> Result<&Player, CovenError> {
        let player = self.player(seat)?;
        if !player.alive {
            return Err(CovenError::Dead);
        }
        Ok(player)
    }

    /// The next living player clockwise from `seat`.
    pub fn left_of(&self, seat: u8) -> u8 {
        let n = self.players.len() as u8;
        let mut next = (seat + 1) % n;
        while !self.players[next as usize].alive && next != seat {
            next = (next + 1) % n;
        }
        next
    }

    fn note(&mut self, event: Value) {
        self.log.push(event);
    }

    pub fn act(&mut self, seat: u8, action: Action) -> Result<(), CovenError> {
        self.living(seat)?;
        match action {
            Action::PlaceFamiliar { target } => self.place_familiar(seat, target),
            Action::Draw => self.draw(seat),
            Action::Play { card, target, second } => self.play(seat, card, target, second),
            Action::EndTurn => self.end_turn(seat),
            Action::Reveal { index } => self.reveal_choice(seat, index),
            Action::ConspiracyPick { index } => self.conspiracy_pick(seat, index),
            Action::Kill { target } => self.kill_vote(seat, target),
            Action::Guard { target } => self.guard(seat, target),
            Action::Confess { index } => self.confess(seat, index),
            Action::Peek { target, index } => self.peek(seat, target, index),
            Action::Tide => self.tide(seat),
            Action::Cure { target } => self.cure(seat, target),
        }
    }

    fn place_familiar(&mut self, seat: u8, target: u8) -> Result<(), CovenError> {
        if !matches!(self.phase, Phase::Dawn { .. }) {
            return Err(CovenError::WrongPhase);
        }
        if !self.players[seat as usize].ever_witch {
            return Err(CovenError::NotAWitch);
        }
        self.living(target)?;
        let witches = self.witches();
        let Phase::Dawn { votes, .. } = &mut self.phase else {
            return Err(CovenError::WrongPhase);
        };
        votes.insert(seat, target);
        if witches.iter().all(|w| votes.contains_key(w)) {
            self.finish_dawn();
        }
        Ok(())
    }

    fn finish_dawn(&mut self) {
        let Phase::Dawn { votes, .. } = &self.phase else { return };
        let target = self.tally(votes).unwrap_or_else(|| self.random_living());
        self.give_table_card(target, Card::Familiar);
        self.note(json!({"type": "dawn", "seat": target}));
        self.round = 1;
        self.phase = Phase::Turn { seat: target, played: false, draws_left: 0, left: TURN_SECONDS };
        self.check_accusations(target, target);
    }

    fn tally(&self, votes: &BTreeMap<u8, u8>) -> Option<u8> {
        let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
        for target in votes.values() {
            *counts.entry(*target).or_default() += 1;
        }
        counts.into_iter().max_by_key(|(_, n)| *n).map(|(seat, _)| seat)
    }

    fn random_living(&mut self) -> u8 {
        let alive: Vec<u8> = self.players.iter().filter(|p| p.alive).map(|p| p.seat).collect();
        alive[self.rng.random_range(0..alive.len())]
    }

    fn turn(&self, seat: u8) -> Result<(bool, u8), CovenError> {
        match self.phase {
            Phase::Turn { seat: current, played, draws_left, .. } if current == seat => Ok((played, draws_left)),
            Phase::Turn { .. } => Err(CovenError::NotYourTurn),
            _ => Err(CovenError::WrongPhase),
        }
    }

    fn draw(&mut self, seat: u8) -> Result<(), CovenError> {
        let (played, _) = self.turn(seat)?;
        if played {
            return Err(CovenError::AlreadyPlayed);
        }
        let n = if self.players[seat as usize].character == Character::Farmer { 3 } else { DRAWS_PER_TURN };
        self.phase = Phase::Turn { seat, played: false, draws_left: n, left: TURN_SECONDS };
        self.continue_drawing();
        Ok(())
    }

    /// Draws until the turn's cards are in hand or a black card interrupts.
    fn continue_drawing(&mut self) {
        loop {
            let Phase::Turn { seat, draws_left, .. } = self.phase else { return };
            if draws_left == 0 {
                self.advance_turn(seat);
                return;
            }
            let Some(card) = self.deck.pop() else {
                self.advance_turn(seat);
                return;
            };
            self.phase = Phase::Turn { seat, played: false, draws_left: draws_left - 1, left: TURN_SECONDS };
            match card {
                Card::Nightfall => {
                    self.note(json!({"type": "nightfall", "seat": seat}));
                    self.discard.push(Card::Nightfall);
                    self.begin_night();
                    return;
                }
                Card::Conspiracy => {
                    self.note(json!({"type": "conspiracy", "seat": seat}));
                    self.discard.push(Card::Conspiracy);
                    self.begin_conspiracy(seat);
                    return;
                }
                card if card.color() == Color::Blue && self.living_count() <= 2 => {
                    self.discard.push(card);
                }
                card => {
                    self.players[seat as usize].hand.push(card);
                    self.note(json!({"type": "draw", "seat": seat}));
                }
            }
        }
    }

    fn living_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive).count()
    }

    fn play(&mut self, seat: u8, index: usize, target: Option<u8>, second: Option<u8>) -> Result<(), CovenError> {
        let (_, draws_left) = self.turn(seat)?;
        if draws_left > 0 {
            return Err(CovenError::WrongPhase);
        }
        let card = *self.players[seat as usize].hand.get(index).ok_or(CovenError::NoSuchCard)?;
        let target = target.ok_or(CovenError::NeedsTarget)?;
        let victim = self.living(target)?.clone();
        let me = self.players[seat as usize].character;
        if target == seat && !(card == Card::Vouching && me == Character::Innkeeper) {
            return Err(CovenError::OnYourself);
        }
        match card {
            Card::Rumor | Card::Testimony | Card::SpectralEvidence if victim.has(Card::GoodStanding) => {
                return Err(CovenError::Blocked("Good Standing turns accusations away"));
            }
            Card::Pickpocket | Card::Deflection => {
                if victim.character == Character::Servant {
                    return Err(CovenError::Blocked("The Servant is beneath notice"));
                }
                let to = second.ok_or(CovenError::NeedsTarget)?;
                if to == seat {
                    return Err(CovenError::OnYourself);
                }
                if to == target {
                    return Err(CovenError::NeedsTarget);
                }
                let receiver = self.living(to)?;
                if receiver.character == Character::Servant {
                    return Err(CovenError::Blocked("The Servant is beneath notice"));
                }
            }
            Card::Hex => {
                let which = second.map(|s| s as usize).ok_or(CovenError::NeedsTarget)?;
                match victim.table.get(which) {
                    Some(c) if c.color() == Color::Blue => {}
                    _ => return Err(CovenError::NoSuchCard),
                }
            }
            Card::Nightfall | Card::Conspiracy => return Err(CovenError::NoSuchCard),
            _ => {}
        }
        self.players[seat as usize].hand.remove(index);
        self.phase = Phase::Turn { seat, played: true, draws_left: 0, left: TURN_SECONDS };
        self.note(json!({"type": "play", "seat": seat, "card": card, "target": target, "second": second}));
        match card {
            Card::Rumor | Card::Testimony | Card::SpectralEvidence => {
                self.give_table_card(target, card);
                self.check_accusations(seat, target);
            }
            Card::Vouching => {
                self.clear_accusations(target, VOUCHING_STRENGTH);
                self.discard.push(card);
            }
            Card::BarnFire => {
                let burnt: Vec<Card> = self.players[target as usize].table.iter().copied().filter(|c| c.color() == Color::Blue).collect();
                self.players[target as usize].table.retain(|c| c.color() != Color::Blue);
                self.discard.extend(burnt);
                self.discard.push(card);
            }
            Card::Hex => {
                let gone = self.players[target as usize].table.remove(second.unwrap() as usize);
                self.discard.push(gone);
                self.discard.push(card);
            }
            Card::Pickpocket => {
                let to = second.unwrap();
                let loot = std::mem::take(&mut self.players[target as usize].hand);
                self.players[to as usize].hand.extend(loot);
                self.discard.push(card);
            }
            Card::Deflection => {
                let to = second.unwrap();
                let moved = std::mem::take(&mut self.players[target as usize].table);
                for c in moved {
                    self.give_table_card(to, c);
                }
                self.discard.push(card);
                self.check_accusations(seat, to);
            }
            Card::Sanctuary | Card::GoodStanding | Card::Pillory | Card::Betrothal | Card::Familiar => {
                self.give_table_card(target, card);
                self.check_accusations(seat, target);
            }
            Card::Nightfall | Card::Conspiracy => unreachable!(),
        }
        Ok(())
    }

    /// Puts a table card in front of a player, honouring the Trapper and the
    /// Betrothal rule that two on one person cancel out.
    fn give_table_card(&mut self, seat: u8, card: Card) {
        let player = &mut self.players[seat as usize];
        if card == Card::Familiar && player.character == Character::Trapper {
            self.discard.push(card);
            return;
        }
        if card == Card::Betrothal && player.has(Card::Betrothal) {
            player.table.retain(|c| *c != Card::Betrothal);
            self.discard.push(Card::Betrothal);
            self.discard.push(Card::Betrothal);
            return;
        }
        player.table.push(card);
    }

    fn clear_accusations(&mut self, seat: u8, strength: u8) {
        let mut budget = strength;
        let mut table = std::mem::take(&mut self.players[seat as usize].table);
        let mut keep = Vec::new();
        table.sort_by_key(|c| c.accusations());
        for card in table {
            if card.color() == Color::Red && card.accusations() <= budget {
                budget -= card.accusations();
                self.discard.push(card);
            } else {
                keep.push(card);
            }
        }
        self.players[seat as usize].table = keep;
    }

    fn reveal_threshold(&self, chooser: u8, accused: u8) -> u8 {
        let mut needed = ACCUSATIONS_TO_REVEAL;
        if self.players[accused as usize].character == Character::Blacksmith {
            needed += 2;
        }
        if self.players[chooser as usize].character == Character::Magistrate {
            needed -= 1;
        }
        needed
    }

    fn check_accusations(&mut self, chooser: u8, accused: u8) {
        let player = &self.players[accused as usize];
        if !player.alive || player.accusations() < self.reveal_threshold(chooser, accused) {
            return;
        }
        if player.hidden().is_empty() {
            return;
        }
        self.interrupt();
        self.phase = Phase::Reveal { chooser, accused, left: CHOICE_SECONDS };
    }

    /// Remembers the running turn so the interrupting phase can hand back to it.
    fn interrupt(&mut self) {
        if let Phase::Turn { seat, played, draws_left, .. } = self.phase {
            self.resume = Some(TurnState { seat, played, draws_left });
        }
    }

    fn resume_turn(&mut self) {
        if self.is_over() {
            return;
        }
        match self.resume.take() {
            Some(TurnState { seat, played, draws_left }) if self.players[seat as usize].alive => {
                self.phase = Phase::Turn { seat, played, draws_left, left: TURN_SECONDS };
                if draws_left > 0 || (!played && draws_left == 0 && self.was_drawing(seat)) {
                    self.continue_drawing();
                }
            }
            Some(TurnState { seat, .. }) => self.advance_turn(seat),
            None => {}
        }
    }

    /// A draw that got interrupted after its last card still has to end the turn.
    fn was_drawing(&self, _seat: u8) -> bool {
        false
    }

    fn reveal_choice(&mut self, seat: u8, index: usize) -> Result<(), CovenError> {
        let Phase::Reveal { chooser, accused, .. } = self.phase else {
            return Err(CovenError::WrongPhase);
        };
        if seat != chooser {
            return Err(CovenError::NotYourTurn);
        }
        let hidden = self.players[accused as usize].hidden();
        if !hidden.contains(&index) {
            return Err(CovenError::NoSuchCard);
        }
        self.reveal(accused, index);
        let reds: Vec<Card> = self.players[accused as usize].table.iter().copied().filter(|c| c.color() == Color::Red).collect();
        self.players[accused as usize].table.retain(|c| c.color() != Color::Red);
        self.discard.extend(reds);
        self.settle();
        if self.is_over() {
            return Ok(());
        }
        if self.conspiracy_after_reveal {
            self.conspiracy_after_reveal = false;
            self.phase = Phase::Conspiracy { picks: BTreeMap::new(), left: CHOICE_SECONDS };
        } else {
            self.resume_turn();
        }
        Ok(())
    }

    fn reveal(&mut self, seat: u8, index: usize) {
        let slot = &mut self.players[seat as usize].verdicts[index];
        slot.revealed = true;
        let card = slot.card;
        self.note(json!({"type": "reveal", "seat": seat, "index": index, "card": card}));
        let player = &self.players[seat as usize];
        if card == Verdict::Witch || player.hidden().is_empty() {
            self.kill(seat, "revealed");
        }
    }

    fn kill(&mut self, seat: u8, how: &str) {
        if !self.players[seat as usize].alive {
            return;
        }
        let n = self.players.len();
        let widow = (0..n)
            .map(|i| ((seat as usize + i + 1) % n) as u8)
            .find(|s| self.players[*s as usize].alive && *s != seat)
            .filter(|s| self.players[*s as usize].character == Character::Widow && self.left_of(*s) == seat);
        let mut hand = std::mem::take(&mut self.players[seat as usize].hand);
        if let Some(widow) = widow {
            self.players[widow as usize].hand.append(&mut hand);
        }
        self.discard.append(&mut hand);
        let mut table = std::mem::take(&mut self.players[seat as usize].table);
        self.discard.append(&mut table);
        let player = &mut self.players[seat as usize];
        player.alive = false;
        for slot in &mut player.verdicts {
            slot.revealed = true;
        }
        let was_witch = player.ever_witch;
        self.note(json!({"type": "death", "seat": seat, "how": how, "witch": was_witch}));
    }

    /// Checks both ways to win after any card turns over or anyone dies.
    fn settle(&mut self) {
        if self.is_over() {
            return;
        }
        let alive: Vec<&Player> = self.players.iter().filter(|p| p.alive).collect();
        let witches_left = alive.iter().filter(|p| p.ever_witch).count();
        if !alive.is_empty() && witches_left == alive.len() {
            self.finish(Team::Witches);
            return;
        }
        let witch_cards_up = self
            .players
            .iter()
            .flat_map(|p| p.verdicts.iter())
            .filter(|v| v.revealed && v.card == Verdict::Witch)
            .count();
        if witch_cards_up >= self.witch_cards {
            self.finish(Team::Town);
        }
    }

    fn finish(&mut self, winner: Team) {
        self.note(json!({"type": "over", "winner": winner}));
        self.phase = Phase::Over { winner };
    }

    fn end_turn(&mut self, seat: u8) -> Result<(), CovenError> {
        let (played, _) = self.turn(seat)?;
        if !played {
            return Err(CovenError::WrongPhase);
        }
        self.advance_turn(seat);
        Ok(())
    }

    fn advance_turn(&mut self, from: u8) {
        if self.is_over() {
            return;
        }
        self.resume = None;
        let mut next = self.left_of(from);
        let mut hops = 0;
        while self.players[next as usize].has(Card::Pillory) && hops < self.players.len() {
            self.players[next as usize].table.retain(|c| *c != Card::Pillory);
            self.discard.push(Card::Pillory);
            self.note(json!({"type": "skipped", "seat": next}));
            next = self.left_of(next);
            hops += 1;
        }
        if next <= from {
            self.round += 1;
        }
        self.phase = Phase::Turn { seat: next, played: false, draws_left: 0, left: TURN_SECONDS };
    }

    fn begin_conspiracy(&mut self, drawer: u8) {
        self.interrupt();
        if let Some(owner) = self.familiar_owner() {
            if !self.players[owner as usize].hidden().is_empty() {
                self.conspiracy_after_reveal = true;
                let chooser = if owner == drawer { owner } else { drawer };
                self.phase = Phase::Reveal { chooser, accused: owner, left: CHOICE_SECONDS };
                return;
            }
        }
        self.phase = Phase::Conspiracy { picks: BTreeMap::new(), left: CHOICE_SECONDS };
    }

    /// Who gives to whom in a conspiracy: each living player takes from the next
    /// living player clockwise, skipping any Schoolmaster on both sides.
    fn conspiracy_pairs(&self) -> Vec<(u8, u8)> {
        let takers: Vec<u8> = self
            .players
            .iter()
            .filter(|p| p.alive && p.character != Character::Schoolmaster)
            .map(|p| p.seat)
            .collect();
        if takers.len() < 2 {
            return Vec::new();
        }
        takers.iter().enumerate().map(|(i, s)| (*s, takers[(i + 1) % takers.len()])).collect()
    }

    fn conspiracy_pick(&mut self, seat: u8, index: usize) -> Result<(), CovenError> {
        let pairs = self.conspiracy_pairs();
        let Phase::Conspiracy { picks, .. } = &mut self.phase else {
            return Err(CovenError::WrongPhase);
        };
        let Some((_, from)) = pairs.iter().find(|(taker, _)| *taker == seat) else {
            return Err(CovenError::WrongPhase);
        };
        if !self.players[*from as usize].hidden().contains(&index) {
            return Err(CovenError::NoSuchCard);
        }
        picks.insert(seat, index);
        if pairs.iter().all(|(taker, _)| picks.contains_key(taker)) {
            self.finish_conspiracy();
        }
        Ok(())
    }

    fn finish_conspiracy(&mut self) {
        let pairs = self.conspiracy_pairs();
        let Phase::Conspiracy { picks, .. } = &self.phase else { return };
        let mut picks = picks.clone();
        for (taker, from) in &pairs {
            if !picks.contains_key(taker) {
                let hidden = self.players[*from as usize].hidden();
                let i = self.rng.random_range(0..hidden.len());
                picks.insert(*taker, hidden[i]);
            }
        }
        let taken: Vec<(u8, VerdictSlot)> = pairs
            .iter()
            .map(|(taker, from)| (*taker, self.players[*from as usize].verdicts[picks[taker]].clone()))
            .collect();
        for (taker, from) in &pairs {
            self.players[*from as usize].verdicts.remove(picks[taker]);
            let removed = picks[taker];
            self.players[*from as usize].peeked.retain(|(s, i)| !(*s == *from && *i == removed));
        }
        for player in &mut self.players {
            player.peeked.clear();
        }
        for (taker, slot) in taken {
            self.players[taker as usize].verdicts.push(slot);
        }
        for player in &mut self.players {
            if player.alive && player.holds(Verdict::Witch) {
                player.ever_witch = true;
            }
        }
        self.note(json!({"type": "conspiracy_done"}));
        self.resume_turn();
    }

    fn begin_night(&mut self) {
        self.interrupt();
        self.phase = Phase::Night {
            kill_votes: BTreeMap::new(),
            guarded: None,
            guard_done: self.constable().is_none(),
            left: NIGHT_SECONDS,
        };
        if self.witches().is_empty() {
            self.finish_night_choices();
        }
    }

    fn kill_vote(&mut self, seat: u8, target: u8) -> Result<(), CovenError> {
        if !self.players[seat as usize].ever_witch {
            return Err(CovenError::NotAWitch);
        }
        self.living(target)?;
        let witches = self.witches();
        let Phase::Night { kill_votes, guard_done, .. } = &mut self.phase else {
            return Err(CovenError::WrongPhase);
        };
        kill_votes.insert(seat, target);
        if *guard_done && witches.iter().all(|w| kill_votes.contains_key(w)) {
            self.finish_night_choices();
        }
        Ok(())
    }

    fn guard(&mut self, seat: u8, target: u8) -> Result<(), CovenError> {
        if self.constable() != Some(seat) {
            return Err(CovenError::NotTheConstable);
        }
        if target == seat {
            return Err(CovenError::OnYourself);
        }
        self.living(target)?;
        let witches = self.witches();
        let Phase::Night { kill_votes, guarded, guard_done, .. } = &mut self.phase else {
            return Err(CovenError::WrongPhase);
        };
        *guarded = Some(target);
        *guard_done = true;
        if witches.iter().all(|w| kill_votes.contains_key(w)) {
            self.finish_night_choices();
        }
        Ok(())
    }

    fn finish_night_choices(&mut self) {
        let Phase::Night { kill_votes, guarded, .. } = &self.phase else { return };
        let victim = self.tally(kill_votes);
        let guarded = *guarded;
        self.phase = Phase::Confess { victim, guarded, confessed: BTreeSet::new(), left: CONFESS_SECONDS };
        self.note(json!({"type": "night"}));
    }

    fn confess(&mut self, seat: u8, index: usize) -> Result<(), CovenError> {
        let hidden = self.players[seat as usize].hidden();
        let Phase::Confess { confessed, .. } = &mut self.phase else {
            return Err(CovenError::WrongPhase);
        };
        if confessed.contains(&seat) {
            return Err(CovenError::AlreadyPlayed);
        }
        if !hidden.contains(&index) {
            return Err(CovenError::NoSuchCard);
        }
        confessed.insert(seat);
        self.note(json!({"type": "confess", "seat": seat}));
        self.reveal(seat, index);
        self.settle();
        Ok(())
    }

    fn finish_night(&mut self) {
        let Phase::Confess { victim, guarded, confessed, .. } = &self.phase else { return };
        let (victim, guarded, confessed) = (*victim, *guarded, confessed.clone());
        if let Some(victim) = victim {
            let target = &self.players[victim as usize];
            let spared = !target.alive || guarded == Some(victim) || confessed.contains(&victim) || target.has(Card::Sanctuary);
            if spared {
                self.note(json!({"type": "spared", "seat": victim}));
            } else {
                let partner = self.betrothed_partner(victim);
                self.kill(victim, "night");
                if let Some(partner) = partner {
                    if self.players[partner as usize].character != Character::Midwife {
                        self.kill(partner, "betrothal");
                    }
                }
            }
        }
        self.settle();
        let mut pile = std::mem::take(&mut self.discard);
        pile.retain(|c| *c != Card::Nightfall);
        pile.shuffle(&mut self.rng);
        self.deck.append(&mut pile);
        self.deck.insert(0, Card::Nightfall);
        self.resume_turn();
    }

    fn betrothed_partner(&self, seat: u8) -> Option<u8> {
        if !self.players[seat as usize].has(Card::Betrothal) {
            return None;
        }
        self.players.iter().find(|p| p.seat != seat && p.alive && p.has(Card::Betrothal)).map(|p| p.seat)
    }

    fn ability(&mut self, seat: u8, character: Character) -> Result<(), CovenError> {
        self.turn(seat)?;
        let player = &self.players[seat as usize];
        if player.character != character {
            return Err(CovenError::NotYourCharacter);
        }
        if player.ability_used {
            return Err(CovenError::AbilityUsed);
        }
        Ok(())
    }

    fn peek(&mut self, seat: u8, target: u8, index: usize) -> Result<(), CovenError> {
        self.ability(seat, Character::Minister)?;
        if target == seat {
            return Err(CovenError::OnYourself);
        }
        if !self.living(target)?.hidden().contains(&index) {
            return Err(CovenError::NoSuchCard);
        }
        let player = &mut self.players[seat as usize];
        player.ability_used = true;
        player.peeked.push((target, index));
        self.note(json!({"type": "peek", "seat": seat, "target": target}));
        Ok(())
    }

    fn tide(&mut self, seat: u8) -> Result<(), CovenError> {
        self.ability(seat, Character::SeaCaptain)?;
        if let Some(pos) = self.deck.iter().position(|c| *c == Card::Nightfall) {
            let night = self.deck.remove(pos);
            self.deck.push(night);
        }
        self.players[seat as usize].ability_used = true;
        self.note(json!({"type": "tide", "seat": seat}));
        Ok(())
    }

    fn cure(&mut self, seat: u8, target: u8) -> Result<(), CovenError> {
        self.ability(seat, Character::Physician)?;
        self.living(target)?;
        self.players[seat as usize].ability_used = true;
        self.clear_accusations(target, u8::MAX);
        self.note(json!({"type": "cure", "seat": seat, "target": target}));
        Ok(())
    }

    /// Who still has to act before the game can move on.
    pub fn pending_actors(&self) -> Vec<u8> {
        match &self.phase {
            Phase::Dawn { votes, .. } => self.witches().into_iter().filter(|w| !votes.contains_key(w)).collect(),
            Phase::Turn { seat, .. } => vec![*seat],
            Phase::Reveal { chooser, .. } => vec![*chooser],
            Phase::Conspiracy { picks, .. } => {
                self.conspiracy_pairs().into_iter().map(|(t, _)| t).filter(|t| !picks.contains_key(t)).collect()
            }
            Phase::Night { kill_votes, guard_done, .. } => {
                let mut waiting: Vec<u8> = self.witches().into_iter().filter(|w| !kill_votes.contains_key(w)).collect();
                if !*guard_done {
                    waiting.extend(self.constable());
                }
                waiting
            }
            Phase::Confess { .. } | Phase::Over { .. } => Vec::new(),
        }
    }

    /// Runs the clocks. Whoever stalls has the game decide for them.
    pub fn tick(&mut self, dt: f32) {
        match &mut self.phase {
            Phase::Dawn { left, .. } | Phase::Turn { left, .. } | Phase::Reveal { left, .. }
            | Phase::Conspiracy { left, .. } | Phase::Night { left, .. } | Phase::Confess { left, .. } => {
                *left -= dt;
                if *left > 0.0 {
                    return;
                }
            }
            Phase::Over { .. } => return,
        }
        self.hurry();
    }

    /// Settles the current phase the way the clock would.
    pub fn hurry(&mut self) {
        match self.phase.clone() {
            Phase::Dawn { .. } => self.finish_dawn(),
            Phase::Turn { seat, played, draws_left, .. } => {
                if played || draws_left > 0 {
                    self.advance_turn(seat);
                } else {
                    let _ = self.draw(seat);
                }
            }
            Phase::Reveal { chooser, accused, .. } => {
                let hidden = self.players[accused as usize].hidden();
                let i = self.rng.random_range(0..hidden.len());
                let _ = self.reveal_choice(chooser, hidden[i]);
            }
            Phase::Conspiracy { .. } => self.finish_conspiracy(),
            Phase::Night { .. } => self.finish_night_choices(),
            Phase::Confess { .. } => self.finish_night(),
            Phase::Over { .. } => {}
        }
    }

    pub fn seconds_left(&self) -> f32 {
        match &self.phase {
            Phase::Dawn { left, .. } | Phase::Turn { left, .. } | Phase::Reveal { left, .. }
            | Phase::Conspiracy { left, .. } | Phase::Night { left, .. } | Phase::Confess { left, .. } => left.max(0.0),
            Phase::Over { .. } => 0.0,
        }
    }

    /// What one seat is allowed to see. Spectators pass `None`.
    pub fn view_for(&self, viewer: Option<u8>) -> Value {
        let me = viewer.map(|s| &self.players[s as usize]);
        let i_am_witch = me.map(|p| p.alive && p.ever_witch).unwrap_or(false);
        let witches = if i_am_witch { self.witches() } else { Vec::new() };
        let players: Vec<Value> = self
            .players
            .iter()
            .map(|p| {
                let verdicts: Vec<Value> = p
                    .verdicts
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let known = v.revealed
                            || viewer == Some(p.seat)
                            || me.map(|m| m.peeked.contains(&(p.seat, i))).unwrap_or(false);
                        json!({"revealed": v.revealed, "card": if known { Some(v.card) } else { None }})
                    })
                    .collect();
                json!({
                    "seat": p.seat,
                    "character": p.character,
                    "character_title": p.character.title(),
                    "alive": p.alive,
                    "witch": if p.alive { i_am_witch && witches.contains(&p.seat) } else { p.ever_witch },
                    "verdicts": verdicts,
                    "table": p.table,
                    "accusations": p.accusations(),
                    "hand_size": p.hand.len(),
                    "ability_used": p.ability_used,
                })
            })
            .collect();
        let phase = match &self.phase {
            Phase::Dawn { votes, .. } => json!({"kind": "dawn", "voted": i_am_witch.then(|| votes.clone())}),
            Phase::Turn { seat, played, draws_left, .. } => json!({"kind": "turn", "seat": seat, "played": played, "drawing": *draws_left > 0}),
            Phase::Reveal { chooser, accused, .. } => json!({"kind": "reveal", "chooser": chooser, "accused": accused}),
            Phase::Conspiracy { picks, .. } => {
                let pairs = self.conspiracy_pairs();
                let mine = viewer.and_then(|v| pairs.iter().find(|(t, _)| *t == v).map(|(_, from)| *from));
                json!({"kind": "conspiracy", "from": mine, "picked": viewer.map(|v| picks.contains_key(&v)).unwrap_or(false), "waiting": pairs.iter().filter(|(t, _)| !picks.contains_key(t)).count()})
            }
            Phase::Night { kill_votes, guard_done, .. } => json!({
                "kind": "night",
                "votes": i_am_witch.then(|| kill_votes.clone()),
                "constable": viewer.is_some() && self.constable() == viewer && !guard_done,
            }),
            Phase::Confess { confessed, .. } => json!({"kind": "confess", "confessed": confessed}),
            Phase::Over { winner } => json!({"kind": "over", "winner": winner}),
        };
        json!({
            "players": players,
            "hand": me.map(|p| p.hand.clone()),
            "phase": phase,
            "seconds_left": self.seconds_left(),
            "deck": self.deck.len(),
            "discard_top": self.discard.last(),
            "round": self.round,
            "witches": witches,
            "constable": viewer.is_some() && self.constable() == viewer,
            "familiar": self.familiar_owner(),
            "log": self.log.iter().rev().take(12).cloned().collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(n: usize) -> Coven {
        let mut g = Coven::new(n, 7);
        g.tick(DAWN_SECONDS + 1.0);
        g
    }

    fn current(g: &Coven) -> u8 {
        match g.phase {
            Phase::Turn { seat, .. } => seat,
            ref other => panic!("not a turn: {other:?}"),
        }
    }

    fn give(g: &mut Coven, seat: u8, card: Card) -> usize {
        g.players[seat as usize].hand.push(card);
        g.players[seat as usize].hand.len() - 1
    }

    #[test]
    fn deals_by_the_player_count() {
        for n in MIN_PLAYERS..=MAX_PLAYERS {
            let g = Coven::new(n, 1);
            let (per, witches) = deal_plan(n);
            assert!(g.players.iter().all(|p| p.verdicts.len() == per && p.hand.len() == HAND_AT_START));
            let all: Vec<Verdict> = g.players.iter().flat_map(|p| p.verdicts.iter().map(|v| v.card)).collect();
            assert_eq!(all.iter().filter(|v| **v == Verdict::Witch).count(), witches);
            assert_eq!(all.iter().filter(|v| **v == Verdict::Constable).count(), 1);
            assert_eq!(g.deck[0], Card::Nightfall);
            assert!(g.deck.contains(&Card::Conspiracy));
            assert_eq!(g.witches().len(), g.players.iter().filter(|p| p.holds(Verdict::Witch)).count());
        }
    }

    #[test]
    fn dawn_places_the_familiar_where_the_witches_agree_and_that_player_starts() {
        let mut g = Coven::new(6, 3);
        let witches = g.witches();
        let target = (0..6u8).find(|s| !witches.contains(s)).unwrap();
        for w in &witches {
            g.act(*w, Action::PlaceFamiliar { target }).unwrap();
        }
        assert!(g.players[target as usize].has(Card::Familiar));
        assert_eq!(current(&g), target);
        assert_eq!(g.act(target, Action::PlaceFamiliar { target }), Err(CovenError::WrongPhase));
    }

    #[test]
    fn a_turn_is_draw_or_play_never_both() {
        let mut g = game(5);
        let seat = current(&g);
        let other = g.left_of(seat);
        let i = give(&mut g, seat, Card::Rumor);
        g.act(seat, Action::Play { card: i, target: Some(other), second: None }).unwrap();
        assert_eq!(g.act(seat, Action::Draw), Err(CovenError::AlreadyPlayed));
        assert_eq!(g.players[other as usize].accusations(), 1 + u8::from(g.players[other as usize].has(Card::Familiar)));
        g.act(seat, Action::EndTurn).unwrap();
        let next = current(&g);
        assert_eq!(next, other);
        let before = g.players[next as usize].hand.len();
        g.act(next, Action::Draw).unwrap();
        assert_eq!(g.players[next as usize].hand.len(), before + 2);
        assert_ne!(current(&g), next);
    }

    #[test]
    fn nobody_plays_on_themselves_except_an_innkeeper_vouching() {
        let mut g = game(4);
        let seat = current(&g);
        let i = give(&mut g, seat, Card::Rumor);
        assert_eq!(g.act(seat, Action::Play { card: i, target: Some(seat), second: None }), Err(CovenError::OnYourself));
        g.players[seat as usize].character = Character::Innkeeper;
        g.players[seat as usize].table.push(Card::Testimony);
        let v = give(&mut g, seat, Card::Vouching);
        g.act(seat, Action::Play { card: v, target: Some(seat), second: None }).unwrap();
        assert!(!g.players[seat as usize].has(Card::Testimony));
    }

    #[test]
    fn seven_accusations_make_the_accuser_pick_a_card_to_turn_over() {
        let mut g = game(6);
        let seat = current(&g);
        let accused = g.left_of(seat);
        g.players[accused as usize].table.retain(|c| *c != Card::Familiar);
        g.players[accused as usize].verdicts = vec![Verdict::Innocent, Verdict::Innocent, Verdict::Witch]
            .into_iter()
            .map(|card| VerdictSlot { card, revealed: false })
            .collect();
        g.players[accused as usize].ever_witch = true;
        g.witch_cards = 1;
        let t = give(&mut g, seat, Card::Testimony);
        g.act(seat, Action::Play { card: t, target: Some(accused), second: None }).unwrap();
        let t = give(&mut g, seat, Card::Testimony);
        g.act(seat, Action::Play { card: t, target: Some(accused), second: None }).unwrap();
        assert!(matches!(g.phase, Phase::Turn { .. }));
        let r = give(&mut g, seat, Card::Rumor);
        g.act(seat, Action::Play { card: r, target: Some(accused), second: None }).unwrap();
        assert_eq!(g.phase, Phase::Reveal { chooser: seat, accused, left: CHOICE_SECONDS });
        assert_eq!(g.act(accused, Action::Reveal { index: 0 }), Err(CovenError::NotYourTurn));
        g.act(seat, Action::Reveal { index: 0 }).unwrap();
        assert!(g.players[accused as usize].verdicts[0].revealed);
        assert_eq!(g.players[accused as usize].accusations(), 0);
        assert!(g.players[accused as usize].alive);
        assert_eq!(current(&g), seat);
        for _ in 0..3 {
            let t = give(&mut g, seat, Card::Testimony);
            g.act(seat, Action::Play { card: t, target: Some(accused), second: None }).unwrap();
        }
        g.act(seat, Action::Reveal { index: 2 }).unwrap();
        assert!(!g.players[accused as usize].alive);
        assert_eq!(g.winner(), Some(Team::Town));
    }

    #[test]
    fn spectral_evidence_forces_a_reveal_on_its_own_and_the_town_needs_every_witch_card() {
        let mut g = game(8);
        let seat = current(&g);
        let accused = g.left_of(seat);
        let e = give(&mut g, seat, Card::SpectralEvidence);
        g.act(seat, Action::Play { card: e, target: Some(accused), second: None }).unwrap();
        assert!(matches!(g.phase, Phase::Reveal { .. }));
        let hidden = g.players[accused as usize].hidden();
        g.act(seat, Action::Reveal { index: hidden[0] }).unwrap();
        assert!(g.winner().is_none() || g.witch_cards == 1);
    }

    #[test]
    fn good_standing_blocks_red_cards_but_deflection_moves_them_anyway() {
        let mut g = game(5);
        let seat = current(&g);
        let a = g.left_of(seat);
        let b = g.left_of(a);
        g.players[b as usize].table.push(Card::GoodStanding);
        let r = give(&mut g, seat, Card::Rumor);
        assert!(matches!(g.act(seat, Action::Play { card: r, target: Some(b), second: None }), Err(CovenError::Blocked(_))));
        g.players[a as usize].table.push(Card::Testimony);
        let d = give(&mut g, seat, Card::Deflection);
        g.act(seat, Action::Play { card: d, target: Some(a), second: Some(b) }).unwrap();
        assert!(g.players[b as usize].has(Card::Testimony));
        assert!(!g.players[a as usize].has(Card::Testimony));
    }

    #[test]
    fn nightfall_lets_the_witches_kill_unless_the_constable_guards_or_the_victim_confesses() {
        let mut g = game(6);
        let seat = current(&g);
        g.deck.push(Card::Nightfall);
        g.act(seat, Action::Draw).unwrap();
        assert!(matches!(g.phase, Phase::Night { .. }));
        let witches = g.witches();
        let constable = g.constable();
        let victim = (0..6u8).find(|s| !witches.contains(s) && Some(*s) != constable).unwrap();
        for w in &witches {
            g.act(*w, Action::Kill { target: victim }).unwrap();
        }
        if let Some(c) = constable {
            assert!(matches!(g.phase, Phase::Night { .. }));
            let other = (0..6u8).find(|s| *s != victim && *s != c).unwrap();
            g.act(c, Action::Guard { target: other }).unwrap();
        }
        assert!(matches!(g.phase, Phase::Confess { .. }));
        g.tick(CONFESS_SECONDS + 1.0);
        assert!(!g.players[victim as usize].alive);
        assert_eq!(g.deck[0], Card::Nightfall);
        assert!(g.discard.is_empty());
        assert!(matches!(g.phase, Phase::Turn { .. }) || g.is_over());
    }

    #[test]
    fn a_guarded_or_confessing_victim_survives_the_night() {
        for confess in [false, true] {
            let mut g = game(7);
            let seat = current(&g);
            g.deck.push(Card::Nightfall);
            g.act(seat, Action::Draw).unwrap();
            let witches = g.witches();
            let constable = g.constable();
            let victim = (0..7u8).find(|s| !witches.contains(s) && Some(*s) != constable).unwrap();
            for w in &witches {
                g.act(*w, Action::Kill { target: victim }).unwrap();
            }
            if let Some(c) = constable {
                g.act(c, Action::Guard { target: if confess { g.left_of(victim) } else { victim } }).unwrap();
            }
            if confess || constable.is_none() {
                let hidden = g.players[victim as usize].hidden();
                g.act(victim, Action::Confess { index: hidden[0] }).unwrap();
            }
            g.tick(CONFESS_SECONDS + 1.0);
            assert!(g.players[victim as usize].alive || g.players[victim as usize].verdicts[0].card == Verdict::Witch);
        }
    }

    #[test]
    fn sanctuary_saves_and_betrothal_drags_a_partner_down() {
        let mut g = game(8);
        let seat = current(&g);
        let witches = g.witches();
        let constable = g.constable();
        let mut others = (0..8u8).filter(|s| !witches.contains(s) && Some(*s) != constable);
        let (victim, partner) = (others.next().unwrap(), others.next().unwrap());
        g.players[victim as usize].table.push(Card::Betrothal);
        g.players[partner as usize].table.push(Card::Betrothal);
        g.players[partner as usize].character = Character::Farmer;
        g.deck.push(Card::Nightfall);
        g.act(seat, Action::Draw).unwrap();
        for w in &witches {
            g.act(*w, Action::Kill { target: victim }).unwrap();
        }
        if let Some(c) = constable {
            g.act(c, Action::Guard { target: if c == g.left_of(c) { seat } else { g.left_of(c) } }).ok();
        }
        g.tick(NIGHT_SECONDS + CONFESS_SECONDS + 2.0);
        assert!(!g.players[victim as usize].alive);
        assert!(!g.players[partner as usize].alive);

        let mut g = game(8);
        let seat = current(&g);
        let witches = g.witches();
        let victim = (0..8u8).find(|s| !witches.contains(s) && *s != seat).unwrap();
        g.players[victim as usize].table.push(Card::Sanctuary);
        g.deck.push(Card::Nightfall);
        g.act(seat, Action::Draw).unwrap();
        for w in &witches {
            g.act(*w, Action::Kill { target: victim }).unwrap();
        }
        g.tick(NIGHT_SECONDS + CONFESS_SECONDS + 2.0);
        assert!(g.players[victim as usize].alive);
    }

    #[test]
    fn witches_win_when_only_witches_are_left() {
        let mut g = game(4);
        let witches = g.witches();
        for p in &mut g.players {
            if !witches.contains(&p.seat) {
                p.alive = false;
            }
        }
        g.players[witches[0] as usize].alive = true;
        g.settle();
        assert_eq!(g.winner(), Some(Team::Witches));
    }

    #[test]
    fn conspiracy_passes_a_hidden_card_left_and_spreads_witchcraft() {
        let mut g = game(5);
        let seat = current(&g);
        for p in &mut g.players {
            p.table.retain(|c| *c != Card::Familiar);
            p.character = Character::Farmer;
        }
        g.deck.push(Card::Conspiracy);
        g.deck.push(Card::Rumor);
        g.deck.push(Card::Rumor);
        g.act(seat, Action::Draw).unwrap();
        assert!(matches!(g.phase, Phase::Conspiracy { .. }));
        let pairs = g.conspiracy_pairs();
        let witch_seat = g.witches()[0];
        let witch_index = g.players[witch_seat as usize]
            .verdicts
            .iter()
            .position(|v| v.card == Verdict::Witch)
            .unwrap();
        let taker = pairs.iter().find(|(_, from)| *from == witch_seat).unwrap().0;
        for (t, from) in &pairs {
            let index = if *t == taker { witch_index } else { g.players[*from as usize].hidden()[0] };
            g.act(*t, Action::ConspiracyPick { index }).unwrap();
        }
        assert!(g.players[taker as usize].holds(Verdict::Witch));
        assert!(g.players[taker as usize].ever_witch);
        assert!(g.players[witch_seat as usize].ever_witch);
        assert!(g.players.iter().all(|p| p.verdicts.len() == 5));
        assert_eq!(current(&g), seat);
        assert_eq!(g.players[seat as usize].hand.len(), HAND_AT_START + 2);
    }

    #[test]
    fn the_familiar_owner_turns_a_card_when_conspiracy_is_drawn() {
        let mut g = game(6);
        let seat = current(&g);
        let owner = g.familiar_owner().unwrap();
        g.deck.push(Card::Conspiracy);
        g.act(seat, Action::Draw).unwrap();
        let chooser = if owner == seat { owner } else { seat };
        assert!(matches!(g.phase, Phase::Reveal { chooser: c, accused, .. } if c == chooser && accused == owner));
        let hidden = g.players[owner as usize].hidden();
        g.act(chooser, Action::Reveal { index: hidden[0] }).unwrap();
        assert!(matches!(g.phase, Phase::Conspiracy { .. }) || g.is_over() || !g.players[owner as usize].alive);
    }

    #[test]
    fn pillory_skips_a_turn_and_pickpocket_moves_a_hand() {
        let mut g = game(5);
        let seat = current(&g);
        let a = g.left_of(seat);
        let b = g.left_of(a);
        for p in &mut g.players {
            p.character = Character::Farmer;
        }
        let p = give(&mut g, seat, Card::Pillory);
        g.act(seat, Action::Play { card: p, target: Some(a), second: None }).unwrap();
        let k = give(&mut g, seat, Card::Pickpocket);
        let loot = g.players[a as usize].hand.len();
        let had = g.players[b as usize].hand.len();
        g.act(seat, Action::Play { card: k, target: Some(a), second: Some(b) }).unwrap();
        assert!(g.players[a as usize].hand.is_empty());
        assert_eq!(g.players[b as usize].hand.len(), had + loot);
        g.act(seat, Action::EndTurn).unwrap();
        assert_eq!(current(&g), b);
        assert!(!g.players[a as usize].has(Card::Pillory));
    }

    #[test]
    fn characters_bend_the_rules() {
        let mut g = game(6);
        let seat = current(&g);
        let other = g.left_of(seat);
        g.players[seat as usize].character = Character::Minister;
        assert_eq!(g.act(seat, Action::Peek { target: seat, index: 0 }), Err(CovenError::OnYourself));
        g.act(seat, Action::Peek { target: other, index: 0 }).unwrap();
        assert_eq!(g.act(seat, Action::Peek { target: other, index: 1 }), Err(CovenError::AbilityUsed));
        let view = g.view_for(Some(seat));
        assert!(view["players"][other as usize]["verdicts"][0]["card"].is_string());
        assert!(view["players"][other as usize]["verdicts"][1]["card"].is_null());

        g.players[seat as usize].character = Character::Blacksmith;
        g.players[other as usize].character = Character::Magistrate;
        assert_eq!(g.reveal_threshold(other, seat), ACCUSATIONS_TO_REVEAL + 1);

        g.players[seat as usize].character = Character::SeaCaptain;
        g.players[seat as usize].ability_used = false;
        g.act(seat, Action::Tide).unwrap();
        assert_eq!(*g.deck.last().unwrap(), Card::Nightfall);

        g.players[seat as usize].character = Character::Physician;
        g.players[seat as usize].ability_used = false;
        g.players[seat as usize].table.push(Card::Testimony);
        g.act(seat, Action::Cure { target: seat }).unwrap();
        assert!(!g.players[seat as usize].has(Card::Testimony));

        g.players[other as usize].character = Character::Servant;
        let k = give(&mut g, seat, Card::Pickpocket);
        assert!(matches!(
            g.act(seat, Action::Play { card: k, target: Some(other), second: Some(g.left_of(other)) }),
            Err(CovenError::Blocked(_))
        ));
    }

    #[test]
    fn views_hide_what_they_should() {
        let g = game(6);
        let seat = current(&g);
        let view = g.view_for(Some(seat));
        assert_eq!(view["hand"].as_array().unwrap().len(), HAND_AT_START);
        for (i, p) in view["players"].as_array().unwrap().iter().enumerate() {
            for v in p["verdicts"].as_array().unwrap() {
                assert_eq!(v["card"].is_string(), i == seat as usize);
            }
        }
        let witch = g.witches()[0];
        let view = g.view_for(Some(witch));
        assert_eq!(view["witches"].as_array().unwrap().len(), g.witches().len());
        let spectator = g.view_for(None);
        assert!(spectator["hand"].is_null());
        assert!(spectator["witches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn stalling_never_stops_the_game() {
        let mut g = Coven::new(9, 11);
        for _ in 0..4000 {
            if g.is_over() {
                break;
            }
            g.tick(TURN_SECONDS + 1.0);
        }
        assert!(g.round > 1);
    }
}
