use game_framework::GameLogic;
use serde::{Deserialize, Serialize};

use crate::action::CardGameAction;
use crate::card::{Card, CardEffect};
use crate::state::CardGameState;
use crate::types::{
    CardGameStage, CardKind, ExpireContract, GameFormat, Keyword, StatType, TargetRef,
};

use super::report::action_label;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentProfile {
    Generalist,
    Neon,
    Glitch,
    Chrome,
    Slag,
    Bio,
}

pub trait Agent {
    fn profile(&self) -> AgentProfile;
    fn choose_action(
        &self,
        state: &CardGameState,
        player_id: &str,
        legal_actions: &[CardGameAction],
    ) -> Option<CardGameAction>;
}

#[derive(Clone, Debug)]
pub struct ArchetypeAgent {
    profile: AgentProfile,
}

impl ArchetypeAgent {
    pub fn new(profile: AgentProfile) -> Self {
        Self { profile }
    }
}

/// Maximum lookahead depth for the agent's search.
/// Depth 0 = original 1-step lookahead. Depth 2 = 3-step lookahead.
const LOOKAHEAD_DEPTH: usize = 1;

/// Fast rollout policy for MCTS — score actions cheaply without cloning state.
/// Scores by action shape + card intrinsic value. ~100x faster than ArchetypeAgent.
pub fn fast_rollout_choose_action(
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
    let mut best: Option<(f64, &CardGameAction)> = None;
    for action in legal_actions {
        let score = score_action_intrinsic(state, player_id, action);
        match best {
            Some((s, _)) if score <= s => {}
            _ => best = Some((score, action)),
        }
    }
    best.map(|(_, a)| a.clone())
}

fn score_action_intrinsic(state: &CardGameState, _player_id: &str, action: &CardGameAction) -> f64 {
    use crate::action::CardGameAction::*;
    match action {
        DeclareAttack { .. } => 8.0,
        CastSpell { card_id, .. } => {
            let intrinsic = find_card(state, card_id)
                .map(card_intrinsic_value)
                .unwrap_or(0.0);
            10.0 + intrinsic * 2.0
        }
        ActivateAbility {
            card_id,
            ability_id,
            ..
        } => {
            if let Some(card) = find_card(state, card_id) {
                if let Some(ab) = card.abilities.iter().find(|a| &a.id == ability_id) {
                    return 5.0 + score_ability_effect(&ab.effect);
                }
            }
            5.0
        }
        PlayLand { .. } => 6.0,
        DeclareBlock { .. } => 4.0,
        _Pass => -1.0,
        _EndPhase => -0.5,
        _ => 0.0,
    }
}

/// Maximum number of actions to consider at each depth level to keep search tractable.
/// At depth 0 (top level) we consider all actions. At deeper levels we cap to avoid explosion.
const DEEP_ACTION_LIMIT: usize = 8;

impl Agent for ArchetypeAgent {
    fn profile(&self) -> AgentProfile {
        self.profile.clone()
    }

    fn choose_action(
        &self,
        state: &CardGameState,
        player_id: &str,
        legal_actions: &[CardGameAction],
    ) -> Option<CardGameAction> {
        let (_, _, action) = best_action_recursive(
            state,
            player_id,
            legal_actions,
            &self.profile,
            LOOKAHEAD_DEPTH,
        )?;
        Some(action)
    }
}

/// Recursively find the best action with multi-step lookahead.
/// Returns (score, label, action) for the best immediate action.
fn best_action_recursive(
    state: &CardGameState,
    player_id: &str,
    legal_actions: &[CardGameAction],
    profile: &AgentProfile,
    depth: usize,
) -> Option<(f64, String, CardGameAction)> {
    use super::action_gen::legal_actions_for_player;

    let context = ActionContext::from_actions(legal_actions);
    let mut best: Option<(f64, String, CardGameAction)> = None;

    for action in legal_actions {
        let mut probe = state.clone();
        if probe.update(player_id, action.clone()).is_err() {
            continue;
        }
        // NOTE: use apply_passive_state_updates (not tick) to avoid recursive AI
        // simulation — probe.tick() would call collect_ai_tick_actions, which
        // re-enters the agent's choose_action on a deeper probe, causing
        // exponential blowup and multi-second ticks.
        probe.apply_passive_state_updates();
        // Force-resolve the stack/showcase in the probe so the AI can see
        // the effect of the action it's evaluating. Without this, casting a
        // bounce/counter against any live opponent hand would leave the
        // probe mid-resolution, making every target choice look identical.
        probe.sim_force_resolve_pending();
        probe.auto_pass_priority(false);

        // Split the action's heuristic bonus from the raw state evaluation
        // so we can propagate the bonus through any deeper search — otherwise
        // penalties like "don't target yourself with removal" vanish at
        // depth > 0 because we'd return the subtree score directly.
        let action_bonus = score_action(state, &probe, player_id, action, &context, profile);
        let immediate_score = evaluate_state(&probe, player_id, profile) + action_bonus;

        // Look deeper if we have depth remaining and it's still our turn
        let score = if depth > 0 {
            let next_actions = legal_actions_for_player(&probe, player_id);
            if next_actions.is_empty() {
                immediate_score
            } else {
                // At deeper levels, pre-score and only explore the top candidates
                let actions_to_explore = if next_actions.len() > DEEP_ACTION_LIMIT {
                    let next_context = ActionContext::from_actions(&next_actions);
                    let mut scored: Vec<(f64, &CardGameAction)> = next_actions
                        .iter()
                        .map(|a| {
                            let mut p = probe.clone();
                            let s = if p.update(player_id, a.clone()).is_ok() {
                                p.apply_passive_state_updates();
                                p.sim_force_resolve_pending();
                                p.auto_pass_priority(false);
                                evaluate_state(&p, player_id, profile)
                                    + score_action(&probe, &p, player_id, a, &next_context, profile)
                            } else {
                                f64::NEG_INFINITY
                            };
                            (s, a)
                        })
                        .collect();
                    scored
                        .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                    scored
                        .into_iter()
                        .take(DEEP_ACTION_LIMIT)
                        .map(|(_, a)| a.clone())
                        .collect::<Vec<_>>()
                } else {
                    next_actions
                };

                if let Some((deeper_score, _, _)) = best_action_recursive(
                    &probe,
                    player_id,
                    &actions_to_explore,
                    profile,
                    depth - 1,
                ) {
                    // Carry the current action's heuristic bonus up so things
                    // like the self-remember penalty or "cast a unit" reward
                    // still influence the decision after deeper search.
                    action_bonus + deeper_score
                } else {
                    immediate_score
                }
            }
        } else {
            immediate_score
        };

        let label = action_label(action);
        match &best {
            Some((best_score, best_label, _))
                if score < *best_score || (score == *best_score && label >= *best_label) => {}
            _ => {
                best = Some((score, label, action.clone()));
            }
        }
    }

    best
}

pub fn profile_for_deck(deck_id: &str) -> AgentProfile {
    match deck_id {
        "neon" => AgentProfile::Neon,
        "glitch" => AgentProfile::Glitch,
        "chrome" => AgentProfile::Chrome,
        "slag" => AgentProfile::Slag,
        "bio" => AgentProfile::Bio,
        // Multi-color themed decks: match the dominant color's play style.
        // rush is slag/neon aggro → Slag profile (prioritizes damage).
        // takeover is bio/glitch control → Glitch profile (prioritizes
        // discard/interaction). fortress is chrome/bio scaling → Chrome
        // (prioritizes artifact synergy).
        "rush" => AgentProfile::Slag,
        "takeover" => AgentProfile::Glitch,
        "fortress" => AgentProfile::Chrome,
        // Syndicate families borrow the closest Mana Clash play style.
        "syn_old_blood" => AgentProfile::Chrome,
        "syn_street" => AgentProfile::Neon,
        "syn_cartel" => AgentProfile::Bio,
        "syn_firm" => AgentProfile::Glitch,
        "syn_wolves" => AgentProfile::Slag,
        _ => AgentProfile::Generalist,
    }
}

