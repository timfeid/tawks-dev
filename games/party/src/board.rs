//! The party board: players take turns rolling around a loop, collecting
//! coins, and hitting each other with items. Between rounds everyone plays a
//! minigame for more coins. Most coins when the rounds run out wins.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use crate::design;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "walking.rs"]
mod walking;
pub use walking::{WalkCoinView, WalkView};
#[path = "positioning.rs"]
mod positioning;
pub use positioning::PositionView;
#[path = "raids.rs"]
mod raids;
pub use raids::RaidView;
#[path = "board_shots.rs"]
mod shots;
#[path="ambient.rs"]
mod ambient;
pub use ambient::AirCoinView;
#[path="zombie.rs"]
mod zombie;
pub use zombie::CrawlerView;
#[path="traffic.rs"]
mod traffic;
pub use traffic::TrafficView;
#[path="driving.rs"]
mod driving;
pub use driving::DriverView;
#[path="candy_van.rs"]
mod candy_van;
pub use candy_van::{CandyVanView,CandyRunView,CandyVanPhase};
#[path="pedestrians.rs"]
mod pedestrians;
pub use pedestrians::PedestrianView;
#[path="map_finale.rs"]
mod map_finale;
pub use map_finale::{MapFinaleView,FinaleRacer,FinaleCheckpoint};

pub const START_COINS: i32 = 10;
pub const MAX_HEALTH: u8 = 3;
pub const MAX_ITEMS: usize = 5;
pub const DICE_SIDES: u8 = 6;
pub const COIN_SPACE: i32 = 3;
pub const LOSS_SPACE: i32 = 3;
pub const KNOCKOUT_COINS: i32 = 10;
/// The furthest a cannon will throw you.
pub const CANNON_RANGE: u8 = 12;
/// How far off a cannon shot can land, either way.
pub const CANNON_SCATTER: u8 = 2;
/// What the winner of a scrap takes off the loser.
pub const SCRAP_COINS: i32 = 5;
/// Paid for getting back at whoever crossed you last.
pub const REVENGE_COINS: i32 = 4;
/// The most anyone can stake on themselves before a bout.
pub const MAX_BET: i32 = 10;
/// How far the storm walks between rounds.
pub const STORM_STEP: u8 = 4;
/// What being caught in the storm costs, in coins and spaces blown back.
pub const STORM_COINS: i32 = 4;
pub const STORM_BACK: u8 = 3;
pub const TRAP_COINS: i32 = 5;
/// What the bomb's blast costs everyone a space either side of where it lands.
pub const BOMB_COINS: i32 = 5;
/// What it costs whoever it lands right on, with a heart as well.
pub const BOMB_DIRECT_COINS: i32 = 8;
/// The bomb's flight: how fast a full-power throw leaves the hand, how hard
/// it falls, and how high the hand is, in world units.
pub const BOMB_SPEED: f32 = 26.0;
pub const BOMB_GRAVITY: f32 = 20.0;
pub const BOMB_HAND: f32 = 1.4;
/// The strongest wind a throw can meet, in units a second.
pub const BOMB_WIND: f32 = 3.5;
/// How near it has to land to be a direct hit, and to catch someone in the blast.
pub const BOMB_DIRECT_RADIUS: f32 = 1.7;
pub const BOMB_BLAST_RADIUS: f32 = 4.6;
/// Seconds before it goes off in the hand; anything else thrown or swung
/// is dropped when its time is up.
pub const BOMB_FUSE: f32 = 8.0;
/// How near a thrown grapple, hex, swap orb or sticky hand has to land to someone to get them.
pub const THROW_HIT_RADIUS: f32 = 2.4;
/// What a perfect stop on a shield or double dice pays on top.
pub const CHECK_BONUS_COINS: i32 = 3;

/// How a needle was stopped, from 0 to 1 with the middle best: the same
/// bands for every skill check, so the meters read the same everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Perfect,
    Good,
    Weak,
    Fail,
}

impl Grade {
    pub fn of(stop: f32) -> Self {
        let off = if stop.is_finite() { ((stop.clamp(0.0, 1.0) - 0.5).abs() * 2.0).min(1.0) } else { 1.0 };
        match off {
            o if o <= 0.15 => Grade::Perfect,
            o if o <= 0.5 => Grade::Good,
            o if o <= 0.85 => Grade::Weak,
            _ => Grade::Fail,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Grade::Perfect => "perfect",
            Grade::Good => "good",
            Grade::Weak => "weak",
            Grade::Fail => "fail",
        }
    }
}

/// How fast each item's needle swings, in seconds a sweep: the big heal and
/// the heist are the hard ones.
fn check_period(item: Item) -> f32 {
    match item {
        Item::Medkit => 0.5,
        Item::Heist => 0.55,
        Item::GoldenDie => 0.7,
        Item::Compass => 0.75,
        _ => 0.9,
    }
}

/// How far the bat reaches, and how wide its arc is either side of where you face.
pub const BAT_RANGE: f32 = 5.0;
pub const BAT_ARC: f32 = 0.75;
pub const GRAPPLE_COINS: i32 = 8;
pub const BAT_SPACES: u8 = 3;
/// How close you have to pass before you hear about someone else's trap.
pub const TRAP_RUMOUR: u8 = 2;
/// The furthest a practice bot will try to throw something, in world units.
const BOT_THROW: f32 = 30.0;
/// The compass drops you this many spaces short of the prize.
pub const COMPASS_SHORT: u8 = 3;
pub const CURSED_SIDES: u8 = 3;
pub const SHOP_COST: i32 = 5;
/// How many items the shop puts out when you stop there.
pub const SHOP_OFFERS: usize = 3;
/// How many the shop puts out on market day, when everything is half price.
pub const MARKET_OFFERS: usize = 5;
/// What each space walked costs on the toll road.
pub const TOLL_ROAD_COINS: i32 = 1;
pub const HAZARD_COINS: i32 = 5;
pub const PRIZE_COST: i32 = 15;
pub const WINDFALL_COINS: i32 = 5;
/// What the prize costs the player in last place, the round it is their turn.
pub const UNDERDOG_PRIZE: i32 = 9;
/// Whoever is alone in last place rolls a small extra die from the second
/// round on, enough to catch up a little without leapfrogging the table.
pub const HANDICAP_SIDES: u8 = 3;
/// Coins for finishing first, second, third and fourth in a minigame.
pub const MINIGAME_COINS: [i32; 4] = [10, 0, 0, 0];

#[derive(Clone,Debug,Serialize)]
pub struct MinigameReward {pub seat:u8,pub place:usize,pub coins:i32,pub tier_one_chance:f32}

/// First place is guaranteed premium; last place always keeps a 20% shot.
pub fn tier_one_chance(place:usize,players:usize)->f32 {
    1.0-0.8*(place.min(players.saturating_sub(1)) as f32/players.saturating_sub(1).max(1) as f32)
}
pub fn item_tier_one(item:Item)->bool {matches!(item,Item::Bomb|Item::DriveBy|Item::Slapshot|Item::DriveCar|Item::GoldenDie|Item::Swap|Item::Heist)}


/// What the tide takes off anyone still standing on a tide space when it comes in.
pub const TIDE_COINS: i32 = 6;
/// What stopping on a tide space pays while the tide is out.
pub const TIDE_FIND: i32 = 5;
/// Gated ways, such as the summit on Frozen Peak, open for this many rounds at the end.
pub const LATE_ROUNDS: u32 = 2;

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
    /// Fires you up the lap as far as you dare.
    Cannon,
    /// Below the tide line: it pays while the tide is out, soaks you while it
    /// is in, and costs whoever is standing on it when the water comes in.
    Tide,
    /// Slick: stopping here carries you on one more space.
    Ice,
}

/// What a player can shout while someone else is taking their turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Taunt {
    Laugh,
    Clap,
    Wave,
    Cry,
    Point,
    Hurry,
}

impl Taunt {
    pub fn label(self) -> &'static str {
        match self {
            Taunt::Laugh => "Ha!",
            Taunt::Clap => "Nice one",
            Taunt::Wave => "Hello",
            Taunt::Cry => "Not fair",
            Taunt::Point => "You're next",
            Taunt::Hurry => "Any day now",
        }
    }
}

/// Something that shakes up the board at the start of a round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundEvent {
    /// Blue spaces pay double all round.
    CoinShower,
    /// Everyone gets coins.
    Windfall,
    /// Everyone draws an item.
    ItemRain,
    /// The prize packs up and moves.
    PrizeMoves,
    /// Last place draws an item and buys the prize cheaply this round.
    Underdog,
    /// Nobody can see anyone else's purse.
    Fog,
    /// Every space walked costs a coin.
    TollRoad,
    /// The shop puts out more and charges half.
    MarketDay,
    /// The shortcuts are closed; everyone takes the lap.
    Earthquake,
    /// The last round: blue and red spaces count double.
    LastRound,
    /// Mature games only: a price on the leader's head, paid to whoever hits them first.
    Bounty,
    /// Mature games only: someone catches the plague, losing a heart every
    /// turn until they pass it on by landing on someone.
    Plague,
    /// Mature games only: last place robs the leader of half their purse.
    Shakedown,
}

impl RoundEvent {
    pub const DRAWN: [RoundEvent; 9] = [
        RoundEvent::CoinShower,
        RoundEvent::Windfall,
        RoundEvent::ItemRain,
        RoundEvent::PrizeMoves,
        RoundEvent::Underdog,
        RoundEvent::Fog,
        RoundEvent::TollRoad,
        RoundEvent::MarketDay,
        RoundEvent::Earthquake,
    ];
    /// The nastier ones, drawn too in a mature game.
    pub const DARK: [RoundEvent; 3] = [RoundEvent::Bounty, RoundEvent::Plague, RoundEvent::Shakedown];
}

/// How far the vacuum reaches, in world units: four or five spaces; how
/// wide its cone is either side of where it points; and how many seconds of
/// sucking it has.
pub const VACUUM_RANGE: f32 = 14.0;
pub const VACUUM_CONE: f32 = 0.22;
pub const VACUUM_SECONDS: f32 = 20.0;
/// How long a rival has to stay in the middle of the cone for each coin
/// drawn out of them.
pub const SIPHON_EVERY: f32 = 0.4;
/// Seconds for the motor to reach full power each time it is switched on,
/// so holding steady beats tapping.
pub const VACUUM_SPIN_UP: f32 = 0.45;
/// How much of its strength the vacuum keeps at the edge of the cone and at
/// the end of its reach; dead centre and point blank are full strength.
pub const VACUUM_EDGE: f32 = 0.65;
pub const VACUUM_FAR: f32 = 0.65;
/// How much of the drain a rival bracing against the vacuum still loses.
pub const BRACED: f32 = 0.5;
/// How long a rival has to stay in the middle of the cone while it blows
/// before they are knocked back a space.
pub const KNOCK_AFTER: f32 = 0.5;
/// What the shop's vacuum upgrades do: the wide nozzle's cone and strength,
/// turbo's spin-up, and the big battery's seconds.
pub const WIDE_NOZZLE_CONE: f32 = 1.4;
pub const WIDE_NOZZLE_STRENGTH: f32 = 1.0;
pub const TURBO_SPIN_UP: f32 = 0.5;
pub const BIG_BATTERY_SECONDS: f32 = 32.0;
/// What the vacuum can pull, as the high byte of its key; the low byte is
/// the space or the special's index.
pub const PULL_LOOSE: u16 = 0;
pub const PULL_CRATE: u16 = 1;
pub const PULL_CHEST: u16 = 2;
pub const PULL_BOX: u16 = 3;
pub const PULL_STORM: u16 = 4;
pub const PULL_PALM: u16 = 5;
pub const PULL_LAVA: u16 = 6;
pub const PULL_FLOTSAM: u16 = 7;
pub const PULL_SNOWMAN: u16 = 8;
pub const PULL_AIR:u16=9;
pub const PULL_PEDESTRIAN:u16=11;
/// Coconuts on each of Palm Island's palms, and what one is worth.
pub const COCONUTS: u8 = 2;
pub const COCONUT_COINS: i32 = 3;
/// Lava rocks the Volcano spits out each round, what one costs whoever stops
/// on it, and what it is worth vacuumed up.
pub const LAVA_ROCKS: usize = 3;
pub const LAVA_COINS: i32 = 4;
pub const OBSIDIAN_COINS: i32 = 3;
/// Flotsam washed into Shipwreck Cove each round.
pub const FLOTSAM: usize = 3;
/// How near something has to be for a practice bot to bother with the vacuum.
const SUCTION_RADIUS: f32 = 12.0;

/// Kept on the token for the rest of the game once bought at the shop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Upgrade {
    /// A cone 1.4 times as wide, at 0.85 of the strength.
    WideNozzle,
    /// Spins up in half the time.
    Turbo,
    /// Thirty-two seconds of suction instead of twenty.
    BigBattery,
    /// Rivals' vacuums cannot drain you, though they can still blow you back.
    Filter,
}

impl Upgrade {
    pub const ALL: [Upgrade; 4] = [Upgrade::Turbo, Upgrade::WideNozzle, Upgrade::BigBattery, Upgrade::Filter];

    pub fn label(self) -> &'static str {
        match self {
            Upgrade::WideNozzle => "Wide nozzle",
            Upgrade::Turbo => "Turbo",
            Upgrade::BigBattery => "Big battery",
            Upgrade::Filter => "Filter",
        }
    }

    pub fn price(self) -> i32 {
        match self {
            Upgrade::Turbo => SHOP_COST + 3,
            Upgrade::WideNozzle => SHOP_COST + 5,
            Upgrade::BigBattery => SHOP_COST + 7,
            Upgrade::Filter => SHOP_COST + 10,
        }
    }
}

/// One vacuum upgrade on the shop's counter.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct UpgradeOffer {
    pub upgrade: Upgrade,
    pub price: i32,
}

/// The vacuum while it is out: where it points, whether it is sucking or
/// blowing, the seconds left and how long it has been out, and how far along
/// each thing being drawn in or blown off is, from 0 to 1, by its space.
#[derive(Clone, Debug, Default)]
pub struct Vacuum {
    /// Walking supplies its actual interpolated feet instead of the final tile.
    pub origin: Option<(f32,f32)>,
    pub yaw: f32,
    pub on: bool,
    pub blow: bool,
    /// The motor's speed, from 0 to 1, spinning up while it is on.
    pub power: f32,
    pub left: f32,
    pub out: f32,
    pub pulling: HashMap<u16, f32>,
    /// Rivals holding on against it.
    pub braced: HashSet<u8>,
    /// How long each rival has been in the cone, toward the next coin drawn
    /// out of them; and those whose shield stopped it.
    pub siphon: HashMap<u8, f32>,
    pub shielded: Vec<u8>,
    /// How long each rival has been in the cone while it blows, toward
    /// knocking them back; those already knocked back once; and whether the
    /// chest has been blown off to another spot.
    pub gust: HashMap<u8, f32>,
    pub knocked: HashSet<u8>,
    pub chest_blown: bool,
    /// Where the turn was when it came out, to go back to once it is put away.
    pub after: Option<Box<TurnStep>>,
}

/// How hard the vacuum at `from`, pointed along `yaw`, pulls on `at`, from 0
/// to 1: strongest dead centre and point blank, weaker toward the edge of
/// the cone and the end of its reach, and nothing outside. `narrow` shrinks
/// the cone for things that have to be held near the middle.
pub fn vacuum_strength(from: (f32, f32), yaw: f32, at: (f32, f32), narrow: f32) -> Option<f32> {
    let (dx, dz) = (at.0 - from.0, at.1 - from.1);
    let away = (dx * dx + dz * dz).sqrt();
    if away > VACUUM_RANGE {
        return None;
    }
    let near = 1.0 - (1.0 - VACUUM_FAR) * away / VACUUM_RANGE;
    if away < 0.001 {
        return Some(near);
    }
    let off = (dx.atan2(dz) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    let across = off.abs() / (VACUUM_CONE * narrow);
    (across <= 1.0).then(|| near * (1.0 - (1.0 - VACUUM_EDGE) * across * across))
}

/// The vacuum as the room sees it.
#[derive(Clone, Debug, Serialize)]
pub struct VacuumView {
    pub yaw: f32,
    pub on: bool,
    pub blowing: bool,
    /// How wide the cone is either side of where it points, in radians, with
    /// a wide nozzle or without.
    pub cone: f32,
    pub rate:f32,
    pub power: f32,
    pub left: f32,
    /// How long it has been out, which is where the chests have drifted to.
    pub out: f32,
    pub pulling: Vec<(u16, f32)>,
    /// The rivals being drained, and those of them bracing.
    pub siphoning: Vec<u8>,
    pub braced: Vec<u8>,
}
/// How far coins knocked loose by a bat fly, in world units.
pub const BAT_SCATTER: f32 = 18.0;
/// Crates on the board at the start, and how many more wash up each round.
pub const START_CRATES: usize = 6;
pub const ROUND_CRATES: usize = 2;

/// What the bounty pays, and the most the shakedown takes.
pub const BOUNTY_COINS: i32 = 10;
pub const SHAKEDOWN_MAX: i32 = 15;

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
    /// Closed until the last rounds, like the summit on Frozen Peak; a
    /// token only steps onto it once `Board::gates_open` says so.
    pub gated: bool,
    /// Underground, in a tunnel: a cave under the island's hill, the
    /// volcano's lava tube, the ice cave through the glacier. The ways into
    /// and out of a tunnel are always above ground.
    pub under: bool,
    /// Which of the map file's paths it was laid along, 0 for the lap.
    pub path: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    Bat,
    Bomb,
    DriveBy,
    Slapshot,
    SlashRush,
    DriveCar,
    Grapple,
    Trap,
    DoubleDice,
    GoldenDie,
    Swap,
    Shield,
    Medkit,
    Compass,
    Curse,
    Pickpocket,
    /// Takes a prize off whoever has the most of them.
    Heist,
    /// A crate picked up by suction, thrown to make a barricade. Never drawn,
    /// never sold, and never in the bag.
    Crate,
    /// Frozen Peak's snowball, packed by vacuuming the snowman and thrown to
    /// freeze someone for a turn. Like the crate, never in the bag.
    Snowball,
}

impl Item {
    pub const ALL: [Item; 17] = [
        Item::Bat,
        Item::Bomb,
        Item::DriveBy,
        Item::Slapshot,
        Item::SlashRush,
        Item::DriveCar,
        Item::Grapple,
        Item::Trap,
        Item::DoubleDice,
        Item::GoldenDie,
        Item::Swap,
        Item::Shield,
        Item::Medkit,
        Item::Compass,
        Item::Curse,
        Item::Pickpocket,
        Item::Heist,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Item::Bat => "Bat",
            Item::Bomb => "Bomb",
            Item::DriveBy => "Drive-by",
            Item::Slapshot => "Slapshot",
            Item::SlashRush => "Slash rush",
            Item::DriveCar => "Rental car",
            Item::Grapple => "Grapple",
            Item::Trap => "Trap",
            Item::DoubleDice => "Double dice",
            Item::GoldenDie => "Golden die",
            Item::Swap => "Swap",
            Item::Shield => "Shield",
            Item::Medkit => "Medkit",
            Item::Compass => "Compass",
            Item::Curse => "Curse",
            Item::Pickpocket => "Pickpocket",
            Item::Heist => "Heist",
            Item::Crate => "Crate",
            Item::Snowball => "Snowball",
        }
    }

    /// Items that change this turn's dice; only one per turn.
    pub fn sets_dice(self) -> bool {
        matches!(self, Item::DoubleDice | Item::GoldenDie)
    }

    /// How often the item turns up; the strongest are the rarest.
    fn weight(self) -> u32 {
        match self {
            Item::Bat | Item::DoubleDice => 4,
            Item::Bomb | Item::Grapple | Item::Trap | Item::Shield | Item::Medkit | Item::Curse => 3,
            Item::GoldenDie | Item::Swap | Item::Pickpocket | Item::DriveBy | Item::Slapshot | Item::SlashRush => 2,
            Item::Compass | Item::Heist | Item::DriveCar => 1,
            Item::Crate | Item::Snowball => 0,
        }
    }

    /// What the shop charges: the rarer it is, the dearer it is.
    pub fn price(self) -> i32 {
        match self.weight() {
            4 => SHOP_COST,
            3 => SHOP_COST + 2,
            2 => SHOP_COST + 5,
            _ => SHOP_COST + 9,
        }
    }

    pub fn draw(rng: &mut impl Rng) -> Item {
        let total: u32 = Item::ALL.iter().map(|item| item.weight()).sum();
        let mut pick = rng.random_range(0..total);
        for item in Item::ALL {
            if pick < item.weight() {
                return item;
            }
            pick -= item.weight();
        }
        Item::Bat
    }
}

/// How much bigger the boards are than they were first drawn, which the
/// client's scenery is scaled by. The hand-built maps are drawn at this size.
pub const WORLD_SCALE: f32 = 2.0;

pub fn prize_spots() -> Vec<u8> {
    prize_spots_in(board())
}

pub fn prize_spots_in(layout: &[Space]) -> Vec<u8> {
    layout
        .iter()
        .filter(|s| s.kind == SpaceKind::PrizeSpot)
        .map(|s| s.id)
        .collect()
}

/// Which board a party is played on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Map {
    #[default]
    Island,
    Volcano,
    Shipwreck,
    Frozen,
}

impl Map {
    pub const ALL: [Map; 4] = [Map::Island, Map::Volcano, Map::Shipwreck, Map::Frozen];

