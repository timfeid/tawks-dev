use crate::card::*;
use crate::effect::*;
use crate::mana::ManaPool;
use crate::pack::{PackInstance, PackOpenResult, PackOpeningResult, PackType};
use crate::repository::CardRepository;
use crate::types::*;
use crate::view::*;
use game_framework::GameEffect;
use rand::rngs::StdRng;
use rand::{SeedableRng, seq::SliceRandom};
use sqlx::PgPool;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum CardBrowserSource {
    Catalog,
    DraftPool,
    Graveyard { target_player_id: String },
}

impl CardBrowserSource {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::DraftPool => "draft_pool",
            Self::Graveyard { .. } => "graveyard",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlayerState {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub instance_id: String,
    pub is_ai: bool,
    pub life: i32,
    pub deck_id: Option<String>,
    pub deck_name: Option<String>,
    pub deck_colors: Vec<String>,
    pub deck_format: Option<DeckFormat>,
    pub deck: Vec<Card>,
    /// Match-level sideboard pool (15 cards by MTG convention). Persists
    /// across games within a Bo3 series so swaps in game 2 are still
    /// available as options in game 3. Empty outside Bo3.
    pub sideboard: Vec<Card>,
    pub hand: Vec<Card>,
    pub battlefield: Vec<Card>,
    pub graveyard: Vec<Card>,
    pub exile: Vec<Card>,
    pub operator_zone: Vec<Card>,
    pub operator_id: Option<String>,
    pub operator_cast_count: usize,
    pub operator_first_cast_round: Option<usize>,
    pub operator_damage: HashMap<String, i32>,
    pub tokens_created: usize,
    pub loyalty_activations: usize,
    pub scry: Vec<Card>,
    pub pending_effects: Vec<(CardEffect, Vec<TargetRef>)>,
    pub scavenge_pending: Option<(i32, Vec<TargetRef>)>, // (counter_amount, targets for counters)
    pub pending_actions: Vec<PendingAction>,
    pub infrastructures_played_this_turn: u8,
    pub nodes_deployed_this_turn: u8,
    pub grid: Vec<GridNode>,
    pub color_identity: Vec<ManaType>,
    pub mana_pool: ManaPool,
    /// Syndicate only: police attention. Crosses a line and the player gets raided.
    pub heat: i32,
    pub deck_builder_deck_id: Option<String>,
    pub deck_builder_active: bool,
    pub deck_builder_name: Option<String>,
    pub deck_builder_description: Option<String>,
    pub deck_builder_format: Option<DeckFormat>,
    pub deck_builder_operator_card_id: Option<String>,
    pub deck_builder_autobuild_colors: Option<Vec<String>>,
    pub deck_builder_operator: Option<Card>,
    pub deck_builder_cards: Vec<DeckBuilderCard>,
    /// Sideboard entries while the deck builder is active. Parallel to
    /// `deck_builder_cards` but targets the sideboard zone on save.
    pub deck_builder_sideboard_cards: Vec<DeckBuilderCard>,
    pub deck_builder_filter_kind: Option<String>,
    pub deck_builder_sort_order: Option<String>,
    pub library_search_results: Vec<Card>,
    pub library_search_active: bool,
    pub surveil_active: bool,
    pub card_browser_active: bool,
    pub card_browser_source: Option<CardBrowserSource>,
    pub card_browser_title: Option<String>,
    pub card_browser_subtitle: Option<String>,
    pub collection_page: i32,
    pub collection_total: i32,
}

#[derive(Clone, Debug)]
pub enum PendingAction {
    TriggerChoice {
        source_card_id: String,
        source_card_name: String,
        effect: CardEffect,
        targets: Vec<TargetRef>,
        optional_cost: Option<AbilityCost>,
    },
    TriggerTargetSelection {
        source_card_id: String,
        source_card_name: String,
        effect: CardEffect,
        target_kind: TargetKind,
        /// When true, the player may skip this selection entirely (no target
        /// will be chosen and the effect is cancelled). Drives the front-end
        /// cancel button for "may destroy target unit"-style ETB triggers.
        optional: bool,
    },
    Discard {
        amount: usize,
        source_description: String,
        source_card_id: Option<String>,
    },
    Sacrifice {
        remaining: usize,
        source_card_id: String,
        ability: CardAbility,
        targets: Vec<TargetRef>,
        uses_stack: bool,
        owner_id: String,
    },
}

#[derive(Clone, Debug)]
pub enum StackItemKind {
    Spell(Card),
    Ability {
        source_card_id: String,
        ability: CardAbility,
    },
}

#[derive(Clone, Debug)]
pub struct StackItem {
    pub kind: StackItemKind,
    pub owner_id: String,
    pub targets: Vec<TargetRef>,
}

/// A Unit/Agent that has been cast and is being showcased to all players
/// before entering the battlefield. Opponents get a response window (they
/// can cast instants/abilities) and the card is displayed prominently.
/// Unlike stack items, showcase entries cannot be countered.
/// A card revealed by an effect (e.g. Chrome Cipher scry). Held on state
/// briefly with an expiry timestamp so the client can display it in the
/// center of the board before it disappears.
#[derive(Clone, Debug)]
pub struct RevealedCard {
    pub card: Card,
    pub owner_id: String,
    pub outcome_label: String,
    pub revealed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
pub struct ShowcaseEntry {
    pub card: Card,
    pub owner_id: String,
    pub targets: Vec<TargetRef>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub resolution_snapshot: GameLogSnapshot,
}

/// Hex draft state for 2+ player drafting.
///
/// Cards are arranged in a 19-card hex (3-4-5-4-3).
/// Picks unlock from the outside inward: a card becomes available once all
/// adjacent cards farther from the center are gone.
///
/// Index layout (top-to-bottom, left-to-right):
///       [0]  [1]  [2]
///    [3]  [4]  [5]  [6]
/// [7]  [8]  [9] [10] [11]
///   [12] [13] [14] [15]
///      [16] [17] [18]
#[derive(Clone, Debug)]
pub struct DraftState {
    /// Player display names captured at draft start, used after eliminations.
    pub player_names: HashMap<String, String>,
    /// Remaining cards to deal into future pyramids
    pub pool: Vec<Card>,
    /// Current hex pack: 19 slots, None = already picked
    pub pyramid: Vec<Option<Card>>,
    /// Cards each player has drafted so far
    pub drafted: HashMap<String, Vec<Card>>,
    /// Who picks next
    pub current_picker: Option<String>,
    /// Index into turn_order for who picks first this pack (alternates)
    pub first_picker_index: usize,
    /// Number of packs completed
    pub packs_completed: usize,
    /// Total packs in this draft
    pub total_packs: usize,
    /// Whether each player has confirmed their draft pool (for DraftBuilding stage)
    pub confirmed: HashSet<String>,
    /// Players who have handed their remaining picks to the auto-drafter.
    pub auto_draft: HashSet<String>,
    /// Award selections submitted during the post-game awards stage.
    pub award_selections: HashMap<String, Vec<String>>,
}

pub const DRAFT_PACK_SIZE: usize = 19;

/// Row boundaries in the hex layout (start_index, count).
const PYRAMID_ROWS: [(usize, usize); 5] = [(0, 3), (3, 4), (7, 5), (12, 4), (16, 3)];

/// Axial coordinates for a radius-2 hex, matching the row layout above.
const HEX_COORDS: [(i32, i32); DRAFT_PACK_SIZE] = [
    (0, -2),
    (1, -2),
    (2, -2),
    (-1, -1),
    (0, -1),
    (1, -1),
    (2, -1),
    (-2, 0),
    (-1, 0),
    (0, 0),
    (1, 0),
    (2, 0),
    (-2, 1),
    (-1, 1),
    (0, 1),
    (1, 1),
    (-2, 2),
    (-1, 2),
    (0, 2),
];

impl DraftState {
    pub fn new(
        pool: Vec<Card>,
        player_ids: &[String],
        player_names: HashMap<String, String>,
        total_packs: usize,
    ) -> Self {
        let mut drafted = HashMap::new();
        for pid in player_ids {
            drafted.insert(pid.clone(), Vec::new());
        }
        Self {
            player_names,
            pool,
            pyramid: Vec::new(),
            drafted,
            current_picker: None,
            first_picker_index: 0,
            packs_completed: 0,
            total_packs,
            confirmed: HashSet::new(),
            auto_draft: HashSet::new(),
            award_selections: HashMap::new(),
        }
    }

