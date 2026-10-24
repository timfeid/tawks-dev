use crate::types::GameFormat;

use super::agent::AgentProfile;

pub const DEFAULT_MATRIX_DECKS: &[&str] = &["neon", "glitch", "chrome", "slag", "bio"];

/// One participant (seat) in a match. Classic matches have 2; operator pods have 4–8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParticipantConfig {
    /// Player id used internally by the engine ("p1", "p2", ...).
    pub id: String,
    pub deck_id: String,
    pub agent: AgentProfile,
    /// Apex operator card id. Required for Operator format, None otherwise.
    pub operator: Option<String>,
    /// Whether this seat uses MCTS search instead of heuristic agent.
    pub uses_mcts: bool,
}

impl ParticipantConfig {
    /// Build a participant from legacy fields (deck_id + agent + operator + mcts flag).
    pub fn new(
        id: impl Into<String>,
        deck_id: impl Into<String>,
        agent: AgentProfile,
        operator: Option<String>,
        uses_mcts: bool,
    ) -> Self {
        Self {
            id: id.into(),
            deck_id: deck_id.into(),
            agent,
            operator,
            uses_mcts,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatchConfig {
    /// Seats in play, ordered by turn order. Classic requires 2; operator requires 4–8.
    pub participants: Vec<ParticipantConfig>,
    pub patch_path: Option<String>,
    pub mcts_iterations: usize,
    pub mcts_rollout_depth: usize,
    pub seed: u64,
    pub format: GameFormat,
    pub max_turns: usize,
    pub max_actions: usize,
    pub repeat_state_limit: usize,
    pub include_trace: bool,
    pub enable_sideboard: bool,
}

impl MatchConfig {
    /// Convenience accessors. Panics if out of range.
    pub fn participant(&self, index: usize) -> &ParticipantConfig {
        &self.participants[index]
    }
    pub fn participant_mut(&mut self, index: usize) -> &mut ParticipantConfig {
        &mut self.participants[index]
    }
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            participants: vec![
                ParticipantConfig::new("p1", "neon", AgentProfile::Neon, None, false),
                ParticipantConfig::new("p2", "chrome", AgentProfile::Chrome, None, false),
            ],
            patch_path: None,
            mcts_iterations: 48,
            mcts_rollout_depth: 10,
            seed: 1,
            format: GameFormat::Classic,
            max_turns: 80,
            max_actions: 1_500,
            repeat_state_limit: 6,
            include_trace: false,
            enable_sideboard: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GauntletConfig {
    pub base_match: MatchConfig,
    pub matches: usize,
    pub alternate_seats: bool,
}

impl Default for GauntletConfig {
    fn default() -> Self {
        Self {
            base_match: MatchConfig::default(),
            matches: 1,
            alternate_seats: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MatrixConfig {
    pub base_match: MatchConfig,
    pub decks: Vec<String>,
    pub matches_per_pair: usize,
    pub alternate_seats: bool,
    pub include_mirrors: bool,
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self {
            base_match: MatchConfig::default(),
            decks: DEFAULT_MATRIX_DECKS
                .iter()
                .map(|deck| deck.to_string())
                .collect(),
            matches_per_pair: 2,
            alternate_seats: true,
            include_mirrors: false,
        }
    }
}

/// A (deck, operator) combination used as a single participant in the operator matrix.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperatorCombo {
    pub deck_id: String,
    pub operator_id: String,
}

impl OperatorCombo {
    pub fn label(&self) -> String {
        format!("{}:{}", self.deck_id, self.operator_id)
    }
}

#[derive(Clone, Debug)]
pub struct OperatorMatrixConfig {
    pub base_match: MatchConfig,
    pub decks: Vec<String>,
    /// Optional filter: only include these operator IDs. Empty means all.
    pub operator_filter: Vec<String>,
    pub matches_per_pair: usize,
    pub alternate_seats: bool,
    pub include_mirrors: bool,
}

impl Default for OperatorMatrixConfig {
    fn default() -> Self {
        Self {
            base_match: MatchConfig {
                format: GameFormat::Operator,
                ..MatchConfig::default()
            },
            decks: DEFAULT_MATRIX_DECKS
                .iter()
                .map(|deck| deck.to_string())
                .collect(),
            operator_filter: Vec::new(),
            matches_per_pair: 2,
            alternate_seats: true,
            include_mirrors: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DraftMatrixConfig {
    pub base_match: MatchConfig,
    /// Number of distinct draft pools to generate (each produces 2 decks).
    pub draft_count: usize,
    /// Number of matches to play per draft pair.
    pub matches_per_draft: usize,
    pub alternate_seats: bool,
    pub base_seed: u64,
}

impl Default for DraftMatrixConfig {
    fn default() -> Self {
        Self {
            base_match: MatchConfig::default(),
            draft_count: 10,
            matches_per_draft: 4,
            alternate_seats: true,
            base_seed: 42,
        }
    }
}
