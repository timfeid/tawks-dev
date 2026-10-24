use crate::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardEffect {
    RememberTargetPlayer,
    DamagePlayer(i32),
    DamageUnit(i32),
    DamageTarget(i32),
    DrawCards(usize),
    GainLife(i32),
    /// Syndicate: the caster's heat rises (or falls when negative).
    Heat(i32),
    /// Syndicate: every opponent's heat rises.
    HeatOpponents(i32),
    /// Syndicate: every opponent gains cash (or loses it when negative, possibly into debt).
    CashOpponents(i32),
    CounterTop,
    AddMana(ManaType, i32),
    CreateToken(TokenSpec),
    CreateInfrastructure(InfrastructureSpec),
    ApplyStatModifier(StatModifierSpec),
    ApplyStatModifiers(Vec<StatModifierSpec>),
    Sequence(Vec<CardEffect>),
    RevealTopIf {
        predicate: Predicate,
        #[serde(default)]
        on_match: RevealOutcome,
        #[serde(default)]
        on_fail: RevealOutcome,
    },
    Freeze(ExpireContract),                // Target cannot untap/attack/block
    RefundMana(i32),                       // Refund X% of cost (basis points)
    Loot(usize, usize),                    // Draw X, Discard Y
    Wheel(usize),                          // Each player discards hand, draws X
    Scry(usize),                           // Look at top X
    Search(Predicate), // Search library for card matching predicate (puts into hand)
    SearchToBattlefield(Predicate), // Search library for card matching predicate, put it onto the battlefield
    Bounce,                         // Return target to hand
    BoardWipe,                      // Destroy all units
    DestroyTargetPlayerUnits,       // Destroy all units controlled by target player
    DestroyUnitsWithToughnessAtLeast(i32), // Destroy all units with toughness >= N
    CopyLastCast,                   // Copy last spell cast
    Purify,                         // Remove all status effects
    GrantKeyword(Keyword, ExpireContract),
    Discard(usize),           // Opponent discards X
    DiscardSelfRandom(usize), // Owner discards X at random
    StealMana(ManaType, i32),
    ShuffleLibrary(TargetScope),
    ForbidName,    // Target unit's name is forbidden next turn
    Fight,         // Target unit fights another target unit
    TapTarget,     // Tap target unit
    UntapTarget,   // Untap target permanent
    DestroyTarget, // Destroy target unit/permanent
    ExileTarget,   // Exile target unit/permanent
    Equip,         // Attach this equipment to target unit you control
    AddManaScaling(ManaType, Option<UnitType>, Option<CardKind>), // Add 1 mana of type for each unit of type X or kind Y
    ModifyLoyalty(i32),                                           // Add/Remove loyalty counters
    Emblem(Vec<Keyword>, Option<StatModifierSpec>),               // Persistent buff emblem
    SacrificeThis,                                                // Sacrifice this permanent
    ReanimateNamedCard(String), // Return card with given name from graveyard to battlefield
    CopyUnit,                   // Copy target unit (creates a copy on battlefield)
    Mill(usize),                // Target player mills X cards from library to graveyard
    BounceAllUnits,             // Return all units to their owners' hands
    BounceAllOpponentPermanents, // Return all opponent permanents to their owners' hands
    BounceAllAttacking,         // Return all attacking units to their owners' hands
    SearchGraveyard(Predicate), // Search graveyard for card matching predicate
    ReanimateTargetUnit,        // Return target unit from graveyard to battlefield
    AddCounters(i32),           // Add X +1/+1 counters to target
    RemoveCounters(i32),        // Remove X +1/+1 counters from target
    PumpByMana, // Target unit gets +X/+X until end of turn where X = owner's total mana
    DamagePlayerPerHandCard, // Deal damage to target player equal to cards in their hand
    DamagePlayerPerBattlefieldKind(CardKind), // Deal damage to each opponent equal to count of cards of this kind you control
    CreateTokensPerBattlefieldKind(TokenSpec, CardKind), // Create N tokens where N is the count of cards of this kind you control

    // --- New effects ---
    Siphon(i32), // Drain X life from target player, you gain that much
    TransmuteMana(ManaType, ManaType, i32), // Convert X mana of one color to another
    Swap,        // Exchange control of two target units
    Phase,       // Target permanent phases out (returns next upkeep)
    MutateTarget(StatModifierSpec), // Permanently alter a unit's base stats (can't be purified)
    Polymorph, // Destroy target unit, controller reveals until they hit a unit, put it on battlefield
    Surveil(usize), // Look at top X cards, put each into hand or graveyard
    Transmit(usize), // Look at top X cards, put one into hand, rest on bottom
    Entomb(Predicate), // Search library for card matching predicate, put into graveyard
    Recycle,   // Shuffle your graveyard into your library
    DamageSplit(i32), // Deal X damage divided among any number of targets (currently: deal X to each target)
    DamageEachOpponentUnit(i32), // Deal X to each opposing unit
    Reflect(i32),     // Next time this unit takes damage, deal that much to the source's controller
    Conscript,        // Reanimate target unit from opponent's graveyard under your control
    Scavenge(i32),    // Exile a unit from your graveyard, target unit gets +X/+X counters
    Assimilate(UnitType), // Take control of target opponent unit; convert its UnitType to the given type
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenSpec {
    pub name: String,
    pub power: i32,
    pub toughness: i32,
    pub unit_type: UnitType,
    #[serde(default)]
    pub triggers: Vec<CardTrigger>, // Triggers the token has
    #[serde(default)]
    pub abilities: Vec<CardAbility>, // Abilities the token has
    #[serde(default)]
    pub enters_tapped: bool,
}

impl TokenSpec {
    pub fn new(name: impl Into<String>, power: i32, toughness: i32, unit_type: UnitType) -> Self {
        Self {
            name: name.into(),
            power,
            toughness,
            unit_type,
            triggers: vec![],
            abilities: vec![],
            enters_tapped: false,
        }
    }

    pub fn enters_tapped(mut self) -> Self {
        self.enters_tapped = true;
        self
    }

    pub fn with_triggers(mut self, triggers: Vec<CardTrigger>) -> Self {
        self.triggers = triggers;
        self
    }

    pub fn with_abilities(mut self, abilities: Vec<CardAbility>) -> Self {
        self.abilities = abilities;
        self
    }

    /// Returns the image URL for a token based on its name.
    pub fn image_url(&self) -> Option<&'static str> {
        token_image_url(&self.name)
    }
}