pub(crate) fn evaluate_state(
    state: &CardGameState,
    player_id: &str,
    profile: &AgentProfile,
) -> f64 {
    if is_winner(state, player_id) {
        return 1_000_000.0 - state.round as f64 * 100.0;
    }
    if !state.players.contains_key(player_id) {
        return -1_000_000.0 + state.round as f64 * 100.0;
    }

    let Some(player) = state.players.get(player_id) else {
        return -1_000_000.0;
    };

    let opponents: Vec<_> = state
        .turn_order
        .iter()
        .filter(|id| id.as_str() != player_id)
        .filter_map(|id| state.players.get(id))
        .collect();

    let opponent_life: i32 = opponents.iter().map(|opponent| opponent.life).sum();
    let opponent_hand: usize = opponents.iter().map(|opponent| opponent.hand.len()).sum();
    let opponent_board: usize = opponents
        .iter()
        .map(|opponent| opponent.battlefield.len())
        .sum();
    let opponent_board_power: i32 = opponents
        .iter()
        .flat_map(|opponent| opponent.battlefield.iter())
        .map(|card| effective_power(state, card))
        .sum();
    let opponent_board_toughness: i32 = opponents
        .iter()
        .flat_map(|opponent| opponent.battlefield.iter())
        .map(|card| effective_toughness(state, card))
        .sum();

    let own_units = player
        .battlefield
        .iter()
        .filter(|card| matches!(card.kind, CardKind::Unit(_)))
        .count() as f64;
    let own_artifacts = player
        .battlefield
        .iter()
        .filter(|card| matches!(card.kind, CardKind::Relic | CardKind::Infrastructure))
        .count() as f64;
    let own_counters = player
        .battlefield
        .iter()
        .map(|card| card.counters.max(0) as f64)
        .sum::<f64>();
    let own_available_mana = mana_pool_total(&state.available_mana_for_player(player_id)) as f64;
    let own_mana_sources = mana_source_count(player) as f64;
    let own_board_power = board_power(state, player_id);
    let own_board_toughness = board_toughness(state, player_id);
    let own_attack_ready_power = player
        .battlefield
        .iter()
        .filter(|card| state.can_attack_card(player_id, card))
        .map(|card| effective_power(state, card) as f64)
        .sum::<f64>();
    let hand_overflow = player.hand.len().saturating_sub(state.max_hand_size()) as f64;

    // Keyword-value aggregates — makes evaluate_state sensitive to
    // threats that the crude power+toughness sum misses (a 2/2
    // Deathtouch is a bigger problem than a 2/2 vanilla).
    let own_keyword_value: f64 = player
        .battlefield
        .iter()
        .flat_map(|c| c.keywords.iter())
        .map(keyword_value)
        .sum();
    let opponent_keyword_value: f64 = opponents
        .iter()
        .flat_map(|opp| opp.battlefield.iter())
        .flat_map(|c| c.keywords.iter())
        .map(keyword_value)
        .sum();

    let delayed_trigger_bias = delayed_trigger_bias(state, player_id);

    // Opponent-hand pressure: if the opponent still has plenty of cards,
    // they're likely sitting on removal / counters / instants. Give a
    // modest reward to keeping our own mana available on their turn, so
    // we don't tap out into a probable response.
    let is_our_turn = state.active_player_id.as_deref() == Some(player_id);
    let opp_hand_pressure = opponent_hand as f64;
    let opp_response_bias = if !is_our_turn {
        // On their turn: having mana up is good (response).
        own_available_mana.min(4.0) * 1.5
    } else {
        // On our turn: emptying our mana against a hand-heavy opponent
        // is slightly risky — they're more likely to have responses.
        -own_available_mana.min(2.0) * (opp_hand_pressure / 7.0).min(1.5)
    };

    let base = (player.life - opponent_life) as f64 * 10.0
        + (player.hand.len() as f64 - opponent_hand as f64) * 2.0
        + (player.battlefield.len() as f64 - opponent_board as f64) * 2.0
        + (own_board_power - opponent_board_power as f64) * 6.5
        + (own_board_toughness - opponent_board_toughness as f64) * 2.0
        + (own_keyword_value - opponent_keyword_value) * 2.0
        + (20.0 * opponents.len() as f64 - opponent_life as f64) * 4.0
        + own_mana_sources * 0.75
        + own_attack_ready_power * 1.5
        - hand_overflow * 15.0
        - own_available_mana * 0.25
        + delayed_trigger_bias
        + opp_response_bias;

    let archetype = match profile {
        AgentProfile::Generalist => 0.0,
        AgentProfile::Neon => {
            (20.0 - opponent_life as f64) * 12.0 + own_attack_ready_power * 6.0 + own_units * 2.0
        }
        AgentProfile::Glitch => {
            (opponents.len() as f64 * 7.0 - opponent_hand as f64) * 6.0
                + player.hand.len() as f64 * 2.0
        }
        AgentProfile::Chrome => {
            own_artifacts * 6.0 + own_board_toughness * 2.0 + player.battlefield.len() as f64 * 2.0
        }
        AgentProfile::Slag => {
            (20.0 * opponents.len() as f64 - opponent_life as f64) * 8.0
                + player.graveyard.len() as f64 * 2.0
                + own_units * 2.0
                + own_board_power * 3.0
        }
        AgentProfile::Bio => {
            own_units * 2.0 + own_counters * 1.0 + own_mana_sources * 1.0 + own_board_power * 1.5
        }
    };

    let stack_pressure =
        if !state.stack.is_empty() && state.effective_priority_holder() == Some(player_id) {
            1.5
        } else {
            0.0
        };

    let format_bonus = match state.format {
        GameFormat::Classic => 0.0,
        GameFormat::Operator => {
            // Reward having operator on battlefield, not sitting in zone
            let operator_on_board = player
                .operator_id
                .as_ref()
                .map(|op_id| player.battlefield.iter().any(|c| c.id == *op_id))
                .unwrap_or(false);
            if operator_on_board { 15.0 } else { 0.0 }
        }
    };

    base + archetype + stack_pressure + format_bonus
}

fn is_winner(state: &CardGameState, player_id: &str) -> bool {
    state.stage == crate::types::CardGameStage::Finished
        && state.turn_order.len() == 1
        && state.turn_order.first().map(String::as_str) == Some(player_id)
}

fn mana_pool_total(pool: &crate::mana::ManaPool) -> i32 {
    pool.neon + pool.glitch + pool.chrome + pool.slag + pool.bio + pool.colorless
}

#[derive(Clone, Copy, Debug, Default)]
struct ActionContext {
    cast_actions: usize,
    ability_actions: usize,
    attack_actions: usize,
    block_actions: usize,
}

impl ActionContext {
    fn from_actions(actions: &[CardGameAction]) -> Self {
        let mut context = Self::default();
        for action in actions {
            match action {
                CardGameAction::CastSpell { .. } => context.cast_actions += 1,
                CardGameAction::ActivateAbility { .. } => context.ability_actions += 1,
                CardGameAction::DeclareAttack { .. } => context.attack_actions += 1,
                CardGameAction::DeclareBlock { .. } => context.block_actions += 1,
                _ => {}
            }
        }
        context
    }
}