    pub fn label(self) -> &'static str {
        match self {
            Map::Island => "Palm Island",
            Map::Volcano => "The Volcano",
            Map::Shipwreck => "Shipwreck Cove",
            Map::Frozen => "Frostline City",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Map::Island => include_str!("../../canvases/src/party/board/maps/island.json"),
            Map::Volcano => include_str!("../../canvases/src/party/board/maps/volcano.json"),
            Map::Shipwreck => include_str!("../../canvases/src/party/board/maps/shipwreck.json"),
            Map::Frozen => include_str!("../../canvases/src/party/board/maps/frozen.json"),
        }
    }

    /// The board's own things for the vacuum.
    pub fn specials(self) -> &'static design::Specials {
        static SPECIALS: OnceLock<[design::Specials; 4]> = OnceLock::new();
        let all = SPECIALS.get_or_init(|| Map::ALL.map(|map| design::specials(map.source())));
        &all[Map::ALL.iter().position(|map| *map == self).unwrap_or(0)]
    }

    pub fn layout(self) -> &'static [Space] {
        static ISLAND: OnceLock<Vec<Space>> = OnceLock::new();
        static VOLCANO: OnceLock<Vec<Space>> = OnceLock::new();
        static SHIPWRECK: OnceLock<Vec<Space>> = OnceLock::new();
        static FROZEN: OnceLock<Vec<Space>> = OnceLock::new();
        match self {
            Map::Island => ISLAND.get_or_init(|| design::build(include_str!("../../canvases/src/party/board/maps/island.json"))),
            Map::Volcano => VOLCANO.get_or_init(|| design::build(include_str!("../../canvases/src/party/board/maps/volcano.json"))),
            Map::Shipwreck => {
                SHIPWRECK.get_or_init(|| design::build(include_str!("../../canvases/src/party/board/maps/shipwreck.json")))
            }
            Map::Frozen => FROZEN.get_or_init(|| design::build(include_str!("../../canvases/src/party/board/maps/frozen.json"))),
        }
    }
}

/// The island, which every party was played on before there was a choice.
pub fn board() -> &'static [Space] {
    Map::Island.layout()
}

pub fn space(id: u8) -> &'static Space {
    &board()[id as usize]
}

