//! Predicate evaluation functions

use crate::card::card_color_identities;
use crate::state::*;
use crate::types::*;

/// Predicate evaluation functions
impl CardGameState {
    pub fn predicate_contains_once_per_turn(predicate: &Predicate) -> bool {
        match predicate {
            Predicate::Condition(Condition::OncePerTurn) => true,
            Predicate::Condition(_) | Predicate::CardKind(_) | Predicate::IsUnit => false,
            Predicate::All(preds) | Predicate::Any(preds) => {
                preds.iter().any(Self::predicate_contains_once_per_turn)
            }
            Predicate::Not(inner) => Self::predicate_contains_once_per_turn(inner),
        }
    }

    pub fn predicate_key(source_id: &str, event: &str) -> String {
        format!("predicate::{}::{}", source_id, event)
    }

    pub fn evaluate_condition(
        &self,
        owner_id: &str,
        source_id: Option<&str>,
        targets: &[TargetRef],
        condition: &Condition,
    ) -> bool {
        match condition {
            Condition::IsActivePlayer => self.is_active_player(owner_id),
            Condition::StageIs(stage) => &self.stage == stage,
            Condition::ManaAtLeast(amount) => {
                let available = self.available_mana_for_player(owner_id);
                available.can_afford(amount)
            }
            Condition::TargetIsOpponentPlayer => targets.iter().any(
                |target| matches!(target, TargetRef::Player(player_id) if player_id != owner_id),
            ),
            Condition::TargetMatchesRememberedPlayer => {
                let Some(source_id) = source_id else {
                    return false;
                };
                let remembered_player_id =
                    self.find_battlefield_card_owner(source_id)
                        .and_then(|(source_owner_id, _)| {
                            self.players.get(&source_owner_id).and_then(|player| {
                                player
                                    .battlefield
                                    .iter()
                                    .find(|card| card.id == source_id)
                                    .and_then(|card| card.remembered_player_id.clone())
                            })
                        });

                targets.iter().any(|target| {
                    matches!(
                        (target, remembered_player_id.as_ref()),
                        (TargetRef::Player(player_id), Some(remembered_player_id))
                            if player_id == remembered_player_id
                    )
                })
            }
            Condition::HasUntappedUnits { count, unit_type } => self
                .players
                .get(owner_id)
                .map(|player| {
                    let mut total = 0;
                    for card in &player.battlefield {
                        if card.tapped {
                            continue;
                        }
                        if let Some(sid) = source_id {
                            if card.id == sid {
                                continue;
                            }
                        }
                        match &card.kind {
                            CardKind::Unit(kind) => {
                                if unit_type
                                    .as_ref()
                                    .map(|filter| filter == kind)
                                    .unwrap_or(true)
                                {
                                    total += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    total >= *count
                })
                .unwrap_or(false),
            Condition::TargetCountAtLeast(count) => targets.len() >= *count,
            Condition::TargetCountExactly(count) => targets.len() == *count,
            Condition::TargetIsUnitType(unit_type) => self.has_target_unit_type(targets, unit_type),
            Condition::TargetIsKind(kind) => self.has_target_kind(targets, kind),
            Condition::TargetIsCardKind(kind) => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.find_battlefield_card_owner(card_id)
                        .map(|(_, card_kind)| card_kind == *kind)
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Condition::TargetIsManaType(mana_type) => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.find_card_with_owner_and_zone(card_id)
                        .map(|(_, card, _)| card_color_identities(&card).contains(mana_type))
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Condition::TargetIsTapped => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.find_battlefield_card_owner(card_id)
                        .and_then(|(owner, _)| {
                            self.players
                                .get(&owner)
                                .and_then(|p| p.battlefield.iter().find(|c| &c.id == card_id))
                        })
                        .map(|card| card.tapped)
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Condition::TargetIsApex => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    // Check the card anywhere (battlefield or graveyard) for Apex supertype.
                    self.find_card_with_owner_and_zone(card_id)
                        .map(|(_, card, _)| card.supertype == CardSupertype::Apex)
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Condition::TargetPlayerHandSizeExactly(count) => targets.iter().any(|target| {
                if let TargetRef::Player(player_id) = target {
                    self.players
                        .get(player_id)
                        .map(|player| player.hand.len() == *count)
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Condition::OncePerTurn => true,
            Condition::LifeLessThan(amount) => self
                .players
                .get(owner_id)
                .map(|p| p.life < *amount)
                .unwrap_or(false),
            Condition::InGraveyard => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.players
                        .values()
                        .any(|p| p.graveyard.iter().any(|c| &c.id == card_id))
                } else {
                    false
                }
            }),
            Condition::LastCastWas(kind) => self
                .history
                .last()
                .map(|item| match &item.kind {
                    StackItemKind::Spell(card) => &card.kind == kind,
                    _ => false,
                })
                .unwrap_or(false),
        }
    }

    pub fn evaluate_predicate(
        &self,
        owner_id: &str,
        source_id: Option<&str>,
        targets: &[TargetRef],
        predicate: &Predicate,
    ) -> bool {
        match predicate {
            Predicate::Condition(condition) => {
                self.evaluate_condition(owner_id, source_id, targets, condition)
            }
            Predicate::CardKind(kind) => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.find_battlefield_card_owner(card_id)
                        .map(|(_, k)| &k == kind)
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Predicate::IsUnit => targets.iter().any(|target| {
                if let TargetRef::Card(card_id) = target {
                    self.find_battlefield_card_owner(card_id)
                        .map(|(_, k)| matches!(k, CardKind::Unit(_)))
                        .unwrap_or(false)
                } else {
                    false
                }
            }),
            Predicate::All(predicates) => predicates
                .iter()
                .all(|predicate| self.evaluate_predicate(owner_id, source_id, targets, predicate)),
            Predicate::Any(predicates) => predicates
                .iter()
                .any(|predicate| self.evaluate_predicate(owner_id, source_id, targets, predicate)),
            Predicate::Not(predicate) => {
                !self.evaluate_predicate(owner_id, source_id, targets, predicate)
            }
        }
    }

    pub fn evaluate_predicate_for_ui(
        &self,
        owner_id: &str,
        source_id: Option<&str>,
        targets: &[TargetRef],
        predicate: &Predicate,
    ) -> bool {
        match predicate {
            Predicate::Condition(Condition::TargetCountAtLeast(_))
            | Predicate::Condition(Condition::TargetCountExactly(_))
            | Predicate::Condition(Condition::TargetIsUnitType(_))
            | Predicate::Condition(Condition::TargetIsKind(_))
            | Predicate::CardKind(_)
            | Predicate::IsUnit => true,
            _ => self.evaluate_predicate(owner_id, source_id, targets, predicate),
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::card::*;
    use crate::state::*;
    use crate::types::*;

    #[test]
    fn test_target_is_mana_type() {
        let mut state = CardGameState::default();

        let neon_card = CardBuilder::new("neon_unit", "Neon Unit", CardKind::Unit(UnitType::Drone))
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .build();

        let glitch_card = CardBuilder::new(
            "glitch_unit",
            "Glitch Unit",
            CardKind::Unit(UnitType::Virus),
        )
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .build();

        // Add them to p1's battlefield
        let mut p1 = PlayerState::new(
            "p1".to_string(),
            "Player 1".to_string(),
            "avatar".into(),
            "instance".into(),
            20,
        );
        p1.battlefield.push(neon_card.clone());
        p1.battlefield.push(glitch_card.clone());
        state.players.insert("p1".to_string(), p1);

        let condition_neon = Condition::TargetIsManaType(ManaType::Neon);
        let condition_glitch = Condition::TargetIsManaType(ManaType::Glitch);
        let condition_chrome = Condition::TargetIsManaType(ManaType::Chrome);

        // Test Neon card matches Neon condition
        assert!(state.evaluate_condition(
            "p1",
            None,
            &[TargetRef::Card("neon_unit".to_string())],
            &condition_neon
        ));

        // Test Neon card does NOT match Glitch condition
        assert!(!state.evaluate_condition(
            "p1",
            None,
            &[TargetRef::Card("neon_unit".to_string())],
            &condition_glitch
        ));

        // Test Glitch card matches Glitch condition
        assert!(state.evaluate_condition(
            "p1",
            None,
            &[TargetRef::Card("glitch_unit".to_string())],
            &condition_glitch
        ));

        // Test Glitch card does NOT match Chrome condition
        assert!(!state.evaluate_condition(
            "p1",
            None,
            &[TargetRef::Card("glitch_unit".to_string())],
            &condition_chrome
        ));
    }
}