fn score_action(
    before: &CardGameState,
    after: &CardGameState,
    player_id: &str,
    action: &CardGameAction,
    context: &ActionContext,
    profile: &AgentProfile,
) -> f64 {
    let before_player = match before.players.get(player_id) {
        Some(player) => player,
        None => return 0.0,
    };
    let after_player = after.players.get(player_id);

    let before_hand = before_player.hand.len() as f64;
    let after_hand = after_player
        .map(|player| player.hand.len() as f64)
        .unwrap_or(0.0);
    let before_overflow = before_player
        .hand
        .len()
        .saturating_sub(before.max_hand_size()) as f64;
    let after_overflow = after_player
        .map(|player| player.hand.len().saturating_sub(after.max_hand_size()) as f64)
        .unwrap_or(0.0);

    let before_opponent_life = total_opponent_life(before, player_id) as f64;
    let after_opponent_life = total_opponent_life(after, player_id) as f64;
    let before_own_life = before_player.life as f64;
    let after_own_life = after_player.map(|player| player.life as f64).unwrap_or(0.0);
    let before_board_power = board_power(before, player_id);
    let after_board_power = board_power(after, player_id);
    let before_board_size = before_player.battlefield.len() as f64;
    let after_board_size = after_player
        .map(|player| player.battlefield.len() as f64)
        .unwrap_or(0.0);

    // Face-damage scoring scales with how close we are to lethal: early
    // burn (opp at 20 life) is worth less per-point than closing burn
    // (opp at 5 life). Without this, the AI bleeds Arc Discharge to face
    // while a Bio value engine snowballs on the board.
    let opp_life_after = after
        .players
        .iter()
        .filter(|(pid, _)| pid.as_str() != player_id)
        .map(|(_, p)| p.life as f64)
        .sum::<f64>()
        .max(1.0);
    let face_damage_weight = if opp_life_after <= 6.0 {
        38.0
    } else if opp_life_after <= 12.0 {
        24.0
    } else {
        16.0
    };
    let mut bonus = (before_opponent_life - after_opponent_life) * face_damage_weight
        + (after_own_life - before_own_life) * 12.0
        + (before_hand - after_hand) * 7.0
        + (after_board_power - before_board_power) * 5.0
        + (after_board_size - before_board_size) * 3.0
        + (before_overflow - after_overflow) * 20.0;

    match action {
        CardGameAction::CastSpell { card_id, targets } => {
            if let Some(card) = find_card(before, card_id) {
                bonus += card_intrinsic_value(card) * 2.5;
                if matches!(card.kind, CardKind::Unit(_) | CardKind::Agent) {
                    bonus += 8.0;
                }
                // Strong bonus for casting operator from operator zone
                let is_operator = before_player
                    .operator_id
                    .as_ref()
                    .map(|op_id| *op_id == card.id)
                    .unwrap_or(false);
                if is_operator {
                    bonus += 50.0;
                }

                // (Delayed-trigger punishment from remembered-player
                // cards — Data Recovery Matrix, Bitrot — is now handled
                // generically in `evaluate_state` via
                // `delayed_trigger_bias`. The probe resolves the cast,
                // sets `remembered_player_id`, and the state eval
                // directly sees "this punishes me" vs "this punishes
                // them" without needing a per-card hook.)

                // Tiebreaker bias: when raw scoring is symmetric between
                // own and opponent targets (e.g. a bounce that could hit
                // either side looks the same to the crude state eval),
                // prefer the opponent. Without this, ASCII ordering of
                // player ids makes ties resolve to self (p1 < p2).
                for target in targets {
                    match target {
                        TargetRef::Card(cid) => {
                            let on_opp_board =
                                state_opponent_battlefield_has(before, player_id, cid);
                            let on_own_board =
                                before_player.battlefield.iter().any(|c| c.id == *cid);
                            if on_opp_board {
                                bonus += 0.5;
                            } else if on_own_board {
                                bonus -= 0.5;
                            }
                        }
                        TargetRef::Player(pid) => {
                            if pid != player_id {
                                bonus += 0.5;
                            } else {
                                bonus -= 0.5;
                            }
                        }
                    }
                }

                // Instant-speed hold-back: during our own main phase with
                // an empty stack and no live combat, a reactive instant
                // (counter / removal) usually wants to wait for the
                // opponent's turn — they might not cast the thing we're
                // trying to answer, or they might cast something bigger.
                // Casting proactive / value instants (cantrip draw, ramp)
                // is still fine.
                let is_active_main = matches!(
                    before.stage,
                    CardGameStage::Main | CardGameStage::PostCombatMain
                ) && before.active_player_id.as_deref() == Some(player_id);
                let stack_quiet = before.stack.is_empty() && before.showcase.is_none();
                if matches!(card.kind, CardKind::Instant)
                    && is_active_main
                    && stack_quiet
                    && spell_is_reactive(card)
                {
                    bonus -= 8.0;
                }

                // Counter-spell value gating: only spend a counter on
                // something worth countering. If the top of the stack is
                // a cheap cantrip (low intrinsic value), penalize burning
                // a counter on it. If it's a big threat, reward.
                let is_counter = spell_is_counter(card);
                if is_counter {
                    if let Some(top) = before.stack.last() {
                        // Never counter your own spells.
                        if top.owner_id == player_id {
                            bonus -= 80.0;
                        } else if let Some(top_card) = stack_item_card(top) {
                            let target_value = card_intrinsic_value(top_card);
                            let our_cost = card_intrinsic_value(card);
                            // Reward countering threats bigger than our counter,
                            // penalize countering cheap stuff.
                            if target_value >= our_cost + 4.0 {
                                bonus += 25.0;
                            } else if target_value < 6.0 {
                                bonus -= 30.0;
                            }
                        }
                    } else {
                        // Nothing on the stack — casting a counter at nothing
                        // is wasteful (it still draws if it cantrips, but
                        // the counter part is burned).
                        bonus -= 40.0;
                    }
                }

                // Friendly-fire guard AND threat-scoring for opponent
                // targets. The crude state eval can miss that a hostile
                // effect on your own unit is a 2-for-0 (especially when
                // the spell bundles the removal with a bonus like draw or
                // token creation), and doesn't strongly differentiate
                // between killing a 5/5 vs a 1/1. We handle both here by
                // walking the spell's effect chain.
                let spell_effect = spell_effect_for_card(card);
                if let Some(effect) = spell_effect {
                    let hostile = hostile_effect_penalty(effect);
                    let damage = damage_effect_amount(effect);
                    for target in targets {
                        match target {
                            TargetRef::Card(cid) => {
                                if let Some(owner_pid) = find_battlefield_owner(before, cid) {
                                    if owner_pid == player_id {
                                        if hostile > 0.0 {
                                            bonus -= hostile;
                                        }
                                    } else if hostile > 0.0 {
                                        // Threat-score: prefer big threats.
                                        if let Some(target_card) =
                                            find_battlefield_card(before, cid)
                                        {
                                            let power = effective_power(before, target_card) as f64;
                                            let toughness =
                                                effective_toughness(before, target_card) as f64;
                                            bonus += power * 5.0 + toughness * 2.5;
                                            // Operator (planeswalker-class) targets:
                                            // power/toughness underrates them since
                                            // Agents typically have low stats but
                                            // generate value every turn through
                                            // loyalty-scaling abilities. Score by
                                            // remaining loyalty + ability menace,
                                            // gated so non-threatening Agents don't
                                            // get over-prioritized.
                                            if matches!(target_card.kind, CardKind::Agent) {
                                                let loyalty =
                                                    target_card.loyalty.unwrap_or(0) as f64;
                                                bonus += loyalty * 3.0;
                                                if loyalty >= 4.0
                                                    && has_damaging_activated_ability(target_card)
                                                {
                                                    bonus += 20.0;
                                                }
                                                if matches!(
                                                    before.format,
                                                    GameFormat::Operator
                                                ) {
                                                    bonus += 12.0;
                                                }
                                            }
                                            // Unblockable bodies (Stealth/Aero with
                                            // no blocker check) lock in chip damage
                                            // every turn. Higher kill priority than
                                            // raw stats suggest.
                                            if target_card.keywords.contains(&Keyword::Stealth)
                                                || target_card.keywords.contains(&Keyword::Aero)
                                            {
                                                bonus += 25.0 + power * 4.0;
                                            }
                                            // Efficient removal: damage
                                            // that would be lethal gets an
                                            // extra kicker; overkill
                                            // (damage >> toughness) is
                                            // wasted.
                                            if damage > 0.0 {
                                                if damage >= toughness {
                                                    bonus += 20.0;
                                                    // Avoid gratuitous overkill
                                                    bonus -= (damage - toughness) * 1.0;
                                                } else {
                                                    // Didn't kill it — penalize
                                                    // vs. finding a lethal option.
                                                    bonus -= 4.0;
                                                }
                                            }
                                            // Units that are attacking now
                                            // are immediate threats.
                                            if before.combat_attackers.iter().any(|a| a == cid) {
                                                bonus += 10.0;
                                            }
                                            // Snowball engines (units with
                                            // cast / enter / death triggers)
                                            // must be killed fast, otherwise
                                            // they generate value every turn.
                                            // This matters especially against
                                            // Bio whose Fauna Whisperer /
                                            // Grove Huntmaster / Thornback
                                            // Riders run away with the game.
                                            if is_snowball_engine(target_card) {
                                                // Kill engines FAST — left
                                                // alone, these win the game
                                                // in 2-3 turns. The state
                                                // eval only reads the 2/3
                                                // of lost board power, not
                                                // the 3-5 cards / turn this
                                                // unit would have generated,
                                                // so the weight here needs
                                                // to account for the hidden
                                                // future value.
                                                bonus += 90.0;
                                            }
                                        }
                                    }
                                }
                            }
                            TargetRef::Player(pid) => {
                                if pid == player_id && hostile > 0.0 {
                                    bonus -= hostile;
                                }
                            }
                        }
                    }
                }
            }
            if before_overflow > 0.0 {
                bonus += 12.0;
            }
        }
        CardGameAction::ActivateAbility {
            card_id,
            ability_id,
            targets,
        } => {
            if let Some(card) = find_card(before, card_id) {
                bonus += if matches!(card.kind, CardKind::Agent) {
                    10.0
                } else {
                    4.0
                };

                // Effect-aware bonus: distinguish strong abilities (mass
                // damage, reanimate, search-to-battlefield) from weak ones
                // (just +1 Scry). Without this the AI scored every ability
                // identically and underused complex agents.
                if let Some(ability) = card.abilities.iter().find(|a| &a.id == ability_id) {
                    bonus += score_ability_effect(&ability.effect);
                }

                // Target-aware threat scoring for damaging / destroying
                // abilities (e.g. Tezzeret [-3] Damage 3, Pyra [+1] Damage 1).
                // Mirrors the CastSpell threat block: prefer big threats,
                // operators, and Stealth/Aero chip attackers as targets.
                if let Some(ability) = card.abilities.iter().find(|a| &a.id == ability_id) {
                    if effect_is_threatening(&ability.effect) {
                        let damage = damage_effect_amount(&ability.effect);
                        for target in targets {
                            let TargetRef::Card(target_id) = target else {
                                continue;
                            };
                            let Some(target_owner_id) =
                                find_battlefield_owner(before, target_id)
                            else {
                                continue;
                            };
                            if target_owner_id == player_id {
                                // Friendly-fire on our own permanent.
                                bonus -= 60.0;
                                continue;
                            }
                            let Some(target_card) = find_battlefield_card(before, target_id)
                            else {
                                continue;
                            };
                            let power = effective_power(before, target_card) as f64;
                            let toughness = effective_toughness(before, target_card) as f64;
                            bonus += power * 4.0 + toughness * 2.0;
                            if damage > 0.0 {
                                if damage >= toughness {
                                    bonus += 18.0;
                                } else {
                                    bonus -= 3.0;
                                }
                            }
                            if matches!(target_card.kind, CardKind::Agent) {
                                let loyalty = target_card.loyalty.unwrap_or(0) as f64;
                                bonus += loyalty * 3.0;
                                if loyalty >= 4.0
                                    && has_damaging_activated_ability(target_card)
                                {
                                    bonus += 18.0;
                                }
                                if matches!(before.format, GameFormat::Operator) {
                                    bonus += 10.0;
                                }
                            }
                            if target_card.keywords.contains(&Keyword::Stealth)
                                || target_card.keywords.contains(&Keyword::Aero)
                            {
                                bonus += 20.0 + power * 3.0;
                            }
                            if before.combat_attackers.iter().any(|a| a == target_id) {
                                bonus += 8.0;
                            }
                            if is_snowball_engine(target_card) {
                                bonus += 70.0;
                            }
                        }
                    }
                }

                // Smarter scoring for "target unit gains keyword until EOT"
                // abilities (e.g. Axiom's Grant Stealth). Without this the
                // AI activates them in post-combat main where the buff
                // expires before it matters.
                if let Some(ability) = card.abilities.iter().find(|a| &a.id == ability_id) {
                    if let Some((keyword, expires)) = grant_keyword_info(&ability.effect) {
                        let is_eot = matches!(expires, ExpireContract::EndOfTurn);
                        let is_own_turn = before
                            .active_player_id
                            .as_deref()
                            .is_some_and(|pid| pid == player_id);
                        let has_attackers = !before.combat_attackers.is_empty();

                        // Collect the targeted unit(s) info (owner, tapped,
                        // existing keywords, power).
                        for target in targets {
                            let TargetRef::Card(target_id) = target else {
                                continue;
                            };
                            let (target_owner, target_card) = before
                                .players
                                .iter()
                                .find_map(|(pid, p)| {
                                    p.battlefield
                                        .iter()
                                        .find(|c| &c.id == target_id)
                                        .map(|c| (pid.clone(), c))
                                })
                                .map_or((String::new(), None), |(p, c)| (p, Some(c)));

                            let Some(tgt) = target_card else {
                                bonus -= 30.0;
                                continue;
                            };
                            let target_is_ours = target_owner == player_id;
                            let target_already_has = tgt.keywords.contains(&keyword);
                            let target_power = effective_power(before, tgt);

                            // Offensive keywords (unblockable etc.) on our
                            // own attackers = good; on enemies = nonsense.
                            let offensive = matches!(
                                keyword,
                                Keyword::Stealth
                                    | Keyword::Aero
                                    | Keyword::Trample
                                    | Keyword::Haste
                                    | Keyword::Deathtouch
                                    | Keyword::Piercing
                            );

                            if offensive && !target_is_ours {
                                // Buffing the enemy — never useful.
                                bonus -= 60.0;
                                continue;
                            }

                            if target_already_has {
                                // Redundant — no value.
                                bonus -= 25.0;
                                continue;
                            }

                            if offensive && target_is_ours {
                                if is_eot {
                                    if is_own_turn
                                        && has_attackers
                                        && before.combat_attackers.contains(&tgt.id)
                                    {
                                        // Perfect — our attacker becomes
                                        // unblockable etc. this combat.
                                        bonus += 14.0 + target_power as f64 * 2.0;
                                    } else if is_own_turn
                                        && !has_attackers
                                        && matches!(before.stage, CardGameStage::Main)
                                        && !tgt.tapped
                                        && !(tgt.summoning_sickness && !tgt.haste)
                                    {
                                        // Pre-combat on a unit that could attack.
                                        bonus += 6.0 + target_power as f64 * 0.5;
                                    } else {
                                        // Post-combat, opponent's turn, or
                                        // target can't attack this turn —
                                        // EOT buff wasted.
                                        bonus -= 40.0;
                                    }
                                } else {
                                    // Permanent grant — always has value.
                                    bonus += 10.0 + target_power as f64;
                                }
                            } else if !offensive && target_is_ours {
                                // Defensive (indestructible etc.) on our unit
                                // is situationally good — small default bonus.
                                bonus += 4.0;
                            }
                        }
                    }
                }
            }
        }
        CardGameAction::DeclareAttack { card_id, target } => {
            let attacker = find_battlefield_card(before, card_id);
            let power = attacker
                .map(|card| effective_power(before, card) as f64)
                .unwrap_or(1.0);
            let profile_bonus = match profile {
                AgentProfile::Neon => 10.0,
                AgentProfile::Slag => 8.0,
                AgentProfile::Bio => 4.0,
                _ => 2.0,
            };
            bonus += 14.0 + power * 7.0 + profile_bonus;

            // Target-aware bonus. The base score above treats every attack
            // as "swing at default defender." When MCTS has chosen an
            // explicit target — an opposing operator (drain loyalty) or a
            // specific opponent in a pod — we score that choice on its
            // merits.
            if let (Some(attacker), Some(t)) = (attacker, target.as_ref()) {
                let kw = &attacker.keywords;
                let evasive = kw.contains(&Keyword::Stealth) || kw.contains(&Keyword::Aero);
                match t {
                    TargetRef::Card(target_id) => {
                        if let Some(operator) = find_battlefield_card(before, target_id) {
                            if matches!(operator.kind, CardKind::Agent) {
                                let loyalty = operator.loyalty.unwrap_or(0) as f64;
                                // Base value of swinging at an operator —
                                // every point of loyalty drained is a step
                                // toward removing a planeswalker-class
                                // threat. Scale stronger when the attack
                                // is lethal-to-loyalty (kills outright).
                                let lethal = power >= loyalty && loyalty > 0.0;
                                if lethal {
                                    bonus += 35.0 + loyalty * 2.0;
                                } else {
                                    bonus += 6.0 + power.min(loyalty) * 3.0;
                                }
                                // Evasive attackers can't be blocked, so
                                // the loyalty drain is guaranteed.
                                if evasive {
                                    bonus += 12.0;
                                }
                                // Operators with damage on +1 / strong
                                // ultimates accumulate value every turn —
                                // killing them faster is higher priority,
                                // but only when they're actually built up.
                                if loyalty >= 4.0 && has_damaging_activated_ability(operator) {
                                    bonus += 14.0;
                                }
                            }
                        }
                    }
                    TargetRef::Player(target_pid) => {
                        // In a pod, picking a specific opponent gets a
                        // small bonus when that opponent has the lowest
                        // life or has been receiving commander damage from
                        // us (finish the kill).
                        if let Some(target_player) = before.players.get(target_pid) {
                            let life = target_player.life as f64;
                            // Lower life = better target.
                            bonus += (20.0 - life).max(0.0) * 0.8;
                            // Commander damage already dealt: lean in.
                            let cd = target_player
                                .operator_damage
                                .get(player_id)
                                .copied()
                                .unwrap_or(0)
                                as f64;
                            if cd > 0.0 {
                                bonus += cd * 1.5;
                            }
                        }
                    }
                }
            }

            // Offensive-keyword bonuses: Trample profits from unblocked
            // excess damage, Menace forces double-blocks, Vigilance means
            // we don't tap (still available for defense), Piercing
            // bypasses armor/shield, Rampage pings the defender per
            // attack.
            if let Some(attacker) = attacker {
                let kw = &attacker.keywords;
                if kw.contains(&Keyword::Trample) {
                    bonus += power * 2.0;
                }
                if kw.contains(&Keyword::Menace) {
                    bonus += 6.0;
                }
                if kw.contains(&Keyword::Vigilance) {
                    bonus += 5.0;
                }
                if kw.contains(&Keyword::Piercing) {
                    bonus += 3.0;
                }
                for keyword in kw.iter() {
                    if let Keyword::Rampage(n) = keyword {
                        bonus += (*n as f64) * 2.0;
                    }
                }
            }

            // Suicide check: if a plausible opponent blocker can kill my
            // attacker without dying back, this attack is probably bad.
            // Find the biggest untapped opponent unit and see whether the
            // trade favors us. This is rough, but catches "throw my 2/2
            // into a 5/5 wall".
            if let Some(attacker) = attacker {
                let a_tough = effective_toughness(before, attacker) as f64;
                let a_power = power;
                let a_val = card_intrinsic_value(attacker);
                let a_evasive = attacker.keywords.contains(&Keyword::Stealth)
                    || attacker.keywords.contains(&Keyword::Aero);
                // If the attacker is evasive, skip the trade check.
                if !a_evasive {
                    let worst_blocker = before
                        .players
                        .iter()
                        .filter(|(pid, _)| pid.as_str() != player_id)
                        .flat_map(|(_, p)| p.battlefield.iter())
                        .filter(|c| {
                            matches!(c.kind, CardKind::Unit(_))
                                && !c.tapped
                                && !(c.summoning_sickness && !c.haste)
                        })
                        .max_by_key(|c| {
                            effective_power(before, c) + effective_toughness(before, c)
                        });
                    if let Some(blocker) = worst_blocker {
                        let b_power = effective_power(before, blocker) as f64;
                        let b_tough = effective_toughness(before, blocker) as f64;
                        let b_val = card_intrinsic_value(blocker);
                        let b_deathtouch = blocker.keywords.contains(&Keyword::Deathtouch);
                        let a_deathtouch = attacker.keywords.contains(&Keyword::Deathtouch);
                        let attacker_dies = if b_deathtouch {
                            b_power > 0.0
                        } else {
                            b_power >= a_tough
                        };
                        let blocker_dies = if a_deathtouch {
                            a_power > 0.0
                        } else {
                            a_power >= b_tough
                        };
                        if attacker_dies && !blocker_dies {
                            bonus -= a_val * 3.0 + 10.0;
                        } else if attacker_dies && blocker_dies {
                            // Even trade, value based on intrinsic values.
                            bonus += (b_val - a_val) * 2.0;
                        } else if blocker_dies {
                            bonus += b_val * 2.0;
                        }
                    }
                }
            }
        }
        CardGameAction::ConfirmAttackers => {
            if context.attack_actions > 0 {
                bonus -= 28.0;
            } else if !before.combat_attackers.is_empty() {
                bonus += 6.0;
            }
        }
        CardGameAction::DeclareBlock {
            attacker_id,
            blocker_id,
        } => {
            // Simulate the damage trade: who dies, how much damage did
            // we absorb? Without this the AI happily chump-blocks a
            // 3/3 with its 6/6 (the raw eval only counts attacker power).
            let attacker = find_battlefield_card(before, attacker_id);
            let blocker = find_battlefield_card(before, blocker_id);
            if let (Some(attacker), Some(blocker)) = (attacker, blocker) {
                let a_power = effective_power(before, attacker) as f64;
                let a_tough = effective_toughness(before, attacker) as f64;
                let b_power = effective_power(before, blocker) as f64;
                let b_tough = effective_toughness(before, blocker) as f64;
                let a_val = card_intrinsic_value(attacker);
                let b_val = card_intrinsic_value(blocker);
                let a_deathtouch = attacker.keywords.contains(&Keyword::Deathtouch);
                let b_deathtouch = blocker.keywords.contains(&Keyword::Deathtouch);
                let a_trample = attacker.keywords.contains(&Keyword::Trample);

                // With Deathtouch, any positive damage kills.
                let blocker_dies = if a_deathtouch {
                    a_power > 0.0
                } else {
                    a_power >= b_tough
                };
                let attacker_dies = if b_deathtouch {
                    b_power > 0.0
                } else {
                    b_power >= a_tough
                };

                let mut trade = 0.0;
                if attacker_dies {
                    trade += a_val;
                }
                if blocker_dies {
                    trade -= b_val;
                }
                // Absorbed damage (saves life). With Trample, excess
                // damage still hits the player — only the portion up to
                // the blocker's toughness is actually absorbed.
                let absorbed = if a_trample {
                    a_power.min(b_tough)
                } else {
                    a_power.min(b_tough)
                };
                trade += absorbed * 2.0;
                // With Trample, overflow damage gets through — blocking
                // with a small blocker vs a big Trample attacker is often
                // worse than taking the hit. Penalize.
                if a_trample && a_power > b_tough {
                    trade -= (a_power - b_tough) * 1.5;
                }
                bonus += 4.0 + trade;
            } else {
                // Fallback to previous heuristic if cards missing.
                let attacker_power = find_card(before, attacker_id)
                    .map(|card| effective_power(before, card) as f64)
                    .unwrap_or(1.0);
                let blocker_value = find_card(before, blocker_id)
                    .map(card_intrinsic_value)
                    .unwrap_or(1.0);
                bonus += attacker_power * 4.0 + blocker_value;
            }
        }
        CardGameAction::ConfirmBlockers => {
            if context.block_actions > 0 {
                bonus -= 18.0;
            } else if !before.combat_blockers.is_empty() {
                bonus += 4.0;
            }
        }
        CardGameAction::NextStage => {
            if matches!(
                before.stage,
                CardGameStage::Main | CardGameStage::PostCombatMain
            ) && (context.cast_actions > 0 || context.ability_actions > 0)
            {
                bonus -= 18.0;
            }
            if before_overflow > 0.0 {
                bonus -= 22.0;
            }
        }
        CardGameAction::PlayLand { .. } | CardGameAction::DeployNode { .. } => {
            bonus += mana_development_bonus(before, player_id, context, profile, before_overflow);
        }
        CardGameAction::Discard { card_id } => {
            if let Some(card) = find_card(before, card_id) {
                bonus -= card_intrinsic_value(card) * 3.0;
            }
        }
        CardGameAction::Sacrifice { card_id } => {
            if let Some(card) = find_card(before, card_id) {
                let penalty = match profile {
                    AgentProfile::Slag => 1.0,
                    _ => 2.0,
                };
                bonus -= card_intrinsic_value(card) * penalty;
            }
        }
        CardGameAction::PassPriority => {
            bonus += if before.stack.is_empty() { -6.0 } else { 2.0 };
        }
        _ => {}
    }

    bonus
}

