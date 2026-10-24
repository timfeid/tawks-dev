use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use game_framework::GameLogic;

use crate::action::CardGameAction;
use crate::state::CardGameState;
use crate::types::{CardGameStage, CardKind, Keyword};

use super::action_gen::legal_actions_for_player;
use super::agent::{
    Agent, AgentProfile, evaluate_state, fast_rollout_choose_action, profile_for_deck,
};
use super::determinize::determinize_state;
use super::report::action_label;
use super::runner::{next_actor_id, try_resolve_unfulfillable_pending};

const DEFAULT_EXPLORATION_CONSTANT: f64 = 1.25;
/// Per-move wall-clock budget. If MCTS hasn't returned by this point, fall back
/// to whatever the search has so far (or a heuristic). Picked at >10s so deep
/// branches still get a chance to converge before we cut.
const PER_MOVE_BUDGET: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub struct MctsConfig {
    pub iterations: usize,
    pub rollout_depth: usize,
    pub exploration_constant: f64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            iterations: 48,
            rollout_depth: 10,
            exploration_constant: DEFAULT_EXPLORATION_CONSTANT,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MctsAgent {
    profile: AgentProfile,
    config: MctsConfig,
}

impl MctsAgent {
    pub fn new(profile: AgentProfile, config: MctsConfig) -> Self {
        Self { profile, config }
    }
}

impl Agent for MctsAgent {
    fn profile(&self) -> AgentProfile {
        self.profile.clone()
    }

    fn choose_action(
        &self,
        state: &CardGameState,
        player_id: &str,
        legal_actions: &[CardGameAction],
    ) -> Option<CardGameAction> {
        if legal_actions.is_empty() {
            return None;
        }
        if legal_actions.len() == 1 {
            return legal_actions.first().cloned();
        }

        // Auto-skip MCTS for trivial priority-pass windows. When it's not
        // our turn AND we have no instant-speed cards/abilities to play,
        // just pass — running 48 MCTS iterations to confirm "yeah, pass"
        // is the dominant cost in 4-player operator matches.
        if let Some(pass_action) = trivial_priority_pass(state, player_id, legal_actions) {
            return Some(pass_action);
        }

        let iterations = self.config.iterations.max(1);
        let mut tree = SearchTree::new();
        synchronize_node(&mut tree, 0, player_id, legal_actions);

        let move_start = Instant::now();
        let mut completed_iterations = 0usize;
        for iteration in 0..iterations {
            let seed = determinization_seed(state, player_id, iteration);
            let mut determinized = determinize_state(state, player_id, seed);
            simulate_iteration(
                &mut tree,
                &mut determinized,
                player_id,
                &self.profile,
                &self.config,
            );
            completed_iterations = iteration + 1;
            if move_start.elapsed() > PER_MOVE_BUDGET {
                eprintln!(
                    "  [mcts] {} hit per-move budget after {} iterations ({:.1}s); falling back to partial search",
                    player_id,
                    completed_iterations,
                    move_start.elapsed().as_secs_f64()
                );
                break;
            }
        }

        let mut best: Option<(usize, f64, String, CardGameAction)> = None;
        for action in legal_actions {
            let stat = root_stats_for_action(&tree, action);
            let average_value = if stat.visits == 0 {
                f64::NEG_INFINITY
            } else {
                stat.total_value / stat.visits as f64
            };
            let label = action_label(action);

            match &best {
                Some((best_visits, best_average, best_label, _))
                    if stat.visits < *best_visits
                        || (stat.visits == *best_visits
                            && (average_value < *best_average
                                || (average_value == *best_average && label >= *best_label))) => {}
                _ => {
                    best = Some((stat.visits, average_value, label, action.clone()));
                }
            }
        }

        best.map(|(_, _, _, action)| action)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RootStats {
    visits: usize,
    total_value: f64,
}

#[derive(Clone, Debug, Default)]
struct SearchNode {
    actor_id: Option<String>,
    visits: usize,
    total_value: f64,
    children: Vec<SearchChild>,
}

#[derive(Clone, Debug)]
struct SearchChild {
    action: CardGameAction,
    label: String,
    child_index: Option<usize>,
    /// Number of determinizations in which this action was legal at this node.
    /// Used as the denominator in ISMCTS UCB exploration term.
    available_count: usize,
}

#[derive(Clone, Debug, Default)]
struct SearchTree {
    nodes: Vec<SearchNode>,
}

impl SearchTree {
    fn new() -> Self {
        Self {
            nodes: vec![SearchNode::default()],
        }
    }
}

fn simulate_iteration(
    tree: &mut SearchTree,
    state: &mut CardGameState,
    root_player_id: &str,
    root_profile: &AgentProfile,
    config: &MctsConfig,
) -> f64 {
    let mut current_node = 0usize;
    let mut path = vec![current_node];
    let mut remaining_depth = config.rollout_depth;

    while remaining_depth > 0 && state.stage != CardGameStage::Finished {
        let Some((actor_id, legal_actions)) = next_decision(state) else {
            break;
        };

        if !synchronize_node(tree, current_node, &actor_id, &legal_actions) {
            break;
        }

        let Some(child_slot) = select_child_slot(
            tree,
            current_node,
            root_player_id,
            &legal_actions,
            config.exploration_constant,
        ) else {
            break;
        };

        let action = tree.nodes[current_node].children[child_slot].action.clone();
        let child_index = ensure_child_node(tree, current_node, child_slot);

        if !advance_state(state, &actor_id, action) {
            backpropagate(tree, &path, f64::NEG_INFINITY);
            return f64::NEG_INFINITY;
        }

        remaining_depth = remaining_depth.saturating_sub(1);
        current_node = child_index;
        path.push(current_node);

        if tree.nodes[current_node].visits == 0 {
            break;
        }
    }

    let value = rollout(state, root_player_id, root_profile, remaining_depth);
    backpropagate(tree, &path, value);
    value
}

fn synchronize_node(
    tree: &mut SearchTree,
    node_index: usize,
    actor_id: &str,
    legal_actions: &[CardGameAction],
) -> bool {
    let node = &mut tree.nodes[node_index];
    match node.actor_id.as_deref() {
        Some(existing) if existing != actor_id => return false,
        Some(_) => {}
        None => node.actor_id = Some(actor_id.to_string()),
    }

    for action in legal_actions {
        let label = action_label(action);
        if let Some(child) = node.children.iter_mut().find(|child| child.label == label) {
            child.available_count += 1;
        } else {
            node.children.push(SearchChild {
                action: action.clone(),
                label,
                child_index: None,
                available_count: 1,
            });
        }
    }
    node.children
        .sort_by(|left, right| left.label.cmp(&right.label));
    true
}

fn select_child_slot(
    tree: &SearchTree,
    node_index: usize,
    root_player_id: &str,
    legal_actions: &[CardGameAction],
    exploration_constant: f64,
) -> Option<usize> {
    let node = &tree.nodes[node_index];

    let mut available_labels = legal_actions.iter().map(action_label).collect::<Vec<_>>();
    available_labels.sort();

    for label in &available_labels {
        if let Some((index, _)) = node
            .children
            .iter()
            .enumerate()
            .find(|(_, child)| child.label == *label && child.child_index.is_none())
        {
            return Some(index);
        }
    }

    let is_root_actor = node.actor_id.as_deref() == Some(root_player_id);
    let mut best: Option<(f64, &str, usize)> = None;

    for label in &available_labels {
        let Some((index, child)) = node
            .children
            .iter()
            .enumerate()
            .find(|(_, child)| child.label == *label)
        else {
            continue;
        };
        let Some(child_index) = child.child_index else {
            continue;
        };
        let child_node = &tree.nodes[child_index];
        if child_node.visits == 0 {
            return Some(index);
        }

        let mean_value = child_node.total_value / child_node.visits as f64;
        let exploitation = if is_root_actor {
            mean_value
        } else {
            -mean_value
        };
        // ISMCTS: use per-action availability count instead of total parent visits.
        // This correctly handles actions that are only legal in some determinizations.
        let availability = child.available_count.max(1) as f64;
        let exploration =
            exploration_constant * (availability.ln() / child_node.visits as f64).sqrt();
        let score = exploitation + exploration;

        match best {
            Some((best_score, best_label, _))
                if score < best_score
                    || (score == best_score && child.label.as_str() >= best_label) => {}
            _ => {
                best = Some((score, child.label.as_str(), index));
            }
        }
    }

    best.map(|(_, _, index)| index)
}

fn ensure_child_node(tree: &mut SearchTree, node_index: usize, child_slot: usize) -> usize {
    if let Some(child_index) = tree.nodes[node_index].children[child_slot].child_index {
        return child_index;
    }

    let child_index = tree.nodes.len();
    tree.nodes.push(SearchNode::default());
    tree.nodes[node_index].children[child_slot].child_index = Some(child_index);
    child_index
}

fn backpropagate(tree: &mut SearchTree, path: &[usize], value: f64) {
    for node_index in path {
        let node = &mut tree.nodes[*node_index];
        node.visits += 1;
        node.total_value += value;
    }
}

fn root_stats_for_action(tree: &SearchTree, action: &CardGameAction) -> RootStats {
    let label = action_label(action);
    let Some(root) = tree.nodes.first() else {
        return RootStats::default();
    };
    let Some(child) = root.children.iter().find(|child| child.label == label) else {
        return RootStats::default();
    };
    let Some(child_index) = child.child_index else {
        return RootStats::default();
    };
    let child_node = &tree.nodes[child_index];
    RootStats {
        visits: child_node.visits,
        total_value: child_node.total_value,
    }
}

fn rollout(
    state: &mut CardGameState,
    root_player_id: &str,
    root_profile: &AgentProfile,
    rollout_depth: usize,
) -> f64 {
    for _ in 0..rollout_depth {
        if state.stage == CardGameStage::Finished {
            break;
        }

        let Some((actor_id, legal_actions)) = next_decision(state) else {
            break;
        };

        // Use fast rollout policy: scores actions intrinsically without cloning state.
        // ArchetypeAgent's recursive lookahead is too expensive inside MCTS rollouts
        // (N² state clones per action choice).
        let Some(action) = fast_rollout_choose_action(state, &actor_id, &legal_actions)
            .or_else(|| legal_actions.first().cloned())
        else {
            break;
        };

        if !advance_state(state, &actor_id, action) {
            break;
        }
    }

    evaluate_state(state, root_player_id, root_profile)
}

fn next_decision(state: &mut CardGameState) -> Option<(String, Vec<CardGameAction>)> {
    loop {
        if state.stage == CardGameStage::Finished {
            return None;
        }

        let actor_id = next_actor_id(state)?;
        let legal_actions = legal_actions_for_player(state, &actor_id);
        if legal_actions.is_empty() && try_resolve_unfulfillable_pending(state, &actor_id) {
            continue;
        }
        if legal_actions.is_empty() {
            return None;
        }
        return Some((actor_id, legal_actions));
    }
}

fn advance_state(state: &mut CardGameState, actor_id: &str, action: CardGameAction) -> bool {
    if state.update(actor_id, action).is_err() {
        return false;
    }
    // Use apply_passive_state_updates (not tick) to avoid recursive AI simulation.
    state.apply_passive_state_updates();
    state.sim_force_resolve_pending();
    state.auto_pass_priority(false);
    true
}

fn rollout_profile_for_player(state: &CardGameState, player_id: &str) -> AgentProfile {
    state
        .players
        .get(player_id)
        .and_then(|player| player.deck_id.as_deref())
        .map(profile_for_deck)
        .unwrap_or(AgentProfile::Generalist)
}

fn determinization_seed(state: &CardGameState, player_id: &str, iteration: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    player_id.hash(&mut hasher);
    state.round.hash(&mut hasher);
    state.active_player_id.hash(&mut hasher);
    iteration.hash(&mut hasher);
    hasher.finish()
}

/// Detects "respond on the priority chain with nothing meaningful to do".
/// In 4-player operator matches, ~80% of all actions are pass_priority
/// chains — every spell triggers a 4-way priority pass. Running full MCTS
/// (48 iter × 10 depth = ~480 simulations) on each is the dominant cost,
/// even though the right answer is almost always "pass".
///
/// Returns Some(PassPriority) when:
/// - PassPriority is among the legal actions
/// - The actor is not the active player (not their turn)
/// - The actor has no instant-speed plays (no Instant cards in hand,
///   no Flash units in hand, no battlefield cards with non-tap-mana
///   activated abilities)
/// - The legal action set has no DeclareBlock (combat blocks are real
///   decisions even when not active player)
fn trivial_priority_pass(
    state: &CardGameState,
    player_id: &str,
    legal_actions: &[CardGameAction],
) -> Option<CardGameAction> {
    let pass = legal_actions
        .iter()
        .find(|a| matches!(a, CardGameAction::PassPriority))?
        .clone();

    let is_active = state.active_player_id.as_deref() == Some(player_id);
    if is_active {
        return None;
    }

    // Combat blocks are meaningful decisions
    if legal_actions.iter().any(|a| {
        matches!(
            a,
            CardGameAction::DeclareBlock { .. } | CardGameAction::ConfirmBlockers
        )
    }) {
        return None;
    }

    let player = state.players.get(player_id)?;

    // Any Instant or Flash card in hand → real decision
    let has_instant_in_hand = player.hand.iter().any(|card| {
        matches!(card.kind, CardKind::Instant) || card.keywords.contains(&Keyword::Flash)
    });
    if has_instant_in_hand {
        return None;
    }

    // Any battlefield card with a non-trivial activated ability → real decision
    // ("Trivial" = tap-only effect like adding mana. Non-trivial = anything
    // that targets, damages, draws, etc.)
    let has_meaningful_ability = player.battlefield.iter().any(|card| {
        card.abilities.iter().any(|ab| {
            // Skip simple tap-mana effects: those don't change much during
            // an opponent's priority pass and aren't worth MCTS time.
            !ab.is_spell_effect_ability() && !is_pure_mana_tap(&ab.effect)
        })
    });
    if has_meaningful_ability {
        return None;
    }

    Some(pass)
}

fn is_pure_mana_tap(effect: &crate::card::CardEffect) -> bool {
    use crate::card::CardEffect::*;
    match effect {
        AddMana(_, _) => true,
        Sequence(effects) => effects.iter().all(is_pure_mana_tap),
        _ => false,
    }
}
