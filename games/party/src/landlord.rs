//! Landlord: a fast property-trading card game in the style of Monopoly Deal,
//! with its own city. Collect three complete districts to win; cash and deeds
//! pay your debts, and every card can be turned into money instead.

use std::collections::VecDeque;
use std::sync::OnceLock;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const KEY: &str = "landlord";
pub const MIN_PLAYERS: usize = 2;
pub const MAX_PLAYERS: usize = 5;
pub const HAND_AT_START: usize = 5;
pub const HAND_LIMIT: usize = 7;
pub const PLAYS_PER_TURN: u8 = 3;
pub const SETS_TO_WIN: usize = 3;
pub const TURN_SECONDS: f32 = 75.0;
pub const RESPOND_SECONDS: f32 = 20.0;
pub const PAY_SECONDS: f32 = 30.0;
pub const DISCARD_SECONDS: f32 = 20.0;
pub const HOUSE_RENT: u8 = 3;
pub const HOTEL_RENT: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Docks,
    Cannery,
    OldTown,
    Market,
    Foundry,
    Strip,
    Parkside,
    Hilltop,
    Rails,
    Works,
}

pub const GROUPS: [Group; 10] = [
    Group::Docks,
    Group::Cannery,
    Group::OldTown,
    Group::Market,
    Group::Foundry,
    Group::Strip,
    Group::Parkside,
    Group::Hilltop,
    Group::Rails,
    Group::Works,
];

impl Group {
    pub fn size(self) -> usize {
        match self {
            Group::Docks | Group::Hilltop | Group::Works => 2,
            Group::Rails => 4,
            _ => 3,
        }
    }

    pub fn value(self) -> u8 {
        match self {
            Group::Docks | Group::Cannery => 1,
            Group::OldTown | Group::Market | Group::Rails | Group::Works => 2,
            Group::Foundry | Group::Strip => 3,
            Group::Parkside | Group::Hilltop => 4,
        }
    }

    pub fn rents(self) -> &'static [u8] {
        match self {
            Group::Docks => &[1, 2],
            Group::Cannery => &[1, 2, 3],
            Group::OldTown => &[1, 2, 4],
            Group::Market => &[1, 3, 5],
            Group::Foundry => &[2, 3, 6],
            Group::Strip => &[2, 4, 6],
            Group::Parkside => &[2, 4, 7],
            Group::Hilltop => &[3, 8],
            Group::Rails => &[1, 2, 3, 4],
            Group::Works => &[1, 2],
        }
    }

    pub fn buildable(self) -> bool {
        !matches!(self, Group::Rails | Group::Works)
    }

    pub fn title(self) -> &'static str {
        match self {
            Group::Docks => "The Docks",
            Group::Cannery => "Cannery Row",
            Group::OldTown => "Old Town",
            Group::Market => "Market Square",
            Group::Foundry => "Foundry District",
            Group::Strip => "Sunset Strip",
            Group::Parkside => "Parkside",
            Group::Hilltop => "Hilltop",
            Group::Rails => "Rail Lines",
            Group::Works => "The Works",
        }
    }

    fn streets(self) -> &'static [&'static str] {
        match self {
            Group::Docks => &["Pier Nine", "Salt Wharf"],
            Group::Cannery => &["Cannery Row", "Net Loft", "Tide Street"],
            Group::OldTown => &["Chapel Lane", "Cobbler's Court", "Ferry Gate"],
            Group::Market => &["Market Square", "Grain Exchange", "Butcher's Row"],
            Group::Foundry => &["Foundry Gate", "Ironworks Lane", "Furnace Street"],
            Group::Strip => &["Sunset Strip", "Neon Alley", "Marquee Row"],
            Group::Parkside => &["Parkside Terrace", "Elm Avenue", "Orchard Walk"],
            Group::Hilltop => &["Hilltop Manor", "Crown Heights"],
            Group::Rails => &["North Line", "Harbor Line", "Valley Line", "Express Line"],
            Group::Works => &["Water Works", "Power Station"],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Kind {
    Cash,
    Property { group: Group },
    /// A deed for either of two districts, or any district when empty.
    Wild { groups: Vec<Group> },
    /// Charges everyone for one of two districts, or one player for any.
    Rent { groups: Vec<Group> },
    Takeover,
    LawyerUp,
    BackroomDeal,
    TradeUp,
    RepoMan,
    Fundraiser,
    RentHike,
    Renovation,
    Tower,
    Payday,
}

#[derive(Clone, Debug, Serialize)]
pub struct CardSpec {
    pub id: u8,
    pub kind: Kind,
    pub value: u8,
    pub title: String,
}

impl CardSpec {
    pub fn is_property(&self) -> bool {
        matches!(self.kind, Kind::Property { .. } | Kind::Wild { .. })
    }

    /// Districts a deed may sit in.
    pub fn groups(&self) -> Vec<Group> {
        match &self.kind {
            Kind::Property { group } => vec![*group],
            Kind::Wild { groups } | Kind::Rent { groups } if groups.is_empty() => GROUPS.to_vec(),
            Kind::Wild { groups } | Kind::Rent { groups } => groups.clone(),
            _ => Vec::new(),
        }
    }
}

/// Every card in the box, in a fixed order so ids mean the same everywhere.
pub fn catalog() -> &'static [CardSpec] {
    static CATALOG: OnceLock<Vec<CardSpec>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut cards = Vec::new();
        let mut push = |kind: Kind, value: u8, title: String| {
            cards.push(CardSpec { id: cards.len() as u8, kind, value, title });
        };
        for (value, n) in [(1, 6), (2, 5), (3, 3), (4, 3), (5, 2), (10, 1)] {
            for _ in 0..n {
                push(Kind::Cash, value, format!("${value}"));
            }
        }
        for group in GROUPS {
            for street in group.streets() {
                push(Kind::Property { group }, group.value(), street.to_string());
            }
        }
        let wilds: [(&[Group], u8, usize); 8] = [
            (&[Group::Docks, Group::Cannery], 1, 1),
            (&[Group::OldTown, Group::Market], 2, 2),
            (&[Group::Foundry, Group::Strip], 3, 2),
            (&[Group::Parkside, Group::Hilltop], 4, 1),
            (&[Group::Rails, Group::Parkside], 4, 1),
            (&[Group::Rails, Group::Cannery], 4, 1),
            (&[Group::Rails, Group::Works], 2, 1),
            (&[], 0, 2),
        ];
        for (groups, value, n) in wilds {
            for _ in 0..n {
                let title = if groups.is_empty() {
                    "Blank Deed".to_string()
                } else {
                    format!("Deed: {} or {}", groups[0].title(), groups[1].title())
                };
                push(Kind::Wild { groups: groups.to_vec() }, value, title);
            }
        }
        let actions: [(Kind, u8, usize, &str); 10] = [
            (Kind::Takeover, 5, 2, "Hostile Takeover"),
            (Kind::LawyerUp, 4, 3, "Lawyer Up"),
            (Kind::BackroomDeal, 3, 3, "Backroom Deal"),
            (Kind::TradeUp, 3, 3, "Trade Up"),
            (Kind::RepoMan, 3, 3, "Repo Man"),
            (Kind::Fundraiser, 2, 3, "Fundraiser"),
            (Kind::RentHike, 1, 2, "Rent Hike"),
            (Kind::Renovation, 3, 3, "Renovation"),
            (Kind::Tower, 4, 2, "Tower"),
            (Kind::Payday, 1, 10, "Payday"),
        ];
        for (kind, value, n, title) in actions {
            for _ in 0..n {
                push(kind.clone(), value, title.to_string());
            }
        }
        let rents: [(&[Group], u8, usize); 6] = [
            (&[Group::Docks, Group::Cannery], 1, 2),
            (&[Group::OldTown, Group::Market], 1, 2),
            (&[Group::Foundry, Group::Strip], 1, 2),
            (&[Group::Parkside, Group::Hilltop], 1, 2),
            (&[Group::Rails, Group::Works], 1, 2),
            (&[], 3, 3),
        ];
        for (groups, value, n) in rents {
            for _ in 0..n {
                let title = if groups.is_empty() {
                    "Rent Notice".to_string()
                } else {
                    format!("Rent: {} & {}", groups[0].title(), groups[1].title())
                };
                push(Kind::Rent { groups: groups.to_vec() }, value, title);
            }
        }
        cards
    })
}