fn total_opponent_life(state: &CardGameState, player_id: &str) -> i32 {
    state
        .turn_order
        .iter()
        .filter(|id| id.as_str() != player_id)
        .filter_map(|id| state.players.get(id))
        .map(|player| player.life)
        .sum()
}

fn board_power(state: &CardGameState, player_id: &str) -> f64 {
    state
        .players
        .get(player_id)
        .map(|player| {
            player
                .battlefield
                .iter()
                .map(|card| effective_power(state, card) as f64)
                .sum()
        })
        .unwrap_or(0.0)
}

fn board_toughness(state: &CardGameState, player_id: &str) -> f64 {
    state
        .players
        .get(player_id)
        .map(|player| {
            player
                .battlefield
                .iter()
                .map(|card| effective_toughness(state, card) as f64)
                .sum()
        })
        .unwrap_or(0.0)
}

/// Heuristic value of an ability's effect, used by the AI to differentiate
/// between weak (+1 Scry) and strong (mass damage, search-to-battlefield)
/// agent abilities. Without this, every Agent ability scored a flat +10,
/// causing the AI to use complex agents about as well as it used "do
/// nothing" agents — which inverted intended balance, with passive Apex
/// agents winning more pods than ones with rich abilities.
///
/// The scale roughly matches the bonuses elsewhere in `score_action`:
/// drawing a card ≈ 5 pts, dealing 1 damage to a player ≈ 3 pts, etc.
fn score_ability_effect(effect: &CardEffect) -> f64 {
    match effect {
        CardEffect::Sequence(effects) => effects.iter().map(score_ability_effect).sum(),
        // The loyalty change itself isn't worth points — only the effect.
        CardEffect::ModifyLoyalty(_) => 0.0,
        // Card filtering / draw / scaling
        CardEffect::Scry(n) => (*n as f64) * 2.0,
        CardEffect::DrawCards(n) => (*n as f64) * 5.0,
        CardEffect::Loot(draw, _) => (*draw as f64) * 3.0,
        CardEffect::Surveil(n) | CardEffect::Transmit(n) => (*n as f64) * 2.5,
        CardEffect::Search(_) => 12.0,
        CardEffect::SearchToBattlefield(_) => 18.0,
        CardEffect::SearchGraveyard(_) => 8.0,
        CardEffect::ReanimateTargetUnit | CardEffect::Conscript => 12.0,
        CardEffect::ReanimateNamedCard(_) => 10.0,
        // Damage
        CardEffect::DamagePlayer(n) => (*n as f64) * 3.0,
        CardEffect::DamageUnit(n) | CardEffect::DamageTarget(n) => (*n as f64) * 2.5,
        CardEffect::DamageSplit(n) | CardEffect::DamageEachOpponentUnit(n) => (*n as f64) * 4.0,
        CardEffect::Siphon(n) => (*n as f64) * 3.5,
        // Disruption
        CardEffect::Discard(n) => (*n as f64) * 6.0,
        CardEffect::DiscardSelfRandom(n) => -(*n as f64) * 3.0,
        CardEffect::Mill(n) => (*n as f64) * 1.5,
        CardEffect::Wheel(n) => (*n as f64) * 4.0,
        // Token / board
        CardEffect::CreateToken(spec) => (spec.power as f64 + spec.toughness as f64).max(2.0) * 1.5,
        CardEffect::CreateInfrastructure(_) => 4.0,
        CardEffect::CreateTokensPerBattlefieldKind(spec, _) => {
            (spec.power as f64 + spec.toughness as f64).max(2.0) * 2.5
        }
        // Removal / control
        CardEffect::DestroyTarget => 10.0,
        CardEffect::ExileTarget => 12.0,
        CardEffect::Bounce => 7.0,
        CardEffect::BoardWipe => 18.0,
        CardEffect::DestroyTargetPlayerUnits => 14.0,
        CardEffect::BounceAllOpponentPermanents => 16.0,
        CardEffect::BounceAllAttacking => 8.0,
        CardEffect::BounceAllUnits => 12.0,
        CardEffect::DestroyUnitsWithToughnessAtLeast(_) => 10.0,
        CardEffect::TapTarget => 4.0,
        CardEffect::UntapTarget => 4.0,
        CardEffect::CopyUnit => 12.0,
        CardEffect::CopyLastCast => 8.0,
        // Stat / counter / keyword
        CardEffect::AddCounters(n) => (*n as f64) * 3.5,
        CardEffect::RemoveCounters(n) => (*n as f64) * 2.0,
        CardEffect::ApplyStatModifier(_) | CardEffect::ApplyStatModifiers(_) => 8.0,
        CardEffect::Emblem(_, _) => 25.0,
        CardEffect::PumpByMana => 6.0,
        // Misc / lifegain
        CardEffect::GainLife(n) => (*n as f64) * 1.0,
        CardEffect::AddMana(_, n) => (*n as f64) * 2.0,
        CardEffect::AddManaScaling(_, _, _) => 4.0,
        CardEffect::DamagePlayerPerHandCard => 6.0,
        CardEffect::DamagePlayerPerBattlefieldKind(_) => 6.0,
        CardEffect::Entomb(_) => 6.0,
        CardEffect::Recycle => 5.0,
        // Other primitives default to a small positive value.
        _ => 2.0,
    }
}

