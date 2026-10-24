//! Validation utilities for card game rules

use crate::card::{Card, CardEffect, TargetSpec};
use crate::deck::DeckTemplateEntry;
use crate::state::*;
use crate::types::CardKind;
use crate::types::*;
use anyhow::{Result, anyhow};
use std::collections::HashSet;

/// Gets the color identity of a card based on its mana costs (casting cost, ability costs, and trigger costs)
pub fn get_card_color_identity(card: &Card) -> HashSet<ManaType> {
    let mut identity = HashSet::new();

    // 1. Casting Cost
    if card.cost.neon > 0 {
        identity.insert(ManaType::Neon);
    }
    if card.cost.glitch > 0 {
        identity.insert(ManaType::Glitch);
    }
    if card.cost.chrome > 0 {
        identity.insert(ManaType::Chrome);
    }
    if card.cost.slag > 0 {
        identity.insert(ManaType::Slag);
    }
    if card.cost.bio > 0 {
        identity.insert(ManaType::Bio);
    }

    // 2. Ability Costs
    for ability in &card.abilities {
        let mana = ability.cost.get_mana();
        if mana.neon > 0 {
            identity.insert(ManaType::Neon);
        }
        if mana.glitch > 0 {
            identity.insert(ManaType::Glitch);
        }
        if mana.chrome > 0 {
            identity.insert(ManaType::Chrome);
        }
        if mana.slag > 0 {
            identity.insert(ManaType::Slag);
        }
        if mana.bio > 0 {
            identity.insert(ManaType::Bio);
        }
    }

    // 3. Trigger Optional Costs
    for trigger in &card.triggers {
        if let Some(cost) = &trigger.optional_cost {
            let mana = cost.get_mana();
            if mana.neon > 0 {
                identity.insert(ManaType::Neon);
            }
            if mana.glitch > 0 {
                identity.insert(ManaType::Glitch);
            }
            if mana.chrome > 0 {
                identity.insert(ManaType::Chrome);
            }
            if mana.slag > 0 {
                identity.insert(ManaType::Slag);
            }
            if mana.bio > 0 {
                identity.insert(ManaType::Bio);
            }
        }
    }

    identity
}

/// Validates that an operator deck follows all operator format rules:
/// - Exactly 66 cards (no basic infrastructure — The Grid provides mana)
/// - Singleton (except Infrastructure)
/// - Operator is apex unit/agent/artifact
/// - All cards match operator's color identity
pub fn validate_operator_deck_templates(
    templates: &[DeckTemplateEntry],
    operator_card_id: &str,
) -> Result<()> {
    let total: i32 = templates.iter().map(|entry| entry.quantity).sum();
    if total != 67 {
        return Err(anyhow!(
            "Operator decks must contain exactly 67 cards (66 + 1 operator), found {}",
            total
        ));
    }

    if templates
        .iter()
        .any(|entry| entry.quantity != 1 && !matches!(entry.card.kind, CardKind::Infrastructure))
    {
        return Err(anyhow!(
            "Operator decks must be singleton (1 copy per card, except for Infrastructure)"
        ));
    }

    let operator_entry = templates
        .iter()
        .find(|entry| entry.card.id == operator_card_id)
        .ok_or_else(|| anyhow!("Operator card not found in deck"))?;

    if operator_entry.card.supertype != CardSupertype::Apex {
        return Err(anyhow!("Operator must be apex"));
    }

    if !matches!(
        operator_entry.card.kind,
        CardKind::Unit(_) | CardKind::Agent | CardKind::Relic
    ) {
        return Err(anyhow!("Operator must be a unit, agent, or artifact"));
    }

    let operator_identity = get_card_color_identity(&operator_entry.card);

    for entry in templates {
        let card_identity = get_card_color_identity(&entry.card);
        for color in card_identity {
            if !operator_identity.contains(&color) {
                return Err(anyhow!(
                    "{} uses {:?} outside operator identity",
                    entry.card.name,
                    color
                ));
            }
        }
    }

    Ok(())
}

/// Filters effect kinds to only include those in the allowed list
pub fn filter_effect_kinds(values: Vec<String>, allowed: &[&str]) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| allowed.iter().any(|allowed| allowed == value))
        .collect()
}