pub fn card(id: u8) -> &'static CardSpec {
    &catalog()[id as usize]
}

#[derive(Clone, Debug, Serialize)]
pub struct PropertySet {
    pub group: Group,
    pub cards: Vec<u8>,
    pub house: bool,
    pub hotel: bool,
}

impl PropertySet {
    pub fn complete(&self) -> bool {
        self.cards.len() >= self.group.size()
    }

    pub fn rent(&self) -> u8 {
        let rents = self.group.rents();
        let base = rents[self.cards.len().clamp(1, rents.len()) - 1];
        base + u8::from(self.house) * HOUSE_RENT + u8::from(self.hotel) * HOTEL_RENT
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub seat: u8,
    pub hand: Vec<u8>,
    pub bank: Vec<u8>,
    pub sets: Vec<PropertySet>,
}

impl Player {
    pub fn bank_total(&self) -> u32 {
        self.bank.iter().map(|c| card(*c).value as u32).sum()
    }

    /// Everything that can settle a debt: the bank and every deed worth something.
    pub fn payable(&self) -> Vec<u8> {
        let mut cards = self.bank.clone();
        cards.extend(self.sets.iter().flat_map(|s| s.cards.iter().copied()).filter(|c| card(*c).value > 0));
        cards
    }

    pub fn worth(&self) -> u32 {
        self.payable().iter().map(|c| card(*c).value as u32).sum()
    }

    pub fn complete_groups(&self) -> Vec<Group> {
        let mut groups: Vec<Group> = self.sets.iter().filter(|s| s.complete()).map(|s| s.group).collect();
        groups.sort_unstable();
        groups.dedup();
        groups
    }

    fn set_holding(&self, card_id: u8) -> Option<usize> {
        self.sets.iter().position(|s| s.cards.contains(&card_id))
    }

    fn take_from_sets(&mut self, card_id: u8) -> Option<u8> {
        let index = self.set_holding(card_id)?;
        let set = &mut self.sets[index];
        set.cards.retain(|c| *c != card_id);
        let mut dropped = false;
        if !set.complete() {
            dropped = set.house || set.hotel;
            set.house = false;
            set.hotel = false;
        }
        if set.cards.is_empty() {
            self.sets.remove(index);
        }
        let _ = dropped;
        Some(card_id)
    }

