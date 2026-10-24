//! Spell and ability casting action handlers

use crate::card::{Card, CardEffect};
use crate::state::CardGameState;
use crate::state::{StackItem, StackItemKind};
use crate::types::{CardGameStage, CardKind, Keyword, TargetKind, TargetRef};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;

pub trait SpellHandlers {
    fn handle_cast_spell(
        &mut self,
        player_id: &str,
        card_id: String,
        targets: Vec<TargetRef>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_activate_ability(
        &mut self,
        player_id: &str,
        card_id: String,
        ability_id: String,
        targets: Vec<TargetRef>,
    ) -> Result<Vec<GameEffect>>;
}

impl SpellHandlers for CardGameState {
    fn handle_cast_spell(
        &mut self,
        player_id: &str,
        card_id: String,
        targets: Vec<TargetRef>,
    ) -> Result<Vec<GameEffect>> {
        let is_active = self.is_active_player(player_id);
        let mut from_operator_zone = false;

        let Some(card) = self.players.get(player_id).and_then(|player| {
            player
                .hand
                .iter()
                .find(|card| card.id == card_id)
                .cloned()
                .or_else(|| {
                    if self.format != crate::types::GameFormat::Operator {
                        return None;
                    }
                    player
                        .operator_zone
                        .iter()
                        .find(|card| card.id == card_id)
                        .cloned()
                })
                .or_else(|| {
                    // Gravecast: allow casting from graveyard, up to the
                    // limit encoded in the keyword variant.
                    player
                        .graveyard
                        .iter()
                        .find(|card| {
                            card.id == card_id
                                && card
                                    .gravecast_limit()
                                    .is_some_and(|limit| card.gravecast_uses < limit)
                        })
                        .cloned()
                })
        }) else {
            return Err(anyhow!("Card not found"));
        };

        if self.format == crate::types::GameFormat::Operator {
            if let Some(player) = self.players.get(player_id) {
                if player.operator_zone.iter().any(|card| card.id == card_id) {
                    if player.operator_id.as_deref() != Some(card_id.as_str()) {
                        return Err(anyhow!("Only your operator can be cast from operator zone"));
                    }
                    from_operator_zone = true;
                }
            }
        }

        if matches!(card.kind, CardKind::Infrastructure) {
            return Err(anyhow!("Infrastructures are not spells"));
        }

        let has_flash = card.keywords.contains(&Keyword::Flash);
        let is_instant_speed = has_flash || matches!(card.kind, CardKind::Instant);
        let is_counter = card
            .triggers
            .iter()
            .any(|t| matches!(t.effect, CardEffect::CounterTop));

        if !is_instant_speed
            && self.stage != CardGameStage::Main
            && self.stage != CardGameStage::PostCombatMain
        {
            return Err(anyhow!("This card can only be cast in a main stage"));
        }
        if !is_active && !is_instant_speed && !is_counter {
            return Err(anyhow!("Only the active player can cast this card now"));
        }
        // Block sorcery-speed casts while a unit is still in showcase
        // (pre-resolution display window). Otherwise `can_cast_card`
        // could return true for another hand card and the user would
        // silently fail with "not enough priority" in the log.
        if !is_instant_speed && !is_counter && self.showcase.is_some() {
            return Err(anyhow!("Resolve the pending spell first"));
        }

        self.validate_targets(player_id, &card, &targets)?;

        let mut effective_cost = self.calculate_effective_cost(player_id, &card);
        effective_cost.colorless += self.firewall_tax_for_targets(player_id, &targets);
        if !self.auto_tap_mana(player_id, &effective_cost) {
            return Err(anyhow!("Not enough mana"));
        }

        let mut card = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };

            if from_operator_zone {
                let idx = player
                    .operator_zone
                    .iter()
                    .position(|card| card.id == card_id)
                    .ok_or_else(|| anyhow!("Operator not found in operator zone"))?;
                player.operator_cast_count += 1;
                if player.operator_first_cast_round.is_none() {
                    player.operator_first_cast_round = Some(self.round);
                }
                player.operator_zone.remove(idx)
            } else if let Some(pos) = player.graveyard.iter().position(|c| c.id == card_id) {
                // Gravecast: keep the card in the graveyard so the resolver
                // can find it by id and increment uses (retaining or exiling
                // based on the limit). Only a clone goes on the stack.
                player.graveyard[pos].clone()
            } else {
                let Some(card) = CardGameState::remove_card_from_hand(player, &card_id) else {
                    return Err(anyhow!("Card not found in hand or graveyard"));
                };
                card
            }
        };