/// Resolves a token image URL by name.
pub fn token_image_url(name: &str) -> Option<&'static str> {
    match name {
        "Spore" => Some("https://cdn.tawks.app/cards/token_spore.png"),
        "Megafauna" => Some("https://cdn.tawks.app/cards/token_megafauna.png"),
        "Drone" => Some("https://cdn.tawks.app/cards/token_drone.png"),
        "Scrap" => Some("https://cdn.tawks.app/cards/token_scrap.png"),
        "Scrap Heap" => Some("https://cdn.tawks.app/cards/token_scrap_heap.png"),
        "Ember" => Some("https://cdn.tawks.app/cards/token_ember.png"),
        "Cinder" => Some("https://cdn.tawks.app/cards/token_cinder.png"),
        "Inferno" => Some("https://cdn.tawks.app/cards/token_inferno.png"),
        "Chassis" => Some("https://cdn.tawks.app/cards/token_chassis.png"),
        "Shield Drone" => Some("https://cdn.tawks.app/cards/token_shield_drone.png"),
        "Virus" => Some("https://cdn.tawks.app/cards/token_virus.png"),
        "Phantom" => Some("https://cdn.tawks.app/cards/token_phantom.png"),
        "Husk" => Some("https://cdn.tawks.app/cards/token_husk.png"),
        "Revenant" => Some("https://cdn.tawks.app/cards/token_revenant.png"),
        "Anomaly" => Some("https://cdn.tawks.app/cards/token_anomaly.png"),
        "Flicker" => Some("https://cdn.tawks.app/cards/token_flicker.png"),
        "Proxy" => Some("https://cdn.tawks.app/cards/token_proxy.png"),
        "Subroutine" => Some("https://cdn.tawks.app/cards/token_subroutine.png"),
        "Crawler" => Some("https://cdn.tawks.app/cards/token_crawler.png"),
        "Gremlin" => Some("https://cdn.tawks.app/cards/token_gremlin.png"),
        "Glider" => Some("https://cdn.tawks.app/cards/token_glider.png"),
        "Tendril" => Some("https://cdn.tawks.app/cards/token_tendril.png"),
        "Rhizome" => Some("https://cdn.tawks.app/cards/token_rhizome.png"),
        "Plague Megafauna" | "Spore Megafauna" => {
            Some("https://cdn.tawks.app/cards/token_megafauna.png")
        }
        "Plague Spore" => Some("https://cdn.tawks.app/cards/token_spore.png"),
        "Scrap Drone" => Some("https://cdn.tawks.app/cards/token_drone.png"),
        _ => None,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InfrastructureSpec {
    pub name: String,
    pub mana_type: ManaType,
    pub amount: i32,
    #[serde(default)]
    pub enters_tapped: bool,
    #[serde(default = "default_infrastructure_count")]
    pub count: usize,
}

fn default_infrastructure_count() -> usize {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatModifierSpec {
    pub stat: StatType,
    pub amount: ModifierAmount,
    pub expires: ExpireContract,
    pub target: EffectTargetSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardTrigger {
    pub event: TriggerEvent,
    pub effect: CardEffect,
    #[serde(default)]
    pub limit: TriggerLimit,
    #[serde(default)]
    pub predicate: Option<Predicate>,
    pub optional_cost: Option<AbilityCost>,
    #[serde(default)]
    pub is_optional: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardAbility {
    pub id: String,
    pub name: String,
    pub cost: AbilityCost,
    pub effect: CardEffect,
    pub target_spec: Option<TargetSpec>,
    #[serde(default)]
    pub once_per_turn: bool,
    #[serde(default)]
    pub uses_stack: bool,
    #[serde(default)]
    pub tap_targets: bool,
    #[serde(default)]
    pub predicate: Option<Predicate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetSpec {
    pub scope: TargetScope,
    pub kind: TargetKind,
    pub max_targets: usize,
    #[serde(default)]
    pub optional: bool,
    pub predicate: Option<Predicate>,
}

impl TargetSpec {
    pub fn new(scope: TargetScope, kind: TargetKind, max_targets: usize) -> Self {
        Self {
            scope,
            kind,
            max_targets,
            optional: false,
            predicate: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn with_predicate(mut self, predicate: Predicate) -> Self {
        self.predicate = Some(predicate);
        self
    }
}

fn default_rarity() -> CardRarity {
    CardRarity::Public
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub auto_description: String,
    #[serde(default = "default_rarity")]
    pub rarity: CardRarity,
    #[serde(default)]
    pub categories: Vec<CardCategory>,
    pub visual_spec: Option<VisualSpec>,
    #[serde(default)]
    pub template_id: Option<String>,
    pub cost: ManaCost,
    pub kind: CardKind,
    #[serde(default)]
    pub supertype: CardSupertype,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub tapped: bool,
    pub enters_tapped: bool,
    pub triggers: Vec<CardTrigger>,
    pub abilities: Vec<CardAbility>,
    pub keywords: Vec<Keyword>,
    pub image_url: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    pub attached_to: Option<String>,
    #[serde(default)]
    pub remembered_player_id: Option<String>,
    pub attachment: Option<AttachmentSpec>,
    pub summoning_sickness: bool,
    pub haste: bool,
    pub burnout: bool,
    #[serde(default)]
    pub echo_pending: bool,
    /// True if this unit has been infected by Malware — abilities are stripped
    /// until it leaves the battlefield. Set when a Malware-keyworded source
    /// deals combat damage to this unit.
    #[serde(default)]
    pub malware_infected: bool,
    pub loyalty: Option<i32>,
    /// +1/+1 counter count on this permanent. Kept as a simple integer
    /// because combat-stat math is tightly coupled to it.
    #[serde(default)]
    pub counters: i32,
    /// Arbitrary typed counters (charge, loyalty, stun, etc.). Keyed by a
    /// lowercase counter-kind string → count. Non-`+1/+1` counters live
    /// here and are opaque to combat math — card logic reads/writes them
    /// by name via helpers on `Card`.
    #[serde(default)]
    pub counters_by_kind: std::collections::HashMap<String, i32>,
    /// Damage marked on this unit that has not yet been healed. Persists until
    /// end of turn (like MTG). A unit dies when `damage >= effective_toughness`.
    #[serde(default)]
    pub damage: i32,
    /// Number of times this card has been cast from the graveyard via
    /// Gravecast. Compared against `Gravecast(N)` keyword value.
    #[serde(default)]
    pub gravecast_uses: u8,
    /// How many remaining untap steps this card skips because of a
    /// Freeze-style effect. 0 = not frozen; >0 = stays tapped for that
    /// many more of this unit's owner's untap steps. Decremented each
    /// time the owner's untap step runs without untapping the card.
    #[serde(default)]
    pub frozen_turns: u8,
    /// Damage absorbed this turn via the Absorb(N) keyword. Reset at
    /// end of turn alongside `damage`. Compared to the card's total
    /// Absorb capacity to compute remaining prevention.
    #[serde(default)]
    pub absorb_used_this_turn: i32,
    #[serde(default)]
    pub creator_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub owned: bool,
    #[serde(default)]
    pub obtained_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cost {
    Mana(ManaCost),
    Tap,              // Tap this permanent
    TapOther(usize),  // Tap N other units
    PayLife(i32),     // Pay N life
    Sacrifice(usize), // Sacrifice N units
    Discard(usize),   // Discard N cards
    Exile(usize),     // Exile N cards from hand
    SpendCharge(i32), // Spend N Charge from this permanent
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AbilityCost {
    #[serde(default)]
    pub costs: Vec<Cost>,
}

impl AbilityCost {
    pub fn new() -> Self {
        Self { costs: Vec::new() }
    }

    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.costs.push(cost);
        self
    }

    pub fn with_mana(self, mana: ManaCost) -> Self {
        if mana.total() > 0 {
            self.with_cost(Cost::Mana(mana))
        } else {
            self
        }
    }

    pub fn with_tap(self) -> Self {
        self.with_cost(Cost::Tap)
    }

    pub fn with_life(self, life: i32) -> Self {
        if life > 0 {
            self.with_cost(Cost::PayLife(life))
        } else {
            self
        }
    }

    pub fn get_mana(&self) -> ManaCost {
        for cost in &self.costs {
            if let Cost::Mana(mana) = cost {
                return mana.clone();
            }
        }
        ManaCost::default()
    }

    pub fn has_tap(&self) -> bool {
        self.costs.iter().any(|c| matches!(c, Cost::Tap))
    }

    pub fn get_life(&self) -> i32 {
        for cost in &self.costs {
            if let Cost::PayLife(life) = cost {
                return *life;
            }
        }
        0
    }

    pub fn get_tap_other_count(&self) -> usize {
        for cost in &self.costs {
            if let Cost::TapOther(count) = cost {
                return *count;
            }
        }
        0
    }

    pub fn get_sacrifice_count(&self) -> usize {
        for cost in &self.costs {
            if let Cost::Sacrifice(count) = cost {
                return *count;
            }
        }
        0
    }

    pub fn with_spend_charge(self, count: i32) -> Self {
        if count > 0 {
            self.with_cost(Cost::SpendCharge(count))
        } else {
            self
        }
    }

    pub fn get_spend_charge(&self) -> i32 {
        for cost in &self.costs {
            if let Cost::SpendCharge(count) = cost {
                return *count;
            }
        }
        0
    }

    pub fn is_empty(&self) -> bool {
        self.costs.is_empty()
    }

    pub fn is_empty_except_tap(&self) -> bool {
        self.costs.len() == 1 && self.has_tap()
    }
}

impl CardAbility {
    pub fn is_spell_effect_ability(&self) -> bool {
        !self.uses_stack && self.cost.is_empty() && !self.once_per_turn
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentSpec {
    pub modifiers: Vec<StatModifierSpec>,
    pub keywords: Vec<Keyword>,
}

impl Card {
    /// True if this card is a token (created on-the-fly by an effect rather
    /// than instantiated from a catalog template). Tokens should cease to
    /// exist when they leave the battlefield — they don't go to hand, deck,
    /// exile, or graveyard.
    pub fn is_token(&self) -> bool {
        self.template_id.is_none()
    }

    /// Add N counters of the given kind. Use lowercase kind names like
    /// "charge", "stun", "shield", etc. +1/+1 counters still live on
    /// `self.counters` since combat math reads them directly.
    pub fn add_counter(&mut self, kind: &str, amount: i32) {
        if kind == "+1/+1" || kind == "plus_one" {
            self.counters = self.counters.saturating_add(amount);
            return;
        }
        let entry = self
            .counters_by_kind
            .entry(kind.to_ascii_lowercase())
            .or_insert(0);
        *entry = entry.saturating_add(amount);
        if *entry <= 0 {
            self.counters_by_kind.remove(&kind.to_ascii_lowercase());
        }
    }

    /// Number of counters of a given kind on this card.
    pub fn counter_count(&self, kind: &str) -> i32 {
        if kind == "+1/+1" || kind == "plus_one" {
            return self.counters;
        }
        self.counters_by_kind
            .get(&kind.to_ascii_lowercase())
            .copied()
            .unwrap_or(0)
    }

    /// Gravecast limit: `Some(N)` if this card has `Gravecast(N)` (i.e.
    /// may be cast from the graveyard up to N times), `None` otherwise.
    pub fn gravecast_limit(&self) -> Option<u8> {
        self.keywords.iter().find_map(|k| match k {
            Keyword::Gravecast(n) => Some(*n),
            _ => None,
        })
    }

    /// True if this card still has gravecast uses remaining based on its
    /// limit and the current `counters` count (which doubles as the uses
    /// tally for gravecast cards while they're in the graveyard).
    pub fn has_gravecast(&self) -> bool {
        self.gravecast_limit().is_some()
    }

    /// Generate the full rules text as a single newline-separated string.
    /// Thin wrapper around [`Card::auto_rule_lines`] for backwards compat.
    pub fn auto_description(&self) -> String {
        let parts = self.auto_rule_lines();
        if parts.is_empty() {
            String::new()
        } else {
            parts.join(".\n")
        }
    }

    /// Generate the card's rules text as an ordered list of lines. Each
    /// entry is one logical rule (keyword, trigger, activated ability, or
    /// attachment effect). Used by the client to render each rule on its
    /// own visual line.
    pub fn auto_rule_lines(&self) -> Vec<String> {
        use crate::state::CardGameState;
        use std::collections::HashMap;

        let mut parts = Vec::new();

        for ability in &self.abilities {
            if !ability.is_spell_effect_ability() {
                continue;
            }
            if ability.effect != CardEffect::RememberTargetPlayer {
                continue;
            }
            let choose_text = match ability.target_spec.as_ref() {
                Some(TargetSpec {
                    scope: TargetScope::Opponent,
                    kind: TargetKind::Player,
                    ..
                }) => "Choose target opponent".to_string(),
                Some(TargetSpec {
                    kind: TargetKind::Player,
                    ..
                }) => "Choose target player".to_string(),
                _ => "Choose a player".to_string(),
            };
            parts.push(choose_text);
        }

        // 1. Add keywords, combining stat buffs where possible
        let mut power_buffs: HashMap<Option<UnitType>, i32> = HashMap::new();
        let mut toughness_buffs: HashMap<Option<UnitType>, i32> = HashMap::new();
        let mut other_keywords = Vec::new();

        // Collect stat buffs and other keywords
        for keyword in &self.keywords {
            match keyword {
                Keyword::StaticPowerBuff(amount, unit_type) => {
                    *power_buffs.entry(unit_type.clone()).or_insert(0) += amount;
                }
                Keyword::StaticToughnessBuff(amount, unit_type) => {
                    *toughness_buffs.entry(unit_type.clone()).or_insert(0) += amount;
                }
                _ => other_keywords.push(keyword),
            }
        }

        // Combine stat buffs
        let mut combined_buffs: HashMap<Option<UnitType>, (i32, i32)> = HashMap::new();
        for (unit_type, power) in power_buffs.iter() {
            let toughness = toughness_buffs.get(unit_type).copied().unwrap_or(0);
            combined_buffs.insert(unit_type.clone(), (*power, toughness));
        }
        for (unit_type, toughness) in toughness_buffs.iter() {
            if !combined_buffs.contains_key(unit_type) {
                combined_buffs.insert(unit_type.clone(), (0, *toughness));
            }
        }

        // Output combined stat buffs
        for (unit_type, (power, toughness)) in combined_buffs {
            let text = match &unit_type {
                Some(ct) => format!(
                    "{} get +{}/+{}",
                    CardGameState::unit_type_name(ct, true),
                    power,
                    toughness
                ),
                None => format!("Units get +{}/+{}", power, toughness),
            };
            parts.push(text);
        }

        // Output other keywords
        for keyword in other_keywords {
            let text = CardGameState::keyword_text(keyword);
            if !text.is_empty() {
                parts.push(text);
            }
        }

        // Output enters_tapped
        if self.enters_tapped {
            parts.push("Enters tapped".to_string());
        }

        // 2. Add triggers
        for trigger in &self.triggers {
            parts.push(CardGameState::trigger_text(trigger));
        }

        // 3. Add abilities
        for ability in &self.abilities {
            if ability.is_spell_effect_ability()
                && !matches!(self.kind, CardKind::Instant | CardKind::Ritual)
            {
                // For permanents (Units, Relics, Auras, Agents) with a synthetic
                // effect ability from .effect(card, target_spec), render as an
                // ETB rules line. Match either the `_effect` suffix OR the
                // default "Effect" name so DB-round-trips that normalize the
                // id don't silently drop the text.
                if ability.id.ends_with("_effect") || ability.name == "Effect" {
                    let effect_text =
                        CardGameState::effect_text(&ability.effect, ability.target_spec.as_ref());
                    parts.push(effect_text.trim_end_matches('.').to_string());
                }
                continue;
            }
            parts.push(CardGameState::ability_text(ability));
        }

        // 4. Add attachment text for equipment and auras.
        if let Some(attachment) = &self.attachment {
            let mut power = 0;
            let mut toughness = 0;
            for modifier in &attachment.modifiers {
                let amount = match modifier.amount {
                    ModifierAmount::Fixed(value) => value,
                    _ => continue,
                };
                match modifier.stat {
                    StatType::Power => power += amount,
                    StatType::Toughness => toughness += amount,
                    StatType::Both => {
                        power += amount;
                        toughness += amount;
                    }
                    _ => {}
                }
            }

            if power != 0 || toughness != 0 {
                let power_text = if power >= 0 {
                    format!("+{}", power)
                } else {
                    power.to_string()
                };
                let toughness_text = if toughness >= 0 {
                    format!("+{}", toughness)
                } else {
                    toughness.to_string()
                };
                parts.push(format!(
                    "Attached unit gets {}/{}",
                    power_text, toughness_text
                ));
            }

            for keyword in &attachment.keywords {
                let text = CardGameState::keyword_text(keyword);
                if !text.is_empty() {
                    parts.push(format!("Attached unit has {}", text));
                }
            }
        }

        parts
    }
}

pub struct CardBuilder {
    card: Card,
}

impl CardBuilder {
    pub fn new(id: &str, name: &str, kind: CardKind) -> Self {
        Self {
            card: Card {
                id: id.to_string(),
                name: name.to_string(),
                description: String::new(),
                auto_description: String::new(),
                rarity: CardRarity::Public,
                categories: Vec::new(),
                visual_spec: None,
                // Default template_id to the card's own id so builder-made
                // cards aren't treated as tokens by `Card::is_token()`.
                // Tokens are created separately via `CreateToken`.
                template_id: Some(id.to_string()),
                cost: ManaCost::default(),
                kind,
                supertype: CardSupertype::Basic,
                power: None,
                toughness: None,
                tapped: false,
                enters_tapped: false,
                triggers: Vec::new(),
                abilities: Vec::new(),
                keywords: Vec::new(),
                image_url: None,
                colors: Vec::new(),
                attached_to: None,
                remembered_player_id: None,
                attachment: None,
                summoning_sickness: false,
                haste: false,
                burnout: false,
                echo_pending: false,
                malware_infected: false,
                loyalty: None,
                counters: 0,
                counters_by_kind: std::collections::HashMap::new(),
                damage: 0,
                gravecast_uses: 0,
                frozen_turns: 0,
                absorb_used_this_turn: 0,
                creator_id: None,
                created_at: None,
                owned: false,
                obtained_at: None,
            },
        }
    }

    pub fn loyalty(mut self, starting: i32) -> Self {
        self.card.loyalty = Some(starting);
        self
    }

    pub fn cost(mut self, cost: ManaCost) -> Self {
        self.card.cost = cost;
        self
    }

    pub fn enters_tapped(mut self) -> Self {
        self.card.enters_tapped = true;
        self
    }

    pub fn stats(mut self, power: i32, toughness: i32) -> Self {
        self.card.power = Some(power);
        self.card.toughness = Some(toughness);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.card.name = name.to_string();
        self
    }

    pub fn kind(mut self, kind: CardKind) -> Self {
        self.card.kind = kind;
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.card.description = desc.to_string();
        self
    }

    pub fn supertype(mut self, supertype: CardSupertype) -> Self {
        self.card.supertype = supertype;
        self
    }

    pub fn keyword(mut self, keyword: Keyword) -> Self {
        if keyword == Keyword::Haste {
            self.card.haste = true;
        }
        self.card.keywords.push(keyword);
        self
    }

    pub fn image(mut self, url: &str) -> Self {
        self.card.image_url = Some(url.to_string());
        self
    }

    pub fn colors(mut self, colors: &[&str]) -> Self {
        self.card.colors = colors
            .iter()
            .map(|color| {
                mana_type_for_color_value(color)
                    .map(|mana| mana_color_for_type(&mana).to_string())
                    .unwrap_or_else(|| color.to_string())
            })
            .collect();
        self
    }

    pub fn trigger(mut self, trigger: CardTrigger) -> Self {
        self.card.triggers.push(trigger);
        self
    }

    pub fn ability(mut self, ability: CardAbility) -> Self {
        self.card.abilities.push(ability);
        self
    }

    // Convenience method: adds an effect as a trigger or ability
    // For instants/sorceries: triggers OnPlay (or ability if target_spec provided)
    // For permanents: triggers OnEnterBattlefield
    pub fn effect(mut self, effect: CardEffect, target_spec: Option<TargetSpec>) -> Self {
        // If a target_spec is provided, we need to create an ability instead of a trigger
        // because CardTrigger doesn't support target_spec
        if let Some(spec) = target_spec {
            let ability = CardAbility {
                id: format!("{}_effect", self.card.id),
                name: "Effect".to_string(),
                cost: AbilityCost::new(), // No cost for spell effects
                effect,
                target_spec: Some(spec),
                once_per_turn: false,
                uses_stack: false,
                tap_targets: false,
                predicate: None,
            };
            self.card.abilities.push(ability);
        } else {
            // No targeting needed, use a trigger
            let event = TriggerEvent::Entered;

            let trigger = CardTrigger {
                event,
                effect,
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            };

            self.card.triggers.push(trigger);
        }

        self
    }

    // Convenience method: adds a tap-activated ability
    pub fn tap_effect(self, effect: CardEffect) -> Self {
        use crate::state::CardGameState;
        let name = format!("Tap: {}", CardGameState::effect_text(&effect, None));
        let ability = CardAbility {
            id: format!("{}_tap", self.card.id),
            name,
            cost: AbilityCost::new().with_tap(),
            effect,
            target_spec: None,
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        };
        self.ability(ability)
    }

    pub fn rarity(mut self, rarity: CardRarity) -> Self {
        self.card.rarity = rarity;
        self
    }

    pub fn category(mut self, category: CardCategory) -> Self {
        if !self.card.categories.contains(&category) {
            self.card.categories.push(category);
        }
        self
    }

    pub fn build(self) -> Card {
        let mut card = self.card;
        if card.colors.is_empty() {
            card.colors = default_card_colors(&card);
        }
        // Generate auto description from card components
        card.auto_description = card.auto_description();
        // Use auto-generated description as the canonical description
        card.description = card.auto_description.clone();
        card
    }
}

pub fn mana_color_for_type(mana_type: &ManaType) -> &'static str {
    match mana_type {
        ManaType::Neon => "#00e5ff",
        ManaType::Glitch => "#a855f7",
        ManaType::Chrome => "#cbd5e1",
        ManaType::Slag => "#f97316",
        ManaType::Bio => "#22c55e",
        ManaType::Colorless => "#64748b",
    }
}

pub fn mana_type_for_color_value(value: &str) -> Option<ManaType> {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "neon" | "#00e5ff" | "#22d3ee" => Some(ManaType::Neon),
        "glitch" | "#a855f7" => Some(ManaType::Glitch),
        "chrome" | "#cbd5e1" => Some(ManaType::Chrome),
        "slag" | "#f97316" => Some(ManaType::Slag),
        "bio" | "#22c55e" | "#15803d" => Some(ManaType::Bio),
        "colorless" | "#64748b" => Some(ManaType::Colorless),
        _ => None,
    }
}

fn default_card_mana_types(card: &Card) -> Vec<ManaType> {
    let mut colors = Vec::new();
    if card.cost.neon > 0 {
        colors.push(ManaType::Neon);
    }
    if card.cost.glitch > 0 {
        colors.push(ManaType::Glitch);
    }
    if card.cost.chrome > 0 {
        colors.push(ManaType::Chrome);
    }
    if card.cost.slag > 0 {
        colors.push(ManaType::Slag);
    }
    if card.cost.bio > 0 {
        colors.push(ManaType::Bio);
    }
    if card.cost.colorless > 0 && colors.is_empty() {
        colors.push(ManaType::Colorless);
    }
    if colors.is_empty() {
        // Check abilities for mana-producing effects
        for ability in &card.abilities {
            if let CardEffect::AddMana(mana_type, _) = &ability.effect {
                colors.push(mana_type.clone());
                break;
            }
        }
    }
    colors
}

pub fn card_color_identities(card: &Card) -> Vec<ManaType> {
    let mut out = Vec::new();
    for color in &card.colors {
        if let Some(mana) = mana_type_for_color_value(color) {
            if !out.contains(&mana) {
                out.push(mana);
            }
        }
    }
    if out.is_empty() {
        out = default_card_mana_types(card);
    }
    out
}

pub fn default_card_colors(card: &Card) -> Vec<String> {
    default_card_mana_types(card)
        .into_iter()
        .map(|mana| mana_color_for_type(&mana).to_string())
        .collect()
}