/// If the effect is a `GrantKeyword` (directly or wrapped in a single-step
/// sequence), return the keyword and its expiry. Used by the AI to avoid
/// activating end-of-turn buff abilities in phases where they can't pay off.
fn grant_keyword_info(effect: &CardEffect) -> Option<(Keyword, ExpireContract)> {
    match effect {
        CardEffect::GrantKeyword(kw, exp) => Some((kw.clone(), exp.clone())),
        CardEffect::Sequence(effects) => effects.iter().find_map(|e| match e {
            CardEffect::GrantKeyword(kw, exp) => Some((kw.clone(), exp.clone())),
            _ => None,
        }),
        _ => None,
    }
}

fn effective_power(state: &CardGameState, card: &Card) -> i32 {
    card.power.unwrap_or(0) + state.modifier_amount_for_card(&card.id, StatType::Power)
}

fn effective_toughness(state: &CardGameState, card: &Card) -> i32 {
    card.toughness.unwrap_or(0) + state.modifier_amount_for_card(&card.id, StatType::Toughness)
}

fn mana_source_count(player: &crate::state::PlayerState) -> usize {
    player.grid.len()
        + player
            .battlefield
            .iter()
            .filter(|card| matches!(card.kind, CardKind::Infrastructure))
            .count()
}