        let card_description = if card.auto_description.is_empty() {
            card.auto_description()
        } else {
            card.auto_description.clone()
        };
        let target_clause = self
            .format_target_list(&targets)
            .map(|target_list| format!(" targeting {}", target_list))
            .unwrap_or_default();
        let log_message = if card_description.is_empty() {
            format!("casts {}{}", card.name, target_clause)
        } else {
            format!(
                "casts {}{} — {}",
                card.name, target_clause, card_description
            )
        };
        self.log(crate::state::GameLogKind::CastSpell, player_id, log_message);

        self.emit_spell_cast_triggers(player_id, &card);

        // Cascade: when a card with the Cascade keyword is cast, exile
        // cards from the top of the caster's library until a non-land
        // card with a *lower* total cost is exiled. Cast that card for
        // free (pushes a new spell onto the stack), then put the other
        // exiled cards on the bottom of the library in a random order.
        if card.keywords.contains(&Keyword::Cascade) {
            let source_cost = card.cost.total();
            self.resolve_cascade(player_id, source_cost);
        }

        if matches!(card.kind, CardKind::Unit(_) | CardKind::Agent) {
            let resolution_snapshot = self.capture_game_log_snapshot();
            if matches!(card.kind, CardKind::Unit(_)) {
                card.summoning_sickness = true;
            }
            if card.enters_tapped {
                card.tapped = true;
            }
            // Place into showcase so opponents get a response window before it enters.
            let now = chrono::Utc::now();
            self.showcase = Some(crate::state::ShowcaseEntry {
                card: card.clone(),
                owner_id: player_id.to_string(),
                targets,
                started_at: now,
                resolution_snapshot,
            });
            self.last_stack_push_at = Some(now);
            self.consecutive_priority_passes = 0;
            self.priority_holder = self.next_player_after(player_id);
        } else if matches!(
            card.kind,
            CardKind::Instant | CardKind::Ritual | CardKind::Aura | CardKind::Relic
        ) {
            self.stack.push(StackItem {
                kind: StackItemKind::Spell(card),
                owner_id: player_id.to_string(),
                targets,
            });
            // Pass priority to the next player so they can respond.
            self.last_stack_push_at = Some(chrono::Utc::now());
            self.consecutive_priority_passes = 0;
            self.priority_holder = self.next_player_after(player_id);
        }

