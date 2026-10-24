use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::card::{Card, CardAbility, CardEffect, Cost};
use crate::card_catalog;
use crate::types::{Keyword, ManaCost, TriggerLimit};

use super::report::{DeckWinRateDelta, MatrixReport, PatchEvaluationDelta, PatchEvaluationReport};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BalancePatch {
    #[serde(default)]
    pub cards: Vec<CardPatch>,
    #[serde(default)]
    pub deck_entries: Vec<DeckEntryPatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardPatch {
    pub card_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<ManaCost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toughness: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loyalty: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_keywords: Vec<Keyword>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_keywords: Vec<Keyword>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_patches: Vec<TriggerPatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ability_patches: Vec<AbilityPatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerPatch {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<TriggerLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<CardEffect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityPatch {
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<ManaCost>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub once_per_turn: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<CardEffect>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeckEntryPatch {
    pub deck_id: String,
    pub card_id: String,
    pub quantity: i32,
}

pub fn load_balance_patch_from_path(path: impl AsRef<Path>) -> Result<BalancePatch> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read balance patch from {}", path.display()))?;
    let patch: BalancePatch = serde_json::from_str(&raw)
        .with_context(|| format!("parse balance patch JSON from {}", path.display()))?;
    patch.validate()?;
    Ok(patch)
}

impl BalancePatch {
    pub fn validate(&self) -> Result<()> {
        let known_cards = known_cards_by_id();
        let mut seen_card_ids = HashSet::new();
        let mut seen_deck_entries = HashSet::new();

        for patch in &self.cards {
            if !seen_card_ids.insert(patch.card_id.clone()) {
                return Err(anyhow!("duplicate card patch for {}", patch.card_id));
            }

            let reference = known_cards
                .get(&patch.card_id)
                .ok_or_else(|| anyhow!("unknown card_id in patch: {}", patch.card_id))?;
            validate_card_patch(reference, patch)?;
        }

        for patch in &self.deck_entries {
            if !is_known_deck(&patch.deck_id) {
                return Err(anyhow!("unknown deck_id in deck patch: {}", patch.deck_id));
            }
            if patch.quantity < 0 {
                return Err(anyhow!(
                    "deck patch quantity must be >= 0 for {}:{}",
                    patch.deck_id,
                    patch.card_id
                ));
            }

            let key = format!("{}:{}", patch.deck_id, patch.card_id);
            if !seen_deck_entries.insert(key) {
                return Err(anyhow!(
                    "duplicate deck patch for {}:{}",
                    patch.deck_id,
                    patch.card_id
                ));
            }

            if !known_cards.contains_key(&patch.card_id) {
                return Err(anyhow!(
                    "unknown card_id in deck patch {}:{}",
                    patch.deck_id,
                    patch.card_id
                ));
            }
        }

        Ok(())
    }

    /// Apply patches to cards using their template_id for matching (for draft decks).
    pub fn apply_to_deck_cards(&self, cards: &mut [Card]) {
        let patches: HashMap<_, _> = self
            .cards
            .iter()
            .map(|patch| (patch.card_id.as_str(), patch))
            .collect();
        for card in cards {
            let lookup_id = card.template_id.as_deref().unwrap_or(card.id.as_str());
            if let Some(patch) = patches.get(lookup_id) {
                let _ = apply_card_patch(card, patch);
            }
        }
    }

    pub fn apply_to_catalog_cards(&self, cards: &mut [(Card, i32)]) -> Result<()> {
        let patches: HashMap<_, _> = self
            .cards
            .iter()
            .map(|patch| (&patch.card_id, patch))
            .collect();
        for (card, _) in cards {
            if let Some(patch) = patches.get(&card.id) {
                apply_card_patch(card, patch)?;
            }
        }
        Ok(())
    }

    pub fn apply_to_deck_quantities(
        &self,
        deck_id: &str,
        quantities: &mut HashMap<String, i32>,
    ) -> Result<()> {
        for patch in self
            .deck_entries
            .iter()
            .filter(|patch| patch.deck_id == deck_id)
        {
            if patch.quantity == 0 {
                quantities.remove(&patch.card_id);
            } else {
                quantities.insert(patch.card_id.clone(), patch.quantity);
            }
        }
        Ok(())
    }
}

pub fn build_patch_evaluation_report(
    patch_path: impl Into<String>,
    baseline: MatrixReport,
    patched: MatrixReport,
) -> PatchEvaluationReport {
    let mut deck_ids = BTreeSet::new();
    deck_ids.extend(
        baseline
            .deck_metrics
            .iter()
            .map(|deck| deck.deck_id.clone()),
    );
    deck_ids.extend(patched.deck_metrics.iter().map(|deck| deck.deck_id.clone()));

    let baseline_by_deck: HashMap<_, _> = baseline
        .deck_metrics
        .iter()
        .map(|deck| (deck.deck_id.clone(), deck.win_rate))
        .collect();
    let patched_by_deck: HashMap<_, _> = patched
        .deck_metrics
        .iter()
        .map(|deck| (deck.deck_id.clone(), deck.win_rate))
        .collect();

    let deck_win_rate_deltas = deck_ids
        .into_iter()
        .map(|deck_id| {
            let baseline_win_rate = baseline_by_deck.get(&deck_id).copied().flatten();
            let patched_win_rate = patched_by_deck.get(&deck_id).copied().flatten();
            let delta = match (baseline_win_rate, patched_win_rate) {
                (Some(baseline), Some(patched)) => Some(patched - baseline),
                _ => None,
            };

            DeckWinRateDelta {
                deck_id,
                baseline_win_rate,
                patched_win_rate,
                delta,
            }
        })
        .collect();

    let delta = PatchEvaluationDelta {
        balance_score_delta: patched.balance.balance_score - baseline.balance.balance_score,
        pairwise_imbalance_delta: patched.balance.pairwise_imbalance
            - baseline.balance.pairwise_imbalance,
        finished_match_rate_delta: patched.balance.finished_match_rate
            - baseline.balance.finished_match_rate,
        decisive_match_rate_delta: patched.balance.decisive_match_rate
            - baseline.balance.decisive_match_rate,
        draw_match_rate_delta: patched.balance.draw_match_rate - baseline.balance.draw_match_rate,
        unfinished_match_rate_delta: patched.balance.unfinished_match_rate
            - baseline.balance.unfinished_match_rate,
        average_rounds_delta: patched.balance.average_rounds - baseline.balance.average_rounds,
        average_actions_delta: patched.balance.average_actions - baseline.balance.average_actions,
        deck_win_rate_deltas,
    };

    PatchEvaluationReport {
        patch_path: patch_path.into(),
        baseline,
        patched,
        delta,
    }
}

fn validate_card_patch(reference: &Card, patch: &CardPatch) -> Result<()> {
    for trigger_patch in &patch.trigger_patches {
        if trigger_patch.index >= reference.triggers.len() {
            return Err(anyhow!(
                "trigger index {} out of bounds for {} ({} triggers)",
                trigger_patch.index,
                patch.card_id,
                reference.triggers.len()
            ));
        }
    }

    for ability_patch in &patch.ability_patches {
        if ability_patch.index >= reference.abilities.len() {
            return Err(anyhow!(
                "ability index {} out of bounds for {} ({} abilities)",
                ability_patch.index,
                patch.card_id,
                reference.abilities.len()
            ));
        }
    }

    Ok(())
}

fn apply_card_patch(card: &mut Card, patch: &CardPatch) -> Result<()> {
    if let Some(cost) = &patch.mana_cost {
        card.cost = cost.clone();
    }
    if let Some(power) = patch.power {
        card.power = Some(power);
    }
    if let Some(toughness) = patch.toughness {
        card.toughness = Some(toughness);
    }
    if let Some(loyalty) = patch.loyalty {
        card.loyalty = Some(loyalty);
    }
    if let Some(keywords) = &patch.keywords {
        card.keywords = keywords.clone();
    }

    for keyword in &patch.add_keywords {
        if !card.keywords.contains(keyword) {
            card.keywords.push(keyword.clone());
        }
    }
    if !patch.remove_keywords.is_empty() {
        card.keywords
            .retain(|keyword| !patch.remove_keywords.contains(keyword));
    }

    for trigger_patch in &patch.trigger_patches {
        let trigger = card.triggers.get_mut(trigger_patch.index).ok_or_else(|| {
            anyhow!(
                "missing trigger {} for {}",
                trigger_patch.index,
                patch.card_id
            )
        })?;
        if let Some(limit) = &trigger_patch.limit {
            trigger.limit = limit.clone();
        }
        if let Some(effect) = &trigger_patch.effect {
            trigger.effect = effect.clone();
        }
    }

    for ability_patch in &patch.ability_patches {
        let ability = card.abilities.get_mut(ability_patch.index).ok_or_else(|| {
            anyhow!(
                "missing ability {} for {}",
                ability_patch.index,
                patch.card_id
            )
        })?;
        if let Some(cost) = &ability_patch.mana_cost {
            replace_ability_mana_cost(ability, cost.clone());
        }
        if let Some(once_per_turn) = ability_patch.once_per_turn {
            ability.once_per_turn = once_per_turn;
        }
        if let Some(effect) = &ability_patch.effect {
            ability.effect = effect.clone();
        }
    }

    card.haste = card
        .keywords
        .iter()
        .any(|keyword| matches!(keyword, Keyword::Haste));

    Ok(())
}

fn replace_ability_mana_cost(ability: &mut CardAbility, mana_cost: ManaCost) {
    let mut replaced = false;
    for cost in &mut ability.cost.costs {
        if let Cost::Mana(existing) = cost {
            *existing = mana_cost.clone();
            replaced = true;
            break;
        }
    }

    if !replaced && mana_cost.total() > 0 {
        ability.cost.costs.insert(0, Cost::Mana(mana_cost));
    }
}

fn is_known_deck(deck_id: &str) -> bool {
    matches!(deck_id, "neon" | "glitch" | "chrome" | "slag" | "bio")
}

fn known_cards_by_id() -> HashMap<String, Card> {
    let mut cards = HashMap::new();

    for (card, _) in card_catalog::neon::all_cards() {
        cards.insert(card.id.clone(), card);
    }
    for (card, _) in card_catalog::glitch::all_cards() {
        cards.insert(card.id.clone(), card);
    }
    for (card, _) in card_catalog::chrome::all_cards() {
        cards.insert(card.id.clone(), card);
    }
    for (card, _) in card_catalog::slag::all_cards() {
        cards.insert(card.id.clone(), card);
    }
    for (card, _) in card_catalog::bio::all_cards() {
        cards.insert(card.id.clone(), card);
    }

    cards
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::{
        DeckTemplates, build_deck_with_templates_and_patch, build_operator_sim_deck_with_patch,
    };
    use crate::types::{CardKind, TriggerEvent};

    #[test]
    fn validates_known_card_and_indices() {
        let patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "signal_blade".into(),
                mana_cost: None,
                power: Some(2),
                toughness: Some(3),
                loyalty: None,
                keywords: None,
                add_keywords: vec![Keyword::Haste],
                remove_keywords: vec![],
                trigger_patches: vec![],
                ability_patches: vec![],
            }],
            deck_entries: vec![DeckEntryPatch {
                deck_id: "neon".into(),
                card_id: "signal_blade".into(),
                quantity: 2,
            }],
        };

        patch.validate().expect("patch should validate");
    }

    #[test]
    fn rejects_unknown_card_ids() {
        let patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "not_a_real_card".into(),
                mana_cost: None,
                power: None,
                toughness: None,
                loyalty: None,
                keywords: None,
                add_keywords: vec![],
                remove_keywords: vec![],
                trigger_patches: vec![],
                ability_patches: vec![],
            }],
            deck_entries: vec![],
        };

        assert!(patch.validate().is_err());
    }

    #[test]
    fn applies_card_and_deck_patches() {
        let mut cards = vec![(
            Card {
                id: "test_card".into(),
                name: "Test".into(),
                description: String::new(),
                auto_description: String::new(),
                rarity: crate::types::CardRarity::Public,
                categories: vec![],
                visual_spec: None,
                template_id: None,
                cost: ManaCost {
                    colorless: 2,
                    ..ManaCost::default()
                },
                kind: CardKind::Instant,
                supertype: crate::types::CardSupertype::Basic,
                power: None,
                toughness: None,
                tapped: false,
                enters_tapped: false,
                triggers: vec![crate::card::CardTrigger {
                    event: TriggerEvent::Entered,
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                }],
                abilities: vec![],
                keywords: vec![],
                image_url: None,
                colors: vec![],
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
            1,
        )];

        let patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "test_card".into(),
                mana_cost: Some(ManaCost {
                    colorless: 1,
                    ..ManaCost::default()
                }),
                power: None,
                toughness: None,
                loyalty: None,
                keywords: None,
                add_keywords: vec![Keyword::Haste],
                remove_keywords: vec![],
                trigger_patches: vec![TriggerPatch {
                    index: 0,
                    limit: Some(TriggerLimit::OncePerTurn),
                    effect: Some(CardEffect::DrawCards(2)),
                }],
                ability_patches: vec![],
            }],
            deck_entries: vec![DeckEntryPatch {
                deck_id: "neon".into(),
                card_id: "signal_blade".into(),
                quantity: 4,
            }],
        };

        patch
            .apply_to_catalog_cards(&mut cards)
            .expect("apply card patch");

        assert_eq!(cards[0].0.cost.colorless, 1);
        assert!(cards[0].0.haste);
        assert_eq!(cards[0].0.triggers[0].limit, TriggerLimit::OncePerTurn);
        assert_eq!(cards[0].0.triggers[0].effect, CardEffect::DrawCards(2));

        let mut quantities = HashMap::from([(String::from("signal_blade"), 1)]);
        patch
            .apply_to_deck_quantities("neon", &mut quantities)
            .expect("apply deck patch");
        assert_eq!(quantities.get("signal_blade"), Some(&4));
    }

    #[test]
    fn builds_patch_evaluation_delta() {
        let baseline = MatrixReport {
            decks: vec!["neon".into()],
            matches_per_pair: 1,
            alternate_seats: true,
            include_mirrors: false,
            deck_metrics: vec![super::super::report::DeckMetrics {
                deck_id: "neon".into(),
                matches: 1,
                finished_matches: 1,
                decisive_matches: 1,
                drawn_matches: 0,
                wins: 1,
                losses: 0,
                unfinished_matches: 0,
                win_rate: Some(1.0),
            }],
            matchup_metrics: vec![],
            balance: super::super::report::BalanceMetrics {
                balance_score: 1.0,
                pairwise_imbalance: 1.0,
                finished_match_rate: 1.0,
                decisive_match_rate: 1.0,
                draw_match_rate: 0.0,
                unfinished_match_rate: 0.0,
                first_player_win_rate: Some(1.0),
                average_rounds: 8.0,
                average_actions: 120.0,
                termination_counts: vec![],
            },
            degeneracy: None,
        };
        let patched = MatrixReport {
            decks: vec!["neon".into()],
            matches_per_pair: 1,
            alternate_seats: true,
            include_mirrors: false,
            deck_metrics: vec![super::super::report::DeckMetrics {
                deck_id: "neon".into(),
                matches: 1,
                finished_matches: 1,
                decisive_matches: 1,
                drawn_matches: 0,
                wins: 0,
                losses: 1,
                unfinished_matches: 0,
                win_rate: Some(0.0),
            }],
            matchup_metrics: vec![],
            balance: super::super::report::BalanceMetrics {
                balance_score: 0.5,
                pairwise_imbalance: 0.25,
                finished_match_rate: 1.0,
                decisive_match_rate: 1.0,
                draw_match_rate: 0.0,
                unfinished_match_rate: 0.0,
                first_player_win_rate: Some(0.5),
                average_rounds: 7.0,
                average_actions: 100.0,
                termination_counts: vec![],
            },
            degeneracy: None,
        };

        let report = build_patch_evaluation_report("patch.json", baseline, patched);
        assert_eq!(report.delta.balance_score_delta, -0.5);
        assert_eq!(report.delta.pairwise_imbalance_delta, -0.75);
        assert_eq!(report.delta.deck_win_rate_deltas.len(), 1);
    }

    #[test]
    fn classic_deck_builder_applies_deck_entry_patch() {
        let patch = BalancePatch {
            cards: vec![],
            deck_entries: vec![DeckEntryPatch {
                deck_id: "neon".into(),
                card_id: "signal_blade".into(),
                quantity: 4,
            }],
        };

        let deck = build_deck_with_templates_and_patch(
            "p1",
            "neon",
            Some(7),
            &DeckTemplates::default(),
            None,
            Some(&patch),
        )
        .expect("patched neon deck should build");

        let copies = deck
            .iter()
            .filter(|card| card.template_id.as_deref() == Some("signal_blade"))
            .count();
        assert_eq!(copies, 4);
    }

    #[test]
    fn operator_builder_applies_operator_card_patch() {
        let patch = BalancePatch {
            cards: vec![CardPatch {
                card_id: "cipher_thief".into(),
                mana_cost: Some(ManaCost {
                    neon: 1,
                    colorless: 1,
                    ..ManaCost::default()
                }),
                power: Some(4),
                toughness: Some(4),
                loyalty: None,
                keywords: None,
                add_keywords: vec![],
                remove_keywords: vec![],
                trigger_patches: vec![],
                ability_patches: vec![],
            }],
            deck_entries: vec![],
        };

        let (_deck, operator) =
            build_operator_sim_deck_with_patch("p1", "neon", "cipher_thief", Some(9), Some(&patch))
                .expect("patched operator deck should build");

        assert_eq!(operator.cost.neon, 1);
        assert_eq!(operator.cost.colorless, 1);
        assert_eq!(operator.power, Some(4));
        assert_eq!(operator.toughness, Some(4));
    }
}