fn mana_development_bonus(
    state: &CardGameState,
    player_id: &str,
    context: &ActionContext,
    profile: &AgentProfile,
    hand_overflow: f64,
) -> f64 {
    let Some(player) = state.players.get(player_id) else {
        return 0.0;
    };

    let sources = mana_source_count(player);
    let profile_bias = match profile {
        AgentProfile::Bio => 3.0,
        AgentProfile::Chrome => 2.0,
        AgentProfile::Slag => 1.0,
        _ => 0.0,
    };

    let mut bonus = if sources < 3 {
        10.0 + profile_bias
    } else if sources < 5 {
        5.0 + profile_bias
    } else if sources < 7 {
        1.0 + profile_bias * 0.5
    } else {
        -8.0
    };

    if context.cast_actions > 0 || context.ability_actions > 0 {
        bonus -= 8.0;
    }
    if hand_overflow > 0.0 {
        bonus -= hand_overflow * 10.0;
    }

    bonus
}

/// Score the "delayed trigger" state of the board for this player. Cards
/// like Data Recovery Matrix or Bitrot store a remembered player on the
/// permanent and then fire a hostile trigger against *that* player when
/// certain events happen. The probe has already set `remembered_player_id`
/// by this point, but the trigger won't fire until later — the basic
/// board-power / life eval can't see the future punishment. Here we look
/// at each permanent with a remembered-player trigger and bias the score
/// toward "remembered someone I want punished."
fn delayed_trigger_bias(state: &CardGameState, player_id: &str) -> f64 {
    

    let mut bias = 0.0;
    for (pid, player) in &state.players {
        for card in &player.battlefield {
            let Some(remembered) = card.remembered_player_id.as_deref() else {
                continue;
            };
            // Does this card have any trigger gated on the remembered
            // player? Look for Predicate::Condition(TargetMatchesRememberedPlayer)
            // at any nesting level.
            let has_remembered_gated = card.triggers.iter().any(|trigger| {
                trigger
                    .predicate
                    .as_ref()
                    .map(predicate_mentions_remembered)
                    .unwrap_or(false)
            });
            if !has_remembered_gated {
                continue;
            }
            // Sum up how "hostile" the remembered-gated triggers are.
            let hostile_total: f64 = card
                .triggers
                .iter()
                .filter(|trigger| {
                    trigger
                        .predicate
                        .as_ref()
                        .map(predicate_mentions_remembered)
                        .unwrap_or(false)
                })
                .map(|trigger| hostile_effect_penalty(&trigger.effect))
                .sum();
            if hostile_total <= 0.0 {
                continue;
            }
            // Apply the bias from the viewer's perspective:
            //   remembered == player_id: punishment lands on us → big
            //   minus (scaled by whether WE own the permanent or they do;
            //   either way the remembered player gets hit).
            //   remembered == opponent: punishment lands on them → plus.
            // `player_id` is the viewer scoring the state. The permanent
            // owner `pid` doesn't change the direction — only who is
            // remembered does.
            let _ = pid; // owner doesn't matter for target direction
            if remembered == player_id {
                bias -= hostile_total;
            } else {
                bias += hostile_total;
            }
        }
    }
    bias
}