        Ok(vec![])
    }

    fn handle_activate_ability(
        &mut self,
        player_id: &str,
        card_id: String,
        ability_id: String,
        targets: Vec<TargetRef>,
    ) -> Result<Vec<GameEffect>> {
        let (card_index, ability) = {
            let Some(player) = self.players.get(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            let card_index = player
                .battlefield
                .iter()
                .position(|card| card.id == card_id)
                .ok_or_else(|| anyhow!("Card not found on battlefield"))?;
            let ability = player.battlefield[card_index]
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .cloned()
                .ok_or_else(|| anyhow!("Ability not found"))?;
            (card_index, ability)
        };

        if ability.is_spell_effect_ability() {
            return Err(anyhow!("This ability only applies as the card is cast"));
        }

        self.validate_targets_with_spec(player_id, ability.target_spec.as_ref(), &targets)?;

        if let Some(predicate) = &ability.predicate {
            if !self.evaluate_predicate(player_id, Some(&card_id), &targets, predicate) {
                return Err(anyhow!("Ability requirements not met"));
            }
            if CardGameState::predicate_contains_once_per_turn(predicate) {
                let key = CardGameState::predicate_key(&ability.id, "ability");
                let count = self
                    .triggers_played_this_turn
                    .get(&key)
                    .copied()
                    .unwrap_or(0);
                if count > 0 {
                    return Err(anyhow!("Ability already used this turn"));
                }
                self.triggers_played_this_turn.insert(key, count + 1);
            }
        }

        if ability.once_per_turn {
            let key = format!("ability::{}::{}", card_id, ability_id);
            let count = self
                .triggers_played_this_turn
                .get(&key)
                .copied()
                .unwrap_or(0);
            if count > 0 {
                return Err(anyhow!("Ability already used this turn"));
            }
            self.triggers_played_this_turn.insert(key, count + 1);
        }

        // Agent: record that a loyalty ability was used this turn
        let is_agent = self
            .players
            .get(player_id)
            .and_then(|p| p.battlefield.get(card_index))
            .map(|c| matches!(c.kind, CardKind::Agent))
            .unwrap_or(false);
        {
            if is_agent && ability.once_per_turn {
                let pw_key = format!("agent::{}", card_id);
                self.triggers_played_this_turn.insert(pw_key, 1);
                if let Some(player) = self.players.get_mut(player_id) {
                    player.loyalty_activations += 1;
                }
            }
        }

        let mut ability_cost = ability.cost.get_mana();
        ability_cost.colorless += self.firewall_tax_for_targets(player_id, &targets);
        if !self.auto_tap_mana(player_id, &ability_cost) {
            return Err(anyhow!("Not enough mana"));
        }

        {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };

            let life_cost = ability.cost.get_life();
            if life_cost > 0 {
                if player.life <= life_cost {
                    return Err(anyhow!("Not enough life"));
                }
                player.life -= life_cost;
            }

            let tap_other_count = ability.cost.get_tap_other_count();
            if tap_other_count > 0 {
                let untapped_count = player
                    .battlefield
                    .iter()
                    .filter(|c| !c.tapped && matches!(c.kind, CardKind::Unit(_)))
                    .count();
                if untapped_count < tap_other_count {
                    return Err(anyhow!("Not enough untapped units to tap"));
                }
                let mut tapped = 0;
                for card in player.battlefield.iter_mut() {
                    if tapped >= tap_other_count {
                        break;
                    }
                    if !card.tapped && matches!(card.kind, CardKind::Unit(_)) {
                        card.tapped = true;
                        tapped += 1;
                    }
                }
            }

            if ability.cost.has_tap() {
                let card_index = player
                    .battlefield
                    .iter()
                    .position(|card| card.id == card_id)
                    .ok_or_else(|| anyhow!("Card not found on battlefield"))?;

                if player.battlefield[card_index].tapped {
                    return Err(anyhow!("Card is already tapped"));
                }
                player.battlefield[card_index].tapped = true;
            }

            let spend_charge = ability.cost.get_spend_charge();
            if spend_charge > 0 {
                let card_index = player
                    .battlefield
                    .iter()
                    .position(|card| card.id == card_id)
                    .ok_or_else(|| anyhow!("Card not found on battlefield"))?;
                if player.battlefield[card_index].counters < spend_charge {
                    return Err(anyhow!("Not enough Charge"));
                }
                player.battlefield[card_index].counters -= spend_charge;
            }

            if ability.tap_targets {
                for target in &targets {
                    if let TargetRef::Card(target_id) = target {
                        let Some(target_card) = player
                            .battlefield
                            .iter_mut()
                            .find(|card| &card.id == target_id)
                        else {
                            return Err(anyhow!("Target not found"));
                        };
                        if target_card.tapped {
                            return Err(anyhow!("Target is already tapped"));
                        }
                        target_card.tapped = true;
                    }
                }
            }
        }

        let sacrifice_count = ability.cost.get_sacrifice_count();
        let source_name = self
            .find_card_name(&card_id)
            .unwrap_or_else(|| card_id.clone());
        let ability_target_clause = self
            .format_target_list(&targets)
            .map(|target_list| format!(" targeting {}", target_list))
            .unwrap_or_default();
        self.log(
            crate::state::GameLogKind::ActivateAbility,
            player_id,
            format!(
                "activates {} — {}{}",
                source_name, ability.name, ability_target_clause
            ),
        );

        if sacrifice_count > 0 {
            let uses_stack = ability.uses_stack;
            // Count sacrificable units (exclude the source card itself)
            let available = self
                .players
                .get(player_id)
                .map(|player| {
                    player
                        .battlefield
                        .iter()
                        .filter(|c| c.id != card_id && matches!(c.kind, CardKind::Unit(_)))
                        .count()
                })
                .unwrap_or(0);
            if available < sacrifice_count {
                return Err(anyhow!("Not enough units to sacrifice"));
            }
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            player
                .pending_actions
                .push(crate::state::PendingAction::Sacrifice {
                    remaining: sacrifice_count,
                    source_card_id: card_id,
                    ability,
                    targets,
                    uses_stack,
                    owner_id: player_id.to_string(),
                });
            return Ok(vec![]);
        }

        if ability.uses_stack {
            self.stack.push(StackItem {
                kind: StackItemKind::Ability {
                    source_card_id: card_id,
                    ability,
                },
                owner_id: player_id.to_string(),
                targets,
            });
            // Pass priority to the next player so they can respond.
            self.consecutive_priority_passes = 0;
            self.priority_holder = self.next_player_after(player_id);
        } else {
            let resolution_snapshot = self.capture_game_log_snapshot();
            self.apply_effect_with_source(player_id, Some(&card_id), &ability.effect, &targets);
            self.log_resolution_summary(
                crate::state::GameLogKind::Resolved,
                player_id,
                format!("{} — {} resolves", source_name, ability.name),
                &resolution_snapshot,
            );
        }

        Ok(vec![])
    }
}

