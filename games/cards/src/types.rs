use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CardRarity {
    Public,      // Common
    Private,     // Uncommon
    Restricted,  // Rare
    Classified,  // Mythic
    Singularity, // Unique
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CardCategory {
    ManaRamp,
    Control,
    Aggro,
    Midrange,
    Removal,
    CardDraw,
    Utility,
    Buff,
    Combo,
    Evasion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VisualSpec {
    pub skin_id: String,
    pub seed: i32, // Used for procedural pattern offsets/colors
    pub wear: f32, // 0.0 to 1.0 (Digital degradation/glitchiness)
    #[serde(default)]
    pub holo: bool, // Independent 1% chance on any card
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy)]
#[serde(rename_all = "snake_case")]
pub enum CardZone {
    Hand,
    Battlefield,
    Graveyard,
    Exile,
    OperatorZone,
    Library,
    Stack,
    Lobby,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CardGameStage {
    Lobby,
    Draft,
    DraftBuilding,
    Awards,
    Main,
    Combat,
    PostCombatMain,
    End,
    /// Between games of a best-of-N match. Players submit sideboard swaps,
    /// then the next game begins.
    SideboardSwap,
    Finished,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MatchFormat {
    Bo1,
    Bo3,
}

impl Default for MatchFormat {
    fn default() -> Self {
        MatchFormat::Bo1
    }
}

impl MatchFormat {
    /// Number of wins required to take the match.
    pub fn wins_required(&self) -> u32 {
        match self {
            MatchFormat::Bo1 => 1,
            MatchFormat::Bo3 => 2,
        }
    }

    /// Maximum games that could be played in the series.
    pub fn max_games(&self) -> u32 {
        match self {
            MatchFormat::Bo1 => 1,
            MatchFormat::Bo3 => 3,
        }
    }
}

/// Records the state of a best-of-N match across multiple games.
///
/// Exists only for multi-game formats; Bo1 matches run with `match_state =
/// None` so nothing about the single-game path changes.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MatchState {
    pub format: MatchFormat,
    /// 1-based index of the game currently being played (or just finished,
    /// while resolving `SideboardSwap`).
    pub current_game: u32,
    /// Wins tallied per player so far. Keyed by player_id.
    pub wins: std::collections::HashMap<String, u32>,
    /// Set once `wins[player] == format.wins_required()`.
    pub series_winner: Option<String>,
    /// Player IDs that still owe a sideboard submission for the upcoming game.
    /// When empty during `SideboardSwap`, the next game starts automatically.
    pub pending_sideboard: Vec<String>,
    /// Winner id of each completed game in order. `game_winners[0]` is the
    /// winner of game 1, etc. Populated on game end; drives the "You won /
    /// You lost this game" banner without reconstruction heuristics.
    #[serde(default)]
    pub game_winners: Vec<String>,
}

impl MatchState {
    pub fn new(format: MatchFormat, player_ids: impl IntoIterator<Item = String>) -> Self {
        let mut wins = std::collections::HashMap::new();
        for id in player_ids {
            wins.insert(id, 0);
        }
        Self {
            format,
            current_game: 1,
            wins,
            series_winner: None,
            pending_sideboard: Vec::new(),
            game_winners: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CombatPhase {
    Idle,
    DeclareAttackers,
    DeclareBlockers,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ManaType {
    Neon,   // Cyan: Speed, Evasion
    Glitch, // Purple: Chaos, Discard
    Chrome, // Silver: Order, Protection, Buffs
    Slag,   // Orange: Brute Force, Sacrifice
    Bio,    // Green: Organic Growth, Counters
    Colorless,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GridNode {
    pub mana_type: ManaType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    None,
    BioOrganism,
    Human,
    Megafauna,
    Anomaly,
    Drone,
    Virus,
    Hacker,
    Node,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CardSupertype {
    Basic,
    Apex,
}

impl Default for CardSupertype {
    fn default() -> Self {
        CardSupertype::Basic
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeckFormat {
    #[serde(rename = "classic", alias = "classic_60")]
    Classic,
    #[serde(rename = "operator", alias = "operator_70", alias = "operator_100")]
    Operator,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GameFormat {
    Classic,
    Operator,
}

/// Which resource rules a match plays under. Mana Clash refills a coloured
/// pool every turn from the grid; Syndicate keeps a cash bankroll across
/// turns, lets players borrow, and tracks heat.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ruleset {
    #[default]
    ManaClash,
    Syndicate,
}

impl Ruleset {
    pub fn for_deck_id(deck_id: &str) -> Self {
        if deck_id.starts_with("syn_") {
            Ruleset::Syndicate
        } else {
            Ruleset::ManaClash
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Infrastructure,
    Unit(UnitType),
    Relic,
    Instant,
    Ritual,
    Aura,
    Agent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StatType {
    // Database-backed stat types (can be persisted)
    Power,
    Toughness,
    Both, // For effects that modify both power and toughness

    // Runtime-only stat types (used for game state tracking, should NOT be persisted to database)
    // These are mapped from Keywords during effect execution
    Armor,
    Shield,
    Deathtouch,
    Trample,
    Aero,
    Interceptor,
    Stealth,
    Camouflage,
    Indestructible,
    Vigilance,
    Haste,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Keyword {
    Haste,
    Deathtouch,
    Piercing,           // Ignore Armor/Shield
    ReactiveArmor(i32), // Reduce incoming damage by X
    Optics(UnitType),   // Can only be blocked by [Type]
    Gravecast(u8),      // Can be cast from graveyard up to N times
    Firewall(i32),      // Targeting this card costs X extra
    SignalJam,          // Opponent cannot use activated abilities
    Stealth,            // Cannot be blocked
    Aero,               // Can only be blocked by Aero/Interceptor
    Throttle(i32),      // Opponent's spells cost X more
    Glitch,             // Nullify unit's text box/abilities
    Malware,            // Units damaged by this lose all abilities
    Ethereal,           // Cannot be targeted by Physical damage/spells
    Trample,            // Excess damage to blocker hits player
    Camouflage,         // Cannot be targeted by opponent spells/abilities
    Interceptor,        // Can block Aero
    Flash,              // Can be cast at instant speed (if implemented)
    StaticPowerBuff(i32, Option<UnitType>),
    StaticToughnessBuff(i32, Option<UnitType>),
    /// Self-scaling buff: this card gets +N power per card of `CardKind`
    /// the controller has on the battlefield (counting this card if it
    /// matches).
    SelfPowerPerKind(i32, CardKind),
    SelfToughnessPerKind(i32, CardKind),
    Indestructible,
    Vigilance,
    Loyalty(i32),             // Add/Remove loyalty counters
    ExtraInfrastructures(u8), // Allow playing additional infrastructures per turn
    UntapDuringOtherTurns, // Permanents don't tap during other players' turns (Rootborn Awakener effect)
    StaticHasteGrant(GrantTarget),
    StaticTrampleGrant(GrantTarget),
    StaticStealthGrant(GrantTarget),
    StaticIndestructibleGrant(GrantTarget),
    StaticKeywordGrant(Box<Keyword>, GrantTarget),
    StaticTapForMana(ManaType, i32, GrantTarget),

    // --- New keywords ---
    Menace,                               // Can only be blocked by two or more units
    Wither,         // Damage dealt to units is dealt as -1/-1 counters (permanent shrink)
    Absorb(i32),    // Prevent the first X damage dealt to this unit each turn
    Modular(i32), // Enters with X +1/+1 counters; when it dies, move counters to another unit you control
    Evolve, // Whenever another unit enters under your control with greater power or toughness, add a +1/+1 counter
    Echo(ManaCost), // At your next upkeep, pay the echo cost or sacrifice
    Cascade, // When cast, exile cards from top until you hit one with lower cost, cast it free
    CostReduction(i32, Option<CardKind>), // Your spells of matching kind cost N less (None = all spells)
    Rampage(i32), // When this unit attacks, deal N damage to the defending player

    // --- Cyberpunk-themed states ---
    Compiling, // Unit's activated abilities are unavailable the turn it enters (it can still attack and block)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlayerScope {
    Owner,
    Opponent,
    All,
}

/// Which players' units a static grant applies to.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantSide {
    #[default]
    Mine,
    Opponents,
    All,
}

/// Additional filter applied to units within the grant's side.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrantFilter {
    UnitType(UnitType),
}

/// Target selector for static keyword/mana grants.
///
/// `GrantTarget::mine()` is the historical default (units you control, any type).
/// Combine side and filter to target opponents or a specific unit type.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GrantTarget {
    #[serde(default)]
    pub side: GrantSide,
    #[serde(default)]
    pub filter: Option<GrantFilter>,
}

impl GrantTarget {
    pub const fn mine() -> Self {
        Self {
            side: GrantSide::Mine,
            filter: None,
        }
    }

    pub const fn opponents() -> Self {
        Self {
            side: GrantSide::Opponents,
            filter: None,
        }
    }

    pub const fn all() -> Self {
        Self {
            side: GrantSide::All,
            filter: None,
        }
    }

    pub fn mine_of(unit_type: UnitType) -> Self {
        Self {
            side: GrantSide::Mine,
            filter: Some(GrantFilter::UnitType(unit_type)),
        }
    }

    pub fn opponents_of(unit_type: UnitType) -> Self {
        Self {
            side: GrantSide::Opponents,
            filter: Some(GrantFilter::UnitType(unit_type)),
        }
    }

    pub fn all_of(unit_type: UnitType) -> Self {
        Self {
            side: GrantSide::All,
            filter: Some(GrantFilter::UnitType(unit_type)),
        }
    }

    /// Returns true if the target card is on the correct side and matches any filter.
    /// `source_owner` is the controller of the card providing the grant; `target_owner`
    /// is the controller of the card potentially receiving it.
    pub fn matches(
        &self,
        source_owner: &str,
        target_owner: &str,
        target_unit_type: Option<&UnitType>,
    ) -> bool {
        let side_ok = match self.side {
            GrantSide::Mine => source_owner == target_owner,
            GrantSide::Opponents => source_owner != target_owner,
            GrantSide::All => true,
        };
        if !side_ok {
            return false;
        }
        match &self.filter {
            None => true,
            Some(GrantFilter::UnitType(t)) => target_unit_type == Some(t),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DynamicSource {
    /// Count cards in a zone with optional filters
    CardsInZone {
        zone: CardZone,
        scope: PlayerScope,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind_filter: Option<CardKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit_type_filter: Option<UnitType>,
    },
    /// Count distinct CardKind discriminants in a zone
    DistinctCardTypes { zone: CardZone, scope: PlayerScope },
    /// Sum of all mana in pool
    ManaTotal { scope: PlayerScope },
    /// Player's life total
    LifeTotal { scope: PlayerScope },
    /// Total damage dealt this turn by the effect's owner
    DamageDealtThisTurn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierAmount {
    Fixed(i32),
    Dynamic {
        source: DynamicSource,
        #[serde(default = "default_multiplier")]
        multiplier: i32,
        #[serde(default)]
        offset: i32,
    },
}

fn default_multiplier() -> i32 {
    1
}

impl From<i32> for ModifierAmount {
    fn from(v: i32) -> Self {
        ModifierAmount::Fixed(v)
    }
}

impl ModifierAmount {
    pub fn fixed(value: i32) -> Self {
        ModifierAmount::Fixed(value)
    }

    pub fn dynamic(source: DynamicSource) -> Self {
        ModifierAmount::Dynamic {
            source,
            multiplier: 1,
            offset: 0,
        }
    }

    pub fn scaled(source: DynamicSource, multiplier: i32) -> Self {
        ModifierAmount::Dynamic {
            source,
            multiplier,
            offset: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpireContract {
    Never,
    EndOfTurn,
    Turns(u8),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectTargetSpec {
    Targets,
    Source,
    OwnerUnits(Option<UnitType>),
    OpponentUnits(Option<UnitType>),
    AllUnits(Option<UnitType>),
    AdjacentUnits,
    LastCast,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectTarget {
    Card(String),
    Player(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ManaCost {
    pub neon: i32,
    pub glitch: i32,
    pub chrome: i32,
    pub slag: i32,
    pub bio: i32,
    pub colorless: i32,
}

impl ManaCost {
    pub fn total(&self) -> i32 {
        self.neon + self.glitch + self.chrome + self.slag + self.bio + self.colorless
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerEvent {
    // Card lifecycle
    Entered,
    Left,
    Died,

    // Combat
    Attacked,
    Blocked,
    DealtDamage,
    ReceivedDamage,

    // Self actions
    Tapped,

    // Turn phases
    TurnStart,
    Upkeep,
    DrawStep,
    EndOfTurn,

    // Owner cast events
    CastInstantOrRitual,
    CastRelic,
    CastAny,
    CastDuringOpponentTurn,

    // Owner resource events
    DrewCard,
    Discarded,
    Sacrificed,
    CounteredSpell,

    // Ally events
    AllyEntered(Option<UnitType>),
    AllyDied,
    AnyAllyDealtDamage,
    TokenCreated,

    // Opponent events
    OpponentCast,
    OpponentCastNonUnit,
    OpponentDrew,
    OpponentUnitDied,

    // Other
    LifeLost,
    LifeGained,
    GraveyardEntry,
    Equipped,
    SearchedLibrary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    IsActivePlayer,
    StageIs(CardGameStage),
    ManaAtLeast(ManaCost),
    TargetIsOpponentPlayer,
    TargetMatchesRememberedPlayer,
    HasUntappedUnits {
        count: usize,
        unit_type: Option<UnitType>,
    },
    TargetCountAtLeast(usize),
    TargetCountExactly(usize),
    TargetIsUnitType(UnitType),
    TargetIsKind(TargetKind),
    TargetIsCardKind(CardKind),
    TargetPlayerHandSizeExactly(usize),
    OncePerTurn,
    LifeLessThan(i32),
    InGraveyard,
    LastCastWas(CardKind),
    TargetIsManaType(ManaType),
    TargetIsTapped,
    /// True when the target card has `CardSupertype::Apex`.
    /// Used on AllyEntered/AllyDied triggers to filter to Apex-only events.
    TargetIsApex,
}

/// What to do with the top card of a stack after a `RevealTopIf` check.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RevealOutcome {
    /// Put onto the battlefield (permanents only; non-permanents fall through to ToTop).
    Deploy,
    /// Put into the revealer's hand.
    ToHand,
    /// Leave on top of the stack (default).
    #[default]
    ToTop,
    /// Put on the bottom of the stack.
    ToBottom,
    /// Send to the recycle bin (graveyard).
    ToGraveyard,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Predicate {
    Condition(Condition),
    CardKind(CardKind),
    IsUnit,
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
    Not(Box<Predicate>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerLimit {
    Unlimited,
    OncePerTurn,
}

impl Default for TriggerLimit {
    fn default() -> Self {
        TriggerLimit::Unlimited
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetScope {
    SelfPlayer,
    Opponent,
    Any,
    AnyPlayerOrUnit,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Player,
    Unit,
    Card,
    PlayerOrUnit,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetRef {
    Player(String),
    Card(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Competitive,
    Casual,
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::Casual
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventKind {
    Created,
    Transferred,
    VisualSpecUpdated,
    Bound,
    Unbound,
    Destroyed,
    PackOpened,
}