    /// Deal the next hex pack from the pool. Returns false if not enough cards.
    pub fn deal_next_pyramid(&mut self, player_ids: &[String]) -> bool {
        if self.pool.len() < DRAFT_PACK_SIZE {
            return false;
        }

        let pack_cards: Vec<Card> = self.pool.drain(..DRAFT_PACK_SIZE).collect();
        self.pyramid = pack_cards.into_iter().map(Some).collect();
        self.current_picker = Some(player_ids[self.first_picker_index % player_ids.len()].clone());
        true
    }

    fn hex_distance(index: usize) -> i32 {
        let (q, r) = HEX_COORDS[index];
        (q.abs() + r.abs() + (q + r).abs()) / 2
    }

    /// Returns adjacent indices farther from the center than this one.
    pub fn supports(&self, index: usize) -> Vec<usize> {
        let (q, r) = HEX_COORDS[index];
        let distance = Self::hex_distance(index);
        const DIRS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

        DIRS.iter()
            .filter_map(|(dq, dr)| {
                let nq = q + dq;
                let nr = r + dr;
                HEX_COORDS
                    .iter()
                    .position(|&(cq, cr)| cq == nq && cr == nr)
                    .filter(|&neighbor| Self::hex_distance(neighbor) > distance)
            })
            .collect()
    }

