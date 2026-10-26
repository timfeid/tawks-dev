//! The party board: players take turns rolling around a loop, collecting
//! coins, and hitting each other with items. Between rounds everyone plays a
//! minigame for more coins. Most coins when the rounds run out wins.

use std::collections::HashMap;
use std::sync::OnceLock;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const START_COINS: i32 = 10;
pub const MAX_HEALTH: u8 = 3;
pub const MAX_ITEMS: usize = 3;
pub const DICE_SIDES: u8 = 6;
pub const COIN_SPACE: i32 = 3;
pub const LOSS_SPACE: i32 = 3;
pub const KNOCKOUT_COINS: i32 = 10;
pub const TRAP_COINS: i32 = 5;
pub const BOMB_COINS: i32 = 5;
pub const GRAPPLE_COINS: i32 = 8;
pub const BAT_SPACES: u8 = 3;
pub const BOMB_RANGE: u8 = 2;
/// How far each item can be thrown, in spaces between you and your mark.
pub const BAT_REACH: u8 = 6;
pub const BOMB_REACH: u8 = 10;
pub const GRAPPLE_REACH: u8 = 14;
pub const SHOP_COST: i32 = 5;
pub const HAZARD_COINS: i32 = 5;
pub const PRIZE_COST: i32 = 15;
/// Coins for finishing first, second, third and fourth in a minigame.
pub const MINIGAME_COINS: [i32; 4] = [10, 6, 3, 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpaceKind {
    Start,
    Coin,
    Loss,
    Item,
    /// Everyone else pays you a toll.
    Event,
    /// Buy an item for coins.
    Shop,
    /// Costs coins and a heart.
    Hazard,
    /// Warps you to its linked space.
    Teleport,
    /// Where the prize can sit; buy it when it is here.
    PrizeSpot,
}

#[derive(Clone, Debug, Serialize)]
pub struct Space {
    pub id: u8,
    pub x: f32,
    pub z: f32,
    pub kind: SpaceKind,
    /// Where a token can step next; more than one means a junction.
    pub next: Vec<u8>,
    /// Where a teleport sends you.
    pub link: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    Bat,
    Bomb,
    Grapple,
    Trap,
}

impl Item {
    pub const ALL: [Item; 4] = [Item::Bat, Item::Bomb, Item::Grapple, Item::Trap];

    pub fn label(self) -> &'static str {
        match self {
            Item::Bat => "Bat",
            Item::Bomb => "Bomb",
            Item::Grapple => "Grapple",
            Item::Trap => "Trap",
        }
    }

    /// Items that pick a rival; the trap goes on a space instead.
    pub fn needs_target(self) -> bool {
        !matches!(self, Item::Trap)
    }

    pub fn reach(self) -> Option<u8> {
        match self {
            Item::Bat => Some(BAT_REACH),
            Item::Bomb => Some(BOMB_REACH),
            Item::Grapple => Some(GRAPPLE_REACH),
            Item::Trap => None,
        }
    }
}

const SPACING: f32 = 3.1;

fn lay(points: &[(f32, f32)], closed: bool) -> Vec<(f32, f32)> {
    let mut out: Vec<(f32, f32)> = Vec::new();
    let mut carry = 0.0f32;
    let count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    for i in 0..count {
        let (ax, az) = points[i];
        let (bx, bz) = points[(i + 1) % points.len()];
        let length = ((bx - ax).powi(2) + (bz - az).powi(2)).sqrt();
        let mut along = carry;
        while along < length {
            let t = along / length;
            out.push((ax + (bx - ax) * t, az + (bz - az) * t));
            along += SPACING;
        }
        carry = along - length;
    }
    if closed && out.len() > 1 {
        let (fx, fz) = out[0];
        let (lx, lz) = *out.last().unwrap();
        if ((lx - fx).powi(2) + (lz - fz).powi(2)).sqrt() < SPACING * 0.75 {
            out.pop();
        }
    }
    out
}