fn predicate_mentions_remembered(predicate: &crate::types::Predicate) -> bool {
    use crate::types::{Condition, Predicate};
    match predicate {
        Predicate::Condition(Condition::TargetMatchesRememberedPlayer) => true,
        Predicate::All(preds) | Predicate::Any(preds) => {
            preds.iter().any(predicate_mentions_remembered)
        }
        Predicate::Not(inner) => predicate_mentions_remembered(inner),
        _ => false,
    }
}

/// Is this unit a "value engine" that snowballs if left unchecked? Used
/// to boost removal priority against cards like Fauna Whisperer (draw on
/// each unit cast), Grove Huntmaster (token on ally ETB), Thornback
/// Riders (tokens + counters each upkeep), etc.
fn is_snowball_engine(card: &Card) -> bool {
    use crate::types::TriggerEvent;
    card.triggers.iter().any(|trigger| {
        matches!(
            trigger.event,
            TriggerEvent::AllyEntered(_)
                | TriggerEvent::Upkeep
                | TriggerEvent::CastInstantOrRitual
                | TriggerEvent::CastRelic
                | TriggerEvent::CastAny
                | TriggerEvent::Discarded
                | TriggerEvent::Died
                | TriggerEvent::AllyDied
                | TriggerEvent::TokenCreated
                | TriggerEvent::AnyAllyDealtDamage
                | TriggerEvent::ReceivedDamage
                | TriggerEvent::DealtDamage
                | TriggerEvent::DrewCard
                | TriggerEvent::Sacrificed
        )
    })
}

/// Does this card have an activated ability that deals damage or destroys
/// permanents? Used to flag operators (Agents) whose [+1] / [-N] abilities
/// generate value every turn — these snowball if left alive even though
/// they don't have triggers.
fn has_damaging_activated_ability(card: &Card) -> bool {
    card.abilities.iter().any(|ability| effect_is_threatening(&ability.effect))
}

fn effect_is_threatening(effect: &CardEffect) -> bool {
    match effect {
        CardEffect::DamagePlayer(_)
        | CardEffect::DamageUnit(_)
        | CardEffect::DamageTarget(_)
        | CardEffect::DamagePlayerPerHandCard
        | CardEffect::DestroyTarget
        | CardEffect::ExileTarget
        | CardEffect::Bounce
        | CardEffect::Polymorph => true,
        CardEffect::Sequence(children) => children.iter().any(effect_is_threatening),
        _ => false,
    }
}

fn state_opponent_battlefield_has(state: &CardGameState, player_id: &str, card_id: &str) -> bool {
    state.players.iter().any(|(pid, player)| {
        pid.as_str() != player_id && player.battlefield.iter().any(|c| c.id == card_id)
    })
}

/// Return the first "spell effect" of a card — the one the cast-time target
/// spec is bound to. Used by scoring to know what an `Any`-scoped spell
/// will actually *do* to the target the AI picks.
fn spell_effect_for_card(card: &Card) -> Option<&CardEffect> {
    card.abilities
        .iter()
        .find(|ability| ability.is_spell_effect_ability())
        .map(|ability| &ability.effect)
        .or_else(|| {
            // Units / permanents use ETB triggers — fall back to the first
            // Entered trigger so units that "destroy target unit on enter"
            // still get friendly-fire scoring.
            card.triggers
                .iter()
                .find(|trigger| matches!(trigger.event, crate::types::TriggerEvent::Entered))
                .map(|trigger| &trigger.effect)
        })
}

/// How much of a penalty to apply when this effect lands on your OWN unit
/// (or yourself, for player-target effects). Walks Sequence/DamageSplit
/// to sum child penalties — a "bounce + draw" sequence still penalizes
/// the bounce even though the draw is fine on self.
fn hostile_effect_penalty(effect: &CardEffect) -> f64 {
    match effect {
        CardEffect::Bounce => 40.0,
        CardEffect::DestroyTarget => 80.0,
        CardEffect::ExileTarget => 90.0,
        CardEffect::Freeze(_) => 25.0,
        CardEffect::TapTarget => 15.0,
        CardEffect::Polymorph => 70.0,
        CardEffect::Fight => 60.0,
        CardEffect::DamageUnit(amount) | CardEffect::DamageTarget(amount) => {
            (*amount as f64).max(1.0) * 10.0
        }
        CardEffect::DamagePlayer(amount) | CardEffect::Siphon(amount) => {
            (*amount as f64).max(1.0) * 10.0
        }
        CardEffect::DamageSplit(amount) => (*amount as f64).max(1.0) * 6.0,
        CardEffect::Mill(amount) => (*amount as f64).max(1.0) * 4.0,
        CardEffect::Discard(amount) => (*amount as f64).max(1.0) * 6.0,
        CardEffect::RemoveCounters(amount) => (*amount as f64).max(1.0) * 8.0,
        CardEffect::MutateTarget(spec) => {
            // Permanent stat changes: penalize when sign is negative.
            if stat_modifier_is_negative(spec) {
                55.0
            } else {
                0.0
            }
        }
        CardEffect::ApplyStatModifier(spec) => {
            if stat_modifier_is_negative(spec) {
                25.0
            } else {
                0.0
            }
        }
        CardEffect::ApplyStatModifiers(specs) => {
            specs
                .iter()
                .filter(|spec| stat_modifier_is_negative(spec))
                .count() as f64
                * 25.0
        }
        CardEffect::Sequence(effects) => effects.iter().map(hostile_effect_penalty).sum(),
        _ => 0.0,
    }
}