    /// Adds a deed to the first set of that district with room, or opens one.
    fn place(&mut self, card_id: u8, group: Group) {
        if let Some(set) = self.sets.iter_mut().find(|s| s.group == group && !s.complete()) {
            set.cards.push(card_id);
            return;
        }
        self.sets.push(PropertySet { group, cards: vec![card_id], house: false, hotel: false });
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Effect {
    Charge { from: u8, to: u8, amount: u32, what: String },
    Steal { from: u8, to: u8, card: u8 },
    Swap { from: u8, to: u8, take: u8, give: u8 },
    Takeover { from: u8, to: u8, group: Group },
}

impl Effect {
    fn victim(&self) -> u8 {
        match self {
            Effect::Charge { from, .. } | Effect::Steal { from, .. } | Effect::Swap { from, .. } | Effect::Takeover { from, .. } => *from,
        }
    }

    fn actor(&self) -> u8 {
        match self {
            Effect::Charge { to, .. } | Effect::Steal { to, .. } | Effect::Swap { to, .. } | Effect::Takeover { to, .. } => *to,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Turn { seat: u8, plays_left: u8, left: f32 },
    /// `target` may Lawyer Up against `effect`; `cancelled` flips with each one.
    Respond { target: u8, effect: Effect, cancelled: bool, left: f32 },
    Pay { from: u8, to: u8, amount: u32, left: f32 },
    Discard { seat: u8, left: f32 },
    Over { winner: u8 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Bank { card: u8 },
    Place { card: u8, #[serde(default)] group: Option<Group> },
    Move { card: u8, group: Group },
    Payday { card: u8 },
    Rent { card: u8, group: Group, #[serde(default)] target: Option<u8>, #[serde(default)] double: Option<u8> },
    Repo { card: u8, target: u8 },
    Fundraiser { card: u8 },
    Backroom { card: u8, target: u8, take: u8 },
    Trade { card: u8, target: u8, take: u8, give: u8 },
    Takeover { card: u8, target: u8, group: Group },
    Build { card: u8, group: Group },
    EndTurn,
    Discard { cards: Vec<u8> },
    Pay { cards: Vec<u8> },
    Lawyer { card: u8 },
    Accept,
}

#[derive(Clone, Debug)]
pub struct Landlord {
    rng: StdRng,
    pub players: Vec<Player>,
    pub deck: Vec<u8>,
    pub discard: Vec<u8>,
    pub phase: Phase,
    /// Effects waiting their turn to be answered and applied.
    queue: VecDeque<Effect>,
    turn_seat: u8,
    plays_left: u8,
    pub round: u32,
    pub log: Vec<Value>,
}

impl Landlord {
    pub fn new(players: usize, seed: u64) -> Landlord {
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&players));
        let mut rng = StdRng::seed_from_u64(seed);
        let mut deck: Vec<u8> = catalog().iter().map(|c| c.id).collect();
        deck.shuffle(&mut rng);
        let seats = (0..players as u8)
            .map(|seat| Player { seat, hand: deck.drain(..HAND_AT_START).collect(), bank: Vec::new(), sets: Vec::new() })
            .collect();
        let mut game = Landlord {
            rng,
            players: seats,
            deck,
            discard: Vec::new(),
            phase: Phase::Turn { seat: 0, plays_left: PLAYS_PER_TURN, left: TURN_SECONDS },
            queue: VecDeque::new(),
            turn_seat: 0,
            plays_left: PLAYS_PER_TURN,
            round: 1,
            log: Vec::new(),
        };
        game.draw_for_turn(0);
        game
    }

    pub fn winner(&self) -> Option<u8> {
        match self.phase {
            Phase::Over { winner } => Some(winner),
            _ => None,
        }
    }

    fn note(&mut self, event: Value) {
        self.log.push(event);
    }

    fn draw(&mut self, seat: u8, n: usize) {
        for _ in 0..n {
            if self.deck.is_empty() {
                let mut pile = std::mem::take(&mut self.discard);
                pile.shuffle(&mut self.rng);
                self.deck = pile;
            }
            let Some(card) = self.deck.pop() else { return };
            self.players[seat as usize].hand.push(card);
        }
    }

    fn draw_for_turn(&mut self, seat: u8) {
        let n = if self.players[seat as usize].hand.is_empty() { 5 } else { 2 };
        self.draw(seat, n);
        self.note(json!({"type": "turn", "seat": seat, "drew": n}));
    }

    fn turn(&self, seat: u8) -> Result<u8, String> {
        match self.phase {
            Phase::Turn { seat: current, plays_left, .. } if current == seat => Ok(plays_left),
            Phase::Turn { .. } => Err("It is not your turn.".into()),
            _ => Err("Not right now.".into()),
        }
    }

    fn hand_card(&self, seat: u8, card_id: u8) -> Result<&'static CardSpec, String> {
        if !self.players[seat as usize].hand.contains(&card_id) {
            return Err("That card is not in your hand.".into());
        }
        Ok(card(card_id))
    }

    fn other(&self, seat: u8, target: u8) -> Result<(), String> {
        if target == seat {
            return Err("Pick someone else.".into());
        }
        if self.players.get(target as usize).is_none() {
            return Err("There is no such player.".into());
        }
        Ok(())
    }

    fn spend(&mut self, seat: u8, card_id: u8, plays: u8) -> Result<(), String> {
        let left = self.turn(seat)?;
        if left < plays {
            return Err("You have no plays left this turn.".into());
        }
        self.players[seat as usize].hand.retain(|c| *c != card_id);
        self.plays_left = left - plays;
        self.phase = Phase::Turn { seat, plays_left: self.plays_left, left: TURN_SECONDS };
        Ok(())
    }

    pub fn act(&mut self, seat: u8, action: Action) -> Result<(), String> {
        if self.players.get(seat as usize).is_none() {
            return Err("There is no such seat.".into());
        }
        match action {
            Action::Bank { card: id } => {
                self.turn(seat)?;
                let spec = self.hand_card(seat, id)?;
                if spec.is_property() {
                    return Err("Deeds go on the table, not in the bank.".into());
                }
                self.spend(seat, id, 1)?;
                self.players[seat as usize].bank.push(id);
                self.note(json!({"type": "bank", "seat": seat, "card": id}));
            }
            Action::Place { card: id, group } => {
                self.turn(seat)?;
                let spec = self.hand_card(seat, id)?;
                if !spec.is_property() {
                    return Err("Only deeds go on the table.".into());
                }
                let choices = spec.groups();
                let group = match group {
                    Some(g) if choices.contains(&g) => g,
                    None if choices.len() == 1 => choices[0],
                    _ => return Err("Pick a district that deed belongs to.".into()),
                };
                self.spend(seat, id, 1)?;
                self.players[seat as usize].place(id, group);
                self.note(json!({"type": "place", "seat": seat, "card": id, "group": group}));
            }
            Action::Move { card: id, group } => {
                self.turn(seat)?;
                let spec = card(id);
                if !matches!(spec.kind, Kind::Wild { .. }) || !spec.groups().contains(&group) {
                    return Err("Only a deed for that district can move there.".into());
                }
                let player = &mut self.players[seat as usize];
                player.take_from_sets(id).ok_or("That deed is not on your table.")?;
                player.place(id, group);
                self.note(json!({"type": "move", "seat": seat, "card": id, "group": group}));
            }
            Action::Payday { card: id } => {
                self.turn(seat)?;
                if card(id).kind != Kind::Payday {
                    return Err("That is not a Payday card.".into());
                }
                self.hand_card(seat, id)?;
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.draw(seat, 2);
                self.note(json!({"type": "payday", "seat": seat}));
            }
            Action::Rent { card: id, group, target, double } => {
                self.turn(seat)?;
                let spec = self.hand_card(seat, id)?;
                let Kind::Rent { groups } = &spec.kind else {
                    return Err("That is not a rent card.".into());
                };
                if !spec.groups().contains(&group) {
                    return Err("That rent card does not cover that district.".into());
                }
                let set_rent = self.players[seat as usize]
                    .sets
                    .iter()
                    .filter(|s| s.group == group)
                    .map(|s| s.rent())
                    .max()
                    .ok_or("You hold nothing in that district.")?;
                let everyone = !groups.is_empty();
                if !everyone {
                    let t = target.ok_or("Pick who pays.")?;
                    self.other(seat, t)?;
                }
                let mut plays = 1;
                if let Some(hike) = double {
                    if card(hike).kind != Kind::RentHike || !self.players[seat as usize].hand.contains(&hike) {
                        return Err("That is not a Rent Hike in your hand.".into());
                    }
                    plays = 2;
                }
                self.spend(seat, id, plays)?;
                self.discard.push(id);
                let mut amount = set_rent as u32;
                if let Some(hike) = double {
                    self.players[seat as usize].hand.retain(|c| *c != hike);
                    self.discard.push(hike);
                    amount *= 2;
                }
                self.note(json!({"type": "rent", "seat": seat, "group": group, "amount": amount, "everyone": everyone}));
                let what = format!("rent for {}", group.title());
                if everyone {
                    for p in 0..self.players.len() as u8 {
                        if p != seat {
                            self.queue.push_back(Effect::Charge { from: p, to: seat, amount, what: what.clone() });
                        }
                    }
                } else {
                    self.queue.push_back(Effect::Charge { from: target.unwrap(), to: seat, amount, what });
                }
                self.resolve();
            }
            Action::Repo { card: id, target } => {
                self.turn(seat)?;
                if card(id).kind != Kind::RepoMan {
                    return Err("That is not a Repo Man.".into());
                }
                self.hand_card(seat, id)?;
                self.other(seat, target)?;
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.note(json!({"type": "repo", "seat": seat, "target": target}));
                self.queue.push_back(Effect::Charge { from: target, to: seat, amount: 5, what: "a debt".into() });
                self.resolve();
            }
            Action::Fundraiser { card: id } => {
                self.turn(seat)?;
                if card(id).kind != Kind::Fundraiser {
                    return Err("That is not a Fundraiser.".into());
                }
                self.hand_card(seat, id)?;
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.note(json!({"type": "fundraiser", "seat": seat}));
                for p in 0..self.players.len() as u8 {
                    if p != seat {
                        self.queue.push_back(Effect::Charge { from: p, to: seat, amount: 2, what: "the fundraiser".into() });
                    }
                }
                self.resolve();
            }
            Action::Backroom { card: id, target, take } => {
                self.turn(seat)?;
                if card(id).kind != Kind::BackroomDeal {
                    return Err("That is not a Backroom Deal.".into());
                }
                self.hand_card(seat, id)?;
                self.other(seat, target)?;
                self.stealable(target, take)?;
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.note(json!({"type": "backroom", "seat": seat, "target": target, "card": take}));
                self.queue.push_back(Effect::Steal { from: target, to: seat, card: take });
                self.resolve();
            }
            Action::Trade { card: id, target, take, give } => {
                self.turn(seat)?;
                if card(id).kind != Kind::TradeUp {
                    return Err("That is not a Trade Up.".into());
                }
                self.hand_card(seat, id)?;
                self.other(seat, target)?;
                self.stealable(target, take)?;
                if self.players[seat as usize].set_holding(give).is_none() {
                    return Err("That deed is not on your table.".into());
                }
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.note(json!({"type": "trade", "seat": seat, "target": target, "take": take, "give": give}));
                self.queue.push_back(Effect::Swap { from: target, to: seat, take, give });
                self.resolve();
            }
            Action::Takeover { card: id, target, group } => {
                self.turn(seat)?;
                if card(id).kind != Kind::Takeover {
                    return Err("That is not a Hostile Takeover.".into());
                }
                self.hand_card(seat, id)?;
                self.other(seat, target)?;
                if !self.players[target as usize].sets.iter().any(|s| s.group == group && s.complete()) {
                    return Err("They have no complete district there.".into());
                }
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                self.note(json!({"type": "takeover", "seat": seat, "target": target, "group": group}));
                self.queue.push_back(Effect::Takeover { from: target, to: seat, group });
                self.resolve();
            }
            Action::Build { card: id, group } => {
                self.turn(seat)?;
                let kind = card(id).kind.clone();
                if !matches!(kind, Kind::Renovation | Kind::Tower) {
                    return Err("That is not something you can build.".into());
                }
                self.hand_card(seat, id)?;
                let set = self.players[seat as usize]
                    .sets
                    .iter()
                    .position(|s| s.group == group && s.complete() && s.group.buildable())
                    .ok_or("Build on a complete district of houses.")?;
                let target = &self.players[seat as usize].sets[set];
                match kind {
                    Kind::Renovation if target.house => return Err("That district already has a renovation.".into()),
                    Kind::Tower if !target.house => return Err("A tower needs a renovation first.".into()),
                    Kind::Tower if target.hotel => return Err("That district already has a tower.".into()),
                    _ => {}
                }
                self.spend(seat, id, 1)?;
                self.discard.push(id);
                let target = &mut self.players[seat as usize].sets[set];
                if kind == Kind::Renovation {
                    target.house = true;
                } else {
                    target.hotel = true;
                }
                self.note(json!({"type": "build", "seat": seat, "group": group, "tower": kind == Kind::Tower}));
            }
            Action::EndTurn => {
                self.turn(seat)?;
                self.finish_turn();
            }
            Action::Discard { cards } => {
                let Phase::Discard { seat: who, .. } = self.phase else {
                    return Err("Not right now.".into());
                };
                if who != seat {
                    return Err("It is not your discard.".into());
                }
                let hand = &self.players[seat as usize].hand;
                let over = hand.len().saturating_sub(HAND_LIMIT);
                if cards.len() != over || !cards.iter().all(|c| hand.contains(c)) {
                    return Err(format!("Discard exactly {over} cards from your hand."));
                }
                self.players[seat as usize].hand.retain(|c| !cards.contains(c));
                self.discard.extend(cards);
                self.next_turn();
            }
            Action::Pay { cards } => {
                let Phase::Pay { from, to, amount, .. } = self.phase.clone() else {
                    return Err("Not right now.".into());
                };
                if from != seat {
                    return Err("It is not your debt.".into());
                }
                let payable = self.players[seat as usize].payable();
                if !cards.iter().all(|c| payable.contains(c)) || cards.iter().collect::<std::collections::BTreeSet<_>>().len() != cards.len() {
                    return Err("Pay with cards from your bank or your table.".into());
                }
                let total: u32 = cards.iter().map(|c| card(*c).value as u32).sum();
                let everything = cards.len() == payable.len();
                if total < amount && !everything {
                    return Err(format!("That comes to ${total}; the debt is ${amount}."));
                }
                self.transfer(from, to, &cards);
                self.note(json!({"type": "paid", "from": from, "to": to, "amount": total}));
                self.after_effect();
            }
            Action::Lawyer { card: id } => {
                let Phase::Respond { target, effect, cancelled, .. } = self.phase.clone() else {
                    return Err("Not right now.".into());
                };
                if target != seat {
                    return Err("It is not your call.".into());
                }
                if card(id).kind != Kind::LawyerUp || !self.players[seat as usize].hand.contains(&id) {
                    return Err("That is not a Lawyer Up in your hand.".into());
                }
                self.players[seat as usize].hand.retain(|c| *c != id);
                self.discard.push(id);
                self.note(json!({"type": "lawyer", "seat": seat}));
                let next = if seat == effect.victim() { effect.actor() } else { effect.victim() };
                if self.players[next as usize].hand.iter().any(|c| card(*c).kind == Kind::LawyerUp) {
                    self.phase = Phase::Respond { target: next, effect, cancelled: !cancelled, left: RESPOND_SECONDS };
                } else {
                    self.settle_response(effect, !cancelled);
                }
            }
            Action::Accept => {
                let Phase::Respond { target, effect, cancelled, .. } = self.phase.clone() else {
                    return Err("Not right now.".into());
                };
                if target != seat {
                    return Err("It is not your call.".into());
                }
                self.settle_response(effect, cancelled);
            }
        }
        self.check_win();
        Ok(())
    }

    fn stealable(&self, target: u8, take: u8) -> Result<(), String> {
        let owner = &self.players[target as usize];
        let index = owner.set_holding(take).ok_or("They do not hold that deed.")?;
        if owner.sets[index].complete() {
            return Err("A complete district can only be taken whole.".into());
        }
        Ok(())
    }

    /// Answers the first queued effect: the victim may lawyer up, or it lands.
    fn resolve(&mut self) {
        let Some(effect) = self.queue.pop_front() else {
            self.phase = Phase::Turn { seat: self.turn_seat, plays_left: self.plays_left, left: TURN_SECONDS };
            return;
        };
        let victim = effect.victim();
        if self.players[victim as usize].hand.iter().any(|c| card(*c).kind == Kind::LawyerUp) {
            self.phase = Phase::Respond { target: victim, effect, cancelled: false, left: RESPOND_SECONDS };
        } else {
            self.apply(effect);
        }
    }

    fn settle_response(&mut self, effect: Effect, cancelled: bool) {
        if cancelled {
            self.note(json!({"type": "blocked", "effect": effect}));
            self.after_effect();
        } else {
            self.apply(effect);
        }
    }

    fn apply(&mut self, effect: Effect) {
        match effect {
            Effect::Charge { from, to, amount, what } => {
                let debtor = &self.players[from as usize];
                if debtor.worth() == 0 {
                    self.note(json!({"type": "broke", "seat": from}));
                    self.after_effect();
                } else if debtor.worth() <= amount {
                    let everything = debtor.payable();
                    self.transfer(from, to, &everything);
                    self.note(json!({"type": "paid", "from": from, "to": to, "amount": everything.iter().map(|c| card(*c).value as u32).sum::<u32>()}));
                    self.after_effect();
                } else {
                    self.phase = Phase::Pay { from, to, amount, left: PAY_SECONDS };
                    self.note(json!({"type": "owes", "from": from, "to": to, "amount": amount, "what": what}));
                }
            }
            Effect::Steal { from, to, card: id } => {
                if let Some(index) = self.players[from as usize].set_holding(id) {
                    let group = self.players[from as usize].sets[index].group;
                    self.players[from as usize].take_from_sets(id);
                    self.players[to as usize].place(id, group);
                    self.note(json!({"type": "stolen", "from": from, "to": to, "card": id}));
                }
                self.after_effect();
            }
            Effect::Swap { from, to, take, give } => {
                let a = self.players[from as usize].set_holding(take).map(|i| self.players[from as usize].sets[i].group);
                let b = self.players[to as usize].set_holding(give).map(|i| self.players[to as usize].sets[i].group);
                if let (Some(take_group), Some(give_group)) = (a, b) {
                    self.players[from as usize].take_from_sets(take);
                    self.players[to as usize].take_from_sets(give);
                    self.players[to as usize].place(take, take_group);
                    self.players[from as usize].place(give, give_group);
                    self.note(json!({"type": "swapped", "from": from, "to": to, "take": take, "give": give}));
                }
                self.after_effect();
            }
            Effect::Takeover { from, to, group } => {
                if let Some(index) = self.players[from as usize].sets.iter().position(|s| s.group == group && s.complete()) {
                    let set = self.players[from as usize].sets.remove(index);
                    self.players[to as usize].sets.push(set);
                    self.note(json!({"type": "taken_over", "from": from, "to": to, "group": group}));
                }
                self.after_effect();
            }
        }
    }

    fn after_effect(&mut self) {
        if self.check_win() {
            return;
        }
        self.resolve();
    }

    fn transfer(&mut self, from: u8, to: u8, cards: &[u8]) {
        for id in cards {
            let spec = card(*id);
            let payer = &mut self.players[from as usize];
            if let Some(i) = payer.bank.iter().position(|c| c == id) {
                payer.bank.remove(i);
                self.players[to as usize].bank.push(*id);
            } else if let Some(index) = payer.set_holding(*id) {
                let group = payer.sets[index].group;
                payer.take_from_sets(*id);
                if spec.is_property() {
                    self.players[to as usize].place(*id, group);
                }
            }
        }
    }

    fn check_win(&mut self) -> bool {
        if matches!(self.phase, Phase::Over { .. }) {
            return true;
        }
        let winner = self.players.iter().find(|p| p.complete_groups().len() >= SETS_TO_WIN).map(|p| p.seat);
        if let Some(winner) = winner {
            self.note(json!({"type": "over", "winner": winner}));
            self.phase = Phase::Over { winner };
            return true;
        }
        false
    }

    fn finish_turn(&mut self) {
        let seat = self.turn_seat;
        if self.players[seat as usize].hand.len() > HAND_LIMIT {
            self.phase = Phase::Discard { seat, left: DISCARD_SECONDS };
            return;
        }
        self.next_turn();
    }

    fn next_turn(&mut self) {
        if self.check_win() {
            return;
        }
        let n = self.players.len() as u8;
        let next = (self.turn_seat + 1) % n;
        if next == 0 {
            self.round += 1;
        }
        self.turn_seat = next;
        self.plays_left = PLAYS_PER_TURN;
        self.phase = Phase::Turn { seat: next, plays_left: PLAYS_PER_TURN, left: TURN_SECONDS };
        self.draw_for_turn(next);
    }

    pub fn pending_actors(&self) -> Vec<u8> {
        match &self.phase {
            Phase::Turn { seat, .. } | Phase::Discard { seat, .. } => vec![*seat],
            Phase::Respond { target, .. } => vec![*target],
            Phase::Pay { from, .. } => vec![*from],
            Phase::Over { .. } => Vec::new(),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        match &mut self.phase {
            Phase::Turn { left, .. } | Phase::Respond { left, .. } | Phase::Pay { left, .. } | Phase::Discard { left, .. } => {
                *left -= dt;
                if *left > 0.0 {
                    return;
                }
            }
            Phase::Over { .. } => return,
        }
        self.hurry();
    }

    /// Settles the current phase the way its clock would.
    pub fn hurry(&mut self) {
        match self.phase.clone() {
            Phase::Turn { .. } => self.finish_turn(),
            Phase::Respond { effect, cancelled, .. } => self.settle_response(effect, cancelled),
            Phase::Pay { from, to, amount, .. } => {
                let cards = self.cheapest_payment(from, amount);
                self.transfer(from, to, &cards);
                self.note(json!({"type": "paid", "from": from, "to": to, "amount": cards.iter().map(|c| card(*c).value as u32).sum::<u32>()}));
                self.after_effect();
            }
            Phase::Discard { seat, .. } => {
                let mut hand = self.players[seat as usize].hand.clone();
                hand.sort_by_key(|c| card(*c).value);
                let over = hand.len().saturating_sub(HAND_LIMIT);
                let cards: Vec<u8> = hand.into_iter().take(over).collect();
                self.players[seat as usize].hand.retain(|c| !cards.contains(c));
                self.discard.extend(cards);
                self.next_turn();
            }
            Phase::Over { .. } => {}
        }
    }

    /// Bank first, then the cheapest deeds, until the debt is covered.
    fn cheapest_payment(&self, from: u8, amount: u32) -> Vec<u8> {
        let player = &self.players[from as usize];
        let mut bank = player.bank.clone();
        bank.sort_by_key(|c| std::cmp::Reverse(card(*c).value));
        let mut deeds: Vec<u8> = player.payable().into_iter().filter(|c| !player.bank.contains(c)).collect();
        deeds.sort_by_key(|c| card(*c).value);
        let mut chosen = Vec::new();
        let mut total = 0u32;
        for id in bank.into_iter().chain(deeds) {
            if total >= amount {
                break;
            }
            total += card(id).value as u32;
            chosen.push(id);
        }
        chosen
    }

    /// A practice bot: grabs what it can from rivals, lays deeds, banks cash.
    pub fn bot_act(&mut self) {
        match self.phase.clone() {
            Phase::Turn { seat, plays_left, .. } => {
                if plays_left == 0 {
                    self.finish_turn();
                    return;
                }
                let hand = self.players[seat as usize].hand.clone();
                if let Some(id) = hand.iter().copied().find(|c| card(*c).kind == Kind::Takeover) {
                    let prize = self
                        .players
                        .iter()
                        .filter(|p| p.seat != seat)
                        .flat_map(|p| p.sets.iter().filter(|s| s.complete()).map(move |s| (p.seat, s.group)))
                        .next();
                    if let Some((target, group)) = prize {
                        if self.act(seat, Action::Takeover { card: id, target, group }).is_ok() {
                            return;
                        }
                    }
                }
                if let Some(id) = hand.iter().copied().find(|c| card(*c).kind == Kind::BackroomDeal) {
                    let mine: Vec<Group> = self.players[seat as usize].sets.iter().map(|s| s.group).collect();
                    let prize = self
                        .players
                        .iter()
                        .filter(|p| p.seat != seat)
                        .flat_map(|p| {
                            p.sets
                                .iter()
                                .filter(|s| !s.complete() && mine.contains(&s.group))
                                .flat_map(move |s| s.cards.iter().map(move |c| (p.seat, *c)))
                        })
                        .next();
                    if let Some((target, take)) = prize {
                        if self.act(seat, Action::Backroom { card: id, target, take }).is_ok() {
                            return;
                        }
                    }
                }
                let pick = hand
                    .iter()
                    .copied()
                    .find(|c| card(*c).is_property())
                    .or_else(|| hand.iter().copied().find(|c| card(*c).kind == Kind::Payday))
                    .or_else(|| hand.iter().copied().max_by_key(|c| card(*c).value));
                let Some(id) = pick else {
                    self.finish_turn();
                    return;
                };
                let spec = card(id);
                let action = if spec.is_property() {
                    Action::Place { card: id, group: spec.groups().first().copied() }
                } else if spec.kind == Kind::Payday {
                    Action::Payday { card: id }
                } else {
                    Action::Bank { card: id }
                };
                if self.act(seat, action).is_err() {
                    self.finish_turn();
                }
            }
            _ => self.hurry(),
        }
    }

    pub fn seconds_left(&self) -> f32 {
        match &self.phase {
            Phase::Turn { left, .. } | Phase::Respond { left, .. } | Phase::Pay { left, .. } | Phase::Discard { left, .. } => left.max(0.0),
            Phase::Over { .. } => 0.0,
        }
    }

    pub fn view_for(&self, viewer: Option<u8>) -> Value {
        let me = viewer.map(|s| &self.players[s as usize]);
        let players: Vec<Value> = self
            .players
            .iter()
            .map(|p| {
                let sets: Vec<Value> = p
                    .sets
                    .iter()
                    .map(|s| json!({"group": s.group, "cards": s.cards, "house": s.house, "hotel": s.hotel, "complete": s.complete(), "rent": s.rent()}))
                    .collect();
                json!({
                    "seat": p.seat,
                    "hand_size": p.hand.len(),
                    "bank": p.bank,
                    "bank_total": p.bank_total(),
                    "sets": sets,
                    "complete": p.complete_groups().len(),
                })
            })
            .collect();
        let phase = match &self.phase {
            Phase::Turn { seat, plays_left, .. } => json!({"kind": "turn", "seat": seat, "plays_left": plays_left}),
            Phase::Respond { target, effect, cancelled, .. } => json!({
                "kind": "respond", "target": target, "effect": effect, "cancelled": cancelled,
                "can_lawyer": self.players[*target as usize].hand.iter().any(|c| card(*c).kind == Kind::LawyerUp),
            }),
            Phase::Pay { from, to, amount, .. } => json!({
                "kind": "pay", "from": from, "to": to, "amount": amount,
                "options": if viewer == Some(*from) { Some(self.players[*from as usize].payable()) } else { None },
            }),
            Phase::Discard { seat, .. } => json!({"kind": "discard", "seat": seat, "over": self.players[*seat as usize].hand.len().saturating_sub(HAND_LIMIT)}),
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
            "queued": self.queue.len(),
            "log": self.log.iter().rev().take(12).cloned().collect::<Vec<_>>(),
        })
    }
}

impl crate::table::TableGame for Landlord {
    fn key(&self) -> &'static str {
        KEY
    }

    fn act(&mut self, seat: u8, action: Value) -> Result<(), String> {
        let action: Action = serde_json::from_value(action).map_err(|_| "That is not a move in this game.".to_string())?;
        Landlord::act(self, seat, action)
    }

    fn tick(&mut self, dt: f32) {
        Landlord::tick(self, dt)
    }

    fn hurry(&mut self) {
        Landlord::hurry(self)
    }

    fn bot_act(&mut self) {
        Landlord::bot_act(self)
    }

    fn pending_actors(&self) -> Vec<u8> {
        Landlord::pending_actors(self)
    }

    fn view_for(&self, seat: Option<u8>) -> Value {
        Landlord::view_for(self, seat)
    }

    fn log_len(&self) -> usize {
        self.log.len()
    }

    fn winners(&self) -> Option<Vec<u8>> {
        self.winner().map(|w| vec![w])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_of(kind: &Kind) -> u8 {
        catalog().iter().find(|c| &c.kind == kind).unwrap().id
    }

    fn ids_of(kind: &Kind) -> Vec<u8> {
        catalog().iter().filter(|c| &c.kind == kind).map(|c| c.id).collect()
    }

    fn street(group: Group, n: usize) -> u8 {
        ids_of(&Kind::Property { group })[n]
    }

    fn give(g: &mut Landlord, seat: u8, id: u8) {
        for p in &mut g.players {
            p.hand.retain(|c| *c != id);
            p.bank.retain(|c| *c != id);
            for s in &mut p.sets {
                s.cards.retain(|c| *c != id);
            }
            p.sets.retain(|s| !s.cards.is_empty());
        }
        g.deck.retain(|c| *c != id);
        g.discard.retain(|c| *c != id);
        g.players[seat as usize].hand.push(id);
    }

    fn lay(g: &mut Landlord, seat: u8, group: Group, n: usize) {
        for i in 0..n {
            let id = street(group, i);
            give(g, seat, id);
            g.players[seat as usize].hand.retain(|c| *c != id);
            g.players[seat as usize].place(id, group);
        }
    }

    /// The browser draws cards from a JSON copy of the catalog; keep it exact.
    #[test]
    fn the_browser_catalog_matches() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../canvases/src/party/landlord/catalog.json");
        let expected = serde_json::to_string_pretty(catalog()).unwrap() + "\n";
        if std::env::var("UPDATE_CATALOG").is_ok() {
            std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
            std::fs::write(path, &expected).unwrap();
        }
        let on_disk = std::fs::read_to_string(path).unwrap_or_default();
        assert_eq!(on_disk, expected, "run UPDATE_CATALOG=1 cargo test -p party the_browser_catalog_matches");
    }

    #[test]
    fn the_box_holds_the_right_cards() {
        let cards = catalog();
        assert_eq!(cards.len(), 106);
        assert_eq!(cards.iter().filter(|c| c.kind == Kind::Cash).count(), 20);
        assert_eq!(cards.iter().filter(|c| matches!(c.kind, Kind::Property { .. })).count(), 28);
        assert_eq!(cards.iter().filter(|c| matches!(c.kind, Kind::Wild { .. })).count(), 11);
        assert_eq!(cards.iter().filter(|c| matches!(c.kind, Kind::Rent { .. })).count(), 13);
        for (i, c) in cards.iter().enumerate() {
            assert_eq!(c.id as usize, i);
        }
    }

    #[test]
    fn a_turn_draws_two_and_allows_three_plays() {
        let mut g = Landlord::new(3, 1);
        assert_eq!(g.players[0].hand.len(), HAND_AT_START + 2);
        assert_eq!(g.players[1].hand.len(), HAND_AT_START);
        let cash = ids_of(&Kind::Cash);
        for id in &cash[..4] {
            give(&mut g, 0, *id);
        }
        for id in &cash[..3] {
            g.act(0, Action::Bank { card: *id }).unwrap();
        }
        assert!(g.act(0, Action::Bank { card: cash[3] }).is_err());
        assert_eq!(g.players[0].bank.len(), 3);
        assert!(g.act(1, Action::EndTurn).is_err());
        let before = g.players[1].hand.len();
        g.act(0, Action::EndTurn).unwrap();
        assert!(matches!(g.phase, Phase::Turn { seat: 1, plays_left: 3, .. }));
        assert_eq!(g.players[1].hand.len(), before + 2);
    }

    #[test]
    fn deeds_go_on_the_table_and_wilds_pick_a_district() {
        let mut g = Landlord::new(2, 2);
        let deed = street(Group::Docks, 0);
        give(&mut g, 0, deed);
        assert!(g.act(0, Action::Bank { card: deed }).is_err());
        g.act(0, Action::Place { card: deed, group: None }).unwrap();
        assert_eq!(g.players[0].sets[0].group, Group::Docks);
        let wild = ids_of(&Kind::Wild { groups: vec![Group::Docks, Group::Cannery] })[0];
        give(&mut g, 0, wild);
        assert!(g.act(0, Action::Place { card: wild, group: Some(Group::Hilltop) }).is_err());
        g.act(0, Action::Place { card: wild, group: Some(Group::Docks) }).unwrap();
        assert!(g.players[0].sets[0].complete());
        g.act(0, Action::Move { card: wild, group: Group::Cannery }).unwrap();
        assert!(!g.players[0].sets[0].complete());
        assert_eq!(g.players[0].sets.len(), 2);
    }

    #[test]
    fn rent_is_charged_and_paid_from_bank_or_deeds() {
        let mut g = Landlord::new(2, 3);
        lay(&mut g, 0, Group::Hilltop, 2);
        let rent = ids_of(&Kind::Rent { groups: vec![Group::Parkside, Group::Hilltop] })[0];
        give(&mut g, 0, rent);
        g.players[1].hand.retain(|c| card(*c).kind != Kind::LawyerUp);
        let five = ids_of(&Kind::Cash).into_iter().find(|c| card(*c).value == 5).unwrap();
        let ten = ids_of(&Kind::Cash).into_iter().find(|c| card(*c).value == 10).unwrap();
        give(&mut g, 1, five);
        give(&mut g, 1, ten);
        g.players[1].hand.retain(|c| *c != five && *c != ten);
        g.players[1].bank.extend([five, ten]);
        g.act(0, Action::Rent { card: rent, group: Group::Hilltop, target: None, double: None }).unwrap();
        assert!(matches!(g.phase, Phase::Pay { from: 1, to: 0, amount: 8, .. }));
        assert!(g.act(1, Action::Pay { cards: vec![five] }).is_err());
        g.act(1, Action::Pay { cards: vec![ten] }).unwrap();
        assert_eq!(g.players[0].bank, vec![ten]);
        assert!(matches!(g.phase, Phase::Turn { seat: 0, plays_left: 2, .. }));
    }

    #[test]
    fn a_broke_player_pays_everything_and_a_rich_one_can_use_deeds() {
        let mut g = Landlord::new(2, 4);
        lay(&mut g, 0, Group::Hilltop, 2);
        g.players[1].hand.clear();
        g.players[1].bank.clear();
        g.players[1].sets.clear();
        lay(&mut g, 1, Group::Docks, 1);
        let repo = id_of(&Kind::RepoMan);
        give(&mut g, 0, repo);
        g.act(0, Action::Repo { card: repo, target: 1 }).unwrap();
        assert!(matches!(g.phase, Phase::Turn { .. }));
        assert!(g.players[1].sets.is_empty());
        assert_eq!(g.players[0].sets.iter().filter(|s| s.group == Group::Docks).count(), 1);
    }

    #[test]
    fn lawyer_up_blocks_and_can_be_countered() {
        let mut g = Landlord::new(2, 5);
        let lawyers = ids_of(&Kind::LawyerUp);
        give(&mut g, 1, lawyers[0]);
        give(&mut g, 0, lawyers[1]);
        g.players[0].hand.retain(|c| *c != lawyers[2]);
        g.players[1].hand.retain(|c| *c != lawyers[2]);
        let fund = id_of(&Kind::Fundraiser);
        give(&mut g, 0, fund);
        let two = ids_of(&Kind::Cash).into_iter().find(|c| card(*c).value == 2).unwrap();
        give(&mut g, 1, two);
        g.players[1].hand.retain(|c| *c != two);
        g.players[1].bank.push(two);
        g.act(0, Action::Fundraiser { card: fund }).unwrap();
        assert!(matches!(g.phase, Phase::Respond { target: 1, cancelled: false, .. }));
        assert!(g.act(0, Action::Accept).is_err());
        g.act(1, Action::Lawyer { card: lawyers[0] }).unwrap();
        assert!(matches!(g.phase, Phase::Respond { target: 0, cancelled: true, .. }));
        g.act(0, Action::Lawyer { card: lawyers[1] }).unwrap();
        assert!(g.players[0].bank.contains(&two), "the counter made the charge land");

        let mut g = Landlord::new(2, 6);
        give(&mut g, 1, lawyers[0]);
        g.players[0].hand.retain(|c| card(*c).kind != Kind::LawyerUp);
        let fund = id_of(&Kind::Fundraiser);
        give(&mut g, 0, fund);
        g.players[1].bank.push(two);
        g.act(0, Action::Fundraiser { card: fund }).unwrap();
        g.act(1, Action::Lawyer { card: lawyers[0] }).unwrap();
        assert!(matches!(g.phase, Phase::Turn { seat: 0, .. }));
        assert!(!g.players[0].bank.contains(&two));
    }

    #[test]
    fn takeover_steals_a_whole_district_and_three_districts_win() {
        let mut g = Landlord::new(2, 7);
        lay(&mut g, 0, Group::Docks, 2);
        lay(&mut g, 0, Group::Works, 2);
        lay(&mut g, 1, Group::Hilltop, 2);
        g.players[1].hand.retain(|c| card(*c).kind != Kind::LawyerUp);
        let takeover = id_of(&Kind::Takeover);
        give(&mut g, 0, takeover);
        assert!(g.act(0, Action::Takeover { card: takeover, target: 1, group: Group::Docks }).is_err());
        g.act(0, Action::Takeover { card: takeover, target: 1, group: Group::Hilltop }).unwrap();
        assert_eq!(g.winner(), Some(0));
        assert_eq!(g.players[0].complete_groups().len(), 3);
    }

    #[test]
    fn backroom_deals_cannot_touch_complete_districts_and_towers_need_renovations() {
        let mut g = Landlord::new(2, 8);
        lay(&mut g, 1, Group::Foundry, 3);
        lay(&mut g, 1, Group::Cannery, 1);
        g.players[1].hand.retain(|c| card(*c).kind != Kind::LawyerUp);
        let deal = id_of(&Kind::BackroomDeal);
        give(&mut g, 0, deal);
        assert!(g.act(0, Action::Backroom { card: deal, target: 1, take: street(Group::Foundry, 0) }).is_err());
        g.act(0, Action::Backroom { card: deal, target: 1, take: street(Group::Cannery, 0) }).unwrap();
        assert!(g.players[0].sets.iter().any(|s| s.group == Group::Cannery));
        lay(&mut g, 0, Group::Strip, 3);
        let tower = id_of(&Kind::Tower);
        let reno = id_of(&Kind::Renovation);
        give(&mut g, 0, tower);
        give(&mut g, 0, reno);
        assert!(g.act(0, Action::Build { card: tower, group: Group::Strip }).is_err());
        g.act(0, Action::Build { card: reno, group: Group::Strip }).unwrap();
        g.act(0, Action::EndTurn).ok();
        g.hurry();
        while !matches!(g.phase, Phase::Turn { seat: 0, .. }) {
            g.hurry();
        }
        give(&mut g, 0, tower);
        g.act(0, Action::Build { card: tower, group: Group::Strip }).unwrap();
        let set = g.players[0].sets.iter().find(|s| s.group == Group::Strip).unwrap();
        assert_eq!(set.rent(), 6 + HOUSE_RENT + HOTEL_RENT);
    }

    #[test]
    fn a_full_hand_must_discard_at_the_end_of_the_turn() {
        let mut g = Landlord::new(2, 9);
        for id in ids_of(&Kind::Cash) {
            if g.players[0].hand.len() < 10 {
                give(&mut g, 0, id);
            }
        }
        g.act(0, Action::EndTurn).unwrap();
        assert!(matches!(g.phase, Phase::Discard { seat: 0, .. }));
        let hand = g.players[0].hand.clone();
        assert!(g.act(0, Action::Discard { cards: hand[..1].to_vec() }).is_err());
        g.act(0, Action::Discard { cards: hand[..3].to_vec() }).unwrap();
        assert_eq!(g.players[0].hand.len(), HAND_LIMIT);
        assert!(matches!(g.phase, Phase::Turn { seat: 1, .. }));
    }

    #[test]
    fn bots_play_a_whole_game_to_the_end() {
        let mut finished = 0;
        for seed in 10..16 {
            let mut g = Landlord::new(3, seed);
            for _ in 0..20000 {
                if g.winner().is_some() {
                    finished += 1;
                    break;
                }
                g.bot_act();
            }
            assert!(g.winner().is_some() || g.round > 100, "seed {seed} stuck at round {} in {:?}", g.round, g.phase);
        }
        assert!(finished >= 3, "only {finished} of 6 bot games ended");
    }

    #[test]
    fn views_keep_hands_private() {
        let g = Landlord::new(3, 11);
        let mine = g.view_for(Some(1));
        assert_eq!(mine["hand"].as_array().unwrap().len(), HAND_AT_START);
        assert!(g.view_for(None)["hand"].is_null());
        assert_eq!(mine["players"][0]["hand_size"], HAND_AT_START as u64 + 2);
    }
}
