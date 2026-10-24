use std::collections::BTreeSet;

use game_framework::GameLogic;

use crate::action::CardGameAction;
use crate::card::{Card, CardAbility, CardEffect, TargetSpec};
use crate::state::{CardGameState, PendingAction};
use crate::types::{
    CardGameStage, CardKind, CombatPhase, ManaType, TargetKind, TargetRef, TargetScope,
};

use super::report::action_label;

const MAX_TARGET_CANDIDATES: usize = 6;

pub fn legal_actions_for_player(state: &CardGameState, player_id: &str) -> Vec<CardGameAction> {
    let Some(player) = state.players.get(player_id) else {
        return Vec::new();
    };

    let mut candidates = Vec::new();

    if player.has_pending_discard() {
        for card in &player.hand {
            candidates.push(CardGameAction::Discard {
                card_id: card.id.clone(),
            });
        }
        return filter_legal_actions(state, player_id, candidates);
    }

    if let Some(PendingAction::Sacrifice { source_card_id, .. }) = player.pending_sacrifice() {
        for card in &player.battlefield {
            if card.id == *source_card_id {
                continue;
            }
            if matches!(card.kind, CardKind::Unit(_)) {
                candidates.push(CardGameAction::Sacrifice {
                    card_id: card.id.clone(),
                });
            }
        }
        return filter_legal_actions(state, player_id, candidates);
    }

    if player.has_pending_trigger_choice() {
        candidates.push(CardGameAction::TriggerChoice { accept: true });
        candidates.push(CardGameAction::TriggerChoice { accept: false });
        return filter_legal_actions(state, player_id, candidates);
    }

    if let Some(PendingAction::TriggerTargetSelection {
        target_kind,
        optional,
        ..
    }) = player.pending_trigger_target()
    {
        for target in pending_trigger_targets(state, player_id, target_kind) {
            candidates.push(CardGameAction::TriggerTargetSelect { target });
        }
        if *optional {
            candidates.push(CardGameAction::TriggerTargetCancel);
        }
        return filter_legal_actions(state, player_id, candidates);
    }

    if player.library_search_active {
        for card in &player.library_search_results {
            candidates.push(CardGameAction::SearchCommit {
                card_id: card.id.clone(),
            });
        }
        for card in &player.scry {
            candidates.push(CardGameAction::SearchCommit {
                card_id: card.id.clone(),
            });
        }
        candidates.push(CardGameAction::SearchCancel);
        return filter_legal_actions(state, player_id, candidates);
    }

    if !player.scry.is_empty() {
        let (top, bottom) = default_scry_choice(player);
        candidates.push(CardGameAction::ScryCommit { top, bottom });
        return filter_legal_actions(state, player_id, candidates);
    }

    // Stack/showcase must fully resolve before combat declarations can happen.
    let combat_ok = state.stack.is_empty() && state.showcase.is_none();

    if combat_ok
        && state.stage == CardGameStage::Combat
        && state.combat_phase == CombatPhase::DeclareBlockers
        && state.combat_blocker_id.as_deref() == Some(player_id)
    {
        candidates.push(CardGameAction::ConfirmBlockers);

        for attacker_id in &state.combat_attackers {
            let targets_player = state
                .combat_targets
                .get(attacker_id)
                .map(|target| matches!(target, TargetRef::Player(id) if id == player_id))
                .unwrap_or_else(|| {
                    state
                        .active_player_id
                        .as_deref()
                        .and_then(|active_id| state.default_defender_id(active_id))
                        .as_deref()
                        == Some(player_id)
                });
            if !targets_player {
                continue;
            }

            for card in &player.battlefield {
                if state.can_block_card(player_id, card) {
                    candidates.push(CardGameAction::DeclareBlock {
                        attacker_id: attacker_id.clone(),
                        blocker_id: card.id.clone(),
                    });
                }
            }
        }

        return filter_legal_actions(state, player_id, candidates);
    }

    if combat_ok
        && state.stage == CardGameStage::Combat
        && state.combat_phase == CombatPhase::DeclareAttackers
        && state.active_player_id.as_deref() == Some(player_id)
    {
        candidates.push(CardGameAction::ConfirmAttackers);

        // Collect opponent agents (planeswalker-style operators) and living
        // opponents. Agents can be attacked to drain loyalty; in pods, the
        // AI also needs an explicit per-opponent player option so MCTS can
        // pick which seat to pressure rather than always defaulting to
        // `default_defender_id`.
        let agent_targets: Vec<String> = state
            .players
            .iter()
            .filter(|(pid, _)| pid.as_str() != player_id)
            .flat_map(|(_, p)| {
                p.battlefield
                    .iter()
                    .filter(|c| matches!(c.kind, CardKind::Agent))
                    .map(|c| c.id.clone())
            })
            .collect();
        let player_targets: Vec<String> = state
            .turn_order
            .iter()
            .filter(|pid| pid.as_str() != player_id)
            .filter(|pid| {
                state
                    .players
                    .get(pid.as_str())
                    .map(|p| p.life > 0)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        let multi_opponent = player_targets.len() > 1;

        for card in &player.battlefield {
            if !state.can_attack_card(player_id, card) || state.combat_attackers.contains(&card.id)
            {
                continue;
            }
            // Default-defender attack (None resolves via default_defender_id).
            candidates.push(CardGameAction::DeclareAttack {
                card_id: card.id.clone(),
                target: None,
            });
            // In pods, expose each opponent player explicitly so the AI can
            // redirect pressure off the default defender.
            if multi_opponent {
                for pid in &player_targets {
                    candidates.push(CardGameAction::DeclareAttack {
                        card_id: card.id.clone(),
                        target: Some(TargetRef::Player(pid.clone())),
                    });
                }
            }
            // Each opposing agent on the battlefield is a legal attack
            // target (drains loyalty). Without enumerating these, MCTS can
            // never decide to swing at an operator.
            for agent_id in &agent_targets {
                candidates.push(CardGameAction::DeclareAttack {
                    card_id: card.id.clone(),
                    target: Some(TargetRef::Card(agent_id.clone())),
                });
            }
        }
        return filter_legal_actions(state, player_id, candidates);
    }

    // Sorcery-speed gate: deploying nodes and playing infrastructure must
    // wait for the stack / showcase to clear.
    let sorcery_ok = state.stack.is_empty() && state.showcase.is_none();

    let can_deploy_node = state.is_active_player(player_id)
        && sorcery_ok
        && matches!(
            state.stage,
            CardGameStage::Main | CardGameStage::PostCombatMain
        )
        && player.nodes_deployed_this_turn < 1;

    if can_deploy_node {
        for node in player
            .color_identity
            .iter()
            .filter(|mana_type| !matches!(mana_type, ManaType::Colorless))
        {
            candidates.push(CardGameAction::DeployNode {
                mana_type: node.clone(),
            });
        }
    }

    if sorcery_ok {
        for card in &player.hand {
            if state.can_play_infrastructure(player_id, card) {
                candidates.push(CardGameAction::PlayLand {
                    card_id: card.id.clone(),
                });
            }
        }
    }

    for zone in [&player.hand[..], &player.operator_zone[..]] {
        for card in zone {
            if !state.can_cast_card(player_id, card) {
                continue;
            }
            for action in spell_actions(state, player_id, Some(card)) {
                candidates.push(action);
            }
        }
    }

    // Gravecast: allow casting spells with Gravecast keyword from graveyard
    for card in &player.graveyard {
        if card
            .gravecast_limit()
            .is_some_and(|limit| card.gravecast_uses < limit)
            && state.can_cast_card(player_id, card)
        {
            for action in spell_actions(state, player_id, Some(card)) {
                candidates.push(action);
            }
        }
    }

    for card in &player.battlefield {
        for ability in &card.abilities {
            if ability.is_spell_effect_ability() {
                continue;
            }
            if state
                .can_activate_ability(player_id, card, ability)
                .is_some()
            {
                continue;
            }
            if is_auto_mana_ability(ability) {
                continue;
            }
            for action in ability_actions(state, player_id, card, ability) {
                candidates.push(action);
            }
        }
    }

    let has_pending = !state.stack.is_empty() || state.showcase.is_some();
    if has_pending && state.effective_priority_holder() == Some(player_id) {
        candidates.push(CardGameAction::PassPriority);
    }

    if state.active_player_id.as_deref() == Some(player_id)
        && !has_pending
        && matches!(
            state.stage,
            CardGameStage::Main | CardGameStage::PostCombatMain
        )
    {
        candidates.push(CardGameAction::NextStage);
    }

    filter_legal_actions(state, player_id, candidates)
}

fn spell_actions(
    state: &CardGameState,
    player_id: &str,
    card: Option<&Card>,
) -> Vec<CardGameAction> {
    let Some(card) = card else {
        return Vec::new();
    };

    target_sets_for_spell(state, player_id, card)
        .into_iter()
        .map(|targets| CardGameAction::CastSpell {
            card_id: card.id.clone(),
            targets,
        })
        .collect()
}

fn ability_actions(
    state: &CardGameState,
    player_id: &str,
    card: &Card,
    ability: &CardAbility,
) -> Vec<CardGameAction> {
    target_sets_for_spec(Some(state), player_id, ability.target_spec.as_ref(), None)
        .into_iter()
        .map(|targets| CardGameAction::ActivateAbility {
            card_id: card.id.clone(),
            ability_id: ability.id.clone(),
            targets,
        })
        .collect()
}

fn target_sets_for_spell(
    state: &CardGameState,
    player_id: &str,
    card: &Card,
) -> Vec<Vec<TargetRef>> {
    // Unit cards handle targeting at ETB time, not cast time
    // Exception: RememberTargetPlayer still needs cast-time targeting
    let spec = if matches!(card.kind, CardKind::Unit(_)) {
        card.abilities
            .iter()
            .find(|ability| {
                ability.is_spell_effect_ability()
                    && matches!(ability.effect, CardEffect::RememberTargetPlayer)
            })
            .and_then(|ability| ability.target_spec.as_ref())
    } else {
        card.abilities
            .iter()
            .find(|ability| ability.is_spell_effect_ability())
            .and_then(|ability| ability.target_spec.as_ref())
    };
    target_sets_for_spec(Some(state), player_id, spec, Some(card))
}

fn target_sets_for_spec(
    state: Option<&CardGameState>,
    player_id: &str,
    spec: Option<&TargetSpec>,
    _card: Option<&Card>,
) -> Vec<Vec<TargetRef>> {
    let Some(spec) = spec else {
        return vec![Vec::new()];
    };

    let Some(state) = state else {
        return vec![Vec::new()];
    };

    let mut targets = match spec.kind {
        TargetKind::Player => player_targets(state, player_id, &spec.scope),
        TargetKind::Unit => unit_targets(state, player_id, &spec.scope),
        TargetKind::Card => card_targets(state, player_id, &spec.scope),
        TargetKind::PlayerOrUnit => {
            let mut out = player_targets(state, player_id, &spec.scope);
            out.extend(unit_targets(state, player_id, &spec.scope));
            out
        }
    };

    targets.sort_by_key(target_key);
    targets.dedup_by(|left, right| target_key(left) == target_key(right));
    if targets.len() > MAX_TARGET_CANDIDATES {
        targets.truncate(MAX_TARGET_CANDIDATES);
    }

    let mut sets = Vec::new();
    if spec.optional {
        sets.push(Vec::new());
    }

    let max_targets = spec.max_targets.min(2);
    if max_targets == 0 {
        sets.push(Vec::new());
        return sets;
    }

    for len in 1..=max_targets {
        let mut current = Vec::new();
        combine_targets(&targets, len, 0, &mut current, &mut sets);
    }

    if sets.is_empty() {
        sets.push(Vec::new());
    }

    sets
}

fn combine_targets(
    pool: &[TargetRef],
    remaining: usize,
    start: usize,
    current: &mut Vec<TargetRef>,
    out: &mut Vec<Vec<TargetRef>>,
) {
    if remaining == 0 {
        out.push(current.clone());
        return;
    }

    for index in start..pool.len() {
        current.push(pool[index].clone());
        combine_targets(pool, remaining - 1, index + 1, current, out);
        current.pop();
    }
}

fn player_targets(state: &CardGameState, player_id: &str, scope: &TargetScope) -> Vec<TargetRef> {
    match scope {
        TargetScope::SelfPlayer => vec![TargetRef::Player(player_id.to_string())],
        TargetScope::Opponent => state
            .turn_order
            .iter()
            .filter(|id| id.as_str() != player_id)
            .cloned()
            .map(TargetRef::Player)
            .collect(),
        TargetScope::Any | TargetScope::AnyPlayerOrUnit => state
            .turn_order
            .iter()
            .cloned()
            .map(TargetRef::Player)
            .collect(),
    }
}

fn unit_targets(state: &CardGameState, player_id: &str, scope: &TargetScope) -> Vec<TargetRef> {
    card_targets_by_predicate(state, player_id, scope, |kind| {
        matches!(kind, CardKind::Unit(_))
    })
}

fn card_targets(state: &CardGameState, player_id: &str, scope: &TargetScope) -> Vec<TargetRef> {
    card_targets_by_predicate(state, player_id, scope, |_| true)
}

fn card_targets_by_predicate<F>(
    state: &CardGameState,
    player_id: &str,
    scope: &TargetScope,
    predicate: F,
) -> Vec<TargetRef>
where
    F: Fn(&CardKind) -> bool,
{
    let mut targets = state
        .players
        .iter()
        .filter(|(owner_id, _)| match scope {
            TargetScope::SelfPlayer => owner_id.as_str() == player_id,
            TargetScope::Opponent => owner_id.as_str() != player_id,
            TargetScope::Any | TargetScope::AnyPlayerOrUnit => true,
        })
        .flat_map(|(_, player)| {
            player
                .battlefield
                .iter()
                .filter(|card| predicate(&card.kind))
                .map(|card| TargetRef::Card(card.id.clone()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    targets.sort_by_key(target_key);
    targets.dedup_by(|left, right| target_key(left) == target_key(right));
    targets
}

fn pending_trigger_targets(
    state: &CardGameState,
    player_id: &str,
    target_kind: &TargetKind,
) -> Vec<TargetRef> {
    let mut targets = match target_kind {
        TargetKind::Player => state
            .turn_order
            .iter()
            .filter(|id| id.as_str() != player_id)
            .cloned()
            .map(TargetRef::Player)
            .collect(),
        TargetKind::Unit => unit_targets(state, player_id, &TargetScope::Any),
        TargetKind::Card => card_targets(state, player_id, &TargetScope::Any),
        TargetKind::PlayerOrUnit => {
            let mut out = state
                .turn_order
                .iter()
                .filter(|id| id.as_str() != player_id)
                .cloned()
                .map(TargetRef::Player)
                .collect::<Vec<_>>();
            out.extend(unit_targets(state, player_id, &TargetScope::Any));
            out
        }
    };

    targets.sort_by_key(target_key);
    targets.dedup_by(|left, right| target_key(left) == target_key(right));
    targets
}

fn default_scry_choice(player: &crate::state::PlayerState) -> (Vec<String>, Vec<String>) {
    let mut ordered = player.scry.clone();
    ordered.sort_by_key(|card| (card.cost.total(), card.name.clone()));

    let mut top = Vec::new();
    let mut bottom = Vec::new();
    for card in ordered {
        if card.cost.total() <= 3 {
            top.push(card.id);
        } else {
            bottom.push(card.id);
        }
    }
    (top, bottom)
}

fn filter_legal_actions(
    state: &CardGameState,
    player_id: &str,
    candidates: Vec<CardGameAction>,
) -> Vec<CardGameAction> {
    let mut seen = BTreeSet::new();
    let mut legal = Vec::new();

    for action in candidates {
        let key = action_label(&action);
        if !seen.insert(key) {
            continue;
        }

        let mut probe = state.clone();
        if probe.update(player_id, action.clone()).is_ok() {
            legal.push(action);
        }
    }

    sort_actions_deterministically(&mut legal);
    legal
}

pub(crate) fn sort_actions_deterministically(actions: &mut Vec<CardGameAction>) {
    actions.sort_by_key(action_label);
}

fn is_auto_mana_ability(ability: &CardAbility) -> bool {
    ability.cost.has_tap()
        && ability.cost.is_empty_except_tap()
        && matches!(
            ability.effect,
            CardEffect::AddMana(_, _) | CardEffect::AddManaScaling(_, _, _)
        )
}

fn target_key(target: &TargetRef) -> String {
    match target {
        TargetRef::Player(id) => format!("p:{id}"),
        TargetRef::Card(id) => format!("c:{id}"),
    }
}
