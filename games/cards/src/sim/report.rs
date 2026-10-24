use serde::{Deserialize, Serialize};

use crate::action::CardGameAction;
use crate::types::CardGameStage;

use super::agent::AgentProfile;
use super::degeneracy::{DegeneracyFlag, DegeneracySummary};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    Finished,
    MaxTurnsExceeded,
    MaxActionsExceeded,
    LoopDetected,
    NoLegalAction,
    TimedOut,
    ProcessCrashed,
    UpdateError,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionTraceEntry {
    pub index: usize,
    pub actor_id: String,
    pub action: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSummary {
    pub player_id: String,
    pub deck_id: Option<String>,
    pub life: i32,
    pub hand_size: usize,
    pub battlefield_size: usize,
    pub battlefield_power: i32,
    pub graveyard_size: usize,
    pub mana_sources: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
    #[serde(default)]
    pub operator_cast_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_first_cast_round: Option<usize>,
    #[serde(default)]
    pub operator_damage_total: i32,
    #[serde(default)]
    pub tokens_created: usize,
    #[serde(default)]
    pub loyalty_activations: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoundSnapshot {
    pub round: usize,
    pub active_player_id: Option<String>,
    pub stage: CardGameStage,
    pub players: Vec<PlayerSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchReport {
    pub seed: u64,
    pub deck_a: String,
    pub deck_b: String,
    pub agent_a: AgentProfile,
    pub agent_b: AgentProfile,
    pub winner_player_id: Option<String>,
    pub winner_deck_id: Option<String>,
    #[serde(default)]
    pub drawn: bool,
    pub rounds: usize,
    pub actions_taken: usize,
    pub termination_reason: TerminationReason,
    pub error: Option<String>,
    pub final_state: Vec<PlayerSummary>,
    pub snapshots: Vec<RoundSnapshot>,
    pub trace: Vec<ActionTraceEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degeneracy_flags: Vec<DegeneracyFlag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub card_metrics: Vec<CardMetrics>,
}

/// Per-card telemetry collected over a single match. Card identity is the
/// catalog template id (e.g. `sol_core`), not the per-instance card id, so
/// rows are aggregatable across matches in a series/matrix. `side` is
/// `"winner"` for cards owned by the winning player, `"loser"` otherwise
/// — keeps the stats directly analyzable ("what does Fusion Core do when
/// you win?") without losing per-match granularity.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CardMetrics {
    pub card_id: String,
    pub card_name: String,
    pub owner_deck_id: String,
    pub side: String,
    pub copies_in_deck: usize,
    pub in_opening_hand: usize,
    pub cast_count: usize,
    pub first_cast_round: Option<usize>,
    pub entered_battlefield: usize,
    pub times_destroyed: usize,
    pub times_exiled: usize,
    pub times_bounced: usize,
    pub times_sacrificed: usize,
    pub times_countered: usize,
    pub times_milled: usize,
    pub times_discarded: usize,
    pub ability_activations: usize,
    pub alive_on_battlefield_at_end: usize,
    pub damage_dealt: i32,
    pub unit_damage_dealt: i32,
    pub tokens_created: usize,
    pub rounds_on_battlefield: usize,
    pub cards_drawn: usize,
    pub life_gained: i32,
}

impl MatchReport {
    pub fn is_finished(&self) -> bool {
        self.termination_reason == TerminationReason::Finished
    }

    pub fn is_draw(&self) -> bool {
        self.drawn
            || (self.is_finished()
                && self.winner_player_id.is_none()
                && self.winner_deck_id.is_none())
    }

    pub fn is_decisive(&self) -> bool {
        self.is_finished() && !self.is_draw() && self.winner_deck_id.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeriesReport {
    pub matches: usize,
    pub deck_a: String,
    pub deck_b: String,
    pub wins_by_deck: Vec<(String, usize)>,
    pub results: Vec<MatchReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerminationCount {
    pub reason: TerminationReason,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeckMetrics {
    pub deck_id: String,
    pub matches: usize,
    pub finished_matches: usize,
    pub decisive_matches: usize,
    pub drawn_matches: usize,
    pub wins: usize,
    pub losses: usize,
    pub unfinished_matches: usize,
    pub win_rate: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatchupMetrics {
    pub deck_a: String,
    pub deck_b: String,
    pub matches: usize,
    pub finished_matches: usize,
    pub decisive_matches: usize,
    pub drawn_matches: usize,
    pub unfinished_matches: usize,
    pub wins_by_deck: Vec<(String, usize)>,
    pub first_player_wins: usize,
    pub first_player_win_rate: Option<f64>,
    pub average_rounds: f64,
    pub average_actions: f64,
    pub deck_a_win_rate: Option<f64>,
    pub termination_counts: Vec<TerminationCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceMetrics {
    pub balance_score: f64,
    pub pairwise_imbalance: f64,
    pub finished_match_rate: f64,
    pub decisive_match_rate: f64,
    pub draw_match_rate: f64,
    pub unfinished_match_rate: f64,
    pub first_player_win_rate: Option<f64>,
    pub average_rounds: f64,
    pub average_actions: f64,
    pub termination_counts: Vec<TerminationCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixReport {
    pub decks: Vec<String>,
    pub matches_per_pair: usize,
    pub alternate_seats: bool,
    pub include_mirrors: bool,
    pub deck_metrics: Vec<DeckMetrics>,
    pub matchup_metrics: Vec<MatchupMetrics>,
    pub balance: BalanceMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degeneracy: Option<DegeneracySummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeckWinRateDelta {
    pub deck_id: String,
    pub baseline_win_rate: Option<f64>,
    pub patched_win_rate: Option<f64>,
    pub delta: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchEvaluationDelta {
    pub balance_score_delta: f64,
    pub pairwise_imbalance_delta: f64,
    pub finished_match_rate_delta: f64,
    pub decisive_match_rate_delta: f64,
    pub draw_match_rate_delta: f64,
    pub unfinished_match_rate_delta: f64,
    pub average_rounds_delta: f64,
    pub average_actions_delta: f64,
    pub deck_win_rate_deltas: Vec<DeckWinRateDelta>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchEvaluationReport {
    pub patch_path: String,
    pub baseline: MatrixReport,
    pub patched: MatrixReport,
    pub delta: PatchEvaluationDelta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorDiagnostics {
    /// Fraction of finished matches where the operator was cast at least once.
    pub cast_rate: Option<f64>,
    /// Average round on which the operator was first cast (among matches where it was cast).
    pub average_first_cast_round: Option<f64>,
    /// Average number of times the operator was re-cast (cast_count - 1, among matches where cast).
    pub average_recast_count: Option<f64>,
    /// Average loyalty ability activations per finished match.
    pub average_loyalty_activations: Option<f64>,
    /// Average total operator-specific damage dealt per finished match.
    pub average_operator_damage: Option<f64>,
    /// Average tokens created per finished match.
    pub average_tokens_created: Option<f64>,
    /// Win rate when this operator's player was first (seat p1).
    pub first_seat_win_rate: Option<f64>,
    /// Win rate when this operator's player was second (seat p2).
    pub second_seat_win_rate: Option<f64>,
    /// Average game length (rounds) for finished matches involving this operator.
    pub average_game_length: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorMetrics {
    pub combo_label: String,
    pub deck_id: String,
    pub operator_id: String,
    pub matches: usize,
    pub finished_matches: usize,
    pub decisive_matches: usize,
    pub drawn_matches: usize,
    pub wins: usize,
    pub losses: usize,
    pub unfinished_matches: usize,
    pub win_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<OperatorDiagnostics>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorMetrics {
    pub color: String,
    pub matches: usize,
    pub finished_matches: usize,
    pub decisive_matches: usize,
    pub drawn_matches: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: Option<f64>,
    /// Spread between best and worst operator win rate within this color.
    pub intra_color_spread: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorMatchupMetrics {
    pub combo_a: String,
    pub combo_b: String,
    pub matches: usize,
    pub finished_matches: usize,
    pub decisive_matches: usize,
    pub drawn_matches: usize,
    pub unfinished_matches: usize,
    pub wins_a: usize,
    pub wins_b: usize,
    pub combo_a_win_rate: Option<f64>,
    pub first_player_wins: usize,
    pub first_player_win_rate: Option<f64>,
    pub average_rounds: f64,
    pub average_actions: f64,
    pub termination_counts: Vec<TerminationCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperatorMatrixReport {
    pub combos: Vec<String>,
    pub matches_per_pair: usize,
    pub alternate_seats: bool,
    pub include_mirrors: bool,
    pub operator_metrics: Vec<OperatorMetrics>,
    pub color_metrics: Vec<ColorMetrics>,
    pub matchup_metrics: Vec<OperatorMatchupMetrics>,
    pub balance: BalanceMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftColorMetrics {
    pub color: String,
    /// Total cards of this color across all drafted decks that played.
    pub total_cards_drafted: usize,
    /// Decisive matches where a deck with this as dominant color won.
    pub wins: usize,
    /// Decisive matches where a deck with this as dominant color lost.
    pub losses: usize,
    pub win_rate: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DraftMatrixReport {
    pub draft_count: usize,
    pub matches_per_draft: usize,
    pub total_matches: usize,
    pub color_metrics: Vec<DraftColorMetrics>,
    pub balance: BalanceMetrics,
}

pub fn action_label(action: &CardGameAction) -> String {
    match action {
        CardGameAction::PlayLand { card_id } => format!("play_land:{card_id}"),
        CardGameAction::DeployNode { mana_type } => format!("deploy_node:{mana_type:?}"),
        CardGameAction::TapCard { card_id } => format!("tap_card:{card_id}"),
        CardGameAction::CastSpell { card_id, targets } => {
            format!("cast_spell:{card_id}:{}", targets_label(targets))
        }
        CardGameAction::ActivateAbility {
            card_id,
            ability_id,
            targets,
        } => format!(
            "activate_ability:{card_id}:{ability_id}:{}",
            targets_label(targets)
        ),
        CardGameAction::DeclareAttack { card_id, target } => format!(
            "declare_attack:{card_id}:{}",
            target
                .as_ref()
                .map(target_label)
                .unwrap_or_else(|| "default".to_string())
        ),
        CardGameAction::DeclareBlock {
            attacker_id,
            blocker_id,
        } => format!("declare_block:{attacker_id}:{blocker_id}"),
        CardGameAction::ScryCommit { top, bottom } => {
            format!("scry_commit:{}:{}", top.join(","), bottom.join(","))
        }
        CardGameAction::SearchCommit { card_id } => format!("search_commit:{card_id}"),
        CardGameAction::SearchCancel => "search_cancel".to_string(),
        CardGameAction::TriggerChoice { accept } => format!("trigger_choice:{accept}"),
        CardGameAction::TriggerTargetSelect { target } => {
            format!("trigger_target_select:{}", target_label(target))
        }
        CardGameAction::TriggerTargetCancel => "trigger_target_cancel".to_string(),
        CardGameAction::DiscardCommit { card_ids } => {
            format!("discard_commit:{}", card_ids.join(","))
        }
        CardGameAction::Discard { card_id } => format!("discard:{card_id}"),
        CardGameAction::Sacrifice { card_id } => format!("sacrifice:{card_id}"),
        CardGameAction::ResolveStackTop => "resolve_stack_top".to_string(),
        CardGameAction::CancelStackTop => "cancel_stack_top".to_string(),
        CardGameAction::PassPriority => "pass_priority".to_string(),
        CardGameAction::NextStage => "next_stage".to_string(),
        CardGameAction::ConfirmAttackers => "confirm_attackers".to_string(),
        CardGameAction::ConfirmBlockers => "confirm_blockers".to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| format!("{other:?}")),
    }
}

fn targets_label(targets: &[crate::types::TargetRef]) -> String {
    if targets.is_empty() {
        return "none".to_string();
    }
    targets
        .iter()
        .map(target_label)
        .collect::<Vec<_>>()
        .join("|")
}

fn target_label(target: &crate::types::TargetRef) -> String {
    match target {
        crate::types::TargetRef::Player(id) => format!("player:{id}"),
        crate::types::TargetRef::Card(id) => format!("card:{id}"),
    }
}