/// The first sample sits on the junction itself, so it is dropped and the
/// branch starts one space clear of the lap.
fn branch(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    lay(points, false).split_off(1)
}

fn nearest(route: &[(f32, f32)], to: (f32, f32), skip: &[u8]) -> u8 {
    route
        .iter()
        .enumerate()
        .filter(|(i, _)| !skip.contains(&(*i as u8)))
        .min_by(|(_, a), (_, b)| {
            let da = (a.0 - to.0).powi(2) + (a.1 - to.1).powi(2);
            let db = (b.0 - to.0).powi(2) + (b.1 - to.1).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}

fn build_board() -> Vec<Space> {
    use SpaceKind::*;
    let lap = lay(
        &[
            (0.0, 20.0),
            (9.0, 19.0),
            (15.0, 15.0),
            (17.0, 8.0),
            (23.0, 4.0),
            (26.0, -3.0),
            (21.0, -9.0),
            (13.0, -11.0),
            (8.0, -16.0),
            (0.0, -18.0),
            (-9.0, -16.0),
            (-14.0, -10.0),
            (-22.0, -8.0),
            (-27.0, -1.0),
            (-24.0, 6.0),
            (-17.0, 9.0),
            (-14.0, 15.0),
            (-7.0, 19.0),
        ],
        true,
    );
    let shortcut = branch(&[
        (15.0, 14.4),
        (9.0, 6.0),
        (5.0, -2.0),
        (4.0, -9.5),
        (7.6, -15.6),
    ]);
    let spur = branch(&[(-14.2, -9.8), (-18.5, -3.0), (-13.0, 2.0), (-8.5, 4.5)]);

    let lap_kinds = [
        Coin, Item, Coin, Loss, Shop, Coin, Hazard, Coin, Item, Event, Coin, Loss, Coin, PrizeSpot,
        Coin, Item, Hazard, Coin, Loss, Coin, Shop, Coin, Item, Event, Coin, Hazard, Coin, Loss,
        PrizeSpot, Coin, Item, Coin, Shop, Coin, Hazard, Coin,
    ];
    let shortcut_kinds = [Hazard, Loss, Item, Hazard, Loss, Hazard, Item];
    let spur_kinds = [Coin, Loss, Item, PrizeSpot, Coin];

    let mut spaces: Vec<Space> = Vec::new();
    for (i, (x, z)) in lap.iter().enumerate() {
        let kind = if i == 0 {
            Start
        } else {
            lap_kinds[(i - 1) % lap_kinds.len()]
        };
        spaces.push(Space {
            id: i as u8,
            x: *x,
            z: *z,
            kind,
            next: vec![((i + 1) % lap.len()) as u8],
            link: None,
        });
    }

    let rejoin = nearest(&lap, *shortcut.last().unwrap(), &[]);
    let shortcut_first = spaces.len() as u8;
    for (i, (x, z)) in shortcut.iter().enumerate() {
        let id = shortcut_first + i as u8;
        let last = i + 1 == shortcut.len();
        let next = if last { rejoin } else { id + 1 };
        spaces.push(Space {
            id,
            x: *x,
            z: *z,
            kind: shortcut_kinds[i % shortcut_kinds.len()],
            next: vec![next],
            link: None,
        });
    }
    let shortcut_entry = nearest(&lap, shortcut[0], &[]);
    spaces[shortcut_entry as usize].next.push(shortcut_first);

    let warp = nearest(&lap, (-14.0, 15.0), &[]);
    let spur_first = spaces.len() as u8;
    for (i, (x, z)) in spur.iter().enumerate() {
        let id = spur_first + i as u8;
        let last = i + 1 == spur.len();
        let kind = if last {
            Teleport
        } else {
            spur_kinds[i % spur_kinds.len()]
        };
        let next = if last { warp } else { id + 1 };
        spaces.push(Space {
            id,
            x: *x,
            z: *z,
            kind,
            next: vec![next],
            link: if last { Some(warp) } else { None },
        });
    }
    let spur_entry = nearest(&lap, spur[0], &[shortcut_entry]);
    spaces[spur_entry as usize].next.push(spur_first);

    spaces
}

pub fn prize_spots() -> Vec<u8> {
    board()
        .iter()
        .filter(|s| s.kind == SpaceKind::PrizeSpot)
        .map(|s| s.id)
        .collect()
}

pub fn board() -> &'static [Space] {
    static BOARD: OnceLock<Vec<Space>> = OnceLock::new();
    BOARD.get_or_init(build_board)
}

pub fn space(id: u8) -> &'static Space {
    &board()[id as usize]
}