/// Target validation functions
impl CardGameState {
    pub fn validate_targets_with_spec(
        &self,
        player_id: &str,
        spec: Option<&TargetSpec>,
        targets: &[TargetRef],
    ) -> Result<()> {
        let Some(spec) = spec else {
            if targets.is_empty() {
                return Ok(());
            }
            return Err(anyhow!("Card does not accept targets"));
        };

        if targets.is_empty() {
            if spec.optional {
                return Ok(());
            }
            return Err(anyhow!("Target required"));
        }

        if !spec.optional && targets.len() < spec.max_targets {
            return Err(anyhow!(
                "Need {} target(s), got {}",
                spec.max_targets,
                targets.len()
            ));
        }

        if targets.len() > spec.max_targets {
            return Err(anyhow!("Too many targets"));
        }

        for target in targets {
            match (&spec.kind, target) {
                (TargetKind::Player, TargetRef::Player(target_player_id)) => {
                    match spec.scope {
                        TargetScope::SelfPlayer if target_player_id.as_str() != player_id => {
                            return Err(anyhow!("Invalid target"));
                        }
                        TargetScope::Opponent if target_player_id.as_str() == player_id => {
                            return Err(anyhow!("Invalid target"));
                        }
                        TargetScope::Any | TargetScope::AnyPlayerOrUnit => {}
                        TargetScope::SelfPlayer | TargetScope::Opponent => {}
                    }
                    if !self.players.contains_key(target_player_id.as_str()) {
                        return Err(anyhow!("Invalid target"));
                    }
                }
                (TargetKind::Unit, TargetRef::Card(card_id)) => {
                    if !self.can_target_card(player_id, card_id.as_str()) {
                        return Err(anyhow!("Cannot target this card (Camouflage/Hexproof)"));
                    }
                    let Some((owner_id, kind)) = self.find_battlefield_card_owner(card_id.as_str())
                    else {
                        return Err(anyhow!("Invalid target"));
                    };
                    if !matches!(kind, CardKind::Unit(_)) {
                        return Err(anyhow!("Invalid target"));
                    }
                    match spec.scope {
                        TargetScope::SelfPlayer if owner_id != player_id => {
                            return Err(anyhow!("Invalid target"));
                        }
                        TargetScope::Opponent if owner_id == player_id => {
                            return Err(anyhow!("Invalid target"));
                        }
                        TargetScope::Any | TargetScope::AnyPlayerOrUnit => {}
                        TargetScope::SelfPlayer | TargetScope::Opponent => {}
                    }
                }
                (TargetKind::PlayerOrUnit, TargetRef::Player(target_player_id)) => {
                    if !matches!(spec.scope, TargetScope::Any | TargetScope::AnyPlayerOrUnit) {
                        return Err(anyhow!("Invalid target"));
                    }
                    if !self.players.contains_key(target_player_id.as_str()) {
                        return Err(anyhow!("Invalid target"));
                    }
                }
                (TargetKind::PlayerOrUnit, TargetRef::Card(card_id)) => {
                    if !self.can_target_card(player_id, card_id.as_str()) {
                        return Err(anyhow!("Cannot target this card (Camouflage/Hexproof)"));
                    }
                    if !matches!(spec.scope, TargetScope::Any | TargetScope::AnyPlayerOrUnit) {
                        return Err(anyhow!("Invalid target"));
                    }
                    let Some((_owner_id, kind)) =
                        self.find_battlefield_card_owner(card_id.as_str())
                    else {
                        return Err(anyhow!("Invalid target"));
                    };
                    if !matches!(kind, CardKind::Unit(_)) {
                        return Err(anyhow!("Invalid target"));
                    }
                }
                _ => return Err(anyhow!("Invalid target")),
            }
        }

        Ok(())
    }

    pub fn validate_targets(
        &self,
        player_id: &str,
        card: &Card,
        targets: &[TargetRef],
    ) -> Result<()> {
        // Unit cards handle targeting at ETB time, not cast time
        // Exception: RememberTargetPlayer abilities still need cast-time targeting
        if matches!(card.kind, CardKind::Unit(_)) {
            let has_remember = card.abilities.iter().any(|a| {
                a.is_spell_effect_ability() && matches!(a.effect, CardEffect::RememberTargetPlayer)
            });
            if !has_remember {
                if !targets.is_empty() {
                    return Err(anyhow!("Unit cards do not take cast-time targets"));
                }
                return Ok(());
            }
        }

        // Find target_spec from spell-effect abilities (not activated/agent abilities)
        let target_spec = card
            .abilities
            .iter()
            .find(|a| a.is_spell_effect_ability())
            .and_then(|a| a.target_spec.as_ref());

        if let Some(spec) = target_spec {
            // Card has a target spec - validate targets
            if !spec.optional && spec.max_targets > 0 && targets.len() < spec.max_targets {
                return Err(anyhow!(
                    "Need {} target(s), got {}",
                    spec.max_targets,
                    targets.len()
                ));
            }
            if targets.len() > spec.max_targets {
                return Err(anyhow!("Too many targets"));
            }
        } else if !targets.is_empty() {
            return Err(anyhow!("Card does not take targets"));
        }

        for target in targets {
            if let TargetRef::Card(target_id) = target {
                if !self.can_target_card(player_id, target_id.as_str()) {
                    return Err(anyhow!("Cannot target this card (Camouflage/Hexproof)"));
                }
            }
        }
        Ok(())
    }
}