    /// Returns the display row (0-4) for a given hex-pack index.
    pub fn row_of(&self, index: usize) -> usize {
        for (row_idx, &(start, count)) in PYRAMID_ROWS.iter().enumerate() {
            if index >= start && index < start + count {
                return row_idx;
            }
        }
        2
    }

    /// Returns how deep a card is in the unlock chain:
    /// 0 = outer ring, 1 = middle ring, 2 = center.
    pub fn tier_of(&self, index: usize) -> usize {
        (2 - Self::hex_distance(index)) as usize
    }

    /// Returns indices of cards that are currently available to pick.
    pub fn available_indices(&self) -> Vec<usize> {
        let remaining = self.pyramid.iter().filter(|slot| slot.is_some()).count();
        if remaining <= 2 {
            return self
                .pyramid
                .iter()
                .enumerate()
                .filter_map(|(i, slot)| slot.as_ref().map(|_| i))
                .collect();
        }

        (0..self.pyramid.len())
            .filter(|&i| {
                if self.pyramid[i].is_none() {
                    return false;
                }
                self.supports(i)
                    .iter()
                    .all(|&support| self.pyramid[support].is_none())
            })
            .collect()
    }

    /// Pick a card at the given index. Returns the card if valid.
    pub fn pick(&mut self, player_id: &str, index: usize) -> Option<Card> {
        if index >= self.pyramid.len() {
            return None;
        }
        if !self.available_indices().contains(&index) {
            return None;
        }
        let card = self.pyramid[index].take()?;
        if let Some(drafted) = self.drafted.get_mut(player_id) {
            drafted.push(card.clone());
        }
        Some(card)
    }

    /// Check if the current pyramid has been fully picked.
    pub fn is_pyramid_empty(&self) -> bool {
        self.pyramid.iter().all(|slot| slot.is_none())
    }

    /// Timer in seconds for a given unlock tier.
    pub fn timer_for_row(&self, row: usize) -> u32 {
        match row {
            0 => 20,
            1 => 12,
            _ => 6,
        }
    }
}

#[cfg(test)]
mod draft_tests {
    use super::{DRAFT_PACK_SIZE, DraftState};
    use crate::card::CardBuilder;
    use crate::types::{CardKind, UnitType};

    fn make_card(index: usize) -> crate::card::Card {
        CardBuilder::new(
            &format!("draft-card-{index}"),
            &format!("Draft Card {index}"),
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 1)
        .build()
    }

    fn filled_draft_state() -> DraftState {
        let player_ids = vec!["p1".to_string(), "p2".to_string()];
        let player_names = player_ids
            .iter()
            .map(|pid| (pid.clone(), pid.clone()))
            .collect();
        let mut draft = DraftState::new(Vec::new(), &player_ids, player_names, 1);
        draft.pyramid = (0..DRAFT_PACK_SIZE).map(|i| Some(make_card(i))).collect();
        draft
    }

    #[test]
    fn hex_pack_starts_with_outer_ring_available() {
        let draft = filled_draft_state();
        assert_eq!(
            draft.available_indices(),
            vec![0, 1, 2, 3, 6, 7, 11, 12, 15, 16, 17, 18]
        );
    }