/// Steps between two spaces along the board, for range checks.
/// How far apart two spaces are, whichever way round the board is shorter.
pub fn gap(from: u8, to: u8) -> u8 {
    distance(from, to).min(distance(to, from))
}

pub fn distance(from: u8, to: u8) -> u8 {
    let mut frontier = vec![from];
    let mut seen = vec![from];
    for step in 0..board().len() as u8 {
        if frontier.contains(&to) {
            return step;
        }
        let mut next = Vec::new();
        for id in frontier {
            for candidate in &space(id).next {
                if !seen.contains(candidate) {
                    seen.push(*candidate);
                    next.push(*candidate);
                }
            }
        }
        frontier = next;
    }
    u8::MAX
}

#[derive(Clone, Debug, Serialize)]
pub struct Token {
    pub seat: u8,
    pub space: u8,
    pub coins: i32,
    pub health: u8,
    pub items: Vec<Item>,
    /// Prizes bought; they beat coins in the standings.
    pub prizes: u32,
    /// Turns this player sits out after a knockout.
    pub resting: u8,
}

/// One way on from a junction. The lap is always the first way out of a
/// space, so anything after it is a shortcut.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BranchOption {
    pub node: u8,
    pub kind: SpaceKind,
    pub shortcut: bool,
    /// Spaces saved by taking this way back to where the ways meet again.
    pub saves: u8,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum TurnStep {
    /// Use an item or pass.
    Items,
    Roll,
    /// The roll landed on a junction; pick the way on.
    Branch {
        options: Vec<BranchOption>,
        left: u8,
    },
    Done,
}

/// Where two ways out of a junction meet again, and how far each one is.
fn branch_options(from: u8) -> Vec<BranchOption> {
    let ways = &space(from).next;
    let lap = ways[0];
    ways.iter()
        .enumerate()
        .map(|(i, node)| {
            let shortcut = i > 0;
            let saves = if shortcut {
                // Both ways rejoin; the gap in steps is what the shortcut saves.
                let rejoin = (0..board().len() as u8)
                    .filter(|id| distance(*node, *id) < u8::MAX && distance(lap, *id) < u8::MAX)
                    .min_by_key(|id| distance(*node, *id) + distance(lap, *id));
                rejoin
                    .map(|id| distance(lap, id).saturating_sub(distance(*node, id)))
                    .unwrap_or(0)
            } else {
                0
            };
            BranchOption {
                node: *node,
                kind: space(*node).kind,
                shortcut,
                saves,
            }
        })
        .collect()
}

#[derive(Debug, PartialEq)]
pub enum BoardError {
    NotYourTurn,
    WrongStep,
    NoSuchItem,
    NeedsTarget,
    UnknownTarget,
    NotAnOption,
    OutOfReach,
}

