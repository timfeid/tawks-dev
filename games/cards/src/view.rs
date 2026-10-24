use crate::card::*;
use crate::mana::ManaPool;
use crate::pack::{PackInstance, PackOpenResult, PackOpeningResult, PackType};
use crate::state::{PingEvent, TriggerLogEntry};
use crate::types::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackItemView {
    pub card: CardView,
    pub owner_id: String,
    pub owner_name: String,
    pub targets: Vec<StackTargetView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RevealedCardView {
    pub card: CardView,
    pub owner_id: String,
    pub owner_name: String,
    pub outcome_label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StackTargetView {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatModifierSourceView {
    pub source_card_id: Option<String>,
    pub source_card_name: Option<String>,
    pub power_mod: i32,
    pub toughness_mod: i32,
    pub label: String,
    /// Keywords this source grants to the card (e.g. "Stealth" from a
    /// Lord-style static grant). Rendered alongside the P/T delta in the
    /// inspect panel so players can see the provenance of granted abilities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_keywords: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CardView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub auto_description: String,
    pub rarity: CardRarity,
    pub categories: Vec<CardCategory>,
    pub visual_spec: Option<VisualSpec>,
    pub cost: ManaCost,
    pub kind_tag: String,
    pub kind_label: String,
    pub frame_style: String,
    pub supertype: CardSupertype,
    pub unit_type: Option<UnitType>,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub base_power: Option<i32>,
    pub base_toughness: Option<i32>,
    pub stat_modifiers: Vec<StatModifierSourceView>,
    pub armor: i32,
    pub shield: i32,
    pub keywords: Vec<Keyword>,
    /// Pre-rendered rule-text lines for card display. Each entry is one
    /// visual line (keyword, activated ability, or static effect).
    #[serde(default)]
    pub rule_lines: Vec<String>,
    pub image_url: Option<String>,
    pub colors: Vec<String>,
    pub effect_text: Option<String>,
    pub target_spec: Option<TargetSpec>,
    /// Server-computed set of IDs (card IDs for units, player IDs for
    /// players) that would satisfy this card's target_spec if cast now.
    /// Empty when the card has no target_spec or can't be cast at all.
    #[serde(default)]
    pub valid_target_ids: Vec<String>,
    pub tapped: bool,
    pub abilities: Vec<CardAbilityView>,
    pub attached_to: Option<String>,
    pub attached_to_name: Option<String>,
    pub remembered_player_id: Option<String>,
    pub remembered_player_name: Option<String>,
    pub can_cast: bool,
    pub can_play_land: bool,
    pub playable: bool,
    pub can_attack: bool,
    pub can_tap: bool,
    pub can_block: bool,
    pub can_discard: bool,
    pub summoning_sickness: bool,
    pub haste: bool,
    pub burnout: bool,
    #[serde(default)]
    pub malware_infected: bool,
    pub loyalty: Option<i32>,
    pub counters: i32,
    #[serde(default)]
    pub damage: i32,
    pub zone: Option<CardZone>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub owned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obtained_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// For ability stack items: the id of the card that activated the
    /// ability. Lets the client highlight the source on the battlefield.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability_source_card_id: Option<String>,
    /// For stack items with resolved targets: ids of the chosen targets
    /// (card ids for units, player ids for players). Lets the client
    /// highlight what's about to be affected.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CardAbilityView {
    pub id: String,
    pub name: String,
    pub cost: AbilityCost,
    pub effect_text: String,
    pub target_spec: Option<TargetSpec>,
    /// Server-computed valid target IDs for this ability's target_spec
    /// (empty when ability has no target_spec or can't be activated).
    #[serde(default)]
    pub valid_target_ids: Vec<String>,
    pub once_per_turn: bool,
    pub tap_targets: bool,
    pub can_activate: bool,
    pub inactive_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriggerChoiceView {
    pub source_card_id: String,
    pub source_card_name: String,
    pub effect_text: String,
    pub cost_text: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TriggerTargetSelectionView {
    pub source_card_id: String,
    pub source_card_name: String,
    pub effect_text: String,
    pub target_kind: TargetKind,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PendingDiscardView {
    pub amount: usize,
    pub source_description: String,
    pub source_card: Option<CardView>,
    pub hand: Vec<CardView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PendingSacrificeView {
    pub remaining: usize,
    pub source_card_name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct GridNodeView {
    pub mana_type: ManaType,
    pub color: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeployableNodeView {
    pub value: ManaType,
    pub label: String,
    pub color: String,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerSummaryView {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub is_ai: bool,
    pub life: i32,
    pub hand_size: usize,
    pub battlefield_count: usize,
    pub battlefield_units: Vec<CardView>,
    pub battlefield_infrastructure: Vec<CardView>,
    pub operator_zone: Vec<CardView>,
    pub grid: Vec<GridNodeView>,
    pub nodes_deployed_this_turn: u8,
    pub infrastructures: i32,
    pub mana_pool: ManaPool,
    pub heat: i32,
    pub is_active: bool,
    pub deck_name: Option<String>,
    pub deck_colors: Vec<String>,
    pub library_size: usize,
    pub graveyard_size: usize,
    pub operator_damage: HashMap<String, i32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ViewerView {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub life: i32,
    pub mana_pool: ManaPool,
    pub heat: i32,
    pub infrastructures_played_this_turn: u8,
    pub nodes_deployed_this_turn: u8,
    pub grid: Vec<GridNodeView>,
    pub can_deploy_node: bool,
    pub deployable_node_types: Vec<DeployableNodeView>,
    pub is_active: bool,
    pub is_host: bool,
    pub deck_id: Option<String>,
    pub deck_name: Option<String>,
    pub deck_colors: Vec<String>,
    pub hand: Vec<CardView>,
    pub scry: Vec<CardView>,
    pub battlefield_units: Vec<CardView>,
    pub battlefield_infrastructure: Vec<CardView>,
    pub operator_zone: Vec<CardView>,
    pub pending_trigger_choice: Option<TriggerChoiceView>,
    pub pending_trigger_target: Option<TriggerTargetSelectionView>,
    pub pending_discard: Option<PendingDiscardView>,
    pub pending_sacrifice: Option<PendingSacrificeView>,
    pub library_search_active: bool,
    pub can_respond: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeckChoiceView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub colors: Vec<String>,
    pub image_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeckCardView {
    pub card: CardView,
    pub quantity: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeckBuilderView {
    pub active: bool,
    pub deck_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub format: Option<DeckFormat>,
    pub operator_card_id: Option<String>,
    pub autobuild_colors: Option<Vec<String>>,
    pub operator: Option<CardView>,
    pub cards: Vec<DeckCardView>,
    /// Sideboard entries for the deck being edited. Parallel to `cards`
    /// but lives in the sideboard zone on save.
    #[serde(default)]
    pub sideboard: Vec<DeckCardView>,
    pub filter_kind: Option<String>,
    pub sort_order: Option<String>,
    pub library: Vec<CardView>,
    pub browser_active: bool,
    pub browser_source: Option<String>,
    pub browser_title: Option<String>,
    pub browser_subtitle: Option<String>,
    pub collection_page: i32,
    pub collection_total: i32,
    pub collection_has_prev: bool,
    pub collection_has_next: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DraftPyramidView {
    /// 19 hex-pack slots (None = already picked)
    pub pyramid: Vec<Option<CardView>>,
    /// Indices that are currently pickable
    pub available_indices: Vec<usize>,
    /// Current picker's player ID
    pub current_picker_id: Option<String>,
    /// Current picker's display name
    pub current_picker_name: Option<String>,
    /// Whether it's the viewing player's turn
    pub is_my_turn: bool,
    /// Cards the viewing player has drafted so far
    pub my_drafted_cards: Vec<CardView>,
    /// Count of cards the viewing player has drafted
    pub my_drafted_count: usize,
    /// Count of cards the opponent has drafted
    pub opponent_drafted_count: usize,
    /// Opponent's drafted cards broken down by rarity
    pub opponent_rarity_counts: HashMap<String, usize>,
    /// Number of packs completed
    pub packs_completed: usize,
    /// Total packs in the draft
    pub total_packs: usize,
    /// Timer in seconds for the current pick (based on deepest available tier)
    pub pick_timer_seconds: u32,
    /// Whether the viewing player has handed their remaining picks to the
    /// auto-drafter.
    pub auto_draft_enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DraftAwardsView {
    pub winner_id: Option<String>,
    pub winner_name: Option<String>,
    pub role: String,
    pub selection_mode: String,
    pub cards: Vec<CardView>,
    pub required_count: usize,
    pub submitted: bool,
    pub waiting_on: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatchStateView {
    pub format: crate::types::MatchFormat,
    pub current_game: u32,
    pub wins_required: u32,
    pub wins: std::collections::HashMap<String, u32>,
    pub series_winner: Option<String>,
    /// Non-empty only during `SideboardSwap` — lists players still owing a
    /// submission. Drives the "waiting for opponent" UI.
    pub pending_sideboard: Vec<String>,
    /// Winner id for each completed game so far, in order.
    pub game_winners: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CardGameView {
    pub stage: CardGameStage,
    pub format: GameFormat,
    pub ruleset: Ruleset,
    pub mode: GameMode,
    pub active_player_id: Option<String>,
    pub active_player_name: Option<String>,
    pub priority_player_id: Option<String>,
    pub round: usize,
    pub players: Vec<PlayerSummaryView>,
    pub viewer: Option<ViewerView>,
    pub deck_choices: Vec<DeckChoiceView>,
    pub deck_builder: DeckBuilderView,
    pub stack: Vec<StackItemView>,
    #[serde(default)]
    pub revealed_cards: Vec<RevealedCardView>,
    pub combat_phase: String,
    pub combat_blocker_id: Option<String>,
    pub combat_attackers: Vec<CardView>,
    pub combat_pairs: Vec<CombatPairView>,
    pub targetable_units: Vec<UnitTargetView>,
    pub pings: Vec<PingEvent>,
    pub trigger_log: Vec<TriggerLogEntry>,
    pub game_log: Vec<crate::state::GameLogEntry>,
    pub cards: Vec<CardView>,
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
    pub winner_player_id: Option<String>,
    pub winner_player_name: Option<String>,
    /// Best-of-N match state. `None` when the game is Bo1 (the current game
    /// is the whole match). Populated for Bo3+ with current game number,
    /// per-player wins, and the series winner once decided.
    #[serde(default)]
    pub match_state: Option<MatchStateView>,
    /// Viewer's sideboard pool (empty when no sideboard or not their view).
    #[serde(default)]
    pub viewer_sideboard: Vec<CardView>,
    /// Viewer's full main decklist (consolidated from every zone minus
    /// tokens). Used by the sideboard swap UI between games.
    #[serde(default)]
    pub viewer_main_deck: Vec<CardView>,
    // Draft system
    pub draft: Option<DraftPyramidView>,
    pub draft_awards: Option<DraftAwardsView>,
    // Pack system
    pub pack_types: Vec<PackType>,
    pub user_packs: Vec<PackInstance>,
    pub pack_open_result: Option<PackOpenResult>,
    pub pack_opening_results: Vec<PackOpeningResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPairView {
    pub attacker: CardView,
    pub blockers: Vec<CardView>,
    pub target_kind: String,
    pub target_label: String,
    pub target_player_id: Option<String>,
    pub target_card_id: Option<String>,
    pub is_blocked: bool,
    /// True if the attacker is predicted to die when combat resolves.
    pub attacker_dies: bool,
    /// Per-blocker flag: true if this blocker is predicted to die.
    pub blocker_dies: Vec<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnitTargetView {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub owner_name: String,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub armor: i32,
    pub shield: i32,
}