    #[test]
    fn center_unlocks_after_middle_ring_is_cleared() {
        let mut draft = filled_draft_state();
        for index in [4, 5, 8, 10, 13, 14] {
            draft.pyramid[index] = None;
        }

        assert!(draft.available_indices().contains(&9));
    }

    #[test]
    fn final_two_cards_are_both_pickable() {
        let mut draft = filled_draft_state();
        for index in 0..DRAFT_PACK_SIZE {
            if index != 4 && index != 9 {
                draft.pyramid[index] = None;
            }
        }

        assert_eq!(draft.available_indices(), vec![4, 9]);
    }
}

#[derive(Clone, Debug)]
pub struct CardGameInitArgs {
    pub starting_life: i32,
    pub opening_hand_size: usize,
    pub deck_seed: Option<u64>,
    pub pool: Option<Arc<PgPool>>,
    pub format: GameFormat,
    pub ruleset: Ruleset,
}

impl Default for CardGameInitArgs {
    fn default() -> Self {
        Self {
            starting_life: 20,
            opening_hand_size: 5,
            deck_seed: None,
            pool: None,
            format: GameFormat::Classic,
            ruleset: Ruleset::ManaClash,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CardGameState {
    pub stage: CardGameStage,
    pub format: GameFormat,
    pub ruleset: Ruleset,
    pub mode: GameMode,
    pub host_id: Option<String>,
    pub players: HashMap<String, PlayerState>,
    pub turn_order: Vec<String>,
    pub active_player_id: Option<String>,
    pub stack: Vec<StackItem>,
    pub combat_attackers: Vec<String>,
    pub combat_blockers: HashMap<String, Vec<String>>,
    pub combat_targets: HashMap<String, TargetRef>,
    pub combat_phase: CombatPhase,
    pub combat_blocker_id: Option<String>,
    pub effect_manager: EffectManager,
    pub triggers_played_this_turn: HashMap<String, usize>,
    pub viewer_instances: HashMap<String, String>,
    pub effect_id_counter: usize,
    pub shuffle_rng_counter: u64,
    /// Seeds shuffles and random discards when the game has no fixed deck
    /// seed. It travels with every clone, so the AI's planning copy makes
    /// the same random choices as the real game.
    pub random_seed: u64,
    /// Numbers tokens and created infrastructure.
    pub spawn_serial: u64,
    pub round: usize,
    pub init: CardGameInitArgs,
    pub history: Vec<StackItem>,
    pub forbidden_names: HashMap<String, Vec<(String, usize)>>, // player_id -> vec<(name, until_round)>
    pub damage_dealt_this_turn: HashMap<String, i32>,
    pub pings: Vec<PingEvent>,
    pub repository: Option<Arc<CardRepository>>,
    pub deck_choices_by_player: HashMap<String, Vec<DeckChoiceView>>,
    pub kind_options: Vec<SelectOption>,
    pub supertype_options: Vec<SelectOption>,
    pub unit_type_options: Vec<SelectOption>,
    pub artifact_type_options: Vec<SelectOption>,
    pub effect_kind_options: Vec<SelectOption>,
    pub mana_type_options: Vec<SelectOption>,
    pub trigger_event_options: Vec<SelectOption>,
    pub stat_type_options: Vec<SelectOption>,
    pub expire_contract_options: Vec<SelectOption>,
    pub effect_target_options: Vec<SelectOption>,
    pub card_options_loaded: bool,
    pub deck_choices_loaded: HashSet<String>,
    /// Workflow instances that were at the table when the last game finished;
    /// a view from any other instance means someone came back for a new game.
    pub finished_instances: HashSet<String>,
    pub priority_holder: Option<String>,
    pub consecutive_priority_passes: usize,
    /// A Unit/Agent being showcased before entering the battlefield.
    pub showcase: Option<ShowcaseEntry>,
    /// Short-lived reveal display for effects like Chrome Cipher's
    /// "Reveal the top of your library" — shown in the center for a few
    /// seconds so everyone can see the card. Not part of the stack.
    pub revealed_cards: Vec<RevealedCard>,
    /// Timestamp of the last stack push or showcase start, used for the
    /// minimum 5-second display window before auto-passing priority.
    pub last_stack_push_at: Option<chrono::DateTime<chrono::Utc>>,
    pub lobby_id: String,
    pub initial_player_count: usize,
    pub deferred_effects: Vec<GameEffect>,
    // Draft system
    pub draft: Option<DraftState>,
    // Pack system caches
    pub pack_types_cache: Vec<PackType>,
    pub user_packs_cache: HashMap<String, Vec<PackInstance>>,
    pub pack_open_result_cache: HashMap<String, Option<PackOpenResult>>,
    pub pack_opening_results_cache: HashMap<String, Vec<PackOpeningResult>>,
    /// Cards that have been phased out (owner_id, card_id) — return at next upkeep
    pub phased_out: Vec<(String, String)>,
    /// Transient counter tracking depth of `apply_effect_with_source` recursion.
    /// Incremented on entry, decremented on exit. Used to break infinite
    /// trigger cascades (e.g. two copies of a unit whose ETB copies the other
    /// — "Synthesis Reconstructor" scenario). Not persisted; resets per
    /// resolution.
    pub effect_resolution_depth: u32,
    /// Cumulative count of effects resolved this match. Used as a pragmatic
    /// cap to catch infinite-loop combos (sac-regen-sac, trigger cascades,
    /// etc.). Once past the threshold, effect application short-circuits and
    /// the match terminates at the next outer-loop check.
    pub match_effects_total: u64,
    /// Set when `match_effects_total` exceeds the cap. Outer game loop sees
    /// this and terminates the match as MaxEffectsExceeded.
    pub effects_cap_exceeded: bool,
    /// Trigger events for the current state update (cleared each view build)
    pub trigger_log: Vec<TriggerLogEntry>,
    /// Persistent game log — never cleared, shown in UI history panel
    pub game_log: Vec<GameLogEntry>,
    /// Best-of-N match state. `None` means Bo1 — the current game ends the
    /// match. `Some` means the game is part of a Bo3/longer series and
    /// completion of the current game transitions through `SideboardSwap`
    /// before spinning up the next game.
    pub match_state: Option<MatchState>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TriggerLogEntry {
    pub source_card_name: String,
    pub source_card_id: String,
    pub effect_description: String,
    pub owner_id: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameLogEntry {
    pub round: u32,
    pub player_id: String,
    pub player_name: String,
    pub kind: GameLogKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameLogKind {
    CastSpell,
    Resolved,
    PlayLand,
    DeployNode,
    ActivateAbility,
    AttackDeclared,
    BlockDeclared,
    CombatDamage,
    UnitDied,
    LifeLost,
    LifeGained,
    CardDrawn,
    CardDiscarded,
    Trigger,
    TurnStart,
    Other,
}

#[derive(Clone, Debug)]
pub(crate) struct GameLogSnapshot {
    pub players: HashMap<String, GameLogPlayerSnapshot>,
}

#[derive(Clone, Debug)]
pub(crate) struct GameLogPlayerSnapshot {
    pub life: i32,
    pub battlefield: HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PingEvent {
    pub x: f32,
    pub z: f32,
    pub player_id: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct DeckBuilderCard {
    pub card: Card,
    pub quantity: i32,
}

impl PlayerState {
    pub fn new(
        id: String,
        name: String,
        avatar_url: String,
        instance_id: String,
        starting_life: i32,
    ) -> Self {
        Self {
            id,
            name,
            avatar_url,
            instance_id,
            is_ai: false,
            life: starting_life,
            deck_id: None,
            deck_name: None,
            deck_colors: Vec::new(),
            deck_format: None,
            deck: Vec::new(),
            sideboard: Vec::new(),
            hand: Vec::new(),
            battlefield: Vec::new(),
            graveyard: Vec::new(),
            exile: Vec::new(),
            operator_zone: Vec::new(),
            operator_id: None,
            operator_cast_count: 0,
            operator_first_cast_round: None,
            operator_damage: HashMap::new(),
            tokens_created: 0,
            loyalty_activations: 0,
            scry: Vec::new(),
            pending_effects: Vec::new(),
            scavenge_pending: None,
            pending_actions: Vec::new(),
            infrastructures_played_this_turn: 0,
            nodes_deployed_this_turn: 0,
            grid: Vec::new(),
            color_identity: vec![ManaType::Colorless],
            mana_pool: ManaPool::default(),
            heat: 0,
            deck_builder_deck_id: None,
            deck_builder_active: false,
            deck_builder_name: None,
            deck_builder_description: None,
            deck_builder_format: None,
            deck_builder_operator_card_id: None,
            deck_builder_autobuild_colors: None,
            deck_builder_operator: None,
            deck_builder_cards: Vec::new(),
            deck_builder_sideboard_cards: Vec::new(),
            deck_builder_filter_kind: None,
            deck_builder_sort_order: None,
            library_search_results: Vec::new(),
            library_search_active: false,
            surveil_active: false,
            card_browser_active: false,
            card_browser_source: None,
            card_browser_title: None,
            card_browser_subtitle: None,
            collection_page: 0,
            collection_total: 0,
        }
    }

    pub fn new_ai(
        id: String,
        name: String,
        avatar_url: String,
        instance_id: String,
        starting_life: i32,
    ) -> Self {
        let mut player = Self::new(id, name, avatar_url, instance_id, starting_life);
        player.is_ai = true;
        player
    }

    pub fn has_pending_discard(&self) -> bool {
        self.pending_actions
            .iter()
            .any(|a| matches!(a, PendingAction::Discard { .. }))
    }

    pub fn has_pending_sacrifice(&self) -> bool {
        self.pending_actions
            .iter()
            .any(|a| matches!(a, PendingAction::Sacrifice { .. }))
    }

    pub fn has_pending_trigger_choice(&self) -> bool {
        self.pending_actions
            .iter()
            .any(|a| matches!(a, PendingAction::TriggerChoice { .. }))
    }

    pub fn has_pending_trigger_target(&self) -> bool {
        self.pending_actions
            .iter()
            .any(|a| matches!(a, PendingAction::TriggerTargetSelection { .. }))
    }

    pub fn has_any_pending(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    pub fn pending_discard(&self) -> Option<&PendingAction> {
        self.pending_actions
            .iter()
            .find(|a| matches!(a, PendingAction::Discard { .. }))
    }

    pub fn pending_discard_amount(&self) -> Option<usize> {
        match self.pending_discard() {
            Some(PendingAction::Discard { amount, .. }) => Some(*amount),
            _ => None,
        }
    }

    pub fn pending_sacrifice(&self) -> Option<&PendingAction> {
        self.pending_actions
            .iter()
            .find(|a| matches!(a, PendingAction::Sacrifice { .. }))
    }

    pub fn pending_trigger_choice(&self) -> Option<&PendingAction> {
        self.pending_actions
            .iter()
            .find(|a| matches!(a, PendingAction::TriggerChoice { .. }))
    }

    pub fn take_pending_discard(&mut self) -> Option<PendingAction> {
        let idx = self
            .pending_actions
            .iter()
            .position(|a| matches!(a, PendingAction::Discard { .. }))?;
        Some(self.pending_actions.remove(idx))
    }

    pub fn take_pending_sacrifice(&mut self) -> Option<PendingAction> {
        let idx = self
            .pending_actions
            .iter()
            .position(|a| matches!(a, PendingAction::Sacrifice { .. }))?;
        Some(self.pending_actions.remove(idx))
    }

    pub fn take_pending_trigger_choice(&mut self) -> Option<PendingAction> {
        let idx = self
            .pending_actions
            .iter()
            .position(|a| matches!(a, PendingAction::TriggerChoice { .. }))?;
        Some(self.pending_actions.remove(idx))
    }

    pub fn pending_trigger_target(&self) -> Option<&PendingAction> {
        self.pending_actions
            .iter()
            .find(|a| matches!(a, PendingAction::TriggerTargetSelection { .. }))
    }

    pub fn take_pending_trigger_target(&mut self) -> Option<PendingAction> {
        let idx = self
            .pending_actions
            .iter()
            .position(|a| matches!(a, PendingAction::TriggerTargetSelection { .. }))?;
        Some(self.pending_actions.remove(idx))
    }
}

impl Default for CardGameState {
    fn default() -> Self {
        Self {
            stage: CardGameStage::Lobby,
            format: GameFormat::Classic,
            ruleset: Ruleset::ManaClash,
            mode: GameMode::default(),
            host_id: None,
            players: HashMap::new(),
            turn_order: Vec::new(),
            active_player_id: None,
            stack: Vec::new(),
            combat_attackers: Vec::new(),
            combat_blockers: HashMap::new(),
            combat_targets: HashMap::new(),
            combat_phase: CombatPhase::Idle,
            combat_blocker_id: None,
            effect_manager: EffectManager::default(),
            triggers_played_this_turn: HashMap::new(),
            viewer_instances: HashMap::new(),
            effect_id_counter: 0,
            shuffle_rng_counter: 0,
            random_seed: rand::random(),
            spawn_serial: 0,
            round: 1,
            init: CardGameInitArgs::default(),
            history: Vec::new(),
            forbidden_names: HashMap::new(),
            damage_dealt_this_turn: HashMap::new(),
            pings: Vec::new(),
            repository: None,
            deck_choices_by_player: HashMap::new(),
            kind_options: Vec::new(),
            supertype_options: Vec::new(),
            unit_type_options: Vec::new(),
            artifact_type_options: Vec::new(),
            effect_kind_options: Vec::new(),
            mana_type_options: Vec::new(),
            trigger_event_options: Vec::new(),
            stat_type_options: Vec::new(),
            expire_contract_options: Vec::new(),
            effect_target_options: Vec::new(),
            card_options_loaded: false,
            deck_choices_loaded: HashSet::new(),
            finished_instances: HashSet::new(),
            priority_holder: None,
            consecutive_priority_passes: 0,
            showcase: None,
            revealed_cards: Vec::new(),
            last_stack_push_at: None,
            lobby_id: String::new(),
            initial_player_count: 0,
            deferred_effects: Vec::new(),
            draft: None,
            pack_types_cache: Vec::new(),
            user_packs_cache: HashMap::new(),
            pack_open_result_cache: HashMap::new(),
            pack_opening_results_cache: HashMap::new(),
            phased_out: Vec::new(),
            effect_resolution_depth: 0,
            match_effects_total: 0,
            effects_cap_exceeded: false,
            trigger_log: Vec::new(),
            game_log: Vec::new(),
            match_state: None,
        }
    }
}

impl CardGameState {
    /// Append an entry to the persistent game log. Caps at 500 entries.
    pub fn log(&mut self, kind: GameLogKind, player_id: &str, message: impl Into<String>) {
        let player_name = self
            .players
            .get(player_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| player_id.to_string());
        self.game_log.push(GameLogEntry {
            round: self.round as u32,
            player_id: player_id.to_string(),
            player_name,
            kind,
            message: message.into(),
        });
        if self.game_log.len() > 500 {
            let drain_count = self.game_log.len() - 500;
            self.game_log.drain(0..drain_count);
        }
    }

    pub(crate) fn capture_game_log_snapshot(&self) -> GameLogSnapshot {
        let players = self
            .players
            .iter()
            .map(|(player_id, player)| {
                let battlefield = player
                    .battlefield
                    .iter()
                    .map(|card| (card.id.clone(), card.name.clone()))
                    .collect();
                (
                    player_id.clone(),
                    GameLogPlayerSnapshot {
                        life: player.life,
                        battlefield,
                    },
                )
            })
            .collect();
        GameLogSnapshot { players }
    }

    pub(crate) fn target_display_name(&self, target: &TargetRef) -> String {
        match target {
            TargetRef::Player(player_id) => self
                .players
                .get(player_id)
                .map(|player| player.name.clone())
                .unwrap_or_else(|| player_id.clone()),
            TargetRef::Card(card_id) => self
                .find_card_name(card_id)
                .unwrap_or_else(|| card_id.clone()),
        }
    }

    pub(crate) fn format_target_list(&self, targets: &[TargetRef]) -> Option<String> {
        if targets.is_empty() {
            return None;
        }

        let names: Vec<String> = targets
            .iter()
            .map(|target| self.target_display_name(target))
            .collect();

        Some(Self::human_list(&names))
    }

    pub(crate) fn log_resolution_summary(
        &mut self,
        kind: GameLogKind,
        player_id: &str,
        subject: impl Into<String>,
        before: &GameLogSnapshot,
    ) {
        let subject = subject.into();
        let summary = self.summarize_game_log_changes(before);
        if summary.is_empty() {
            self.log(kind, player_id, subject);
        } else {
            self.log(
                kind,
                player_id,
                format!("{}: {}", subject, summary.join("; ")),
            );
        }
    }

    fn summarize_game_log_changes(&self, before: &GameLogSnapshot) -> Vec<String> {
        let mut changes = Vec::new();

        for player_id in self.ordered_player_ids() {
            let Some(before_player) = before.players.get(&player_id) else {
                continue;
            };
            let Some(after_player) = self.players.get(&player_id) else {
                continue;
            };

            let player_name = after_player.name.clone();
            match after_player.life.cmp(&before_player.life) {
                std::cmp::Ordering::Less => changes.push(format!(
                    "{} loses {} life",
                    player_name,
                    before_player.life - after_player.life
                )),
                std::cmp::Ordering::Greater => changes.push(format!(
                    "{} gains {} life",
                    player_name,
                    after_player.life - before_player.life
                )),
                std::cmp::Ordering::Equal => {}
            }

            let after_battlefield: HashMap<String, String> = after_player
                .battlefield
                .iter()
                .map(|card| (card.id.clone(), card.name.clone()))
                .collect();

            let mut removed_cards: Vec<_> = before_player
                .battlefield
                .iter()
                .filter(|(card_id, _)| !after_battlefield.contains_key(*card_id))
                .map(|(card_id, name)| (card_id.clone(), name.clone()))
                .collect();
            removed_cards.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

            for (card_id, name) in removed_cards {
                let zone_change = if after_player.graveyard.iter().any(|card| card.id == card_id) {
                    "dies"
                } else if after_player.exile.iter().any(|card| card.id == card_id) {
                    "is exiled"
                } else if after_player
                    .operator_zone
                    .iter()
                    .any(|card| card.id == card_id)
                {
                    "returns to the operator zone"
                } else {
                    "leaves the battlefield"
                };
                changes.push(format!("{} {}", name, zone_change));
            }

            let mut added_cards: Vec<_> = after_battlefield
                .iter()
                .filter(|(card_id, _)| !before_player.battlefield.contains_key(*card_id))
                .map(|(card_id, name)| (card_id.clone(), name.clone()))
                .collect();
            added_cards.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

            for (_, name) in added_cards {
                changes.push(format!("{} enters the battlefield", name));
            }
        }

        changes
    }

    pub(crate) fn human_list(items: &[String]) -> String {
        match items.len() {
            0 => String::new(),
            1 => items[0].clone(),
            2 => format!("{} and {}", items[0], items[1]),
            _ => {
                let mut head = items[..items.len() - 1].join(", ");
                head.push_str(", and ");
                head.push_str(&items[items.len() - 1]);
                head
            }
        }
    }

    /// Returns the player who currently holds priority, defaulting to the active player.
    pub fn effective_priority_holder(&self) -> Option<&str> {
        self.priority_holder
            .as_deref()
            .or(self.active_player_id.as_deref())
    }

    /// Returns the next player in turn order after `player_id`, wrapping around.
    pub fn next_player_after(&self, player_id: &str) -> Option<String> {
        let idx = self.turn_order.iter().position(|id| id == player_id)?;
        Some(self.turn_order[(idx + 1) % self.turn_order.len()].clone())
    }

    /// Clears priority state (called after resolution or turn advance).
    pub fn reset_priority(&mut self) {
        self.priority_holder = None;
        self.consecutive_priority_passes = 0;
    }

    pub(crate) fn next_spawn_serial(&mut self) -> u64 {
        self.spawn_serial += 1;
        self.spawn_serial
    }

    pub(crate) fn next_random_seed(&mut self, player_id: &str, reason: &str) -> Option<u64> {
        let base_seed = self.init.deck_seed.unwrap_or(self.random_seed);
        let counter = self.shuffle_rng_counter;
        self.shuffle_rng_counter = self.shuffle_rng_counter.saturating_add(1);

        let mut hasher = DefaultHasher::new();
        base_seed.hash(&mut hasher);
        self.round.hash(&mut hasher);
        self.effect_id_counter.hash(&mut hasher);
        counter.hash(&mut hasher);
        player_id.hash(&mut hasher);
        reason.hash(&mut hasher);
        Some(hasher.finish())
    }

    pub fn shuffle_deck_for_player(&mut self, player_id: &str, reason: &str) {
        let seed = self.next_random_seed(player_id, reason);
        let Some(player) = self.players.get_mut(player_id) else {
            return;
        };

        match seed {
            Some(seed) => {
                let mut rng = StdRng::seed_from_u64(seed);
                player.deck.shuffle(&mut rng);
            }
            None => {
                let mut rng = rand::rng();
                player.deck.shuffle(&mut rng);
            }
        }
    }
}
