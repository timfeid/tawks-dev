//! Stack management action handlers

use crate::card::CardEffect;
use crate::state::CardGameState;
use crate::state::StackItemKind;
use crate::types::{CardKind, TargetRef, TriggerEvent};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;

pub trait StackHandlers {
    fn handle_resolve_stack_top(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_cancel_stack_top(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_pass_priority(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
}

impl StackHandlers for CardGameState {
    fn handle_resolve_stack_top(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        // Active-player guard removed: resolution is now triggered by the priority system
        // (handle_pass_priority) or manually by the active player as a fallback.
        let _ = player_id;
        let Some(item) = self.stack.pop() else {
            // Stack is empty — if there's a showcase entry, resolve it instead.
            if let Some(entry) = self.showcase.take() {
                self.resolve_showcase(entry);
                self.reset_priority();
                return Ok(vec![]);
            }
            return Err(anyhow!("Stack is empty"));
        };

        if std::env::var("CARDGAME_SIM_DEBUG")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
        {
            eprintln!(
                "[stack] resolving owner={} kind={:?} targets={:?}",
                item.owner_id, item.kind, item.targets
            );
        }

        self.history.push(item.clone());
        let resolution_snapshot = self.capture_game_log_snapshot();
        let resolution_subject = match &item.kind {
            StackItemKind::Spell(card) => format!("{} resolves", card.name),
            StackItemKind::Ability {
                source_card_id,
                ability,
            } => {
                let source_name = self
                    .find_card_name(source_card_id)
                    .unwrap_or_else(|| source_card_id.clone());
                format!("{} — {} resolves", source_name, ability.name)
            }
        };

        match &item.kind {
            StackItemKind::Spell(card) => {
                if matches!(card.kind, CardKind::Aura)
                    && card.attachment.is_some()
                    && !item.targets.is_empty()
                {
                    self.resolve_attachment(&item.owner_id, card.clone(), &item.targets);
                } else if matches!(card.kind, CardKind::Aura | CardKind::Relic) {
                    let mut resolved_card = card.clone();
                    if resolved_card.enters_tapped {
                        resolved_card.tapped = true;
                    }
                    if let Some(TargetRef::Player(target_player_id)) = item.targets.first() {
                        resolved_card.attached_to = Some(target_player_id.clone());
                    }
                    if let Some(owner) = self.players.get_mut(&item.owner_id) {
                        owner.battlefield.push(resolved_card);
                        let entered = owner.battlefield.last().cloned();
                        drop(owner);
                        if let Some(card) = entered {
                            self.apply_resolving_permanent_spell_effects(
                                &item.owner_id,
                                &card,
                                &item.targets,
                            );
                            self.handle_card_played(&item.owner_id, &card, &item.targets);
                        }
                    }
                } else {
                    // Apply OnPlay triggers for instants/sorceries
                    for trigger in &card.triggers {
                        if trigger.event == TriggerEvent::Entered {
                            self.apply_effect_with_source(
                                &item.owner_id,
                                Some(&card.id),
                                &trigger.effect,
                                &item.targets,
                            );
                        }
                    }
                    // Also apply abilities with no cost (used for targeted effects)
                    for ability in &card.abilities {
                        if ability.cost.is_empty() {
                            self.apply_effect_with_source(
                                &item.owner_id,
                                Some(&card.id),
                                &ability.effect,
                                &item.targets,
                            );
                        }
                    }
                    if let Some(owner) = self.players.get_mut(&item.owner_id) {
                        // Gravecast cards cast from graveyard: increment use count;
                        // if they've hit their limit exile them, otherwise leave
                        // them in the graveyard for another cast.
                        let was_in_graveyard = owner.graveyard.iter().any(|c| c.id == card.id);
                        if was_in_graveyard {
                            let limit = card.gravecast_limit().unwrap_or(1);
                            let existing = owner.graveyard.iter_mut().find(|c| c.id == card.id);
                            if let Some(existing) = existing {
                                existing.gravecast_uses = existing.gravecast_uses.saturating_add(1);
                                if existing.gravecast_uses >= limit {
                                    owner.graveyard.retain(|c| c.id != card.id);
                                }
                            }
                        } else {
                            owner.graveyard.push(card.clone());
                        }
                    }
                }
            }
            StackItemKind::Ability {
                source_card_id,
                ability,
            } => {
                if matches!(ability.effect, CardEffect::Equip) {
                    let Some(TargetRef::Card(target_id)) = item.targets.first() else {
                        return Err(anyhow!("Invalid target"));
                    };
                    self.attach_equipment(&item.owner_id, source_card_id, target_id)?;
                } else {
                    self.apply_effect_with_source(
                        &item.owner_id,
                        Some(source_card_id),
                        &ability.effect,
                        &item.targets,
                    );
                }
            }
        }

        // Let state-based actions (e.g. lethal damage) settle before we
        // snapshot the board for the resolution log so entries like "Weak
        // Unit dies" appear alongside the life-loss summary.
        self.apply_passive_state_updates();
        self.log_resolution_summary(
            crate::state::GameLogKind::Resolved,
            &item.owner_id,
            resolution_subject,
            &resolution_snapshot,
        );

        // Reset priority after resolving.
        // This must be after effects (e.g. CounterTop may pop additional stack items).
        if self.stack.is_empty() && self.showcase.is_none() {
            self.reset_priority();
            self.last_stack_push_at = None;
        } else {
            // More items on stack or showcase pending — reassign priority to active player
            self.consecutive_priority_passes = 0;
            self.priority_holder = self.active_player_id.clone();
        }

        Ok(vec![])
    }

    fn handle_cancel_stack_top(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can cancel the stack"));
        }
        if self.stack.pop().is_none() {
            return Err(anyhow!("Stack is empty"));
        }
        Ok(vec![])
    }

    fn handle_pass_priority(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.effective_priority_holder() != Some(player_id) {
            return Err(anyhow!("You do not hold priority"));
        }

        self.consecutive_priority_passes += 1;

        if self.consecutive_priority_passes >= self.turn_order.len() {
            // All players have passed — resolve the top of the stack if non-empty.
            if !self.stack.is_empty() {
                let active_id = self
                    .active_player_id
                    .clone()
                    .unwrap_or_else(|| player_id.to_string());
                self.handle_resolve_stack_top(&active_id)?;
            }

            // If the stack is now empty and there's a showcase entry, resolve it.
            if self.stack.is_empty() {
                if let Some(entry) = self.showcase.take() {
                    self.resolve_showcase(entry);
                }
            }

            self.reset_priority();
        } else {
            self.priority_holder = self.next_player_after(player_id);
        }

        Ok(vec![])
    }
}

impl CardGameState {
    /// Resolve a showcase entry — the Unit/Agent enters the battlefield.
    pub(crate) fn resolve_showcase(&mut self, entry: crate::state::ShowcaseEntry) {
        let crate::state::ShowcaseEntry {
            card,
            owner_id,
            targets,
            resolution_snapshot,
            ..
        } = entry;

        let entered = {
            let Some(player) = self.players.get_mut(&owner_id) else {
                return;
            };
            player.battlefield.push(card);
            player.battlefield.last().cloned()
        };
        if let Some(card) = entered {
            self.apply_resolving_permanent_spell_effects(&owner_id, &card, &targets);
            self.handle_card_played(&owner_id, &card, &targets);
            self.log_resolution_summary(
                crate::state::GameLogKind::Resolved,
                &owner_id,
                format!("{} resolves", card.name),
                &resolution_snapshot,
            );
        }
        self.last_stack_push_at = None;
    }
}