impl std::fmt::Display for BoardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BoardError::NotYourTurn => "Wait for your turn.",
            BoardError::WrongStep => "Not right now.",
            BoardError::NoSuchItem => "You do not have that item.",
            BoardError::NeedsTarget => "Pick someone first.",
            BoardError::UnknownTarget => "That player is not on the board.",
            BoardError::NotAnOption => "That way is not open.",
            BoardError::OutOfReach => "Too far away. Get closer, or use something with more reach.",
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BoardView {
    pub spaces: &'static [Space],
    pub tokens: Vec<Token>,
    pub traps: Vec<u8>,
    /// Where the prize is sitting, and what it costs.
    pub prize_at: u8,
    pub prize_cost: i32,
    pub order: Vec<u8>,
    pub turn_seat: u8,
    pub turn: TurnStep,
    pub round: u32,
    pub rounds: u32,
    pub last_roll: Option<u8>,
    pub log: Vec<Value>,
    /// How far the player taking their turn is from everyone else.
    pub aim: Vec<AimView>,
    pub reach: Vec<ReachView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AimView {
    pub seat: u8,
    pub spaces: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct ReachView {
    pub item: Item,
    pub spaces: u8,
}

pub struct Board {
    tokens: Vec<Token>,
    traps: HashMap<u8, u8>,
    order: Vec<u8>,
    turn_index: usize,
    step: TurnStep,
    pub round: u32,
    pub rounds: u32,
    prize_at: u8,
    last_roll: Option<u8>,
    rng: StdRng,
    log: Vec<Value>,
    events: Vec<Value>,
}

impl Board {
    pub fn new(seats: &[u8], rounds: u32, seed: u64) -> Self {
        let mut board = Board {
            tokens: seats
                .iter()
                .map(|seat| Token {
                    seat: *seat,
                    space: 0,
                    coins: START_COINS,
                    health: MAX_HEALTH,
                    items: Vec::new(),
                    prizes: 0,
                    resting: 0,
                })
                .collect(),
            traps: HashMap::new(),
            order: seats.to_vec(),
            turn_index: 0,
            step: TurnStep::Items,
            round: 1,
            rounds,
            prize_at: prize_spots().first().copied().unwrap_or(0),
            last_roll: None,
            rng: StdRng::seed_from_u64(seed),
            log: Vec::new(),
            events: Vec::new(),
        };
        let spots = prize_spots();
        if !spots.is_empty() {
            let pick = board.rng.random_range(0..spots.len());
            board.prize_at = spots[pick];
        }
        board.say(json!({ "kind": "round", "round": 1 }));
        board.skip_resting();
        board
    }

    fn say(&mut self, entry: Value) {
        self.log.push(entry.clone());
        self.events.push(entry);
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    pub fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
    }

    pub fn view(&self) -> BoardView {
        BoardView {
            spaces: board(),
            tokens: self.tokens.clone(),
            traps: self.traps.keys().copied().collect(),
            prize_at: self.prize_at,
            prize_cost: PRIZE_COST,
            order: self.order.clone(),
            turn_seat: self.turn_seat(),
            turn: self.step.clone(),
            round: self.round,
            rounds: self.rounds,
            last_roll: self.last_roll,
            log: self.log.clone(),
            aim: self.aim(),
            reach: Item::ALL
                .iter()
                .filter_map(|item| {
                    item.reach().map(|spaces| ReachView {
                        item: *item,
                        spaces,
                    })
                })
                .collect(),
        }
    }

    fn aim(&self) -> Vec<AimView> {
        let seat = self.turn_seat();
        let from = self
            .tokens
            .iter()
            .find(|t| t.seat == seat)
            .map(|t| t.space)
            .unwrap_or(0);
        self.tokens
            .iter()
            .filter(|t| t.seat != seat)
            .map(|t| AimView {
                seat: t.seat,
                spaces: gap(from, t.space),
            })
            .collect()
    }

    pub fn turn_seat(&self) -> u8 {
        self.order[self.turn_index % self.order.len()]
    }

    pub fn step(&self) -> &TurnStep {
        &self.step
    }

    /// Seats with their score: prizes first, coins as the tiebreak.
    pub fn standings(&self) -> Vec<(u8, (u32, i32))> {
        let mut rows: Vec<(u8, (u32, i32))> = self
            .tokens
            .iter()
            .map(|t| (t.seat, (t.prizes, t.coins)))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        rows
    }

    /// Seats in placement order, tied seats grouped, for the final standings.
    pub fn placements(&self) -> Vec<Vec<u8>> {
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, i32)> = None;
        for (seat, score) in self.standings() {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == score => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(score);
        }
        groups
    }

    fn token(&mut self, seat: u8) -> Option<&mut Token> {
        self.tokens.iter_mut().find(|t| t.seat == seat)
    }

    fn coins(&mut self, seat: u8, delta: i32) {
        if let Some(token) = self.token(seat) {
            token.coins = (token.coins + delta).max(0);
        }
    }

    pub fn use_item(&mut self, seat: u8, item: Item, target: Option<u8>) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if self.step != TurnStep::Items {
            return Err(BoardError::WrongStep);
        }
        let holder = self
            .tokens
            .iter()
            .position(|t| t.seat == seat)
            .ok_or(BoardError::UnknownTarget)?;
        let slot = self.tokens[holder]
            .items
            .iter()
            .position(|held| *held == item)
            .ok_or(BoardError::NoSuchItem)?;
        if item.needs_target() {
            let target = target.ok_or(BoardError::NeedsTarget)?;
            let mark = self
                .tokens
                .iter()
                .find(|t| t.seat == target && t.seat != seat)
                .ok_or(BoardError::UnknownTarget)?;
            if let Some(reach) = item.reach() {
                if gap(self.tokens[holder].space, mark.space) > reach {
                    return Err(BoardError::OutOfReach);
                }
            }
        }
        self.tokens[holder].items.remove(slot);

        match item {
            Item::Bat => {
                let target = target.unwrap();
                let from = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == target)
                    .map(|t| t.space)
                    .unwrap_or(0);
                let to = self.walk_back(from, BAT_SPACES);
                if let Some(token) = self.token(target) {
                    token.space = to;
                }
                self.say(json!({ "kind": "item", "item": "bat", "by": seat, "target": target, "from": from, "to": to }));
                self.hurt(target, 1);
            }
            Item::Bomb => {
                let target = target.unwrap();
                let center = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == target)
                    .map(|t| t.space)
                    .unwrap_or(0);
                let caught: Vec<u8> = self
                    .tokens
                    .iter()
                    .filter(|t| {
                        t.seat != seat
                            && distance(t.space, center).min(distance(center, t.space))
                                <= BOMB_RANGE
                    })
                    .map(|t| t.seat)
                    .collect();
                self.say(json!({ "kind": "item", "item": "bomb", "by": seat, "target": target, "space": center, "caught": caught.clone(), "coins": BOMB_COINS }));
                for hit in caught {
                    self.coins(hit, -BOMB_COINS);
                    self.hurt(hit, 1);
                }
            }
            Item::Grapple => {
                let target = target.unwrap();
                let stolen = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == target)
                    .map(|t| t.coins.min(GRAPPLE_COINS))
                    .unwrap_or(0);
                self.coins(target, -stolen);
                self.coins(seat, stolen);
                self.say(json!({ "kind": "item", "item": "grapple", "by": seat, "target": target, "coins": stolen }));
            }
            Item::Trap => {
                let at = self.tokens[holder].space;
                self.traps.insert(at, seat);
                self.say(json!({ "kind": "item", "item": "trap", "by": seat, "space": at }));
            }
        }
        Ok(())
    }

    pub fn pass_items(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if self.step != TurnStep::Items {
            return Err(BoardError::WrongStep);
        }
        self.step = TurnStep::Roll;
        Ok(())
    }

    pub fn roll(&mut self, seat: u8) -> Result<u8, BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if !matches!(self.step, TurnStep::Roll | TurnStep::Items) {
            return Err(BoardError::WrongStep);
        }
        let roll = self.rng.random_range(1..=DICE_SIDES);
        self.last_roll = Some(roll);
        self.say(json!({ "kind": "roll", "seat": seat, "roll": roll }));
        self.advance(seat, roll);
        Ok(roll)
    }

    /// Walk forward one space at a time, stopping at a junction to ask.
    fn advance(&mut self, seat: u8, steps: u8) {
        let mut left = steps;
        while left > 0 {
            let at = self
                .tokens
                .iter()
                .find(|t| t.seat == seat)
                .map(|t| t.space)
                .unwrap_or(0);
            let ways = &space(at).next;
            if ways.len() > 1 {
                self.step = TurnStep::Branch {
                    options: branch_options(at),
                    left,
                };
                return;
            }
            let to = ways[0];
            if let Some(token) = self.token(seat) {
                token.space = to;
            }
            left -= 1;
        }
        self.land(seat);
    }

    pub fn choose_branch(&mut self, seat: u8, node: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        let TurnStep::Branch { options, left } = self.step.clone() else {
            return Err(BoardError::WrongStep);
        };
        if !options.iter().any(|option| option.node == node) {
            return Err(BoardError::NotAnOption);
        }
        if let Some(token) = self.token(seat) {
            token.space = node;
        }
        self.say(json!({ "kind": "branch", "seat": seat, "space": node }));
        self.step = TurnStep::Roll;
        self.advance(seat, left - 1);
        Ok(())
    }

    fn walk_back(&self, from: u8, steps: u8) -> u8 {
        let mut at = from;
        for _ in 0..steps {
            // The way back is whichever space leads here; the first one will do.
            match board().iter().find(|s| s.next.contains(&at)) {
                Some(previous) => at = previous.id,
                None => break,
            }
        }
        at
    }

    fn hurt(&mut self, seat: u8, damage: u8) {
        let (knocked, health) = {
            let Some(token) = self.token(seat) else {
                return;
            };
            token.health = token.health.saturating_sub(damage);
            (token.health == 0, token.health)
        };
        self.say(json!({ "kind": "hurt", "seat": seat, "damage": damage, "health": health }));
        if knocked {
            self.coins(seat, -KNOCKOUT_COINS);
            if let Some(token) = self.token(seat) {
                token.health = MAX_HEALTH;
                token.resting = 1;
            }
            self.say(json!({ "kind": "knockout", "seat": seat, "coins": KNOCKOUT_COINS }));
        }
    }

    fn land(&mut self, seat: u8) {
        let at = self
            .tokens
            .iter()
            .find(|t| t.seat == seat)
            .map(|t| t.space)
            .unwrap_or(0);
        match space(at).kind {
            SpaceKind::Shop => {
                let affordable = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == seat)
                    .map(|t| t.coins >= SHOP_COST && t.items.len() < 3)
                    .unwrap_or(false);
                if affordable {
                    let item = Item::ALL[self.rng.random_range(0..Item::ALL.len())];
                    self.coins(seat, -SHOP_COST);
                    if let Some(token) = self.token(seat) {
                        token.items.push(item);
                    }
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "shop", "item": item, "coins": SHOP_COST }));
                } else {
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "shop_closed" }));
                }
            }
            SpaceKind::Hazard => {
                self.coins(seat, -HAZARD_COINS);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "hazard", "coins": HAZARD_COINS }));
                self.hurt(seat, 1);
            }
            SpaceKind::Teleport => {
                let to = space(at).link.unwrap_or(at);
                if let Some(token) = self.token(seat) {
                    token.space = to;
                }
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "teleport", "to": to }));
            }
            SpaceKind::PrizeSpot => {
                let here = self.prize_at == at;
                let rich = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == seat)
                    .map(|t| t.coins >= PRIZE_COST)
                    .unwrap_or(false);
                if here && rich {
                    self.coins(seat, -PRIZE_COST);
                    if let Some(token) = self.token(seat) {
                        token.prizes += 1;
                    }
                    let spots: Vec<u8> = prize_spots().into_iter().filter(|id| *id != at).collect();
                    if !spots.is_empty() {
                        let pick = self.rng.random_range(0..spots.len());
                        self.prize_at = spots[pick];
                    }
                    self.say(json!({ "kind": "prize", "seat": seat, "space": at, "coins": PRIZE_COST, "moved_to": self.prize_at }));
                } else if here {
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "prize_too_dear", "coins": PRIZE_COST }));
                } else {
                    self.coins(seat, COIN_SPACE);
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "coins", "coins": COIN_SPACE }));
                }
            }
            SpaceKind::Coin | SpaceKind::Start => {
                self.coins(seat, COIN_SPACE);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "coins", "coins": COIN_SPACE }));
            }
            SpaceKind::Loss => {
                self.coins(seat, -LOSS_SPACE);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "loss", "coins": LOSS_SPACE }));
            }
            SpaceKind::Item => {
                let item = Item::ALL[self.rng.random_range(0..Item::ALL.len())];
                if let Some(token) = self.token(seat) {
                    if token.items.len() < MAX_ITEMS {
                        token.items.push(item);
                    }
                }
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "item", "item": item }));
            }
            SpaceKind::Event => {
                // Everyone else pays the traveller a toll.
                let others: Vec<u8> = self
                    .tokens
                    .iter()
                    .filter(|t| t.seat != seat)
                    .map(|t| t.seat)
                    .collect();
                let mut taken = 0;
                for other in others {
                    let amount = self
                        .tokens
                        .iter()
                        .find(|t| t.seat == other)
                        .map(|t| t.coins.min(2))
                        .unwrap_or(0);
                    self.coins(other, -amount);
                    taken += amount;
                }
                self.coins(seat, taken);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "toll", "coins": taken }));
            }
        }

        if let Some(owner) = self.traps.remove(&at) {
            if owner != seat {
                self.coins(seat, -TRAP_COINS);
                self.coins(owner, TRAP_COINS);
                self.say(
                    json!({ "kind": "trap", "seat": seat, "owner": owner, "coins": TRAP_COINS }),
                );
                self.hurt(seat, 1);
            } else {
                self.traps.insert(at, owner);
            }
        }
        self.step = TurnStep::Done;
    }

    /// Hand the turn on. Returns true when the round is over and it is time
    /// for a minigame.
    pub fn end_turn(&mut self) -> bool {
        self.last_roll = None;
        self.turn_index += 1;
        self.step = TurnStep::Items;
        let round_over = self.turn_index % self.order.len() == 0;
        if !round_over {
            self.skip_resting();
        }
        round_over
    }

    /// Players knocked out last round sit this one out.
    fn skip_resting(&mut self) {
        for _ in 0..self.order.len() {
            let seat = self.turn_seat();
            let resting = self
                .tokens
                .iter()
                .find(|t| t.seat == seat)
                .map(|t| t.resting)
                .unwrap_or(0);
            if resting == 0 {
                return;
            }
            if let Some(token) = self.token(seat) {
                token.resting -= 1;
            }
            self.say(json!({ "kind": "resting", "seat": seat }));
            self.turn_index += 1;
            self.step = TurnStep::Items;
        }
    }

    /// Pay out a minigame and start the next round. Returns false when the
    /// party is over.
    pub fn finish_round(&mut self, placements: &[Vec<u8>]) -> bool {
        let mut place = 0;
        for group in placements {
            let reward = MINIGAME_COINS.get(place).copied().unwrap_or(0);
            for seat in group {
                self.coins(*seat, reward);
                self.say(json!({ "kind": "minigame", "seat": seat, "place": place + 1, "coins": reward }));
            }
            place += group.len();
        }
        if let Some(winners) = placements.first() {
            for seat in winners.clone() {
                self.award_item(seat);
            }
        }
        self.reorder(placements);
        if self.round >= self.rounds {
            return false;
        }
        self.round += 1;
        self.turn_index = 0;
        self.step = TurnStep::Items;
        self.say(json!({ "kind": "round", "round": self.round }));
        self.skip_resting();
        true
    }

    /// Bout winners draw a random item, if their bag has room.
    fn award_item(&mut self, seat: u8) {
        let item = Item::ALL[self.rng.random_range(0..Item::ALL.len())];
        let Some(token) = self.token(seat) else { return };
        if token.items.len() >= MAX_ITEMS {
            self.say(json!({ "kind": "award", "seat": seat, "item": null }));
            return;
        }
        token.items.push(item);
        self.say(json!({ "kind": "award", "seat": seat, "item": item }));
    }

    /// The next round goes in minigame finishing order; ties keep the order
    /// they already had, and anyone missing from the results goes last.
    fn reorder(&mut self, placements: &[Vec<u8>]) {
        let before = self.order.clone();
        let rank = |seat: &u8| before.iter().position(|s| s == seat).unwrap_or(usize::MAX);
        let mut order: Vec<u8> = Vec::new();
        for group in placements {
            let mut group: Vec<u8> = group.iter().copied().filter(|s| before.contains(s) && !order.contains(s)).collect();
            group.sort_by_key(rank);
            order.extend(group);
        }
        let missing: Vec<u8> = before.iter().copied().filter(|s| !order.contains(s)).collect();
        order.extend(missing);
        self.order = order;
        self.say(json!({ "kind": "order", "order": self.order.clone() }));
    }

    /// Put a token on a space with a set purse, for tests.
    #[cfg(test)]
    pub fn place_for_test(&mut self, seat: u8, space: u8, coins: i32) {
        if let Some(token) = self.token(seat) {
            token.space = space;
            token.coins = coins;
        }
    }

    #[cfg(test)]
    pub fn give_for_test(&mut self, seat: u8, item: Item) {
        if let Some(token) = self.token(seat) {
            token.items.push(item);
        }
    }

    /// Resolve the space a token is standing on, for tests.
    #[cfg(test)]
    pub fn land_for_test(&mut self, seat: u8) {
        self.land(seat);
    }

    /// A practice bot's move for the step it is on.
    pub fn bot_move(&mut self) -> BotMove {
        let seat = self.turn_seat();
        let me = self.tokens.iter().find(|t| t.seat == seat).cloned();
        match self.step.clone() {
            TurnStep::Items => {
                let Some(me) = me else { return BotMove::Pass };
                // Go after whoever is winning, with whatever reaches them.
                let mut pick: Option<(Item, Option<u8>)> = None;
                for item in me.items.iter().copied() {
                    if !item.needs_target() {
                        pick = pick.or(Some((item, None)));
                        continue;
                    }
                    let reach = item.reach().unwrap_or(u8::MAX);
                    let target = self
                        .tokens
                        .iter()
                        .filter(|t| t.seat != seat && gap(me.space, t.space) <= reach)
                        .max_by_key(|t| t.coins)
                        .map(|t| t.seat);
                    if target.is_some() {
                        pick = Some((item, target));
                        break;
                    }
                }
                let Some((item, target)) = pick else {
                    return BotMove::Pass;
                };
                if self.rng.random_bool(0.75) {
                    BotMove::Item { item, target }
                } else {
                    BotMove::Pass
                }
            }
            TurnStep::Roll => BotMove::Roll,
            TurnStep::Branch { options, .. } => BotMove::Branch {
                node: options[self.rng.random_range(0..options.len())].node,
            },
            TurnStep::Done => BotMove::EndTurn,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BotMove {
    Item { item: Item, target: Option<u8> },
    Pass,
    Roll,
    Branch { node: u8 },
    EndTurn,
}