fn stat_modifier_is_negative(spec: &crate::card::StatModifierSpec) -> bool {
    use crate::types::ModifierAmount;
    match &spec.amount {
        ModifierAmount::Fixed(v) => *v < 0,
        _ => false,
    }
}

/// If the effect deals a fixed amount of damage to a unit target, return
/// that amount. Recurses into Sequence so "bounce + draw" or
/// "damage 3 + create token" report correctly.
/// "Reactive" = usually wants to be cast in response to something, not
/// pre-emptively. Counters, single-target bounce/destroy/damage all
/// qualify. Cantrips, sweepers, pump/draw spells do not.
fn spell_is_reactive(card: &Card) -> bool {
    let check = |e: &CardEffect| effect_is_reactive(e);
    card.abilities.iter().any(|a| check(&a.effect))
        || card.triggers.iter().any(|t| check(&t.effect))
}

fn effect_is_reactive(effect: &CardEffect) -> bool {
    match effect {
        CardEffect::CounterTop
        | CardEffect::Bounce
        | CardEffect::DestroyTarget
        | CardEffect::ExileTarget
        | CardEffect::Freeze(_)
        | CardEffect::TapTarget
        | CardEffect::Polymorph
        | CardEffect::Fight
        | CardEffect::DamageUnit(_)
        | CardEffect::DamageTarget(_) => true,
        CardEffect::Sequence(effects) => effects.iter().any(effect_is_reactive),
        _ => false,
    }
}

/// Does the card's effect chain include a CounterTop anywhere?
fn spell_is_counter(card: &Card) -> bool {
    let check = |e: &CardEffect| -> bool { effect_contains_counter(e) };
    card.abilities.iter().any(|a| check(&a.effect))
        || card.triggers.iter().any(|t| check(&t.effect))
}

fn effect_contains_counter(effect: &CardEffect) -> bool {
    match effect {
        CardEffect::CounterTop => true,
        CardEffect::Sequence(effects) => effects.iter().any(effect_contains_counter),
        _ => false,
    }
}

fn stack_item_card(item: &crate::state::StackItem) -> Option<&Card> {
    match &item.kind {
        crate::state::StackItemKind::Spell(card) => Some(card),
        crate::state::StackItemKind::Ability { .. } => None,
    }
}

fn damage_effect_amount(effect: &CardEffect) -> f64 {
    match effect {
        CardEffect::DamageUnit(amount)
        | CardEffect::DamageTarget(amount)
        | CardEffect::DamageSplit(amount) => *amount as f64,
        CardEffect::Sequence(effects) => {
            effects.iter().map(damage_effect_amount).fold(0.0, f64::max)
        }
        _ => 0.0,
    }
}

fn find_battlefield_owner(state: &CardGameState, card_id: &str) -> Option<String> {
    for (pid, player) in &state.players {
        if player.battlefield.iter().any(|c| c.id == card_id) {
            return Some(pid.clone());
        }
    }
    None
}

fn find_battlefield_card<'a>(state: &'a CardGameState, card_id: &str) -> Option<&'a Card> {
    state
        .players
        .values()
        .flat_map(|p| p.battlefield.iter())
        .find(|c| c.id == card_id)
}

fn find_card<'a>(state: &'a CardGameState, card_id: &str) -> Option<&'a Card> {
    for player_id in &state.turn_order {
        let Some(player) = state.players.get(player_id) else {
            continue;
        };
        for zone in [
            &player.hand,
            &player.battlefield,
            &player.graveyard,
            &player.operator_zone,
            &player.exile,
        ] {
            if let Some(card) = zone.iter().find(|card| card.id == card_id) {
                return Some(card);
            }
        }
    }

    let mut overflow_player_ids = state
        .players
        .keys()
        .filter(|player_id| !state.turn_order.contains(*player_id))
        .cloned()
        .collect::<Vec<_>>();
    overflow_player_ids.sort();

    for player_id in overflow_player_ids {
        let Some(player) = state.players.get(&player_id) else {
            continue;
        };
        for zone in [
            &player.hand,
            &player.battlefield,
            &player.graveyard,
            &player.operator_zone,
            &player.exile,
        ] {
            if let Some(card) = zone.iter().find(|card| card.id == card_id) {
                return Some(card);
            }
        }
    }

    None
}

fn card_intrinsic_value(card: &Card) -> f64 {
    let stats = card.power.unwrap_or(0).max(0) + card.toughness.unwrap_or(0).max(0);
    let loyalty = card.loyalty.unwrap_or(0).max(0);
    let keyword_value: f64 = card.keywords.iter().map(keyword_value).sum();
    card.cost.total() as f64 * 2.0
        + stats as f64 * 1.5
        + loyalty as f64
        + card.abilities.len() as f64 * 1.5
        + card.triggers.len() as f64 * 1.5
        + keyword_value
}

/// Rough value of a keyword on a unit/permanent — used by
/// `card_intrinsic_value` to stop the AI from treating a 2/2 with
/// Deathtouch as equivalent to a vanilla 2/2. Values are conservative;
/// they multiply through scoring (e.g. threat-pick penalty for own
/// bounce, removal priority for opponent) via card_intrinsic_value.
fn keyword_value(keyword: &Keyword) -> f64 {
    match keyword {
        // Evasion / dodging removal.
        Keyword::Stealth => 3.0,
        Keyword::Camouflage => 3.0,
        Keyword::Ethereal => 2.5,
        Keyword::Indestructible => 4.0,
        // Combat keywords.
        Keyword::Haste => 2.0,
        Keyword::Deathtouch => 3.0,
        Keyword::Trample => 2.0,
        Keyword::Piercing => 1.5,
        Keyword::Vigilance => 1.5,
        Keyword::Flash => 1.5,
        Keyword::Menace => 1.5,
        Keyword::Aero => 1.5,
        Keyword::Interceptor => 0.5,
        // Scaling / static defense.
        Keyword::ReactiveArmor(n) | Keyword::Absorb(n) => (*n as f64) * 1.0,
        Keyword::Firewall(n) => (*n as f64) * 0.5,
        // Ongoing-value engines.
        Keyword::Rampage(n) => (*n as f64) * 1.5,
        Keyword::Modular(n) => (*n as f64) * 1.0,
        Keyword::Evolve => 2.0,
        Keyword::Cascade => 3.0,
        // Static grants — roughly the value of what they grant, plus a
        // small bonus for persistent board effect.
        Keyword::StaticPowerBuff(n, _) | Keyword::StaticToughnessBuff(n, _) => {
            (*n as f64) * 1.0 + 1.0
        }
        // Self-scaling keywords: assume a modest 2-3 permanents of the
        // scaling kind will be on the board by mid-game. This gives the
        // AI a reason to cast the card itself, and to value it as a
        // removal target from the other side.
        Keyword::SelfPowerPerKind(n, _) | Keyword::SelfToughnessPerKind(n, _) => (*n as f64) * 2.5,
        Keyword::StaticHasteGrant(_)
        | Keyword::StaticTrampleGrant(_)
        | Keyword::StaticStealthGrant(_)
        | Keyword::StaticIndestructibleGrant(_) => 2.0,
        Keyword::StaticKeywordGrant(_, _) => 1.5,
        Keyword::StaticTapForMana(_, amount, _) => (*amount as f64) * 1.0,
        // Gravecast is recursion-able value.
        Keyword::Gravecast(n) => (*n as f64) * 2.0,
        // Downside-ish — keep at 0 for now.
        Keyword::Compiling | Keyword::Echo(_) | Keyword::Wither => 0.0,
        // Misc.
        Keyword::SignalJam | Keyword::Malware | Keyword::Glitch => 2.0,
        Keyword::Throttle(n) => (*n as f64) * 1.0,
        Keyword::Optics(_) => 1.0,
        _ => 0.0,
    }
}