pub fn space_in(layout: &'static [Space], id: u8) -> &'static Space {
    &layout[id as usize]
}

/// How far apart two spaces are, whichever way round the board is shorter.
pub fn gap(from: u8, to: u8) -> u8 {
    gap_in(board(), from, to)
}

pub fn gap_in(layout: &[Space], from: u8, to: u8) -> u8 {
    distance_in(layout, from, to).min(distance_in(layout, to, from))
}

pub fn distance(from: u8, to: u8) -> u8 {
    distance_in(board(), from, to)
}

pub fn distance_in(layout: &[Space], from: u8, to: u8) -> u8 {
    let mut frontier = vec![from];
    let mut seen = vec![from];
    for step in 0..layout.len() as u8 {
        if frontier.contains(&to) {
            return step;
        }
        let mut next = Vec::new();
        for id in frontier {
            for candidate in &layout[id as usize].next {
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
    /// The physical edge just walked; persists between turns to prevent U-turns.
    pub previous: Option<u8>,
    /// Cosmetic standing direction; movement and item aiming take priority.
    pub facing: Option<f32>,
    #[serde(skip_serializing)]
    pub coins: i32,
    /// The purse as one viewer sees it: hidden from everyone else in fog.
    #[serde(rename = "coins")]
    pub purse: Option<i32>,
    pub health: u8,
    pub items: Vec<Item>,
    /// Prizes bought; they beat coins in the standings.
    pub prizes: u32,
    /// Turns this player sits out after a knockout.
    pub resting: u8,
    /// Blocks the next item or trap aimed at this player.
    pub shield: bool,
    /// The next roll only goes up to three.
    pub cursed: bool,
    /// Who crossed this player last, and owes them for it.
    pub grudge: Option<u8>,
    /// Holding a crate picked up by the vacuum, to throw as a barricade, and
    /// a snowball from Frozen Peak's snowman.
    pub carrying: bool,
    pub snowball: bool,
    /// Vacuum upgrades bought at the shop, kept for the game.
    pub upgrades: Vec<Upgrade>,
    pub stats: Stats,
}

impl Token {
    pub fn has(&self, upgrade: Upgrade) -> bool {
        self.upgrades.contains(&upgrade)
    }
}

/// What the bonus trophies are counted from at the end.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Stats {
    pub bouts_won: u32,
    pub spaces: u32,
    pub items_used: u32,
    /// The most coins taken in one go.
    pub best_swing: i32,
    /// The most spaces walked in a single turn.
    pub longest_walk: u32,
    /// How many times they were knocked out.
    pub knockouts: u32,
    /// Dice rolled and the pips they came up, for who had the luck.
    pub rolls: u32,
    pub pips: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Bonus {
    /// Most bouts won.
    Champion,
    /// Most spaces walked.
    Explorer,
    /// Most items used.
    Troublemaker,
}

impl Bonus {
    pub const ALL: [Bonus; 3] = [Bonus::Champion, Bonus::Explorer, Bonus::Troublemaker];

    fn count(self, stats: &Stats) -> u32 {
        match self {
            Bonus::Champion => stats.bouts_won,
            Bonus::Explorer => stats.spaces,
            Bonus::Troublemaker => stats.items_used,
        }
    }
}

/// Something worth remembering from the party, handed out after the podium.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Highlight {
    /// The most coins anyone took in one go.
    BiggestSwing,
    /// The most spaces anyone covered in a single turn.
    LongestWalk,
    /// Knocked out more than anyone.
    MostKnockouts,
    /// The worst average roll of the party.
    WorstDice,
}

#[derive(Clone, Debug, Serialize)]
pub struct HighlightAward {
    pub highlight: Highlight,
    pub seat: u8,
    /// What they did, as a number the client can put in a sentence.
    pub value: i32,
}

/// A bonus trophy handed out at the end, worth a prize to everyone tied for it.
#[derive(Clone, Debug, Serialize)]
pub struct BonusAward {
    pub bonus: Bonus,
    pub seats: Vec<u8>,
    pub count: u32,
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

/// One item on the shop's counter.
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct ShopOffer {
    pub item: Item,
    pub price: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum TurnStep {
    /// The turn owner is using Slash Rush; its previous step resumes afterward.
    Raiding,
    /// Use an item or pass.
    Items,
    /// Move into a throwing or striking position, then activate the held item.
    Position { item: Item },
    Roll,
    /// A timed first-person walk. Route choices and landing wait until it ends.
    Walking,
    Driving,
    /// The roll landed on a junction; pick the way on.
    Branch {
        options: Vec<BranchOption>,
        left: u8,
    },
    /// Stopped at the shop; buy one of these or walk on. The upgrades are
    /// the vacuum's, those this player does not have yet.
    Shop {
        offers: Vec<ShopOffer>,
        upgrades: Vec<UpgradeOffer>,
    },
    /// Loaded into a cannon; say how hard to fire it.
    Cannon {
        max: u8,
    },
    /// A bomb in hand, thrown in first person: aim, swing the power and
    /// throw before the fuse burns down, with the wind blowing it off line.
    Lob {
        /// What is being thrown: the bomb, or a grapple, hex, swap orb or sticky hand.
        item: Item,
        /// Which way and how hard the wind blows, in units a second.
        wind: (f32, f32),
        fuse: f32,
    },
    /// A bat in hand, swung in first person: face the way and stop the
    /// timing needle. It hits whoever is in the arc in front.
    Swing {
        fuse: f32,
    },
    /// An item that works on yourself, or the heist, and its skill check: a
    /// needle swinging every `period` seconds, stopped as near the middle as
    /// you can. The better the stop, the more it does.
    Check {
        item: Item,
        period: f32,
        fuse: f32,
        /// The number asked of a golden die.
        number: Option<u8>,
    },
    /// The vacuum out, aimed in first person, with `seconds` of sucking.
    Vacuum {
        seconds: f32,
    },
    Done,
}

/// The legal physical exits at a crossroads. A side route is not necessarily
/// a shortcut now that the player can arrive from either direction.
fn branch_options(layout: &'static [Space], from: u8, ways: &[u8]) -> Vec<BranchOption> {
    ways.iter()
        .map(|node| {
            let shortcut = layout[*node as usize].path != 0 && layout[*node as usize].path != layout[from as usize].path;
            BranchOption {
                node: *node,
                kind: layout[*node as usize].kind,
                shortcut,
                saves: 0,
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
    TooDear,
    BagFull,
    DiceSet,
    PickANumber,
    NothingToTake,
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
            BoardError::TooDear => "You cannot afford that.",
            BoardError::BagFull => "Your bag is full.",
            BoardError::DiceSet => "You already changed your dice this turn.",
            BoardError::PickANumber => "Pick a number from 1 to 6.",
            BoardError::NothingToTake => "They have nothing to take.",
        })
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct BoardView {
    pub map: Map,
    /// How much bigger than its drawing the board is, so the client can dress
    /// a bigger world as thickly.
    pub world_scale: f32,
    pub spaces: &'static [Space],
    pub tokens: Vec<Token>,
    pub standings:Vec<Vec<u8>>,
    /// Only the traps this viewer knows about: their own, and any they have
    /// walked close enough to hear about.
    pub traps: Vec<TrapView>,
    /// Coins lying on the board, for anyone who walks over them.
    pub loose: Vec<LooseCoins>,
    /// Crates lying about for suction to pick up, and barricades thrown up
    /// across the way, each by the space it is on.
    pub crates: Vec<u8>,
    pub blockades: Vec<u8>,
    /// Whether the player taking their turn has used their vacuum yet, and the vacuum while it is out.
    pub sucked: bool,
    pub vacuum: Option<VacuumView>,
    pub walk: Option<WalkView>,
    pub positioning: Option<PositionView>,
    pub raids: Vec<RaidView>,
    pub finale: Option<MapFinaleView>,
    pub air_coins:Vec<AirCoinView>,
    pub pedestrians:Vec<PedestrianView>,
    pub air_remaining:u8,
    pub crawler:Option<CrawlerView>,
    pub traffic:Option<TrafficView>,
    pub driving:Vec<DriverView>,
    pub candy_van:Option<CandyVanView>,
    pub candy_runs:Vec<CandyRunView>,
    /// The board's own things for the vacuum: coconuts on each palm, whether
    /// each snowman is built, boxes taken this round, lava rocks by space,
    /// and the flotsam points with something bobbing at them.
    pub coconuts: Vec<u8>,
    pub snowmen: Vec<bool>,
    pub boxes_taken: Vec<u8>,
    pub lava_rocks: Vec<u8>,
    pub flotsam: Vec<u8>,
    /// Where the prize is sitting, and what it costs.
    pub prize_at: u8,
    pub prize_cost: i32,
    /// Every spot a prize can be bought at right now.
    pub open_prizes: Vec<u8>,
    pub order: Vec<u8>,
    pub turn_seat: u8,
    pub turn: TurnStep,
    pub round: u32,
    pub rounds: u32,
    pub last_roll: Option<u8>,
    /// How many dice the player taking their turn will roll, and the face
    /// they fixed with a golden die.
    pub dice: u8,
    pub fixed_roll: Option<u8>,
    /// This round's shake-up, from round two on.
    pub event: Option<RoundEvent>,
    /// In a mature game, who has a price on their head, and who has the plague.
    pub bounty: Option<u8>,
    pub plague: Option<u8>,
    /// Who buys the prize cheaply this round, and what it costs them.
    pub underdog: Option<u8>,
    pub underdog_cost: i32,
    /// Who rolls the extra handicap die on their turns this round.
    pub handicap: Option<u8>,
    /// What everyone has staked on the coming bout, and the most anyone may.
    pub bets: Vec<Bet>,
    pub max_bet: i32,
    /// Bonus trophies, once the last round is over.
    pub bonuses: Vec<BonusAward>,
    /// What the party will be remembered for.
    pub highlights: Vec<HighlightAward>,
    /// Where the storm is, and where it moves next, so it can be seen coming.
    pub storm: Option<u8>,
    pub storm_to: Option<u8>,
    /// Whether the tide is in, on a board with tide spaces. It turns every round.
    pub tide: Option<bool>,
    /// The round gated ways such as the summit open in, on a board that has
    /// them, and whether they are open now.
    pub gates_open_from: Option<u32>,
    pub gates_open: bool,
    pub log: Vec<Value>,
}

/// What one player has staked on themselves.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Bet {
    pub seat: u8,
    pub coins: i32,
}

/// A trap as one player sees it.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct TrapView {
    pub space: u8,
    /// True when it is this viewer's own trap.
    pub mine: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct LooseCoins {
    pub space: u8,
    pub coins: i32,
}

pub struct Board {
    /// Which board this party is on, and its spaces.
    map: Map,
    layout: &'static [Space],
    tokens: Vec<Token>,
    traps: HashMap<u8, u8>,
    /// Who has passed close enough to a trap's space to have heard about it.
    rumours: HashMap<u8, Vec<u8>>,
    /// Coins lying on the board, by space, waiting to be walked over.
    loose: HashMap<u8, i32>,
    /// Crates and barricades by space, with any coins hidden inside.
    crates: HashMap<u8, i32>,
    blockades: HashMap<u8, i32>,
    sucked: bool,
    vacuum: Vacuum,
    /// Real rooms animate walking on the server; rules-only simulations can
    /// continue resolving movement immediately.
    pub timed_walk: bool,
    pub timed_items: bool,
    positioning: Option<positioning::Position>,
    raids: HashMap<u8,raids::Raid>,
    finale: Option<map_finale::MapFinale>,
    walk: Option<walking::Walk>,
    walk_bonus: u8,
    walk_earned:i32,
    air_coins:Vec<ambient::AirCoin>,
    /// Legacy rules fixtures keep airborne coins; real rooms use city cash.
    pub floating_coins:bool,
    pedestrians:Vec<pedestrians::Pedestrian>,
    ambient_earned:u8,
    next_air_id:u32,
    next_walk_id: u32,
    crawler:zombie::Crawler,
    traffic:traffic::Traffic,
    driver:Option<driving::Driver>,
    candy_van:Option<candy_van::CandyVan>,
    /// The board's own things for the vacuum: coconuts left on each palm,
    /// the snowman built or not, the purple spaces' boxes already taken this
    /// round, lava rocks lying on spaces, and flotsam bobbing at its points
    /// with the coins in it (none for an item).
    coconuts: Vec<u8>,
    snowmen: Vec<bool>,
    boxes_taken: std::collections::HashSet<u8>,
    lava_rocks: std::collections::HashSet<u8>,
    flotsam: HashMap<u8, i32>,
    order: Vec<u8>,
    turn_index: usize,
    step: TurnStep,
    pub round: u32,
    pub rounds: u32,
    prize_at: u8,
    last_roll: Option<u8>,
    dice: u8,
    fixed_roll: Option<u8>,
    event: Option<RoundEvent>,
    /// A mature game: the dark round events are drawn too.
    pub mature: bool,
    /// The sandbox: the vacuum can come out any number of times a turn, and
    /// never runs down until it is put away.
    pub endless_vacuum: bool,
    pub manual_events:bool,
    /// In pairs, each seat's partner: never hurt, robbed or drained by them,
    /// and kept on the same side in a team bout.
    partners: HashMap<u8, u8>,
    bounty: Option<u8>,
    plague: Option<u8>,
    /// Who is playing catch-up this round, and buys the prize cheaply.
    underdog: Option<u8>,
    /// What each seat has staked on itself for the coming bout.
    bets: HashMap<u8, i32>,
    /// Spaces walked in the turn being taken, for the longest-walk award.
    walked: u32,
    /// Where the storm is sitting, once it has rolled in.
    storm: Option<u8>,
    bonuses: Vec<BonusAward>,
    highlights: Vec<HighlightAward>,
    rng: StdRng,
    log: Vec<Value>,
    log_sequence: u64,
    events: Vec<Value>,
}

impl Board {
    pub fn new(seats: &[u8], rounds: u32, seed: u64) -> Self {
        Self::on(Map::Island, seats, rounds, seed)
    }

    /// A party on a particular board.
    pub fn on(map: Map, seats: &[u8], rounds: u32, seed: u64) -> Self {
        Self::on_with_floating(map,seats,rounds,seed,true)
    }

    pub fn for_room(map:Map,seats:&[u8],rounds:u32,seed:u64)->Self {
        Self::on_with_floating(map,seats,rounds,seed,false)
    }

    fn on_with_floating(map:Map,seats:&[u8],rounds:u32,seed:u64,floating_coins:bool)->Self {
        let layout = map.layout();
        let mut board = Board {
            map,
            layout,
            tokens: seats
                .iter()
                .map(|seat| Token {
                    seat: *seat,
                    space: 0,
                    facing: None,
                    previous: layout.iter().find(|s| s.path == 0 && s.next.contains(&0)).map(|s| s.id),
                    coins: START_COINS,
                    purse: None,
                    health: MAX_HEALTH,
                    items: Vec::new(),
                    prizes: 0,
                    resting: 0,
                    shield: false,
                    cursed: false,
                    grudge: None,
                    carrying: false,
                    snowball: false,
                    upgrades: Vec::new(),
                    stats: Stats::default(),
                })
                .collect(),
            traps: HashMap::new(),
            rumours: HashMap::new(),
            loose: HashMap::new(),
            crates: HashMap::new(),
            blockades: HashMap::new(),
            sucked: false,
            vacuum: Vacuum::default(),
            timed_walk: false,
            timed_items: false,
            positioning: None,
            raids: HashMap::new(),
            finale: None,
            walk: None,
            walk_bonus: 0,
            walk_earned:0,
            air_coins:Vec::new(),
            floating_coins,
            pedestrians:Vec::new(),
            ambient_earned:0,
            next_air_id:0,
            next_walk_id: 0,
            crawler:zombie::Crawler::new(layout),
            traffic:traffic::Traffic::new(),
            driver:None,
            candy_van:candy_van::CandyVan::new(map,seed),
            coconuts: Vec::new(),
            snowmen: Vec::new(),
            boxes_taken: std::collections::HashSet::new(),
            lava_rocks: std::collections::HashSet::new(),
            flotsam: HashMap::new(),
            order: seats.to_vec(),
            turn_index: 0,
            step: TurnStep::Items,
            round: 1,
            rounds,
            prize_at: prize_spots_in(layout).first().copied().unwrap_or(0),
            last_roll: None,
            dice: 1,
            fixed_roll: None,
            event: None,
            mature: false,
            endless_vacuum: false,
            manual_events:false,
            partners: HashMap::new(),
            bounty: None,
            plague: None,
            underdog: None,
            bets: HashMap::new(),
            walked: 0,
            storm: None,
            bonuses: Vec::new(),
            highlights: Vec::new(),
            rng: StdRng::seed_from_u64(seed),
            log: Vec::new(),
            log_sequence: 0,
            events: Vec::new(),
        };
        let spots = board.reachable_prize_spots();
        if !spots.is_empty() {
            let pick = board.rng.random_range(0..spots.len());
            board.prize_at = spots[pick];
        }
        board.wash_up(START_CRATES);
        board.restock();
        board.say(json!({ "kind": "round", "round": 1 }));
        board.skip_resting();
        board
    }

    fn say(&mut self, mut entry: Value) {
        if let Some(position) = self.positioning.as_ref().filter(|position| entry["kind"] == "item" && entry["by"].as_u64() == Some(position.seat as u64)) {
            entry["origin_x"] = json!(position.x);
            entry["origin_z"] = json!(position.z);
        }
        self.log_sequence += 1;
        if let Some(fields) = entry.as_object_mut() { fields.insert("seq".into(), json!(self.log_sequence)); }
        self.log.push(entry.clone());
        self.events.push(entry);
        // Browser rooms consume the bounded snapshot log; native rooms can
        // drain this queue. Keep unattended browser games bounded as well.
        if self.events.len() > 256 {
            self.events.remove(0);
        }
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    /// Shouted from the sidelines; everyone sees it over that player's token.
    pub fn taunt(&mut self, seat: u8, taunt: Taunt) {
        self.say(json!({ "kind": "taunt", "seat": seat, "taunt": taunt, "label": taunt.label() }));
    }

    pub fn take_events(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.events)
    }

    pub fn view(&self) -> BoardView {
        self.view_for(None)
    }

    /// The board as one seat sees it; `None` sees everything but the traps.
    pub fn view_for(&self, seat: Option<u8>) -> BoardView {
        BoardView {
            map: self.map,
            world_scale: WORLD_SCALE,
            spaces: self.layout,
            standings:self.placements(),
            tokens: self
                .tokens
                .iter()
                .map(|token| {
                    let hidden = self.event == Some(RoundEvent::Fog) && Some(token.seat) != seat;
                    Token { purse: (!hidden).then_some(token.coins), ..token.clone() }
                })
                .collect(),
            traps: self
                .traps
                .iter()
                .filter(|(space, owner)| {
                    Some(**owner) == seat
                        || seat.is_some_and(|me| {
                            self.rumours.get(space).is_some_and(|heard| heard.contains(&me))
                        })
                })
                .map(|(space, owner)| TrapView { space: *space, mine: Some(*owner) == seat })
                .collect(),
            loose: {
                let mut rows: Vec<LooseCoins> = self
                    .loose
                    .iter()
                    .map(|(space, coins)| LooseCoins { space: *space, coins: *coins })
                    .collect();
                rows.sort_by_key(|row| row.space);
                rows
            },
            air_coins:self.air_view(),
            pedestrians:self.pedestrian_views(),
            air_remaining:if self.floating_coins {ambient::AMBIENT_TURN_CAP-self.ambient_earned}else{0},
            crawler:Some(self.crawler_view()),
            traffic:self.traffic_view(),
            driving:self.driving_view(),
            candy_van:self.candy_van_view_for(seat),
            candy_runs:self.candy_runs_view(),
            crates: {
                let mut spaces: Vec<u8> = self.crates.keys().copied().collect();
                spaces.sort();
                spaces
            },
            blockades: {
                let mut spaces: Vec<u8> = self.blockades.keys().copied().collect();
                spaces.sort();
                spaces
            },
            sucked: self.sucked,
            coconuts: self.coconuts.clone(),
            snowmen: self.snowmen.clone(),
            boxes_taken: {
                let mut spaces: Vec<u8> = self.boxes_taken.iter().copied().collect();
                spaces.sort();
                spaces
            },
            lava_rocks: {
                let mut spaces: Vec<u8> = self.lava_rocks.iter().copied().collect();
                spaces.sort();
                spaces
            },
            flotsam: {
                let mut points: Vec<u8> = self.flotsam.keys().copied().collect();
                points.sort();
                points
            },
            vacuum: matches!(self.step, TurnStep::Vacuum { .. }).then(|| VacuumView {
                yaw: self.vacuum.yaw,
                on: self.vacuum.on,
                blowing: self.vacuum.blow,
                cone: VACUUM_CONE * self.nozzle(self.turn_seat()).0,
                rate:self.pull_rate(self.turn_seat()),
                power: self.vacuum.power,
                left: self.vacuum.left.max(0.0),
                out: self.vacuum.out,
                siphoning: {
                    let mut seats: Vec<u8> = self.vacuum.siphon.keys().copied().collect();
                    seats.sort();
                    seats
                },
                braced: {
                    let mut seats: Vec<u8> = self.vacuum.braced.iter().copied().collect();
                    seats.sort();
                    seats
                },
                pulling: {
                    let mut rows: Vec<(u16, f32)> = self.vacuum.pulling.iter().map(|(key, pull)| (*key, pull.min(1.0))).collect();
                    rows.sort_by_key(|row| row.0);
                    rows
                },
            }),
            walk: self.walk_view(),
            positioning: self.position_view(),
            raids: self.raid_views(),
            finale: self.finale_view(),
            prize_at: self.prize_at,
            prize_cost: PRIZE_COST,
            open_prizes: self.open_prizes(),
            order: self.order.clone(),
            turn_seat: self.turn_seat(),
            turn: self.step.clone(),
            round: self.round,
            rounds: self.rounds,
            last_roll: self.last_roll,
            dice: self.dice,
            fixed_roll: self.fixed_roll,
            event: self.event,
            bounty: self.bounty,
            plague: self.plague,
            underdog: self.underdog,
            underdog_cost: UNDERDOG_PRIZE,
            handicap: self.handicapped(),
            bets: {
                let mut rows: Vec<Bet> = self
                    .bets
                    .iter()
                    .map(|(seat, coins)| Bet { seat: *seat, coins: *coins })
                    .collect();
                rows.sort_by_key(|row| row.seat);
                rows
            },
            max_bet: MAX_BET,
            bonuses: self.bonuses.clone(),
            highlights: self.highlights.clone(),
            storm: self.storm,
            storm_to: self.storm.map(|at| self.storm_ahead(at)),
            tide: self.has_tide().then(|| self.tide_in()),
            gates_open_from: self.gates_open_from(),
            gates_open: self.gates_open(),
            log: self.log.clone(),
        }
    }

    /// Seat 0 stands in while nobody is seated, as when only spectators are in the room.
    pub fn turn_seat(&self) -> u8 {
        self.order.get(self.turn_index % self.order.len().max(1)).copied().unwrap_or(0)
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

    /// The seat alone at the bottom of the standings, once a round has been
    /// played to put them there.
    pub fn handicapped(&self) -> Option<u8> {
        if self.round < 2 {
            return None;
        }
        let rows = self.standings();
        match rows.as_slice() {
            [.., above, last] if above.1 > last.1 => Some(last.0),
            _ => None,
        }
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
            if delta > token.stats.best_swing {
                token.stats.best_swing = delta;
            }
        }
    }

    pub fn use_item(&mut self, seat: u8, item: Item, target: Option<u8>) -> Result<(), BoardError> {
        self.use_item_with(seat, item, target, None)
    }

    /// Use an item; `number` is the face a golden die is set to.
    pub fn use_item_with(
        &mut self,
        seat: u8,
        item: Item,
        target: Option<u8>,
        number: Option<u8>,
    ) -> Result<(), BoardError> {
        if matches!(item,Item::SlashRush|Item::DriveBy) {return self.start_raid_item(seat,item);}
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
        if item.sets_dice() && (self.dice > 1 || self.fixed_roll.is_some()) {
            return Err(BoardError::DiceSet);
        }
        if item == Item::Heist && !self.tokens.iter().any(|t| self.rival(seat, t.seat) && t.prizes > 0) {
            return Err(BoardError::NothingToTake);
        }
        if item == Item::GoldenDie && !matches!(number, Some(1..=DICE_SIDES)) {
            return Err(BoardError::PickANumber);
        }
        // Nothing is ever aimed by picking someone: what is thrown or swung
        // gets whoever it gets. See `lob` and `swing`.
        let _ = target;
        if item==Item::DriveCar && self.map!=Map::Frozen {return Err(BoardError::OutOfReach);}
        if self.timed_items && positioning::aimed(item) {
            self.begin_position(seat, item);
            return Ok(());
        }
        self.tokens[holder].items.remove(slot);
        self.tokens[holder].stats.items_used += 1;

        match item {
            Item::Bat => {
                self.say(json!({ "kind": "item", "item": "bat_ready", "by": seat }));
                self.step = TurnStep::Swing { fuse: BOMB_FUSE };
            }
            // Never in the bag; thrown with `throw_crate` and `throw_snowball`.
            Item::Crate | Item::Snowball => {}
            Item::Bomb | Item::Grapple | Item::Swap | Item::Curse | Item::Pickpocket | Item::DriveBy | Item::Slapshot => {
                let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
                let strength = self.rng.random_range(0.0..BOMB_WIND);
                self.say(json!({ "kind": "item", "item": "throw_ready", "thrown": item, "by": seat }));
                self.step = TurnStep::Lob { item, wind: (angle.sin() * strength, angle.cos() * strength), fuse: BOMB_FUSE };
            }
            Item::Trap => {
                // Thrown, and set wherever it lands; see `land_throw`.
                let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
                let strength = self.rng.random_range(0.0..BOMB_WIND);
                self.say(json!({ "kind": "item", "item": "throw_ready", "thrown": item, "by": seat }));
                self.step = TurnStep::Lob { item, wind: (angle.sin() * strength, angle.cos() * strength), fuse: BOMB_FUSE };
            }
            Item::DoubleDice | Item::GoldenDie | Item::Shield | Item::Medkit | Item::Compass | Item::Heist => {
                self.say(json!({ "kind": "item", "item": "check_ready", "checking": item, "by": seat }));
                self.step = TurnStep::Check { item, period: check_period(item), fuse: BOMB_FUSE, number };
            }
            Item::DriveCar => self.begin_drive(seat),
            Item::SlashRush => {}
        }
        Ok(())
    }

    fn draw_item(&mut self)->Item {let choices:Vec<_>=Item::ALL.into_iter().filter(|item|*item!=Item::DriveCar||(self.map==Map::Frozen&&self.timed_items)).collect();let total:u32=choices.iter().map(|item|item.weight()).sum();let mut pick=self.rng.random_range(0..total);for item in choices{if pick<item.weight(){return item;}pick-=item.weight();}Item::Bat}

    fn space_of(&self, seat: u8) -> u8 {
        self.tokens
            .iter()
            .find(|t| t.seat == seat)
            .map(|t| t.space)
            .unwrap_or(0)
    }

    /// One player crossing another: pays out a grudge the aggressor was
    /// holding against them, then leaves the victim holding one.
    fn crossed(&mut self, victim: u8, by: u8) {
        if victim == by {
            return;
        }
        if self.bounty == Some(victim) {
            self.bounty = None;
            self.coins(by, BOUNTY_COINS);
            self.say(json!({ "kind": "bounty", "seat": by, "target": victim, "coins": BOUNTY_COINS }));
        }
        let owed = self
            .tokens
            .iter()
            .find(|t| t.seat == by)
            .and_then(|t| t.grudge)
            == Some(victim);
        if owed {
            self.coins(by, REVENGE_COINS);
            if let Some(token) = self.token(by) {
                token.grudge = None;
            }
            self.say(json!({ "kind": "revenge", "seat": by, "against": victim, "coins": REVENGE_COINS }));
        }
        if let Some(token) = self.token(victim) {
            token.grudge = Some(by);
        }
    }

    /// A shield soaks up one attack and is spent doing it.
    fn blocked(&mut self, by: u8, seat: u8, item: Item) -> bool {
        let Some(token) = self.token(seat) else {
            return false;
        };
        if !token.shield {
            return false;
        }
        token.shield = false;
        self.say(json!({ "kind": "blocked", "seat": seat, "by": by, "item": item }));
        true
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
        self.walk_bonus = 0;
        self.walk_earned=0;
        let cursed = self.token(seat).map(|t| std::mem::take(&mut t.cursed)).unwrap_or(false);
        let sides = if cursed { CURSED_SIDES } else { DICE_SIDES };
        let dice: Vec<u8> = match self.fixed_roll {
            Some(face) => vec![face.min(sides)],
            None => (0..self.dice).map(|_| self.rng.random_range(1..=sides)).collect(),
        };
        let handicap = (self.fixed_roll.is_none() && self.handicapped() == Some(seat))
            .then(|| self.rng.random_range(1..=HANDICAP_SIDES));
        let roll: u8 = dice.iter().sum::<u8>() + handicap.unwrap_or(0);
        if let Some(token) = self.token(seat) {
            token.stats.rolls += dice.len() as u32;
            token.stats.pips += roll as u32;
        }
        self.last_roll = Some(roll);
        self.say(json!({ "kind": "roll", "seat": seat, "roll": roll, "dice": dice, "cursed": cursed, "golden": self.fixed_roll.is_some(), "handicap": handicap }));
        self.advance(seat, roll, Vec::new());
        Ok(roll)
    }

    pub fn last_roll(&self) -> Option<u8> {
        self.last_roll
    }

    /// Walk forward one space at a time, stopping at a junction to ask. The
    /// spaces walked go in the log so the board can hop through each one.
    fn advance(&mut self, seat: u8, steps: u8, mut path: Vec<u8>) {
        let initial = path.is_empty();
        let from = if initial { self.space_of(seat) } else {
            self.tokens.iter().find(|token| token.seat == seat).and_then(|token| token.previous).unwrap_or(self.space_of(seat))
        };
        let mut left = steps;
        let mut path_logged = false;
        while left > 0 {
            let at = self
                .tokens
                .iter()
                .find(|t| t.seat == seat)
                .map(|t| t.space)
                .unwrap_or(0);
            // A portal keeps its directed link, without turning that link
            // into a road that can be walked backwards from its destination.
            if let Some(to) = self.layout[at as usize].link {
                if let Some(token) = self.token(seat) {
                    token.space = to;
                    token.previous = None;
                }
                path.push(to);
                left -= 1;
                continue;
            }
            let ways = self.walk_ways(seat, at);
            if ways.is_empty() { break; }
            if ways.len() > 1 && self.event != Some(RoundEvent::Earthquake) {
                self.step = TurnStep::Branch {
                    options: branch_options(self.layout, at, &ways)
                        .into_iter()
                        .filter(|option| ways.contains(&option.node))
                        .collect(),
                    left,
                };
                self.say_path(seat, path.clone());
                if self.timed_walk && !path.is_empty() {
                    self.start_walk(seat, from, path, initial, walking::AfterWalk::Step(self.step.clone()));
                }
                return;
            }
            let to = ways[0];
            if let Some(token) = self.token(seat) {
                token.previous = Some(at);
                token.space = to;
            }
            path.push(to);
            left -= 1;
            // Walking into a barricade stops you on it, and smashes it.
            if let Some(hidden) = self.blockades.remove(&to) {
                self.say_path(seat, path.clone());
                path_logged = true;
                self.say(json!({ "kind": "barricade", "seat": seat, "space": to, "left": left }));
                self.scatter(to, hidden, 7.0, None);
                break;
            }
        }
        if !path_logged { self.say_path(seat, path.clone()); }
        if self.timed_walk && !path.is_empty() {
            self.start_walk(seat, from, path, initial, walking::AfterWalk::Land { slide: true });
        } else { self.land(seat); }
    }

    fn say_path(&mut self, seat: u8, path: Vec<u8>) {
        self.walked += path.len() as u32;
        if let Some(token) = self.token(seat) {
            token.stats.spaces += path.len() as u32;
        }
        if path.is_empty() {
            return;
        }
        let walked = path.len() as i32;
        self.say(json!({ "kind": "move", "seat": seat, "path": path }));
        if self.event == Some(RoundEvent::TollRoad) {
            let purse = self.tokens.iter().find(|t| t.seat == seat).map(|t| t.coins).unwrap_or(0);
            let paid = purse.min(walked * TOLL_ROAD_COINS);
            self.coins(seat, -paid);
            self.say(json!({ "kind": "toll_road", "seat": seat, "spaces": walked, "coins": paid }));
        }
    }

    /// Different items on the counter, priced by how rare they are.
    fn stock(&mut self) -> Vec<ShopOffer> {
        let market = self.event == Some(RoundEvent::MarketDay);
        let count = if market { MARKET_OFFERS } else { SHOP_OFFERS };
        let mut offers: Vec<ShopOffer> = Vec::new();
        for _ in 0..80 {
            if offers.len() >= count {
                break;
            }
            let item = self.draw_item();
            if offers.iter().any(|offer| offer.item == item) {
                continue;
            }
            let price = if market { (item.price() + 1) / 2 } else { item.price() };
            offers.push(ShopOffer { item, price });
        }
        offers
    }

    /// The vacuum upgrades this player does not have yet, half price on market day.
    fn upgrade_stock(&self, seat: u8) -> Vec<UpgradeOffer> {
        let market = self.event == Some(RoundEvent::MarketDay);
        let Some(me) = self.tokens.iter().find(|t| t.seat == seat) else {
            return Vec::new();
        };
        Upgrade::ALL
            .into_iter()
            .filter(|upgrade| !me.has(*upgrade))
            .map(|upgrade| UpgradeOffer {
                upgrade,
                price: if market { (upgrade.price() + 1) / 2 } else { upgrade.price() },
            })
            .collect()
    }

    /// Fire the cannon. The charge is what you asked for, give or take a
    /// couple of spaces, so aiming at the prize is a gamble.
    pub fn fire(&mut self, seat: u8, power: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        let TurnStep::Cannon { max } = self.step.clone() else {
            return Err(BoardError::WrongStep);
        };
        let charge = power.clamp(1, max);
        let drift = self.rng.random_range(0..=(CANNON_SCATTER * 2)) as i16 - CANNON_SCATTER as i16;
        let flown = (charge as i16 + drift).max(1) as u8;
        let from = self.space_of(seat);
        let mut at = from;
        let mut path = Vec::new();
        for _ in 0..flown {
            at = space_in(self.layout, at).next.first().copied().unwrap_or(at);
            path.push(at);
        }
        if let Some(token) = self.token(seat) {
            token.previous = None;
            token.space = at;
        }
        self.say(json!({
            "kind": "cannon", "seat": seat, "power": charge, "flew": flown,
            "from": from, "to": at, "path": path,
        }));
        self.after_landing(seat);
        Ok(())
    }

    /// Where a bomb thrown from `(x, z)` comes down: `yaw` is the way it is
    /// thrown (0 is +z, a quarter turn is +x), `pitch` how steeply, and
    /// `power` from 0 to 1 how hard. The wind carries it the whole way.
    pub fn bomb_landing(from: (f32, f32), yaw: f32, pitch: f32, power: f32, wind: (f32, f32)) -> ((f32, f32), f32) {
        let speed = power.clamp(0.0, 1.0) * BOMB_SPEED;
        let pitch = pitch.clamp(0.0, 1.45);
        let (along, up) = (speed * pitch.cos(), speed * pitch.sin());
        // Up from the hand and down to the ground it was thrown over.
        let time = (up + (up * up + 2.0 * BOMB_GRAVITY * BOMB_HAND).sqrt()) / BOMB_GRAVITY;
        let x = from.0 + (yaw.sin() * along + wind.0) * time;
        let z = from.1 + (yaw.cos() * along + wind.1) * time;
        ((x, z), time)
    }

    /// Throw the bomb in hand. Whoever it comes down on loses the most and a
    /// heart; anyone near enough is caught in the blast, the thrower too.
    pub fn lob(&mut self, seat: u8, yaw: f32, pitch: f32, power: f32) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        let TurnStep::Lob { item, wind, .. } = self.step.clone() else {
            return Err(BoardError::WrongStep);
        };
        let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
        let from = self.spot_of(seat);
        let ((x, z), flight) = Self::bomb_landing(from, finite(yaw), finite(pitch), finite(power), wind);
        if matches!(item, Item::DriveBy | Item::Slapshot) {
            self.board_shot(seat, item, (x, z));
        } else if item == Item::Bomb {
            self.blow_up(seat, (x, z), flight, false);
        } else {
            self.land_throw(seat, item, (x, z), flight);
        }
        self.finish_position();
        Ok(())
    }

    /// The time ran out with something in hand: the bomb goes off in it, and
    /// anything else is dropped, wasted.
    pub fn fuse_out(&mut self, seat: u8) {
        if seat != self.turn_seat() { return; }
        match self.step.clone() {
            TurnStep::Lob { item: Item::Bomb, .. } => {
                self.blow_up(seat, self.spot_of(seat), 0.0, true);
            }
            TurnStep::Lob { item, .. } => {
                self.say(json!({ "kind": "item", "item": item, "by": seat, "quality": "dropped" }));
                self.step = TurnStep::Items;
            }
            TurnStep::Swing { .. } => {
                self.say(json!({ "kind": "item", "item": "bat", "by": seat, "quality": "dropped" }));
                self.step = TurnStep::Items;
            }
            TurnStep::Check { item, .. } => {
                self.say(json!({ "kind": "item", "item": item, "by": seat, "quality": "dropped" }));
                self.step = TurnStep::Items;
            }
            _ => {}
        }
        self.finish_position();
    }

    /// Where a token stands on the board, in world units.
    fn spot_of(&self, seat: u8) -> (f32, f32) {
        if let Some(raid)=self.raids.get(&seat) {return (raid.x,raid.z);}
        if let Some(position) = self.positioning.as_ref().filter(|position| position.seat == seat) { return (position.x, position.z); }
        let at = space_in(self.layout, self.space_of(seat));
        (at.x, at.z)
    }

    /// The same standing position as the client's shared-tile fan-out.
    fn rendered_spot_of(&self,seat:u8)->(f32,f32) {
        if let Some(raid)=self.raids.get(&seat) {return (raid.x,raid.z);}
        if let Some(walk)=self.walk_view().filter(|walk|walk.seat==seat) {return (walk.x,walk.z);}
        if let Some(position) = self.positioning.as_ref().filter(|position| position.seat == seat) { return (position.x, position.z); }
        let space=self.space_of(seat);
        let sharing:Vec<u8>=self.tokens.iter().filter(|token|token.space==space).map(|token|token.seat).collect();
        let index=sharing.iter().position(|other|*other==seat).unwrap_or(0);
        let mut from=self.spot_of(seat);
        if sharing.len()>1 {
            let angle=std::f32::consts::TAU*index as f32/sharing.len() as f32+std::f32::consts::FRAC_PI_4;
            let spread=0.5+sharing.len() as f32*0.12;
            from.0+=angle.cos()*spread;from.1+=angle.sin()*spread;
        }
        from
    }

    /// One world origin for moving, positioning, raiding and standing actors.
    pub(super) fn actual_spot_of(&self,seat:u8)->(f32,f32) {if let Some(at)=self.candy_visit_spot(seat){return at;}if let Some(at)=self.finale_spot(seat){return at;}if let Some(driver)=self.driver.as_ref().filter(|d|d.view.seat==seat){return(driver.view.x,driver.view.z);}self.rendered_spot_of(seat)}

    fn crate_spot(&self,space:u8)->(f32,f32) {
        let at=&self.layout[space as usize];let angle=space as f32*2.3;
        (at.x+angle.cos()*0.95,at.z+angle.sin()*0.95)
    }

    fn box_spot(&self,space:u8)->(f32,f32) {
        static ISLAND:OnceLock<(f32,f32)>=OnceLock::new();
        static VOLCANO:OnceLock<(f32,f32)>=OnceLock::new();
        static SHIPWRECK:OnceLock<(f32,f32)>=OnceLock::new();
        static FROZEN:OnceLock<(f32,f32)>=OnceLock::new();
        let cache=match self.map{Map::Island=>&ISLAND,Map::Volcano=>&VOLCANO,Map::Shipwreck=>&SHIPWRECK,Map::Frozen=>&FROZEN};
        let &(cx,cz)=cache.get_or_init(||{
            let mut xs:Vec<f32>=self.layout.iter().map(|space|space.x).collect();
            let mut zs:Vec<f32>=self.layout.iter().map(|space|space.z).collect();
            let source:Value=serde_json::from_str(self.map.source()).expect("map visuals");
            for point in source["coast"].as_array().into_iter().flatten(){if let (Some(x),Some(z))=(point[0].as_f64(),point[1].as_f64()){xs.push(x as f32);zs.push(z as f32);}}
            ((xs.iter().copied().fold(f32::INFINITY,f32::min)+xs.iter().copied().fold(f32::NEG_INFINITY,f32::max))*0.5,
             (zs.iter().copied().fold(f32::INFINITY,f32::min)+zs.iter().copied().fold(f32::NEG_INFINITY,f32::max))*0.5)
        });
        let at=&self.layout[space as usize];let (dx,dz)=(at.x-cx,at.z-cz);let length=dx.hypot(dz).max(0.001);
        (at.x+dx/length*1.15,at.z+dz/length*1.15)
    }

    /// A grapple, hex, swap orb or sticky hand comes down: it gets the rival
    /// nearest where it lands, if anyone is near enough, and is wasted if not.
    fn land_throw(&mut self, seat: u8, item: Item, (x, z): (f32, f32), flight: f32) {
        self.step = TurnStep::Items;
        if item == Item::Snowball {
            // Whoever it comes down by is frozen solid for their next turn.
            let struck = self
                .tokens
                .iter()
                .filter(|t| self.rival(seat, t.seat))
                .map(|t| {
                    let at = self.item_spot_of(t.seat);
                    (t.seat, (at.0 - x).hypot(at.1 - z))
                })
                .filter(|(_, away)| *away <= THROW_HIT_RADIUS)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(seat, _)| seat);
            match struck {
                Some(target) if !self.blocked(seat, target, Item::Snowball) => {
                    if let Some(token) = self.token(target) {
                        token.resting = token.resting.max(1);
                    }
                    self.say(json!({ "kind": "item", "item": "snowball", "by": seat, "target": target, "x": x, "z": z, "flight": flight, "quality": "hit" }));
                    self.crossed(target, seat);
                }
                Some(target) => self.say(json!({ "kind": "item", "item": "snowball", "by": seat, "target": target, "x": x, "z": z, "flight": flight, "quality": "blocked" })),
                None => self.say(json!({ "kind": "item", "item": "snowball", "by": seat, "x": x, "z": z, "flight": flight, "quality": "miss" })),
            }
            return;
        }
        if item == Item::Crate {
            // A barricade on the space it comes down on, or nothing if it
            // falls short of every space.
            let landed = self
                .layout
                .iter()
                .filter(|space| !space.under && space.kind != SpaceKind::Start)
                .map(|space| (space.id, ((space.x - x).powi(2) + (space.z - z).powi(2)).sqrt()))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match landed {
                Some((at, away)) if away <= THROW_HIT_RADIUS && !self.blockades.contains_key(&at) => {
                    let hidden = self.crates.remove(&at).unwrap_or(0);
                    self.blockades.insert(at, hidden);
                    self.say(json!({ "kind": "item", "item": "crate", "by": seat, "space": at, "x": x, "z": z, "flight": flight, "quality": "hit" }));
                }
                _ => self.say(json!({ "kind": "item", "item": "crate", "by": seat, "x": x, "z": z, "flight": flight, "quality": "miss" })),
            }
            return;
        }
        if item == Item::Trap {
            // Set on the space it comes down on, if it comes down on one.
            let landed = self
                .layout
                .iter()
                .map(|space| (space.id, ((space.x - x).powi(2) + (space.z - z).powi(2)).sqrt()))
                .filter(|(_, away)| *away <= THROW_HIT_RADIUS)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(id, _)| id);
            match landed {
                Some(at) => {
                    self.traps.insert(at, seat);
                    self.rumours.entry(at).or_default().push(seat);
                    self.say(json!({ "kind": "item", "item": "trap", "by": seat, "space": at, "x": x, "z": z, "flight": flight, "quality": "hit" }));
                }
                None => self.say(json!({ "kind": "item", "item": "trap", "by": seat, "x": x, "z": z, "flight": flight, "quality": "miss" })),
            }
            return;
        }
        let struck = self
            .tokens
            .iter()
            .filter(|t| self.rival(seat, t.seat))
            .map(|t| {
                let at = self.item_spot_of(t.seat);
                (t.seat, (at.0 - x).hypot(at.1 - z))
            })
            .filter(|(_, away)| *away <= THROW_HIT_RADIUS)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(seat, _)| seat);
        let flown = json!({ "x": x, "z": z, "flight": flight });
        let say = |board: &mut Self, mut entry: Value| {
            for (key, value) in flown.as_object().unwrap() {
                entry[key] = value.clone();
            }
            board.say(entry);
        };
        let Some(target) = struck else {
            say(self, json!({ "kind": "item", "item": item, "by": seat, "quality": "miss" }));
            return;
        };
        if self.blocked(seat, target, item) {
            say(self, json!({ "kind": "item", "item": item, "by": seat, "target": target, "quality": "blocked" }));
            return;
        }
        match item {
            Item::Grapple => {
                let stolen = self.tokens.iter().find(|t| t.seat == target).map(|t| t.coins.min(GRAPPLE_COINS)).unwrap_or(0);
                self.coins(target, -stolen);
                self.coins(seat, stolen);
                say(self, json!({ "kind": "item", "item": "grapple", "by": seat, "target": target, "coins": stolen, "quality": if stolen > 0 {"hit"} else {"empty"} }));
                self.crossed(target, seat);
            }
            Item::Swap => {
                let mine = self.space_of(seat);
                let theirs = self.space_of(target);
                if let Some(token) = self.token(seat) {
                    token.previous = None;
                    token.space = theirs;
                }
                if let Some(token) = self.token(target) {
                    token.previous = None;
                    token.space = mine;
                }
                say(self, json!({ "kind": "item", "item": "swap", "by": seat, "target": target, "from": mine, "to": theirs, "quality": "hit" }));
            }
            Item::Curse => {
                if let Some(token) = self.token(target) {
                    token.cursed = true;
                }
                say(self, json!({ "kind": "item", "item": "curse", "by": seat, "target": target, "quality": "hit" }));
                self.crossed(target, seat);
            }
            Item::Pickpocket => {
                let mark = self.tokens.iter().position(|t| t.seat == target).unwrap();
                if self.tokens[mark].items.is_empty() {
                    say(self, json!({ "kind": "item", "item": "pickpocket", "by": seat, "target": target, "quality": "empty" }));
                    return;
                }
                let pick = self.rng.random_range(0..self.tokens[mark].items.len());
                let taken = self.tokens[mark].items.remove(pick);
                if let Some(token) = self.token(seat) {
                    token.items.push(taken);
                }
                say(self, json!({ "kind": "item", "item": "pickpocket", "by": seat, "target": target, "took": taken, "quality": "hit" }));
                self.crossed(target, seat);
            }
            _ => {}
        }
    }

    /// Stop the needle of the item in hand at `stop`, from 0 to 1, best at 0.5.
    pub fn check(&mut self, seat: u8, stop: f32) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        let TurnStep::Check { item, number, .. } = self.step.clone() else {
            return Err(BoardError::WrongStep);
        };
        self.step = TurnStep::Items;
        let grade = Grade::of(stop);
        let quality = grade.label();
        let holder = self.tokens.iter().position(|t| t.seat == seat).ok_or(BoardError::UnknownTarget)?;
        match item {
            Item::Medkit => {
                let heal = match grade {
                    Grade::Perfect => MAX_HEALTH,
                    Grade::Good => 2,
                    Grade::Weak => 1,
                    Grade::Fail => 0,
                };
                let before = self.tokens[holder].health;
                self.tokens[holder].health = (before + heal).min(MAX_HEALTH);
                let healed = self.tokens[holder].health - before;
                self.say(json!({ "kind": "item", "item": "medkit", "by": seat, "healed": healed, "quality": quality }));
            }
            Item::Shield => {
                let up = matches!(grade, Grade::Perfect | Grade::Good);
                self.tokens[holder].shield |= up;
                if grade == Grade::Perfect {
                    self.coins(seat, CHECK_BONUS_COINS);
                }
                self.say(json!({ "kind": "item", "item": "shield", "by": seat, "up": up, "quality": quality }));
            }
            Item::Compass => {
                let short = match grade {
                    Grade::Perfect => Some(1),
                    Grade::Good => Some(COMPASS_SHORT),
                    Grade::Weak => Some(COMPASS_SHORT * 2),
                    Grade::Fail => None,
                };
                let from = self.tokens[holder].space;
                let to = short.map(|short| self.walk_back(self.prize_at, short)).unwrap_or(from);
                self.tokens[holder].previous = None;
                self.tokens[holder].space = to;
                self.say(json!({ "kind": "item", "item": "compass", "by": seat, "from": from, "to": to, "quality": quality }));
            }
            Item::DoubleDice => {
                if matches!(grade, Grade::Perfect | Grade::Good) {
                    self.dice = 2;
                }
                if grade == Grade::Perfect {
                    self.coins(seat, CHECK_BONUS_COINS);
                }
                self.say(json!({ "kind": "item", "item": "double_dice", "by": seat, "dice": self.dice, "quality": quality }));
            }
            Item::GoldenDie => {
                let asked = number.unwrap_or(DICE_SIDES) as i16;
                let drift = match grade {
                    Grade::Perfect => Some(0),
                    Grade::Good => Some(1),
                    Grade::Weak => Some(2),
                    Grade::Fail => None,
                };
                self.fixed_roll = drift.map(|drift| {
                    let off = if drift == 0 { 0 } else if self.rng.random_bool(0.5) { drift } else { -drift };
                    (asked + off).clamp(1, DICE_SIDES as i16) as u8
                });
                self.say(json!({ "kind": "item", "item": "golden_die", "by": seat, "asked": asked, "number": self.fixed_roll, "quality": quality }));
            }
            Item::Heist => {
                let mark = self
                    .tokens
                    .iter()
                    .filter(|t| self.rival(seat, t.seat) && t.prizes > 0)
                    .max_by_key(|t| (t.prizes, t.coins))
                    .map(|t| t.seat);
                let cracked = matches!(grade, Grade::Perfect | Grade::Good);
                match mark {
                    Some(mark) if cracked => {
                        if self.blocked(seat, mark, item) {
                            return Ok(());
                        }
                        if let Some(token) = self.token(mark) {
                            token.prizes = token.prizes.saturating_sub(1);
                        }
                        if let Some(token) = self.token(seat) {
                            token.prizes += 1;
                        }
                        self.say(json!({ "kind": "item", "item": "heist", "by": seat, "target": mark, "quality": quality }));
                        self.crossed(mark, seat);
                    }
                    _ => self.say(json!({ "kind": "item", "item": "heist", "by": seat, "quality": "alarm" })),
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Swing the bat in hand toward `yaw`, with the timing needle stopped at
    /// `stop` from 0 to 1, best at 0.5. Everyone in the arc in front is hit,
    /// and the better the timing, the further back they go.
    pub fn swing(&mut self, seat: u8, yaw: f32, stop: f32) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if !matches!(self.step, TurnStep::Swing { .. }) {
            return Err(BoardError::WrongStep);
        }
        self.step = TurnStep::Items;
        let (quality, knock, hurts) = match Grade::of(stop) {
            Grade::Perfect => ("perfect", BAT_SPACES + 1, true),
            Grade::Good => ("good", BAT_SPACES, true),
            Grade::Weak => ("weak", 1, false),
            Grade::Fail => ("whiff", 0, false),
        };
        let yaw = if yaw.is_finite() { yaw } else { 0.0 };
        let (mx, mz) = self.spot_of(seat);
        let in_arc: Vec<u8> = if knock == 0 {
            Vec::new()
        } else {
            self.tokens
                .iter()
                .filter(|t| self.rival(seat, t.seat))
                .filter(|t| {
                    let at = self.item_spot_of(t.seat);
                    let (dx, dz) = (at.0 - mx, at.1 - mz);
                    let away = (dx * dx + dz * dz).sqrt();
                    // Sharing your space, they are in reach whichever way you face.
                    let off_line = (dx.atan2(dz) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    away <= BAT_RANGE && (away < 0.8 || off_line.abs() <= BAT_ARC)
                })
                .map(|t| t.seat)
                .collect()
        };
        if in_arc.is_empty() {
            let quality = if knock == 0 { "whiff" } else { "miss" };
            self.say(json!({ "kind": "item", "item": "bat", "by": seat, "yaw": yaw, "quality": quality }));
            self.finish_position();
            return Ok(());
        }
        // Barricades and crates in the arc are smashed too.
        if knock > 0 {
            let arc_spaces: Vec<u8> = self
                .blockades
                .keys()
                .chain(self.crates.keys())
                .copied()
                .filter(|&space| {
                    let at = space_in(self.layout, space);
                    let (dx, dz) = (at.x - mx, at.z - mz);
                    let away = (dx * dx + dz * dz).sqrt();
                    let off_line = (dx.atan2(dz) - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    away <= BAT_RANGE && (away < 0.8 || off_line.abs() <= BAT_ARC)
                })
                .collect();
            for space in arc_spaces {
                let at = space_in(self.layout, space);
                self.smash_near((at.x, at.z), 0.5, seat);
            }
        }
        let knocked_out = match quality {
            "perfect" => 6,
            "good" => 4,
            _ => 2,
        };
        for target in in_arc {
            if self.blocked(seat, target, Item::Bat) {
                continue;
            }
            let from = self.space_of(target);
            // The hit knocks coins out of them, flying off across the board.
            let purse = self.tokens.iter().find(|t| t.seat == target).map(|t| t.coins).unwrap_or(0);
            let lost = purse.min(knocked_out);
            self.coins(target, -lost);
            self.scatter(from, lost, BAT_SCATTER, Some(target));
            let to = self.walk_back(from, knock);
            if let Some(token) = self.token(target) {
                token.previous = None;
                token.space = to;
            }
            self.say(json!({ "kind": "item", "item": "bat", "by": seat, "target": target, "from": from, "to": to, "yaw": yaw, "quality": quality }));
            self.crossed(target, seat);
            if hurts {
                self.hurt(target, 1);
            }
        }
        self.finish_position();
        Ok(())
    }

    fn blow_up(&mut self, seat: u8, (x, z): (f32, f32), flight: f32, in_hand: bool) {
        let away = |seat: u8| {
            let at = self.item_spot_of(seat);
            (at.0 - x).hypot(at.1 - z)
        };
        let partner = self.partner_of(seat);
        let direct: Vec<u8> = self
            .tokens
            .iter()
            .filter(|t| Some(t.seat) != partner && away(t.seat) <= BOMB_DIRECT_RADIUS)
            .map(|t| t.seat)
            .collect();
        let splash: Vec<u8> = self
            .tokens
            .iter()
            .filter(|t| Some(t.seat) != partner && !direct.contains(&t.seat) && away(t.seat) <= BOMB_BLAST_RADIUS)
            .map(|t| t.seat)
            .collect();
        let direct: Vec<u8> = direct.into_iter().filter(|&hit| hit == seat || !self.blocked(seat, hit, Item::Bomb)).collect();
        let splash: Vec<u8> = splash.into_iter().filter(|&hit| hit == seat || !self.blocked(seat, hit, Item::Bomb)).collect();
        self.smash_near((x, z), BOMB_BLAST_RADIUS, seat);
        let quality = if in_hand {
            "in_hand"
        } else if direct.iter().any(|&hit| hit != seat) {
            "direct"
        } else if splash.iter().any(|&hit| hit != seat) {
            "splash"
        } else {
            "miss"
        };
        self.say(json!({
            "kind": "item", "item": "bomb", "by": seat, "x": x, "z": z, "flight": flight, "quality": quality,
            "direct": direct.clone(), "caught": splash.clone(),
            "coins": BOMB_COINS, "direct_coins": BOMB_DIRECT_COINS,
        }));
        for hit in direct {
            self.coins(hit, -BOMB_DIRECT_COINS);
            if hit != seat {
                self.crossed(hit, seat);
            }
            self.hurt(hit, 1);
        }
        for hit in splash {
            self.coins(hit, -BOMB_COINS);
            if hit != seat {
                self.crossed(hit, seat);
            }
        }
        self.step = TurnStep::Items;
    }

    /// Buy one of the items the shop put out, or `None` to walk on.
    pub fn shop(&mut self, seat: u8, item: Option<Item>) -> Result<(), BoardError> {
        self.shop_with(seat, item, None)
    }

    /// Buy an item or a vacuum upgrade, or neither to walk on. One thing a visit.
    pub fn shop_with(&mut self, seat: u8, item: Option<Item>, upgrade: Option<Upgrade>) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        let TurnStep::Shop { offers, upgrades } = self.step.clone() else {
            return Err(BoardError::WrongStep);
        };
        if let Some(upgrade) = upgrade {
            let offer = upgrades
                .iter()
                .find(|offer| offer.upgrade == upgrade)
                .ok_or(BoardError::NotAnOption)?;
            let holder = self
                .tokens
                .iter()
                .position(|t| t.seat == seat)
                .ok_or(BoardError::UnknownTarget)?;
            if self.tokens[holder].coins < offer.price {
                return Err(BoardError::TooDear);
            }
            self.coins(seat, -offer.price);
            self.tokens[holder].upgrades.push(upgrade);
            self.say(json!({ "kind": "upgraded", "seat": seat, "upgrade": upgrade, "coins": offer.price }));
        } else if let Some(item) = item {
            let offer = offers
                .iter()
                .find(|offer| offer.item == item)
                .ok_or(BoardError::NotAnOption)?;
            let holder = self
                .tokens
                .iter()
                .position(|t| t.seat == seat)
                .ok_or(BoardError::UnknownTarget)?;
            if self.tokens[holder].coins < offer.price {
                return Err(BoardError::TooDear);
            }
            if self.tokens[holder].items.len() >= MAX_ITEMS {
                return Err(BoardError::BagFull);
            }
            self.coins(seat, -offer.price);
            self.tokens[holder].items.push(item);
            self.say(json!({ "kind": "bought", "seat": seat, "item": item, "coins": offer.price }));
        } else {
            self.say(json!({ "kind": "bought", "seat": seat, "item": null, "coins": 0 }));
        }
        self.after_landing(seat);
        Ok(())
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
        let from = self.space_of(seat);
        if let Some(token) = self.token(seat) {
            token.previous = Some(from);
            token.space = node;
        }
        self.say(json!({ "kind": "branch", "seat": seat, "space": node }));
        self.step = TurnStep::Roll;
        self.advance(seat, left - 1, vec![node]);
        Ok(())
    }

    fn walk_back(&self, from: u8, steps: u8) -> u8 {
        let mut at = from;
        for _ in 0..steps {
            // The way back is whichever space leads here; the first one will do.
            match self.layout.iter().find(|s| s.next.contains(&at)) {
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
            let purse = self.tokens.iter().find(|t| t.seat == seat).map(|t| t.coins).unwrap_or(0);
            let lost = purse.min(KNOCKOUT_COINS);
            self.coins(seat, -KNOCKOUT_COINS);
            if let Some(token) = self.token(seat) {
                token.health = MAX_HEALTH;
                token.resting = 1;
            }
            if let Some(token) = self.token(seat) {
                token.stats.knockouts += 1;
            }
            self.say(json!({ "kind": "knockout", "seat": seat, "coins": KNOCKOUT_COINS }));
            self.spill(seat, lost);
        }
    }

    /// Stopping on someone already standing there settles it with a roll
    /// each: the higher roll takes coins and a heart off the other.
    fn scrap(&mut self, seat: u8, at: u8) {
        let Some(rival) = self
            .tokens
            .iter()
            .filter(|t| self.rival(seat, t.seat) && t.space == at && t.resting == 0)
            .max_by_key(|t| t.coins)
            .map(|t| t.seat)
        else {
            return;
        };
        let mine = self.rng.random_range(1..=DICE_SIDES);
        let theirs = self.rng.random_range(1..=DICE_SIDES);
        if mine == theirs {
            self.say(json!({ "kind": "scrap", "by": seat, "against": rival, "rolls": [mine, theirs], "winner": null, "coins": 0 }));
            return;
        }
        let (winner, loser) = if mine > theirs { (seat, rival) } else { (rival, seat) };
        let taken = self
            .tokens
            .iter()
            .find(|t| t.seat == loser)
            .map(|t| t.coins.min(SCRAP_COINS))
            .unwrap_or(0);
        self.coins(loser, -taken);
        self.coins(winner, taken);
        self.say(json!({ "kind": "scrap", "by": seat, "against": rival, "rolls": [mine, theirs], "winner": winner, "coins": taken }));
        self.crossed(loser, winner);
        self.hurt(loser, 1);
    }

    /// Coins knocked out of someone land on the board for whoever walks over
    /// them next, spread over the space they fell on and the way ahead.
    /// Crates washed up on open spaces: not the start, a shop or a prize
    /// spot, and not where a crate or a barricade already is. Some hide coins.
    fn wash_up(&mut self, count: usize) {
        let open: Vec<u8> = self
            .layout
            .iter()
            .filter(|space| {
                !matches!(space.kind, SpaceKind::Start | SpaceKind::Shop | SpaceKind::PrizeSpot)
                    && !space.gated
                    && !space.under
                    && !self.crates.contains_key(&space.id)
                    && !self.blockades.contains_key(&space.id)
            })
            .map(|space| space.id)
            .collect();
        let mut washed = Vec::new();
        for _ in 0..count.min(open.len()) {
            let at = open[self.rng.random_range(0..open.len())];
            if self.crates.contains_key(&at) {
                continue;
            }
            let hidden = if self.rng.random_bool(0.4) { self.rng.random_range(3..=6) } else { 0 };
            self.crates.insert(at, hidden);
            washed.push(at);
        }
        if !washed.is_empty() {
            self.say(json!({ "kind": "crates", "spaces": washed }));
        }
    }

    /// Coins sent flying from a space to others round it, as far as `reach`
    /// in world units, in small piles for anyone to walk over.
    fn scatter(&mut self, from: u8, coins: i32, reach: f32, seat: Option<u8>) {
        if coins <= 0 {
            return;
        }
        let origin = space_in(self.layout, from);
        let near: Vec<u8> = self
            .layout
            .iter()
            .filter(|space| space.id != from && !space.under && !space.gated)
            .filter(|space| ((space.x - origin.x).powi(2) + (space.z - origin.z).powi(2)).sqrt() <= reach)
            .map(|space| space.id)
            .collect();
        if near.is_empty() {
            *self.loose.entry(from).or_insert(0) += coins;
            return;
        }
        let mut left = coins;
        let mut piles: HashMap<u8, i32> = HashMap::new();
        while left > 0 {
            let amount = self.rng.random_range(1..=3).min(left);
            let at = near[self.rng.random_range(0..near.len())];
            *piles.entry(at).or_insert(0) += amount;
            left -= amount;
        }
        let mut dropped: Vec<Value> = Vec::new();
        let mut spots: Vec<(u8, i32)> = piles.into_iter().collect();
        spots.sort();
        for (at, amount) in spots {
            *self.loose.entry(at).or_insert(0) += amount;
            dropped.push(json!({ "space": at, "coins": amount }));
        }
        self.say(json!({ "kind": "scatter", "from": from, "seat": seat, "dropped": dropped }));
    }

    /// Smashes the barricades and crates within `reach` of a point, sending
    /// whatever they hid flying.
    fn smash_near(&mut self, (x, z): (f32, f32), reach: f32, by: u8) {
        let layout = self.layout;
        let near = |space: u8| {
            let at = space_in(layout, space);
            ((at.x - x).powi(2) + (at.z - z).powi(2)).sqrt() <= reach
        };
        let barricades: Vec<u8> = self.blockades.keys().copied().filter(|&space| near(space)).collect();
        let crates: Vec<u8> = self.crates.keys().copied().filter(|&space| near(space)).collect();
        for space in barricades {
            let hidden = self.blockades.remove(&space).unwrap_or(0);
            self.say(json!({ "kind": "smashed", "space": space, "by": by, "barricade": true }));
            self.scatter(space, hidden, 7.0, None);
        }
        for space in crates {
            let hidden = self.crates.remove(&space).unwrap_or(0);
            self.say(json!({ "kind": "smashed", "space": space, "by": by, "barricade": false }));
            self.scatter(space, hidden, 7.0, None);
        }
    }

    /// Once a turn, before rolling: take out the vacuum. It runs in first
    /// person for `VACUUM_SECONDS` of sucking, aimed with `vacuum`.
    pub fn suck(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        // Before rolling, or once you have landed.
        if !matches!(self.step, TurnStep::Items | TurnStep::Done) || (self.sucked && !self.endless_vacuum) {
            return Err(BoardError::WrongStep);
        }
        self.sucked = true;
        let seconds = self.battery(seat);
        self.vacuum = Vacuum {
            left: seconds,
            after: Some(Box::new(self.step.clone())),
            ..Vacuum::default()
        };
        self.say(json!({ "kind": "vacuum_ready", "seat": seat }));
        self.step = TurnStep::Vacuum { seconds };
        Ok(())
    }

    fn upgraded(&self, seat: u8, upgrade: Upgrade) -> bool {
        self.tokens.iter().any(|t| t.seat == seat && t.has(upgrade))
    }

    /// Seconds of sucking or blowing a seat's vacuum has.
    pub fn battery(&self, seat: u8) -> f32 {
        if self.upgraded(seat, Upgrade::BigBattery) { BIG_BATTERY_SECONDS } else { VACUUM_SECONDS }
    }

    /// Seconds a seat's motor takes to spin up.
    pub fn spin_up(&self, seat: u8) -> f32 {
        if self.upgraded(seat, Upgrade::Turbo) { VACUUM_SPIN_UP * TURBO_SPIN_UP } else { VACUUM_SPIN_UP }
    }

    /// How much wider a seat's cone is, and how much of its strength it keeps.
    pub fn nozzle(&self, seat: u8) -> (f32, f32) {
        if self.upgraded(seat, Upgrade::WideNozzle) { (WIDE_NOZZLE_CONE, WIDE_NOZZLE_STRENGTH) } else { (1.0, 1.0) }
    }

    /// Every owned upgrade improves throughput as well as its named benefit.
    pub fn pull_rate(&self,seat:u8)->f32 {
        1.0+Upgrade::ALL.into_iter().filter(|upgrade|self.upgraded(seat,*upgrade)).count() as f32*0.15
    }

    /// How hard a seat's vacuum, as aimed now, works on `at`.
    fn reach_of(&self, seat: u8) -> impl Fn((f32, f32), f32) -> Option<f32> + use<> {
        let from = self.vacuum.origin.unwrap_or_else(||self.rendered_spot_of(seat));
        let yaw = self.vacuum.yaw;
        let (widen, keep) = self.nozzle(seat);
        move |at, narrow| vacuum_strength(from, yaw, at, narrow * widen).map(|pull| pull * keep)
    }

    /// Where the vacuum points, and whether it is sucking.
    pub fn aim_vacuum(&mut self, seat: u8, yaw: f32, on: bool) -> Result<(), BoardError> {
        self.aim_vacuum_with(seat, yaw, on, false)
    }

    /// Where the vacuum points, whether it is running, and which way: sucking
    /// in or blowing out. Turning it round starts the motor over.
    pub fn aim_vacuum_with(&mut self, seat: u8, yaw: f32, on: bool, blow: bool) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if !matches!(self.step, TurnStep::Vacuum { .. }) {
            return Err(BoardError::WrongStep);
        }
        self.vacuum.yaw = if yaw.is_finite() { yaw } else { 0.0 };
        self.vacuum.on = on;
        if on && blow != self.vacuum.blow {
            self.vacuum.blow = blow;
            self.vacuum.power = 0.0;
            self.vacuum.pulling.clear();
            self.vacuum.siphon.clear();
            self.vacuum.gust.clear();
        }
        Ok(())
    }

    /// A rival holds on against the vacuum while it drains them, keeping
    /// more of their coins.
    pub fn brace(&mut self, seat: u8, on: bool) -> Result<(), BoardError> {
        if matches!(self.step,TurnStep::Walking) {return self.brace_walk(seat,on);}
        if !matches!(self.step, TurnStep::Vacuum { .. }) || seat == self.turn_seat() {
            return Err(BoardError::WrongStep);
        }
        if on {
            self.vacuum.braced.insert(seat);
        } else {
            self.vacuum.braced.remove(&seat);
        }
        Ok(())
    }

    /// Puts the vacuum away, sucking or not.
    pub fn stow_vacuum(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if !matches!(self.step, TurnStep::Vacuum { .. }) {
            return Err(BoardError::WrongStep);
        }
        self.end_vacuum(seat);
        Ok(())
    }

    fn end_vacuum(&mut self, seat: u8) {
        self.vacuum.on = false;
        self.vacuum.power = 0.0;
        self.vacuum.pulling.clear();
        self.vacuum.siphon.clear();
        self.vacuum.gust.clear();
        self.vacuum.braced.clear();
        self.step = self.vacuum.after.take().map(|step| *step).unwrap_or(TurnStep::Items);
        self.say(json!({ "kind": "vacuum_done", "seat": seat }));
    }

    /// Everything the vacuum can draw in right now, each with its key, where
    /// it is, and how slow it is to pull (seconds at point blank).
    fn pullables(&self, seat: u8) -> Vec<(u16, (f32, f32), f32)> {
        let layout = self.layout;
        let at = |space: u8| {
            let space = space_in(layout, space);
            (space.x, space.z)
        };
        let key = |kind: u16, index: usize| (kind << 8) | (index as u16 & 0xff);
        let mut found = Vec::new();
        found.extend(self.pedestrians.iter().filter(|npc|npc.cash>0).map(|npc|((PULL_PEDESTRIAN<<8)|npc.id as u16,npc.at(),1.1)));
        if self.ambient_earned<ambient::AMBIENT_TURN_CAP {
            for (index,coin) in self.air_coins.iter().enumerate().filter(|(index,_)|self.air_available(*index)) {
                found.push((key(PULL_AIR,index),(coin.view.x,coin.view.z),0.45));
            }
        }
        for &space in self.loose.keys() {
            found.push((key(PULL_LOOSE, space as usize), at(space), 0.35));
        }
        let me = self.tokens.iter().find(|t| t.seat == seat);
        let hands_free = me.is_some_and(|t| !t.carrying && !t.snowball);
        if hands_free {
            for &space in self.crates.keys() {
                found.push((key(PULL_CRATE, space as usize), self.crate_spot(space), 0.5));
            }
            for (index, ready) in self.snowmen.iter().enumerate() {
                if *ready {
                    found.push((key(PULL_SNOWMAN, index), self.map.specials().snowmen[index], 1.5));
                }
            }
        }
        let price = self.prize_cost(seat);
        if me.is_some_and(|t| t.coins >= price) {
            let (ox, oz) = Self::chest_drift(self.vacuum.out);
            for space in self.open_prizes() {
                let (x, z) = at(space);
                found.push((key(PULL_CHEST, space as usize), (x + ox, z + oz), 1.4));
            }
        }
        // Each purple space's box, once a round.
        for space in layout.iter().filter(|space| space.kind == SpaceKind::Item && !self.boxes_taken.contains(&space.id)) {
            found.push((key(PULL_BOX, space.id as usize), self.box_spot(space.id), 1.6));
        }
        if let Some(storm) = self.storm {
            found.push((key(PULL_STORM, 0), at(storm), 1.4));
        }
        for (index, coconuts) in self.coconuts.iter().enumerate() {
            if *coconuts > 0 {
                found.push((key(PULL_PALM, index), self.map.specials().palms[index], 1.2));
            }
        }
        for &space in &self.lava_rocks {
            found.push((key(PULL_LAVA, space as usize), at(space), 0.8));
        }
        for &index in self.flotsam.keys() {
            found.push((key(PULL_FLOTSAM, index as usize), self.map.specials().flotsam[index as usize], 1.0));
        }
        found
    }

    /// One tick of the vacuum: whatever lies in the cone in front is drawn
    /// in, the nearer the faster, and does what it does once it arrives:
    /// coins, a crate or a snowball to throw, the chest, an item, the storm
    /// dragged a space nearer. Rivals in the cone are drained of coins.
    /// Returns whether anything changed that the room should show.
    pub fn vacuum_tick(&mut self, dt: f32) -> bool {
        if !matches!(self.step, TurnStep::Vacuum { .. }) {
            return false;
        }
        let seat = self.turn_seat();
        self.vacuum.out += dt;
        if self.vacuum.out >= self.battery(seat) * 3.0 && !self.endless_vacuum {
            // Out too long without being used up: put away.
            self.end_vacuum(seat);
            return true;
        }
        if !self.vacuum.on {
            self.vacuum.power = (self.vacuum.power - dt * 3.0).max(0.0);
            // Let go, whatever was coming drifts back.
            let before = self.vacuum.pulling.len();
            self.vacuum.pulling.retain(|_, pull| {
                *pull -= dt * 1.5;
                *pull > 0.0
            });
            return before != self.vacuum.pulling.len();
        }
        if !self.endless_vacuum {
            self.vacuum.left -= dt;
        }
        self.vacuum.power = (self.vacuum.power + dt / self.spin_up(seat)).min(1.0);
        let power = self.vacuum.power;
        if self.vacuum.blow {
            let changed = self.blow_tick(seat, dt, power);
            if self.vacuum.left <= 0.0 {
                self.end_vacuum(seat);
                return true;
            }
            return changed;
        }
        let (changed,finished)=self.suction_tick(seat,dt,power,false);
        if finished || self.vacuum.left<=0.0 {
            self.end_vacuum(seat);
            return true;
        }
        changed
    }

    /// The same targets, cone, motor strength and upgrade rate for standing
    /// and walking. Airborne walking catches keep their separate three cap.
    fn suction_tick(&mut self,seat:u8,dt:f32,power:f32,skip_air:bool)->(bool,bool) {
        self.crawler_pressure(seat,dt,power,false);
        self.pedestrian_pressure(seat,dt,power);
        let strength = self.reach_of(seat);
        let rate=self.pull_rate(seat);
        let mut changed = false;

        // Rivals in the cone are drained of a coin every so often.
        let rivals: Vec<u8> = self
            .tokens
            .iter()
            .filter(|t| self.rival(seat, t.seat) && t.coins > 0 && !t.has(Upgrade::Filter))
            .filter(|t| {
                strength(self.rendered_spot_of(t.seat), 1.0).is_some()
            })
            .map(|t| t.seat)
            .collect();
        self.vacuum.siphon.retain(|rival, _| rivals.contains(rival));
        for rival in rivals {
            if self.vacuum.shielded.contains(&rival) {
                continue;
            }
            let pull = strength(self.rendered_spot_of(rival), 1.0).unwrap_or(0.0);
            let hold = if self.vacuum.braced.contains(&rival) { BRACED } else { 1.0 };
            let drawn = self.vacuum.siphon.entry(rival).or_insert(0.0);
            *drawn += dt * power * pull * hold * rate;
            if *drawn >= SIPHON_EVERY {
                *drawn -= SIPHON_EVERY;
                changed = true;
                if self.blocked(seat, rival, Item::Bat) {
                    self.vacuum.shielded.push(rival);
                    self.vacuum.siphon.remove(&rival);
                    continue;
                }
                self.coins(rival, -1);
                self.coins(seat, 1);
                self.say(json!({ "kind": "siphon", "seat": seat, "target": rival, "coins": 1 }));
            }
        }

        let mut caught = Vec::new();
        // Every target uses the complete cone shown to the player.
        let targets: Vec<(u16, f32, f32)> = self
            .pullables(seat)
            .into_iter()
            .filter(|(key,..)| !skip_air || *key>>8!=PULL_AIR)
            .filter_map(|(key, at, slow)| {
                strength(at,1.0).map(|pull| (key, pull, slow))
            })
            .collect();
        let inside: Vec<u16> = targets.iter().map(|(key, ..)| *key).collect();
        for (key, pull_here, slow) in targets {
            let pull = self.vacuum.pulling.entry(key).or_insert(0.0);
            *pull += dt * power * pull_here * rate * 1.3 / slow;
            if *pull >= 1.0 {
                caught.push(key);
            }
        }
        // Out of the cone, it settles back where it was.
        self.vacuum.pulling.retain(|key, pull| {
            if inside.contains(key) {
                return true;
            }
            *pull -= dt * 1.5;
            *pull > 0.0
        });
        for key in caught {
            self.vacuum.pulling.remove(&key);
            changed = true;
            if self.collect(seat, key) {
                return (true,true);
            }
        }
        (changed,false)
    }

    /// One tick of blowing: the storm, the chest and loose coins in the cone
    /// are pushed off once enough has built up on them, and rivals in it are
    /// knocked a space back along the path, each once.
    fn blow_tick(&mut self, seat: u8, dt: f32, power: f32) -> bool {
        self.crawler_pressure(seat,dt,power,true);
        let strength = self.reach_of(seat);
        let rate=self.pull_rate(seat);
        let layout = self.layout;
        let mut changed = false;

        let rivals: Vec<(u8, f32)> = self
            .tokens
            .iter()
            .filter(|t| self.rival(seat, t.seat) && !self.vacuum.knocked.contains(&t.seat))
            .filter_map(|t| {
                strength(self.rendered_spot_of(t.seat), 1.0).map(|push| (t.seat, push))
            })
            .collect();
        self.vacuum.gust.retain(|rival, _| rivals.iter().any(|(seat, _)| seat == rival));
        for (rival, push) in rivals {
            let gust = self.vacuum.gust.entry(rival).or_insert(0.0);
            *gust += dt * power * push * rate;
            if *gust < KNOCK_AFTER {
                continue;
            }
            self.vacuum.gust.remove(&rival);
            self.vacuum.knocked.insert(rival);
            changed = true;
            if self.blocked(seat, rival, Item::Bat) {
                continue;
            }
            let from = self.space_of(rival);
            let to = self.walk_back(from, 1);
            if let Some(token) = self.token(rival) {
                token.previous = None;
                token.space = to;
            }
            self.say(json!({ "kind": "blown_back", "seat": seat, "target": rival, "from": from, "to": to }));
        }

        let mut found: Vec<(u16, (f32, f32), f32, f32)> = Vec::new();
        let at = |space: u8| {
            let space = space_in(layout, space);
            (space.x, space.z)
        };
        let key = |kind: u16, index: u8| (kind << 8) | index as u16;
        for &space in self.loose.keys() {
            found.push((key(PULL_LOOSE, space), at(space), 0.35, 1.0));
        }
        if let Some(storm) = self.storm {
            found.push((key(PULL_STORM, 0), at(storm), 1.4, 1.0));
        }
        if !self.vacuum.chest_blown && self.event != Some(RoundEvent::LastRound) {
            let (x, z) = at(self.prize_at);
            let (ox, oz) = Self::chest_drift(self.vacuum.out);
            found.push((key(PULL_CHEST, self.prize_at), (x + ox, z + oz), 1.4, 1.0));
        }
        let targets: Vec<(u16, f32, f32)> = found
            .into_iter()
            .filter_map(|(key, at, slow, narrow)| strength(at, narrow).map(|push| (key, push, slow)))
            .collect();
        let inside: Vec<u16> = targets.iter().map(|(key, ..)| *key).collect();
        let mut gone = Vec::new();
        for (key, push, slow) in targets {
            let progress = self.vacuum.pulling.entry(key).or_insert(0.0);
            *progress += dt * power * push * rate * 1.3 / slow;
            if *progress >= 1.0 {
                gone.push(key);
            }
        }
        self.vacuum.pulling.retain(|key, progress| {
            if inside.contains(key) {
                return true;
            }
            *progress -= dt * 1.5;
            *progress > 0.0
        });
        for key in gone {
            self.vacuum.pulling.remove(&key);
            changed = true;
            self.blow_off(seat, key);
        }
        changed
    }

    /// The neighbouring space, ahead or behind, further from `from`.
    fn away_from(&self, at: u8, from: u8) -> u8 {
        let ahead = space_in(self.layout, at).next.first().copied().unwrap_or(at);
        let behind = self.walk_back(at, 1);
        if gap_in(self.layout, ahead, from) >= gap_in(self.layout, behind, from) { ahead } else { behind }
    }

    fn vacuum_space_of(&self,seat:u8)->u8 {
        self.vacuum.origin.and_then(|origin|self.layout.iter().min_by(|a,b|(a.x-origin.0).hypot(a.z-origin.1).total_cmp(&(b.x-origin.0).hypot(b.z-origin.1))).map(|space|space.id)).unwrap_or_else(||self.space_of(seat))
    }

    fn vacuum_storm_caught(&mut self,holder:u8,space:u8)->Vec<u8> {
        let mut caught:Vec<u8>=self.tokens.iter().filter(|token|token.seat!=holder && token.space==space).map(|token|token.seat).collect();
        let at=&self.layout[space as usize];
        let holder_here=self.vacuum.origin.map(|origin|(origin.0-at.x).hypot(origin.1-at.z)<=1.5).unwrap_or_else(||self.space_of(holder)==space);
        if holder_here {
            if self.vacuum.origin.is_some() {if let Some(token)=self.token(holder){token.space=space;token.previous=None;}}
            caught.push(holder);
        }
        caught
    }

    /// What blowing something off does.
    fn blow_off(&mut self, seat: u8, key: u16) {
        let (kind, index) = (key >> 8, (key & 0xff) as u8);
        let mine = self.vacuum_space_of(seat);
        match kind {
            PULL_LOOSE => {
                if let Some(coins) = self.loose.remove(&index) {
                    self.scatter(index, coins, 7.0, Some(seat));
                }
            }
            PULL_STORM => {
                let Some(storm) = self.storm else { return };
                let to = self.away_from(storm, mine);
                self.storm = Some(to);
                self.say(json!({ "kind": "storm_blown", "seat": seat, "from": storm, "to": to }));
                let caught = self.vacuum_storm_caught(seat,to);
                for one in caught {
                    self.blown_away(one);
                }
            }
            PULL_CHEST => {
                self.vacuum.chest_blown = true;
                let from = self.prize_at;
                let here = self.distance_now(mine, from);
                let further = self
                    .reachable_prize_spots()
                    .into_iter()
                    .filter(|spot| *spot != from)
                    .map(|spot| (self.distance_now(mine, spot), spot))
                    .filter(|(away, _)| *away > here)
                    .min();
                if let Some((_, to)) = further {
                    self.prize_at = to;
                    self.say(json!({ "kind": "chest_blown", "seat": seat, "from": from, "to": to }));
                }
            }
            _ => {}
        }
    }

    /// What something vacuumed in does. Returns true when it ends the vacuum,
    /// as winning the chest does.
    fn collect(&mut self, seat: u8, key: u16) -> bool {
        let (kind, index) = (key >> 8, (key & 0xff) as u8);
        match kind {
            PULL_PEDESTRIAN=>{self.rob_pedestrian(seat,index);}
            PULL_AIR=>{
                let coins=self.take_air_coin(seat,index as usize);
                if coins>0 {
                    let coin=&self.air_coins[index as usize].view;
                    self.say(json!({"kind":"vacuumed","what":"air_coin","seat":seat,"coin_id":coin.id,"x":coin.x,"z":coin.z,"coins":coins}));
                }
            }
            PULL_LOOSE => {
                if let Some(coins) = self.loose.remove(&index) {
                    self.coins(seat, coins);
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "coins", "space": index, "coins": coins }));
                }
            }
            PULL_CRATE => {
                if self.tokens.iter().any(|token|token.seat==seat && (token.carrying || token.snowball)) {return false;}
                if let Some(hidden) = self.crates.remove(&index) {
                    self.coins(seat, hidden);
                    if let Some(token) = self.token(seat) {
                        token.carrying = true;
                    }
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "crate", "space": index, "coins": hidden }));
                }
            }
            PULL_CHEST => {
                if !self.open_prizes().contains(&index) || !self.tokens.iter().any(|token|token.seat==seat && token.coins>=self.prize_cost(seat)) {return false;}
                self.claim_prize(seat, index);
                return true;
            }
            PULL_BOX => {
                if self.boxes_taken.contains(&index) {return false;}
                self.boxes_taken.insert(index);
                let item = self.draw_item();
                let bag = self.tokens.iter().find(|t| t.seat == seat).map(|t| t.items.len()).unwrap_or(MAX_ITEMS);
                if bag < MAX_ITEMS {
                    if let Some(token) = self.token(seat) {
                        token.items.push(item);
                    }
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "box", "space": index, "item": item }));
                } else {
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "box", "space": index, "item": null }));
                }
            }
            PULL_STORM => {
                // A space nearer, along whichever way round is shorter.
                let Some(storm) = self.storm else { return false };
                let mine = self.vacuum_space_of(seat);
                let ahead = space_in(self.layout, storm).next.first().copied().unwrap_or(storm);
                let behind = self.walk_back(storm, 1);
                let to = if gap_in(self.layout, ahead, mine) <= gap_in(self.layout, behind, mine) { ahead } else { behind };
                self.storm = Some(to);
                self.say(json!({ "kind": "storm_dragged", "seat": seat, "from": storm, "to": to }));
                let caught = self.vacuum_storm_caught(seat,to);
                let caught_holder=caught.contains(&seat);
                for one in caught {
                    self.blown_away(one);
                }
                if caught_holder{return true;}
            }
            PULL_PALM => {
                let index = index as usize;
                if self.coconuts.get(index).copied().unwrap_or(0) > 0 {
                    self.coconuts[index] -= 1;
                    // Now and then a golden one, with an item in it.
                    let golden = self.rng.random_bool(0.2);
                    let item = golden.then(|| self.draw_item());
                    let room = self.tokens.iter().find(|t| t.seat == seat).is_some_and(|t| t.items.len() < MAX_ITEMS);
                    match item {
                        Some(item) if room => {
                            if let Some(token) = self.token(seat) {
                                token.items.push(item);
                            }
                        }
                        _ => self.coins(seat, COCONUT_COINS),
                    }
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "coconut", "palm": index, "coins": if item.is_some() && room { 0 } else { COCONUT_COINS }, "item": if room { item } else { None } }));
                }
            }
            PULL_LAVA => {
                if self.lava_rocks.remove(&index) {
                    self.coins(seat, OBSIDIAN_COINS);
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "lava", "space": index, "coins": OBSIDIAN_COINS }));
                }
            }
            PULL_FLOTSAM => {
                if let Some(coins) = self.flotsam.remove(&index) {
                    let room = self.tokens.iter().find(|t| t.seat == seat).is_some_and(|t| t.items.len() < MAX_ITEMS);
                    if coins == 0 && room {
                        let item = self.draw_item();
                        if let Some(token) = self.token(seat) {
                            token.items.push(item);
                        }
                        self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "flotsam", "point": index, "item": item, "coins": 0 }));
                    } else {
                        let coins = coins.max(3);
                        self.coins(seat, coins);
                        self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "flotsam", "point": index, "coins": coins }));
                    }
                }
            }
            PULL_SNOWMAN => {
                if self.tokens.iter().any(|token|token.seat==seat && (token.carrying || token.snowball)) {return false;}
                let index = index as usize;
                if self.snowmen.get(index).copied().unwrap_or(false) {
                    self.snowmen[index] = false;
                    if let Some(token) = self.token(seat) {
                        token.snowball = true;
                    }
                    self.say(json!({ "kind": "vacuumed", "seat": seat, "what": "snowball", "snowman": index }));
                }
            }
            _ => {}
        }
        false
    }

    /// Each round: palms grow their coconuts back, the crater spits lava
    /// rocks, flotsam washes into the bay, and the snowman is built again.
    fn restock(&mut self) {
        self.restock_candy_van();
        self.restock_pedestrians();
        self.refill_air_coins();
        let specials = self.map.specials();
        self.coconuts = vec![COCONUTS; specials.palms.len()];
        self.snowmen = vec![true; specials.snowmen.len()];
        self.boxes_taken.clear();
        if self.map == Map::Volcano {
            let open: Vec<u8> = self
                .layout
                .iter()
                .filter(|space| !space.under && !matches!(space.kind, SpaceKind::Start | SpaceKind::Shop | SpaceKind::PrizeSpot))
                .map(|space| space.id)
                .collect();
            let mut landed = Vec::new();
            for _ in 0..LAVA_ROCKS {
                let at = open[self.rng.random_range(0..open.len())];
                if self.lava_rocks.insert(at) {
                    landed.push(at);
                }
            }
            if !landed.is_empty() {
                self.say(json!({ "kind": "lava_rocks", "spaces": landed }));
            }
        }
        if !specials.flotsam.is_empty() {
            let mut washed = Vec::new();
            for _ in 0..FLOTSAM {
                let index = self.rng.random_range(0..specials.flotsam.len()) as u8;
                if self.flotsam.contains_key(&index) {
                    continue;
                }
                // Coins, or now and then nothing but an item.
                let coins = if self.rng.random_bool(0.25) { 0 } else { self.rng.random_range(3..=6) };
                self.flotsam.insert(index, coins);
                washed.push(index);
            }
            if !washed.is_empty() {
                self.say(json!({ "kind": "flotsam", "points": washed }));
            }
        }
    }

    /// Throw the snowball you are holding; whoever it hits is frozen for a turn.
    pub fn throw_snowball(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if self.step != TurnStep::Items {
            return Err(BoardError::WrongStep);
        }
        let token = self.token(seat).ok_or(BoardError::UnknownTarget)?;
        if !token.snowball {
            return Err(BoardError::NoSuchItem);
        }
        if self.timed_items {
            self.begin_position(seat, Item::Snowball);
            return Ok(());
        }
        let token = self.token(seat).ok_or(BoardError::UnknownTarget)?;
        token.snowball = false;
        let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
        let strength = self.rng.random_range(0.0..BOMB_WIND);
        self.say(json!({ "kind": "item", "item": "throw_ready", "thrown": Item::Snowball, "by": seat }));
        self.step = TurnStep::Lob { item: Item::Snowball, wind: (angle.sin() * strength, angle.cos() * strength), fuse: BOMB_FUSE };
        Ok(())
    }

    /// Throw the crate you are holding; it becomes a barricade where it lands.
    pub fn throw_crate(&mut self, seat: u8) -> Result<(), BoardError> {
        if seat != self.turn_seat() {
            return Err(BoardError::NotYourTurn);
        }
        if self.step != TurnStep::Items {
            return Err(BoardError::WrongStep);
        }
        let token = self.token(seat).ok_or(BoardError::UnknownTarget)?;
        if !token.carrying {
            return Err(BoardError::NoSuchItem);
        }
        if self.timed_items {
            self.begin_position(seat, Item::Crate);
            return Ok(());
        }
        let token = self.token(seat).ok_or(BoardError::UnknownTarget)?;
        token.carrying = false;
        let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
        let strength = self.rng.random_range(0.0..BOMB_WIND);
        self.say(json!({ "kind": "item", "item": "throw_ready", "thrown": Item::Crate, "by": seat }));
        self.step = TurnStep::Lob { item: Item::Crate, wind: (angle.sin() * strength, angle.cos() * strength), fuse: BOMB_FUSE };
        Ok(())
    }

    fn spill(&mut self, seat: u8, coins: i32) {
        if coins <= 0 {
            return;
        }
        let at = self.space_of(seat);
        let mut spots = vec![at];
        let mut walk = at;
        for _ in 0..2 {
            walk = space_in(self.layout, walk).next.first().copied().unwrap_or(walk);
            if !spots.contains(&walk) {
                spots.push(walk);
            }
        }
        let each = (coins / spots.len() as i32).max(1);
        let mut left = coins;
        let mut dropped = Vec::new();
        let last = spots.len() - 1;
        for (index, spot) in spots.into_iter().enumerate() {
            if left <= 0 {
                break;
            }
            // The last pile takes the remainder, so none of it is lost.
            let amount = if index == last { left } else { each.min(left) };
            left -= amount;
            *self.loose.entry(spot).or_insert(0) += amount;
            dropped.push(json!({ "space": spot, "coins": amount }));
        }
        self.say(json!({ "kind": "spill", "seat": seat, "dropped": dropped }));
    }

    fn land(&mut self, seat: u8) {
        self.land_on(seat, true);
    }

    /// Resolve the space a token stopped on. `slide` is false once ice has
    /// already carried it a space, so a second patch of ice holds it.
    fn land_on(&mut self, seat: u8, slide: bool) {
        let at = self
            .tokens
            .iter()
            .find(|t| t.seat == seat)
            .map(|t| t.space)
            .unwrap_or(0);
        // A lava rock lying here burns whoever stops on it.
        if self.lava_rocks.contains(&at) {
            self.coins(seat, -LAVA_COINS);
            self.say(json!({ "kind": "lava_burn", "seat": seat, "space": at, "coins": LAVA_COINS }));
            self.hurt(seat, 1);
        }
        // Being picked up by the storm comes before whatever the space does.
        if self.storm == Some(at) {
            self.blown_away(seat);
            self.step = TurnStep::Done;
            return;
        }
        match space_in(self.layout, at).kind {
            SpaceKind::Ice if slide => {
                // A slide never stops to ask at a junction; it carries straight on.
                let to = self.walk_ways(seat, at).first().copied().unwrap_or(at);
                if let Some(token) = self.token(seat) {
                    token.previous = Some(at);
                    token.space = to;
                }
                self.say_path(seat, vec![to]);
                self.say(json!({ "kind": "slide", "seat": seat, "from": at, "to": to }));
                if self.timed_walk && to != at {
                    self.start_walk(seat, at, vec![to], false, walking::AfterWalk::Land { slide: false });
                } else { self.land_on(seat, false); }
                return;
            }
            SpaceKind::Ice => {}
            SpaceKind::Shop => {
                let room_in_bag = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == seat)
                    .map(|t| t.items.len() < MAX_ITEMS)
                    .unwrap_or(false);
                let offers = if room_in_bag { self.stock() } else { Vec::new() };
                let upgrades = self.upgrade_stock(seat);
                if offers.is_empty() && upgrades.is_empty() {
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "shop_closed" }));
                } else {
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "shop", "offers": offers.clone(), "upgrades": upgrades.clone() }));
                    self.step = TurnStep::Shop { offers, upgrades };
                    return;
                }
            }
            SpaceKind::Cannon => {
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "cannon", "max": CANNON_RANGE }));
                self.step = TurnStep::Cannon { max: CANNON_RANGE };
                return;
            }
            SpaceKind::Hazard => {
                self.coins(seat, -HAZARD_COINS);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "hazard", "coins": HAZARD_COINS }));
                self.hurt(seat, 1);
            }
            SpaceKind::Teleport => {
                let to = space_in(self.layout, at).link.unwrap_or(at);
                if let Some(token) = self.token(seat) {
                    token.previous = None;
                    token.space = to;
                }
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "teleport", "to": to }));
            }
            SpaceKind::PrizeSpot => {
                let here = self.open_prizes().contains(&at);
                let price = self.prize_cost(seat);
                let rich = self
                    .tokens
                    .iter()
                    .find(|t| t.seat == seat)
                    .map(|t| t.coins >= price)
                    .unwrap_or(false);
                if here && rich {
                    // The chest hangs out of reach: it has to be vacuumed in.
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "prize_here", "coins": price }));
                } else if here {
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "prize_too_dear", "coins": price }));
                } else {
                    let pay = self.coin_pay();
                    self.coins(seat, pay);
                    self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "coins", "coins": pay }));
                }
            }
            SpaceKind::Coin | SpaceKind::Start => {}
            SpaceKind::Loss => {
                let loss = LOSS_SPACE * if self.event == Some(RoundEvent::LastRound) { 2 } else { 1 };
                self.coins(seat, -loss);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "loss", "coins": loss }));
            }
            SpaceKind::Tide if self.tide_in() => {
                self.coins(seat, -LOSS_SPACE);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "soaked", "coins": LOSS_SPACE }));
            }
            SpaceKind::Tide => {
                self.coins(seat, TIDE_FIND);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "washed_up", "coins": TIDE_FIND }));
            }
            SpaceKind::Item => {
                let item = self.draw_item();
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
                    .filter(|t| self.rival(seat, t.seat))
                    .map(|t| t.seat)
                    .collect();
                let mut taken = 0;
                let mut paid = Vec::new();
                for other in others {
                    let amount = self
                        .tokens
                        .iter()
                        .find(|t| t.seat == other)
                        .map(|t| t.coins.min(2))
                        .unwrap_or(0);
                    self.coins(other, -amount);
                    taken += amount;
                    if amount > 0 {
                        paid.push(json!({ "seat": other, "coins": amount }));
                    }
                }
                self.coins(seat, taken);
                self.say(json!({ "kind": "space", "space": at, "seat": seat, "effect": "toll", "coins": taken, "paid": paid }));
            }
        }

        self.after_landing(seat);
    }

    /// Everything that happens wherever you stopped, after the space itself:
    /// whoever is standing there, anything lying on it, and any trap set on
    /// it. The shop waits for an answer first, so it comes back here after.
    fn after_landing(&mut self, seat: u8) {
        let at = self.space_of(seat);
        let near: Vec<u8> = self
            .traps
            .keys()
            .copied()
            .filter(|space| gap_in(self.layout, at, *space) <= TRAP_RUMOUR || gap_in(self.layout, *space, at) <= TRAP_RUMOUR)
            .collect();
        for space in near {
            let heard = self.rumours.entry(space).or_default();
            if !heard.contains(&seat) {
                heard.push(seat);
            }
        }
        self.scrap(seat, at);
        // The plague passes to whoever the sick one lands beside.
        if self.plague == Some(seat) {
            let near: Vec<u8> = self.tokens.iter().filter(|t| t.seat != seat && t.space == at).map(|t| t.seat).collect();
            if !near.is_empty() {
                let caught = near[self.rng.random_range(0..near.len())];
                self.plague = Some(caught);
                self.say(json!({ "kind": "plague", "seat": caught, "from": seat }));
            }
        }

        if let Some(found) = self.loose.remove(&at) {
            self.coins(seat, found);
            self.say(json!({ "kind": "pickup", "seat": seat, "space": at, "coins": found }));
        }

        if let Some(owner) = self.traps.remove(&at) {
            self.rumours.remove(&at);
            if owner != seat && self.blocked(owner, seat, Item::Trap) {
            } else if owner != seat {
                self.coins(seat, -TRAP_COINS);
                self.coins(owner, TRAP_COINS);
                self.say(
                    json!({ "kind": "trap", "seat": seat, "owner": owner, "coins": TRAP_COINS }),
                );
                self.crossed(seat, owner);
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
        if self.walk.is_some() || self.positioning.is_some() { return false; }
        let walked = std::mem::take(&mut self.walked);
        let seat = self.turn_seat();
        if let Some(token) = self.token(seat) {
            if walked > token.stats.longest_walk {
                token.stats.longest_walk = walked;
            }
        }
        self.last_roll = None;
        self.dice = 1;
        self.fixed_roll = None;
        self.sucked = false;
        self.ambient_earned=0;
        self.turn_index += 1;
        self.step = TurnStep::Items;
        let round_over = self.turn_index % self.order.len() == 0;
        if round_over {self.clear_raids("round");}
        if !round_over {
            self.skip_resting();
            let next = self.turn_seat();
            self.end_raid(next,"turn");
            if self.plague == Some(next) {
                self.say(json!({ "kind": "plague_bite", "seat": next }));
                self.hurt(next, 1);
            }
        }
        round_over
    }

    /// Players knocked out last round sit this one out.
    fn skip_resting(&mut self) {
        for _ in 0..self.order.len() {
            let seat = self.turn_seat();
            self.crawler_turn_arrived(seat);
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

    /// Spread new players across distinct, safe surface spaces. Placement is
    /// not a landing: it grants no pickup, trap, shop or candy reward.
    pub fn scatter_starts(&mut self) {
        let mut candidates:Vec<u8>=self.layout.iter().filter(|s|!s.gated&&!s.under&&s.link.is_none()&&matches!(s.kind,SpaceKind::Coin|SpaceKind::Start)).map(|s|s.id).collect();
        candidates.shuffle(&mut self.rng);
        let mut chosen:Vec<u8>=Vec::new();
        for token in &mut self.tokens {
            let index=candidates.iter().position(|id|chosen.iter().all(|other|{
                let a=&self.layout[*id as usize];let b=&self.layout[*other as usize];(a.x-b.x).hypot(a.z-b.z)>=10.
            })).unwrap_or(0);
            if candidates.is_empty(){break;}
            token.space=candidates.remove(index);token.previous=None;token.facing=None;chosen.push(token.space);
        }
        self.place_crawler_far_from_players();
        self.say(json!({"kind":"starting_positions","positions":self.tokens.iter().map(|t|(t.seat,t.space)).collect::<Vec<_>>()}));
    }

    pub fn prepare_opening(&mut self) {
        self.scatter_starts();
        for token in &mut self.tokens {token.coins=0;}
    }

    pub fn minigame_rewards(&self,placements:&[Vec<u8>],opening:bool)->Vec<MinigameReward> {
        let count=self.tokens.len();let mut seen=HashSet::new();let mut rewards=Vec::new();let mut place=0;
        for group in placements {
            for &seat in group {
                if !self.order.contains(&seat)||!seen.insert(seat){continue;}
                rewards.push(MinigameReward{seat,place:place+1,coins:if opening {25-(15*place/count.saturating_sub(1).max(1)) as i32}else if place==0{MINIGAME_COINS[0]}else{0},tier_one_chance:tier_one_chance(place,count)});
            }
            place+=group.len();
        }
        for &seat in &self.order {if seen.insert(seat){rewards.push(MinigameReward{seat,place:count,coins:if opening{10}else{0},tier_one_chance:0.2});}}
        rewards
    }

    fn pay_minigame(&mut self,placements:&[Vec<u8>],opening:bool) {
        for reward in self.minigame_rewards(placements,opening) {
            self.coins(reward.seat,reward.coins);
            self.say(json!({"kind":"minigame","seat":reward.seat,"place":reward.place,"coins":reward.coins,"opening":opening}));
            self.award_ranked_item(reward.seat,reward.tier_one_chance);
        }
        if let Some(winners)=placements.first(){for &seat in winners {if let Some(token)=self.token(seat){token.stats.bouts_won+=1;}}}
    }

    /// The opener funds the first board round and decides order without
    /// consuming a round or restocking/changing the random starting spaces.
    pub fn finish_opening(&mut self,placements:&[Vec<u8>]) {
        self.pay_minigame(placements,true);self.reorder(placements);
        self.turn_index=0;self.step=TurnStep::Items;
        self.assign_crawler_controller(placements.last().and_then(|group|group.last()).copied());
    }

    /// Pay out a minigame and start the next round. Returns false when the
    /// party is over.
    pub fn finish_round(&mut self, placements: &[Vec<u8>]) -> bool {
        self.finale=None;
        self.pay_minigame(placements,false);
        self.settle_bets(placements.first().map(|group|group.as_slice()).unwrap_or(&[]));
        self.reorder(placements);
        if self.round >= self.rounds {
            self.collect_highlights();
            self.award_bonuses();
            return false;
        }
        self.round += 1;
        self.turn_index = 0;
        self.step = TurnStep::Items;
        self.say(json!({ "kind": "round", "round": self.round }));
        self.shake_up();
        self.skip_resting();
        self.assign_crawler_controller(placements.last().and_then(|group|group.last()).copied());
        true
    }

    /// The moments worth remembering: one winner each, and nothing for a
    /// category where nothing happened.
    fn collect_highlights(&mut self) {
        let mut out: Vec<HighlightAward> = Vec::new();
        let best = |pick: fn(&Stats) -> i32, tokens: &[Token]| -> Option<(u8, i32)> {
            tokens
                .iter()
                .map(|t| (t.seat, pick(&t.stats)))
                .filter(|(_, value)| *value > 0)
                .max_by_key(|(_, value)| *value)
        };
        if let Some((seat, value)) = best(|s| s.best_swing, &self.tokens) {
            out.push(HighlightAward { highlight: Highlight::BiggestSwing, seat, value });
        }
        if let Some((seat, value)) = best(|s| s.longest_walk as i32, &self.tokens) {
            out.push(HighlightAward { highlight: Highlight::LongestWalk, seat, value });
        }
        if let Some((seat, value)) = best(|s| s.knockouts as i32, &self.tokens) {
            out.push(HighlightAward { highlight: Highlight::MostKnockouts, seat, value });
        }
        // Worst luck, as an average roll in tenths, among anyone who rolled.
        let unlucky = self
            .tokens
            .iter()
            .filter(|t| t.stats.rolls > 0)
            .min_by_key(|t| (t.stats.pips * 10 / t.stats.rolls.max(1)) as i32);
        if let Some(token) = unlucky {
            out.push(HighlightAward {
                highlight: Highlight::WorstDice,
                seat: token.seat,
                value: (token.stats.pips * 10 / token.stats.rolls.max(1)) as i32,
            });
        }
        self.highlights = out;
    }

    fn award_bonuses(&mut self) {
        for bonus in Bonus::ALL {
            let best = self
                .tokens
                .iter()
                .map(|t| bonus.count(&t.stats))
                .max()
                .unwrap_or(0);
            if best == 0 {
                continue;
            }
            let seats: Vec<u8> = self
                .tokens
                .iter()
                .filter(|t| bonus.count(&t.stats) == best)
                .map(|t| t.seat)
                .collect();
            for seat in &seats {
                if let Some(token) = self.token(*seat) {
                    token.prizes += 1;
                }
            }
            self.say(json!({ "kind": "bonus", "bonus": bonus, "seats": seats.clone(), "count": best }));
            self.bonuses.push(BonusAward {
                bonus,
                seats,
                count: best,
            });
        }
    }

    /// What the prize costs this player: less for whoever is playing catch-up,
    /// and half price for everyone in the last round.
    pub fn prize_cost(&self, seat: u8) -> i32 {
        let base = if self.event == Some(RoundEvent::LastRound) {
            PRIZE_COST / 2
        } else {
            PRIZE_COST
        };
        if self.underdog == Some(seat) {
            base.min(UNDERDOG_PRIZE)
        } else {
            base
        }
    }

    /// Where a prize can be bought right now. In the last round every spot on
    /// the island is open, so there is more than one way to score.
    pub fn open_prizes(&self) -> Vec<u8> {
        if self.event == Some(RoundEvent::LastRound) {
            prize_spots_in(self.layout)
        } else {
            vec![self.prize_at]
        }
    }

    /// Where the storm will be after the next round.
    fn storm_ahead(&self, from: u8) -> u8 {
        let mut at = from;
        for _ in 0..STORM_STEP {
            at = space_in(self.layout, at).next.first().copied().unwrap_or(at);
        }
        at
    }

    /// The storm rolls in on the second round and walks the lap from then on,
    /// catching anyone standing where it arrives.
    fn move_storm(&mut self) {
        let to = match self.storm {
            Some(at) => self.storm_ahead(at),
            None => {
                // It starts a little way around from the start, so nobody is
                // caught by it the moment it appears.
                let mut at = 0;
                for _ in 0..(STORM_STEP * 2) {
                    at = space_in(self.layout, at).next.first().copied().unwrap_or(at);
                }
                at
            }
        };
        self.storm = Some(to);
        let caught: Vec<u8> = self
            .tokens
            .iter()
            .filter(|t| t.space == to && t.resting == 0)
            .map(|t| t.seat)
            .collect();
        self.say(json!({ "kind": "storm", "space": to, "caught": caught.clone() }));
        for seat in caught {
            self.blown_away(seat);
        }
    }

    /// Picked up by the storm: a few coins gone and a few spaces back.
    fn blown_away(&mut self, seat: u8) {
        let from = self.space_of(seat);
        let to = self.walk_back(from, STORM_BACK);
        self.coins(seat, -STORM_COINS);
        if let Some(token) = self.token(seat) {
            token.previous = None;
            token.space = to;
        }
        self.say(json!({ "kind": "blown", "seat": seat, "from": from, "to": to, "coins": STORM_COINS }));
    }

    fn has_gates(&self) -> bool {
        self.layout.iter().any(|space| space.gated)
    }

    /// The round gated ways open in, on a board that has any.
    pub fn gates_open_from(&self) -> Option<u32> {
        self.has_gates()
            .then(|| (self.rounds + 1).saturating_sub(LATE_ROUNDS).max(1))
    }

    pub fn gates_open(&self) -> bool {
        self.gates_open_from().is_none_or(|round| self.round >= round)
    }

    /// Physical paths run both ways. Teleport links are not walkable roads.
    fn open_ways(&self, from: u8) -> Vec<u8> {
        let space = space_in(self.layout, from);
        let mut ways = if space.link.is_none() { space.next.clone() } else { Vec::new() };
        for incoming in self.layout.iter().filter(|s| s.link.is_none() && s.next.contains(&from)) {
            if !ways.contains(&incoming.id) { ways.push(incoming.id); }
        }
        ways.retain(|way| self.gates_open() || !self.layout[*way as usize].gated);
        ways
    }

    /// Keep the arrival edge across turns, so stopping at a junction never
    /// lets a player reverse and farm the same two spaces.
    fn walk_ways(&self, seat: u8, from: u8) -> Vec<u8> {
        let previous = self.tokens.iter().find(|t| t.seat == seat).and_then(|t| t.previous);
        let mut ways = self.open_ways(from);
        if let Some(previous) = previous {
            ways.retain(|way| *way != previous);
        } else {
            // A teleport or knockback has no arrival road: start in the
            // direction drawn for this route, then track each actual step.
            ways.retain(|way| self.layout[from as usize].next.contains(way));
        }
        ways
    }

    /// How many steps from one space to another by the ways open right now.
    fn distance_now(&self, from: u8, to: u8) -> u8 {
        let mut frontier = vec![from];
        let mut seen = vec![from];
        for step in 0..self.layout.len() as u8 {
            if frontier.contains(&to) {
                return step;
            }
            let mut next = Vec::new();
            for id in frontier {
                let mut ways = self.open_ways(id);
                if let Some(link) = self.layout[id as usize].link { ways.push(link); }
                for candidate in ways {
                    if !seen.contains(&candidate) {
                        seen.push(candidate);
                        next.push(candidate);
                    }
                }
            }
            frontier = next;
        }
        u8::MAX
    }

    /// Prize spots the prize may move to: none behind a closed gate.
    fn reachable_prize_spots(&self) -> Vec<u8> {
        let open = self.gates_open();
        prize_spots_in(self.layout)
            .into_iter()
            .filter(|id| open || !self.layout[*id as usize].gated)
            .collect()
    }

    fn has_tide(&self) -> bool {
        self.layout.iter().any(|space| space.kind == SpaceKind::Tide)
    }

    /// The tide is out for the first round and turns every round after, so
    /// it is in on even rounds.
    pub fn tide_in(&self) -> bool {
        self.has_tide() && self.round % 2 == 0
    }

    /// Turns the tide at the start of a round. Coming in, it takes coins off
    /// anyone left standing on a tide space.
    fn turn_tide(&mut self) {
        if !self.has_tide() {
            return;
        }
        if !self.tide_in() {
            self.say(json!({ "kind": "tide", "flooded": false }));
            return;
        }
        let layout = self.layout;
        let caught: Vec<u8> = self
            .tokens
            .iter()
            .filter(|t| layout[t.space as usize].kind == SpaceKind::Tide)
            .map(|t| t.seat)
            .collect();
        for seat in &caught {
            self.coins(*seat, -TIDE_COINS);
        }
        self.say(json!({ "kind": "tide", "flooded": true, "caught": caught, "coins": TIDE_COINS }));
    }

    fn coin_pay(&self) -> i32 {
        match self.event {
            Some(RoundEvent::CoinShower | RoundEvent::LastRound) => COIN_SPACE * 2,
            _ => COIN_SPACE,
        }
    }

    /// The chest at a prize spot, bought by whoever vacuumed it in.
    fn claim_prize(&mut self, seat: u8, at: u8) {
        let price = self.prize_cost(seat);
        self.coins(seat, -price);
        if let Some(token) = self.token(seat) {
            token.prizes += 1;
        }
        self.move_prize();
        self.say(json!({ "kind": "prize", "seat": seat, "space": at, "coins": price, "moved_to": self.prize_at }));
    }

    /// Where a chest hangs while the vacuum is out: drifting about over its
    /// spot, so it has to be followed to be pulled in.
    pub fn chest_drift(out: f32) -> (f32, f32) {
        ((out * 1.1).sin() * 3.2, (out * 0.8).cos() * 2.2)
    }

    fn move_prize(&mut self) {
        let spots: Vec<u8> = self
            .reachable_prize_spots()
            .into_iter()
            .filter(|id| *id != self.prize_at)
            .collect();
        if !spots.is_empty() {
            let pick = self.rng.random_range(0..spots.len());
            self.prize_at = spots[pick];
        }
    }

    /// Restock the world and advance its physical hazards. Rounds never
    /// apply table-wide modifiers, including on the final round.
    fn shake_up(&mut self) {
        self.wash_up(ROUND_CRATES);
        self.restock();
        self.underdog = None;
        self.bounty = None;
        self.plague = None;
        self.event = None;
        self.move_storm();
        self.turn_tide();
    }

    fn run_event(&mut self, event: RoundEvent) {
        match event {
            RoundEvent::Bounty => {
                let leader = self.standings().first().map(|(seat, _)| *seat);
                self.bounty = leader;
                self.say(json!({ "kind": "event", "event": event, "seat": leader, "coins": BOUNTY_COINS }));
            }
            RoundEvent::Plague => {
                let seats: Vec<u8> = self.tokens.iter().map(|t| t.seat).collect();
                let sick = seats[self.rng.random_range(0..seats.len())];
                self.plague = Some(sick);
                self.say(json!({ "kind": "event", "event": event, "seat": sick }));
            }
            RoundEvent::Shakedown => {
                let standings = self.standings();
                let (Some((leader, _)), Some((last, _))) = (standings.first().copied(), standings.last().copied()) else {
                    return;
                };
                let purse = self.tokens.iter().find(|t| t.seat == leader).map(|t| t.coins).unwrap_or(0);
                let taken = if leader == last { 0 } else { (purse / 2).min(SHAKEDOWN_MAX) };
                self.coins(leader, -taken);
                self.coins(last, taken);
                self.say(json!({ "kind": "event", "event": event, "seat": last, "target": leader, "coins": taken }));
            }
            RoundEvent::CoinShower
            | RoundEvent::LastRound
            | RoundEvent::Fog
            | RoundEvent::TollRoad
            | RoundEvent::MarketDay
            | RoundEvent::Earthquake => {
                self.say(json!({ "kind": "event", "event": event }));
            }
            RoundEvent::Windfall => {
                let seats: Vec<u8> = self.tokens.iter().map(|t| t.seat).collect();
                for seat in seats {
                    self.coins(seat, WINDFALL_COINS);
                }
                self.say(json!({ "kind": "event", "event": event, "coins": WINDFALL_COINS }));
            }
            RoundEvent::ItemRain => {
                let mut given = Vec::new();
                for index in 0..self.tokens.len() {
                    if self.tokens[index].items.len() < MAX_ITEMS {
                        let item = self.draw_item();
                        self.tokens[index].items.push(item);
                        given.push(json!({ "seat": self.tokens[index].seat, "item": item }));
                    }
                }
                self.say(json!({ "kind": "event", "event": event, "given": given }));
            }
            RoundEvent::PrizeMoves => {
                let from = self.prize_at;
                self.move_prize();
                self.say(json!({ "kind": "event", "event": event, "from": from, "to": self.prize_at }));
            }
            RoundEvent::Underdog => {
                // Something to act with, rather than a handout: an item, and
                // the prize within reach for one round.
                let last = self.standings().last().map(|(seat, _)| *seat);
                let Some(last) = last else { return };
                self.underdog = Some(last);
                let item = self.draw_item();
                let given = match self.token(last) {
                    Some(token) if token.items.len() < MAX_ITEMS => {
                        token.items.push(item);
                        Some(item)
                    }
                    _ => None,
                };
                self.say(json!({ "kind": "event", "event": event, "seat": last, "item": given, "prize_cost": UNDERDOG_PRIZE }));
            }
        }
    }

    /// Plays in pairs: each pair's two seats are partners.
    pub fn set_partners(&mut self, pairs: &[(u8, u8)]) {
        self.partners.clear();
        for &(a, b) in pairs {
            self.partners.insert(a, b);
            self.partners.insert(b, a);
        }
    }

    pub fn partner_of(&self, seat: u8) -> Option<u8> {
        self.partners.get(&seat).copied()
    }

    /// Someone a seat can hurt, rob or drain: anyone but themselves and their partner.
    fn rival(&self, seat: u8, other: u8) -> bool {
        other != seat && self.partners.get(&seat) != Some(&other)
    }

    /// How a team bout splits the table: either pairs taken from the turn
    /// order, or whoever is winning against everyone else. Logged so the
    /// client can show the split before the bout starts. Played in pairs,
    /// partners always stand together, the pairs dealt out between the sides.
    pub fn team_split(&mut self, seats: &[u8]) -> Vec<(u8, u8)> {
        if !self.partners.is_empty() {
            let mut sides: Vec<(u8, u8)> = Vec::new();
            let mut next = 0u8;
            for &seat in seats {
                if sides.iter().any(|(s, _)| *s == seat) {
                    continue;
                }
                sides.push((seat, next));
                if let Some(partner) = self.partner_of(seat).filter(|p| seats.contains(p)) {
                    sides.push((partner, next));
                }
                next = 1 - next;
            }
            self.say(json!({
                "kind": "teams",
                "split": "partners",
                "teams": sides.iter().map(|(seat, team)| json!({ "seat": seat, "team": team })).collect::<Vec<_>>(),
            }));
            return sides;
        }
        let leader = self.standings().first().map(|(seat, _)| *seat);
        // One against the rest only makes sense with a table to gang up on,
        // and it is the harsher of the two, so it comes up less often.
        let gang_up = seats.len() >= 3 && leader.is_some_and(|l| seats.contains(&l)) && self.rng.random_bool(0.35);
        let split: Vec<(u8, u8)> = if gang_up {
            let leader = leader.unwrap();
            seats.iter().map(|seat| (*seat, u8::from(*seat == leader))).collect()
        } else {
            // Pairs from the turn order, so first and third play second and fourth.
            let order: Vec<u8> = self.order.iter().copied().filter(|seat| seats.contains(seat)).collect();
            seats
                .iter()
                .map(|seat| {
                    let place = order.iter().position(|s| s == seat).unwrap_or(0);
                    (*seat, (place % 2) as u8)
                })
                .collect()
        };
        self.say(json!({
            "kind": "teams",
            "split": if gang_up { "leader" } else { "pairs" },
            "teams": split.iter().map(|(seat, team)| json!({ "seat": seat, "team": team })).collect::<Vec<_>>(),
        }));
        split
    }

    /// Stake coins on yourself for the coming bout. Staking again replaces
    /// what you had on, and the coins are held until the bout is settled.
    pub fn bet(&mut self, seat: u8, coins: i32) -> Result<(), BoardError> {
        let held = self.bets.remove(&seat).unwrap_or(0);
        self.coins(seat, held);
        let purse = self
            .tokens
            .iter()
            .find(|t| t.seat == seat)
            .map(|t| t.coins)
            .ok_or(BoardError::UnknownTarget)?;
        let stake = coins.clamp(0, MAX_BET.min(purse));
        if stake > 0 {
            self.coins(seat, -stake);
            self.bets.insert(seat, stake);
        }
        self.say(json!({ "kind": "bet", "seat": seat, "coins": stake }));
        Ok(())
    }

    /// The pot goes to whoever staked and won. With no winning staker it
    /// goes back where it came from.
    fn settle_bets(&mut self, winners: &[u8]) {
        if self.bets.is_empty() {
            return;
        }
        let stakes: Vec<(u8, i32)> = self.bets.drain().collect();
        let pot: i32 = stakes.iter().map(|(_, coins)| *coins).sum();
        let paid: Vec<u8> = stakes
            .iter()
            .filter(|(seat, _)| winners.contains(seat))
            .map(|(seat, _)| *seat)
            .collect();
        if paid.is_empty() {
            for (seat, coins) in &stakes {
                self.coins(*seat, *coins);
            }
            self.say(json!({ "kind": "bets", "pot": pot, "winners": [], "each": 0 }));
            return;
        }
        let each = pot / paid.len() as i32;
        for seat in &paid {
            self.coins(*seat, each);
        }
        self.say(json!({ "kind": "bets", "pot": pot, "winners": paid, "each": each }));
    }

    /// Everyone draws; placement sets the chance of the premium pool. A
    /// full bag replaces its oldest item so nobody loses their earned draw.
    fn award_ranked_item(&mut self,seat:u8,chance:f32) {
        let premium=self.rng.random_bool(f64::from(chance));
        let choices:Vec<_>=Item::ALL.into_iter().filter(|item|item_tier_one(*item)==premium&&(*item!=Item::DriveCar||self.map==Map::Frozen)).collect();
        let item=choices[self.rng.random_range(0..choices.len())];
        let Some(token)=self.token(seat)else{return;};
        let replaced=if token.items.len()>=MAX_ITEMS {Some(token.items.remove(0))}else{None};
        token.items.push(item);
        self.say(json!({"kind":"award","seat":seat,"item":item,"tier":if premium{1}else{2},"tier_one_chance":chance,"replaced":replaced}));
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
            token.previous = None;
            token.space = space;
            token.coins = coins;
        }
    }

    // The sandbox's tools, on a dev stack only; see `Room::dev`.

    /// Starts the turn order with this seat.
    pub fn dev_first(&mut self, seat: u8) {
        if let Some(at) = self.order.iter().position(|s| *s == seat) {
            self.order.rotate_left(at);
            self.turn_index = 0;
            self.step = TurnStep::Items;
        }
    }

    /// Turn only this seat's standing model, without changing its route or turn.
    pub fn face(&mut self, seat: u8, yaw: f32) -> Result<(), BoardError> {
        if !yaw.is_finite() { return Err(BoardError::OutOfReach); }
        if self.walk.is_some() && seat == self.turn_seat() { return Ok(()); }
        let token = self.token(seat).ok_or(BoardError::UnknownTarget)?;
        token.facing = Some(yaw.rem_euclid(std::f32::consts::TAU));
        Ok(())
    }

    /// Hands over an item, letting the oldest go if the bag is full.
    pub fn dev_give(&mut self, seat: u8, item: Item) {
        if let Some(token) = self.token(seat) {
            // A crate or a snowball is carried, as the vacuum hands it over, not bagged.
            match item {
                Item::Crate => token.carrying = true,
                Item::Snowball => token.snowball = true,
                _ => {}
            }
            if matches!(item, Item::Crate | Item::Snowball) {
                return;
            }
            let replaced = if token.items.len() >= MAX_ITEMS { Some(token.items.remove(0)) } else { None };
            token.items.push(item);
            // Logged like a won item, so the sandbox shows it dropping into the sack.
            self.say(json!({"kind":"award","seat":seat,"item":item,"replaced":replaced}));
        }
    }

    pub fn dev_coins(&mut self, seat: u8, coins: i32) {
        self.coins(seat, coins);
    }

    pub fn dev_prize(&mut self, seat: u8) {
        if let Some(token) = self.token(seat) {
            token.prizes += 1;
        }
    }

    pub fn dev_heal(&mut self, seat: u8) {
        if let Some(token) = self.token(seat) {
            token.health = MAX_HEALTH;
            token.resting = 0;
        }
    }

    /// Puts a token on a space. With `land`, on your own turn, the space then
    /// does what it would if you had walked there: a shop opens, coins pay out,
    /// a trap goes off.
    pub fn dev_place(&mut self, seat: u8, space: u8, land: bool) {
        if self.walk.is_some() { return; }
        if (space as usize) >= self.layout.len() {
            return;
        }
        let from = self.space_of(seat);
        if let Some(token) = self.token(seat) {
            token.previous = None;
            token.space = space;
        }
        self.say(json!({ "kind": "dev_teleport", "seat": seat, "space": space, "from": from }));
        if land && seat == self.turn_seat() {
            // The sandbox's own jump starts a fresh landing even after Done
            // or while a shop is open. Dragging another token only places it.
            self.vacuum = Vacuum::default();
            self.step = TurnStep::Items;
            self.land(seat);
            // Anything that does not ask for an answer leaves you free to carry on.
            if self.step == TurnStep::Done {
                self.step = TurnStep::Items;
            }
        }
    }

    #[cfg(test)]
    pub fn hurt_for_test(&mut self, seat: u8) {
        if let Some(token) = self.token(seat) {
            token.health = token.health.saturating_sub(1);
        }
    }

    #[cfg(test)]
    pub fn move_for_test(&mut self, seat: u8, space: u8) {
        if let Some(token) = self.token(seat) {
            token.previous = None;
            token.space = space;
        }
    }

    #[cfg(test)]
    pub fn calm_for_test(&mut self) {
        self.wind_for_test((0.0, 0.0));
    }

    #[cfg(test)]
    pub fn wind_for_test(&mut self, wind: (f32, f32)) {
        if let TurnStep::Lob { wind: blowing, .. } = &mut self.step {
            *blowing = wind;
        }
    }

    /// Makes it this seat's turn, at the start of it, for tests.
    #[cfg(test)]
    pub fn turn_to_for_test(&mut self, seat: u8) {
        if let Some(at) = self.order.iter().position(|s| *s == seat) {
            let len = self.order.len();
            self.turn_index = (self.turn_index / len) * len + at;
        }
        self.step = TurnStep::Items;
        self.sucked = false;
        self.ambient_earned=0;
    }

    #[cfg(test)]
    pub fn storm_for_test(&mut self, space: u8) {
        self.storm = Some(space);
    }

    #[cfg(test)]
    pub fn clear_crates_for_test(&mut self) {
        self.crates.clear();
        self.blockades.clear();
        self.loose.clear();
    }

    #[cfg(test)]
    pub fn clear_air_for_test(&mut self){self.air_coins.clear();}

    #[cfg(test)]
    pub fn box_spot_for_test(&self,space:u8)->(f32,f32){self.box_spot(space)}

    #[cfg(test)]
    pub fn crate_for_test(&mut self, space: u8) {
        self.crates.insert(space, 0);
    }

    #[cfg(test)]
    pub fn loose_for_test(&mut self, space: u8, coins: i32) {
        *self.loose.entry(space).or_insert(0) += coins;
    }

    #[cfg(test)]
    pub fn plague_for_test(&mut self, seat: u8) {
        self.plague = Some(seat);
    }

    #[cfg(test)]
    pub fn bounty_for_test(&mut self, seat: u8) {
        self.bounty = Some(seat);
    }

    #[cfg(test)]
    pub fn give_for_test(&mut self, seat: u8, item: Item) {
        if let Some(token) = self.token(seat) {
            token.items.push(item);
        }
    }

    #[cfg(test)]
    pub fn give_prize_for_test(&mut self, seat: u8, prizes: u32) {
        if let Some(token) = self.token(seat) {
            token.prizes = prizes;
        }
    }

    #[cfg(test)]
    pub fn set_round_for_test(&mut self, round: u32) {
        self.round = round;
    }

    #[cfg(test)]
    pub fn rest_for_test(&mut self, seat: u8) {
        if let Some(token) = self.token(seat) {
            token.resting = 1;
        }
    }

    #[cfg(test)]
    pub fn upgrade_for_test(&mut self, seat: u8, upgrade: Upgrade) {
        if let Some(token) = self.token(seat) {
            token.upgrades.push(upgrade);
        }
    }

    #[cfg(test)]
    pub fn raise_shield_for_test(&mut self, seat: u8) {
        if let Some(token) = self.token(seat) {
            token.shield = true;
        }
    }

    #[cfg(test)]
    pub fn set_event_for_test(&mut self, event: RoundEvent) {
        self.event = Some(event);
    }

    /// Runs the event as a round start would, not just flagging it.
    #[cfg(test)]
    pub fn set_event_for_test_full(&mut self, event: RoundEvent) {
        self.underdog = None;
        self.event = Some(event);
        self.run_event(event);
    }

    /// Resolve the space a token is standing on, for tests.
    #[cfg(test)]
    pub fn land_for_test(&mut self, seat: u8) {
        self.land(seat);
    }

    /// Who a practice bot goes for with an item, if anyone is worth it: the
    /// leader, as long as they are in range. Nobody is picked as a target in
    /// the game; this is only where the bot points its throw or swing.
    fn bot_mark(&self, seat: u8, item: Item) -> Option<&Token> {
        let me = self.tokens.iter().find(|t| t.seat == seat)?;
        let (mx, mz) = self.spot_of(seat);
        let to_prize = self.distance_now(me.space, self.prize_at);
        self.tokens
            .iter()
            .filter(|t| self.rival(seat, t.seat))
            .filter(|t| {
                let at = space_in(self.layout, t.space);
                let away = ((at.x - mx).powi(2) + (at.z - mz).powi(2)).sqrt();
                match item {
                    Item::Bat => away <= BAT_RANGE + if self.timed_items && self.step == TurnStep::Items { positioning::POSITION_RADIUS } else { 0.0 },
                    // Clear of its own blast.
                    Item::Bomb => (BOMB_BLAST_RADIUS + 1.0..=BOT_THROW).contains(&away),
                    Item::Pickpocket => away <= BOT_THROW && !t.items.is_empty(),
                    Item::Swap => away <= BOT_THROW && self.distance_now(t.space, self.prize_at) < to_prize,
                    _ => away <= BOT_THROW,
                }
            })
            .max_by_key(|t| (t.prizes, t.coins))
    }

    /// Whether anything is near enough for a practice bot to take out its
    /// vacuum: a chest it can pay for, coins or a crate, or a rival to drain.
    fn bot_wants_vacuum(&self, seat: u8) -> bool {
        let (x, z) = self.actual_spot_of(seat);
        if self.bot_pedestrian_aim((x,z)).is_some() {return true;}
        let near = |(px, pz): (f32, f32)| ((px - x).powi(2) + (pz - z).powi(2)).sqrt() <= SUCTION_RADIUS;
        let layout = self.layout;
        self.pullables(seat).into_iter().any(|(key, at, _)| key >> 8 != PULL_STORM && near(at))
            || self.tokens.iter().any(|t| {
                let at = space_in(layout, t.space);
                self.rival(seat, t.seat) && t.coins > 0 && !t.has(Upgrade::Filter) && near((at.x, at.z))
            })
            || self.bot_storm_near(seat).is_some()
    }

    /// The storm, when it is close enough to a practice bot to blow away.
    fn bot_storm_near(&self, seat: u8) -> Option<(f32, f32)> {
        let storm = self.storm?;
        let (x, z) = self.spot_of(seat);
        let at = space_in(self.layout, storm);
        let close = gap_in(self.layout, storm, self.space_of(seat)) <= 2;
        let reach = ((at.x - x).powi(2) + (at.z - z).powi(2)).sqrt() <= VACUUM_RANGE * 0.8;
        (close && reach).then_some((at.x, at.z))
    }

    /// The item a practice bot reaches for: an attack on whoever is winning
    /// if one reaches, otherwise anything that helps itself.
    fn bot_item(&self, me: &Token) -> Option<(Item, Option<u8>, Option<u8>)> {
        let to_prize = self.distance_now(me.space, self.prize_at);
        let dice_free = self.dice == 1 && self.fixed_roll.is_none();
        for item in me.items.iter().copied() {
            let pick = match item {
                // Not while a raid of its own is still going: the board takes one at a time.
                Item::DriveBy if self.raids.contains_key(&me.seat) || me.resting > 0 => None,
                Item::Bat | Item::Bomb | Item::Grapple | Item::Curse | Item::Pickpocket | Item::Swap | Item::DriveBy | Item::Slapshot => {
                    self.bot_mark(me.seat, item).map(|_| (None, None))
                }
                // Only a rival's prize is there to take, never a partner's.
                Item::Heist => self
                    .tokens
                    .iter()
                    .any(|t| self.rival(me.seat, t.seat) && t.prizes > 0)
                    .then_some((None, None)),
                Item::GoldenDie if dice_free && (1..=DICE_SIDES).contains(&to_prize) => {
                    Some((None, Some(to_prize)))
                }
                Item::GoldenDie => None,
                Item::DoubleDice if dice_free => Some((None, None)),
                Item::DoubleDice => None,
                Item::Medkit if me.health < MAX_HEALTH => Some((None, None)),
                Item::Medkit => None,
                Item::Shield if !me.shield => Some((None, None)),
                Item::Shield => None,
                Item::Compass if me.coins >= PRIZE_COST && to_prize > COMPASS_SHORT => {
                    Some((None, None))
                }
                Item::Compass => None,
                Item::Trap => Some((None, None)),
                Item::DriveCar if self.map==Map::Frozen && self.timed_items => Some((None,None)),
                Item::DriveCar => None,
                Item::Crate | Item::Snowball | Item::SlashRush => None,
            };
            if let Some((target, number)) = pick {
                return Some((item, target, number));
            }
        }
        None
    }

    /// A practice bot's move for the step it is on.
    pub fn bot_move(&mut self) -> BotMove {
        let seat = self.turn_seat();
        let me = self.tokens.iter().find(|t| t.seat == seat).cloned();
        match self.step.clone() {
            TurnStep::Position { .. } | TurnStep::Driving | TurnStep::Raiding => BotMove::Walk,
            TurnStep::Walking => BotMove::Walk,
            TurnStep::Items => {
                let Some(me) = me else { return BotMove::Pass };
                // Suction first, if anything is in reach; then a crate thrown
                // in the leader's way.
                if !self.sucked && self.bot_wants_vacuum(seat) {
                    return BotMove::Suck;
                }
                if me.carrying && self.bot_mark(seat, Item::Crate).is_some() && self.rng.random_bool(0.6) {
                    return BotMove::ThrowCrate;
                }
                if me.snowball && self.bot_mark(seat, Item::Snowball).is_some() {
                    return BotMove::ThrowSnowball;
                }
                let Some((item, target, number)) = self.bot_item(&me) else {
                    return BotMove::Pass;
                };
                if self.rng.random_bool(0.75) {
                    BotMove::Item {
                        item,
                        target,
                        number,
                    }
                } else {
                    BotMove::Pass
                }
            }
            TurnStep::Roll => BotMove::Roll,
            TurnStep::Cannon { max } => {
                // Aim for the prize if it is in range, otherwise let fly.
                let to_prize = me.map(|token| distance_in(self.layout, token.space, self.prize_at)).unwrap_or(u8::MAX);
                let power = if to_prize >= 1 && to_prize <= max { to_prize } else { max / 2 + 1 };
                BotMove::Fire { power }
            }
            TurnStep::Lob { item, wind, .. } => {
                // At its mark, with a hand that is good but not perfect: it
                // judges the distance roughly and half allows for the wind.
                let (mx, mz) = self.spot_of(seat);
                let (dx, dz) = self
                    .bot_mark(seat, item)
                    .map(|t| space_in(self.layout, t.space))
                    .map(|at| (at.x - mx, at.z - mz))
                    .unwrap_or((0.0, 12.0));
                let flight = 1.6;
                let (dx, dz) = (dx - wind.0 * flight * 0.5, dz - wind.1 * flight * 0.5);
                let reach = (dx * dx + dz * dz).sqrt();
                let pitch = std::f32::consts::FRAC_PI_4;
                let power = ((reach * BOMB_GRAVITY / (2.0 * pitch).sin()).sqrt() / BOMB_SPEED).clamp(0.1, 1.0);
                let yaw = dx.atan2(dz) + (self.rng.random::<f32>() - 0.5) * 0.25;
                let power = power * (0.9 + self.rng.random::<f32>() * 0.2);
                BotMove::Lob { yaw, pitch, power }
            }
            TurnStep::Vacuum { .. } => {
                // The storm bearing down is blown off first; otherwise point
                // at the chest if it can pay, else the nearest thing worth
                // having or a rival to drain, and suck.
                let (x, z) = self.vacuum.origin.unwrap_or_else(||self.actual_spot_of(seat));
                if let Some((sx, sz)) = self.bot_storm_near(seat) {
                    return BotMove::Vacuum { yaw: (sx - x).atan2(sz - z), on: true, blow: true };
                }
                let layout = self.layout;
                let near = |(px, pz): (f32, f32)| ((px - x).powi(2) + (pz - z).powi(2)).sqrt();
                let mut spots: Vec<(u16, (f32, f32))> = self.pullables(seat).into_iter().map(|(key, at, _)| (key, at)).collect();
                spots.retain(|(key, at)| near(*at) <= VACUUM_RANGE && *key >> 8 != PULL_STORM);
                spots.extend(self.tokens.iter().filter(|t| self.rival(seat, t.seat) && t.coins > 0 && !t.has(Upgrade::Filter)).map(|t| {
                    let at = space_in(layout, t.space);
                    (u16::MAX, (at.x, at.z))
                }));
                let chest = spots.iter().find(|(key, _)| *key >> 8 == PULL_CHEST).map(|(_, at)| *at);
                let target = chest.or_else(|| {
                    spots
                        .iter()
                        .map(|(_, at)| *at)
                        .filter(|at| near(*at) <= VACUUM_RANGE)
                        .min_by(|a, b| near(*a).total_cmp(&near(*b)))
                });
                match target {
                    Some((sx, sz)) => {
                        // A bot's aim wanders a little.
                        let wobble = (self.vacuum.out * 2.3).sin() * 0.15;
                        BotMove::Vacuum { yaw: (sx - x).atan2(sz - z) + wobble, on: true, blow: false }
                    }
                    None => BotMove::StowVacuum,
                }
            }
            TurnStep::Check { .. } => BotMove::Check {
                stop: 0.5 + (self.rng.random::<f32>() - 0.5) * 0.7,
            },
            TurnStep::Swing { .. } => {
                let (mx, mz) = self.spot_of(seat);
                let yaw = self
                    .bot_mark(seat, Item::Bat)
                    .map(|t| space_in(self.layout, t.space))
                    .map(|at| (at.x - mx).atan2(at.z - mz))
                    .unwrap_or(0.0)
                    + (self.rng.random::<f32>() - 0.5) * 0.4;
                let stop = 0.5 + (self.rng.random::<f32>() - 0.5) * 0.8;
                BotMove::Swing { yaw, stop }
            }
            TurnStep::Shop { offers, upgrades } => {
                // Now and then an upgrade it can afford with a prize's worth
                // to spare; otherwise the dearest item, since price follows worth.
                let purse = me.map(|token| token.coins).unwrap_or(0);
                let upgrade = upgrades
                    .iter()
                    .filter(|offer| offer.price + PRIZE_COST <= purse)
                    .max_by_key(|offer| offer.price)
                    .map(|offer| offer.upgrade);
                if upgrade.is_some() && self.rng.random_bool(0.4) {
                    return BotMove::Shop { item: None, upgrade };
                }
                let pick = offers
                    .iter()
                    .filter(|offer| offer.price <= purse)
                    .max_by_key(|offer| offer.price)
                    .map(|offer| offer.item);
                BotMove::Shop { item: pick, upgrade: None }
            }
            TurnStep::Branch { options, .. } => {
                // A gate is only on offer once it has opened, and it is
                // there to be raced for.
                let summit = options.iter().find(|option| self.layout[option.node as usize].gated);
                BotMove::Branch {
                    node: summit.unwrap_or(&options[self.rng.random_range(0..options.len())]).node,
                }
            }
            TurnStep::Done => {
                if !self.sucked && self.bot_wants_vacuum(seat) {
                    BotMove::Suck
                } else {
                    BotMove::EndTurn
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BotMove {
    Walk,
    Shop { item: Option<Item>, upgrade: Option<Upgrade> },
    Fire { power: u8 },
    Lob { yaw: f32, pitch: f32, power: f32 },
    Swing { yaw: f32, stop: f32 },
    Check { stop: f32 },
    Suck,
    Vacuum { yaw: f32, on: bool, blow: bool },
    StowVacuum,
    ThrowCrate,
    ThrowSnowball,
    Item {
        item: Item,
        target: Option<u8>,
        number: Option<u8>,
    },
    Pass,
    Roll,
    Branch { node: u8 },
    EndTurn,
}

#[cfg(test)]
mod routing_tests {
    use super::*;

    fn junction(map: Map) -> (u8, Vec<u8>) {
        let board = Board::on(map, &[0, 1], 6, 77);
        board.layout.iter()
            .filter(|space| !space.gated && !space.under && space.kind == SpaceKind::Coin)
            .map(|space| (space.id, board.open_ways(space.id)))
            .find(|(_, ways)| ways.len() >= 3)
            .expect("every board offers a real crossroads")
    }

    #[test]
    fn every_board_has_shared_crossroads_and_no_open_road_dead_ends() {
        for map in Map::ALL {
            let board = Board::on(map, &[0, 1], 6, 77);
            assert!(board.layout.iter().any(|s| !s.gated && board.open_ways(s.id).len() >= 4), "{map:?}: a shared crossroads connects four roads");
            for space in board.layout.iter().filter(|s| !s.gated && s.link.is_none()) {
                assert!(board.open_ways(space.id).len() >= 2, "{map:?}: road {} must never force a forbidden U-turn", space.id);
            }
        }
    }

    #[test]
    fn a_crossroads_offers_every_open_exit_except_the_arrival_road() {
        for map in Map::ALL {
            let mut board = Board::on(map, &[0, 1], 6, 77);
            let seat = board.turn_seat();
            let (at, exits) = junction(map);
            let arrived = exits[0];
            let token = board.token(seat).unwrap();
            token.space = at;
            token.previous = Some(arrived);
            board.fixed_roll = Some(1);
            board.roll(seat).unwrap();
            let TurnStep::Branch { options, .. } = board.step() else { panic!("{map:?}: a crossroads asks for a route"); };
            let offered: HashSet<u8> = options.iter().map(|option| option.node).collect();
            assert_eq!(offered, exits.into_iter().filter(|exit| *exit != arrived).collect(), "{map:?}: both directions are legal, U-turns aren't");
            assert_eq!(board.choose_branch(seat, arrived), Err(BoardError::NotAnOption));
        }
    }

    #[test]
    fn stopping_at_a_crossroads_does_not_allow_a_u_turn_next_turn() {
        let mut board = Board::on(Map::Island, &[0, 1], 6, 77);
        let seat = board.turn_seat();
        let (at, exits) = junction(Map::Island);
        let arrived = exits[0];
        let token = board.token(seat).unwrap();
        token.space = at;
        token.previous = Some(arrived);
        board.step = TurnStep::Done;
        board.end_turn();
        board.end_turn();
        assert_eq!(board.turn_seat(), seat);
        board.fixed_roll = Some(1);
        board.roll(seat).unwrap();
        let TurnStep::Branch { options, .. } = board.step() else { panic!("still at a crossroads"); };
        assert!(!options.iter().any(|option| option.node == arrived));
        let next = options[0].node;
        board.choose_branch(seat, next).unwrap();
        let token = board.tokens.iter().find(|token| token.seat == seat).unwrap();
        assert_eq!(token.previous, Some(at));
        assert_eq!(token.space, next);
    }

    #[test]
    fn ice_keeps_a_reverse_walker_moving_in_their_arrival_direction() {
        let mut board = Board::on(Map::Frozen, &[0, 1], 6, 77);
        let (ice, before) = board.layout.iter().filter(|s| s.kind == SpaceKind::Ice && !s.gated && s.next.len() == 1)
            .find_map(|ice| board.layout.iter().find(|s| s.kind == SpaceKind::Coin && s.next == vec![ice.id]).map(|before| (ice.clone(), before.id)))
            .expect("an ice stretch has a plain space on its reverse side");
        let seat = board.turn_seat();
        let token = board.token(seat).unwrap();
        token.space = ice.id;
        token.previous = Some(ice.next[0]);
        board.land_for_test(seat);
        let token = board.tokens.iter().find(|t| t.seat == seat).unwrap();
        assert_eq!(token.space, before, "sliding must never force a U-turn");
        assert_eq!(token.previous, Some(ice.id));
    }

    #[test]
    fn the_summit_gate_blocks_entry_from_both_ends() {
        let mut board = Board::on(Map::Frozen, &[0, 1], 6, 77);
        let entrances: Vec<(u8, u8)> = board.layout.iter().flat_map(|space| space.next.iter().map(move |to| (space.id, *to)))
            .filter(|(a, b)| board.layout[*a as usize].gated != board.layout[*b as usize].gated)
            .map(|(a, b)| if board.layout[a as usize].gated { (b, a) } else { (a, b) })
            .collect();
        assert!(entrances.len() >= 2);
        for (outside, inside) in &entrances { assert!(!board.open_ways(*outside).contains(inside)); }
        board.round = board.gates_open_from().unwrap();
        for (outside, inside) in entrances { assert!(board.open_ways(outside).contains(&inside)); }
    }

    #[test]
    fn walking_through_a_portal_keeps_the_rest_of_the_roll() {
        let mut board = Board::on(Map::Island, &[0, 1], 6, 77);
        let seat = board.turn_seat();
        let portal = board.layout.iter().find(|s| s.link.is_some()).unwrap().clone();
        let before = board.layout.iter().find(|s| s.next == vec![portal.id]).unwrap().id;
        board.place_for_test(seat, before, 20);
        board.fixed_roll = Some(2);
        board.roll(seat).unwrap();
        let token = board.tokens.iter().find(|t| t.seat == seat).unwrap();
        assert_eq!(token.space, portal.link.unwrap());
        assert_eq!(token.previous, None, "the portal did not invent a physical arrival road");
        assert!(board.view().log.iter().any(|entry| entry["kind"] == "move" && entry["path"].as_array().is_some_and(|path| path.contains(&json!(portal.link.unwrap())))));
    }

    #[test]
    fn teleport_links_never_become_reverse_walkable_roads() {
        let board = Board::on(Map::Island, &[0, 1], 6, 77);
        for portal in board.layout.iter().filter(|space| space.link.is_some()) {
            let destination = portal.link.unwrap();
            assert!(!board.open_ways(destination).contains(&portal.id));
            assert!(!board.open_ways(portal.id).contains(&destination));
        }
    }

    #[test]
    fn undrained_board_events_remain_bounded_and_keep_latest_sequence() {
        let mut board=Board::on(Map::Island,&[0,1],6,77);
        for i in 0..1000 { board.say(json!({"kind":"review","index":i})); }
        assert_eq!(board.log.len(),40);
        let events=board.take_events();
        assert_eq!(events.len(),256);
        assert_eq!(events.last().unwrap()["index"],999);
        assert!(events.windows(2).all(|pair|pair[0]["seq"].as_u64().unwrap()<pair[1]["seq"].as_u64().unwrap()));
        assert!(board.take_events().is_empty());
    }

    #[test]
    fn a_board_with_nobody_seated_has_no_turn_to_divide_by() {
        let board = Board::new(&[], 3, 1);
        assert_eq!(board.turn_seat(), 0);
        let _ = board.view();
    }
}