impl CardGameState {
    pub fn apply_resolving_permanent_spell_effects(
        &mut self,
        owner_id: &str,
        card: &Card,
        targets: &[TargetRef],
    ) {
        use crate::state::PendingAction;

        for ability in &card.abilities {
            if !ability.is_spell_effect_ability() {
                continue;
            }
            // For unit cards with targeted ETB effects, defer targeting to trigger system
            // Exception: RememberTargetPlayer still applies at cast time with provided targets
            if matches!(card.kind, CardKind::Unit(_))
                && !matches!(ability.effect, CardEffect::RememberTargetPlayer)
            {
                if let Some(spec) = &ability.target_spec {
                    let target_kind = spec.kind.clone();
                    // Try auto-resolve (e.g. single opponent for player targets)
                    if let Some(auto_target) =
                        Self::auto_resolve_spell_target(self, owner_id, &target_kind)
                    {
                        self.apply_effect_with_source(
                            owner_id,
                            Some(&card.id),
                            &ability.effect,
                            &[auto_target],
                        );
                    } else {
                        if let Some(player) = self.players.get_mut(owner_id) {
                            player
                                .pending_actions
                                .push(PendingAction::TriggerTargetSelection {
                                    source_card_id: card.id.clone(),
                                    source_card_name: card.name.clone(),
                                    effect: ability.effect.clone(),
                                    target_kind,
                                    optional: false,
                                });
                        }
                    }
                    continue;
                }
            }
            self.apply_effect_with_source(owner_id, Some(&card.id), &ability.effect, targets);
        }
    }

    fn auto_resolve_spell_target(
        state: &Self,
        owner_id: &str,
        target_kind: &TargetKind,
    ) -> Option<TargetRef> {
        match target_kind {
            TargetKind::Player => {
                let opponents: Vec<_> = state
                    .players
                    .keys()
                    .filter(|id| id.as_str() != owner_id)
                    .collect();
                if opponents.len() == 1 {
                    Some(TargetRef::Player(opponents[0].clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn resolve_cascade(&mut self, player_id: &str, source_cost: i32) {
        use crate::state::{StackItem, StackItemKind};

        let Some(player) = self.players.get_mut(player_id) else {
            return;
        };
        let mut skipped: Vec<Card> = Vec::new();
        let mut cascaded: Option<Card> = None;
        // Walk the top of the deck looking for a non-Infrastructure card
        // with cost < source_cost. Cap the walk to avoid burning an entire
        // library if everything costs >= source_cost (e.g. mono 0/1-cost
        // decks).
        const MAX_PEEK: usize = 20;
        for _ in 0..MAX_PEEK {
            let Some(top) = player.deck.pop() else { break };
            let is_eligible =
                !matches!(top.kind, CardKind::Infrastructure) && top.cost.total() < source_cost;
            if is_eligible {
                cascaded = Some(top);
                break;
            }
            skipped.push(top);
        }
        // Put skipped cards on the bottom of the library.
        player.deck.splice(0..0, skipped);

        if let Some(cascaded) = cascaded {
            self.log(
                crate::state::GameLogKind::CastSpell,
                player_id,
                format!("cascades into {} (free)", cascaded.name),
            );
            self.stack.push(StackItem {
                kind: StackItemKind::Spell(cascaded),
                owner_id: player_id.to_string(),
                targets: vec![],
            });
            self.last_stack_push_at = Some(chrono::Utc::now());
            self.consecutive_priority_passes = 0;
        }
    }

    fn emit_spell_cast_triggers(&mut self, caster_id: &str, card: &Card) {
        let targets = vec![TargetRef::Player(caster_id.to_string())];

        self.apply_all_battlefield_triggers_for_event(
            &crate::types::TriggerEvent::CastAny,
            &targets,
        );
        self.apply_battlefield_triggers_for_event_except_player(
            caster_id,
            &crate::types::TriggerEvent::OpponentCast,
            &targets,
        );

        if !matches!(card.kind, CardKind::Unit(_)) {
            self.apply_battlefield_triggers_for_event_except_player(
                caster_id,
                &crate::types::TriggerEvent::OpponentCastNonUnit,
                &targets,
            );
        }

        if matches!(card.kind, CardKind::Instant | CardKind::Ritual) {
            self.apply_battlefield_triggers_for_player_event(
                caster_id,
                &crate::types::TriggerEvent::CastInstantOrRitual,
                &targets,
            );
        }

        if matches!(card.kind, CardKind::Relic) {
            self.apply_battlefield_triggers_for_player_event(
                caster_id,
                &crate::types::TriggerEvent::CastRelic,
                &targets,
            );
        }

        if self.active_player_id.as_deref() != Some(caster_id) {
            self.apply_battlefield_triggers_for_player_event(
                caster_id,
                &crate::types::TriggerEvent::CastDuringOpponentTurn,
                &targets,
            );
        }
    }
}
