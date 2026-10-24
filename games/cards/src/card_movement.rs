//! Card movement functions - moving cards between zones

use crate::card::*;
use crate::state::*;
use crate::types::*;

/// Card movement functions
impl CardGameState {
    pub fn detach_attachments(&mut self, card_id: &str) {
        for player in self.players.values_mut() {
            let mut idx = 0;
            while idx < player.battlefield.len() {
                let attached = player.battlefield[idx].attached_to.as_deref() == Some(card_id);
                if !attached {
                    idx += 1;
                    continue;
                }
                if matches!(player.battlefield[idx].kind, CardKind::Relic) {
                    player.battlefield[idx].attached_to = None;
                    idx += 1;
                } else {
                    player.battlefield.remove(idx);
                }
            }
        }
    }

    pub fn remove_from_battlefield_and_attachments(&mut self, owner_id: &str, card_id: &str) {
        let mut card_to_trigger = None;
        if let Some(owner) = self.players.get_mut(owner_id) {
            if let Some(card) = Self::remove_from_battlefield(owner, card_id) {
                card_to_trigger = Some(card);
            }
        }
        if let Some(card) = card_to_trigger {
            self.apply_triggers_for_event(
                owner_id,
                card_id,
                Some(&card.name),
                &card.triggers,
                &TriggerEvent::Left,
                &[],
            );
            self.apply_triggers_for_event(
                owner_id,
                card_id,
                Some(&card.name),
                &card.triggers,
                &TriggerEvent::Died,
                &[],
            );

            // Fire OnUnitDies for other units on the battlefield
            if matches!(card.kind, CardKind::Unit(_)) {
                let other_triggers: Vec<(String, String, String, Vec<CardTrigger>)> = self
                    .players
                    .iter()
                    .flat_map(|(pid, player)| {
                        player.battlefield.iter().map(move |c| {
                            (
                                pid.clone(),
                                c.id.clone(),
                                c.name.clone(),
                                c.triggers.clone(),
                            )
                        })
                    })
                    .collect();
                // Pass the mover's card id in targets so trigger predicates
                // (e.g. `TargetIsApex`) can filter on the dying card.
                let mover_targets = vec![TargetRef::Card(card_id.to_string())];
                for (pid, cid, name, triggers) in other_triggers {
                    self.apply_triggers_for_event(
                        &pid,
                        &cid,
                        Some(&name),
                        &triggers,
                        &TriggerEvent::AllyDied,
                        &mover_targets,
                    );
                }
            }

            // Refund logic for Recursion
            for trigger in &card.triggers {
                if trigger.event == TriggerEvent::Died {
                    if let CardEffect::RefundMana(percentage) = trigger.effect {
                        let total_cost = card.cost.total();
                        let refund =
                            (total_cost as f32 * (percentage as f32 / 10000.0)).floor() as i32;
                        if let Some(owner) = self.players.get_mut(owner_id) {
                            owner.mana_pool.colorless += refund;
                        }
                    }
                }
            }

            // Modular: when this dies, move its +1/+1 counters to another unit
            // the owner controls. If no other friendly unit exists the counters
            // are lost.
            let is_modular = card
                .keywords
                .iter()
                .any(|kw| matches!(kw, Keyword::Modular(_)));
            if is_modular && card.counters > 0 {
                let has_recipient = self
                    .players
                    .get(owner_id)
                    .map(|p| {
                        p.battlefield
                            .iter()
                            .any(|c| matches!(c.kind, CardKind::Unit(_)))
                    })
                    .unwrap_or(false);
                if has_recipient {
                    if let Some(owner) = self.players.get_mut(owner_id) {
                        owner
                            .pending_actions
                            .push(PendingAction::TriggerTargetSelection {
                                source_card_id: card.id.clone(),
                                source_card_name: card.name.clone(),
                                effect: CardEffect::AddCounters(card.counters),
                                target_kind: TargetKind::Unit,
                                optional: false,
                            });
                    }
                }
            }

            if let Some(owner) = self.players.get_mut(owner_id) {
                if self.format == GameFormat::Operator
                    && owner.operator_id.as_deref() == Some(card.id.as_str())
                {
                    let mut operator = card;
                    operator.attached_to = None;
                    operator.remembered_player_id = None;
                    owner.operator_zone.push(operator);
                } else {
                    let mut card = card;
                    card.remembered_player_id = None;
                    owner.graveyard.push(card);
                }
            }
        }
        self.detach_attachments(card_id);
    }

    pub fn exile_from_battlefield_and_attachments(&mut self, owner_id: &str, card_id: &str) {
        let mut card_to_trigger = None;
        if let Some(owner) = self.players.get_mut(owner_id) {
            if let Some(card) = Self::remove_from_battlefield(owner, card_id) {
                card_to_trigger = Some(card);
            }
        }
        if let Some(card) = card_to_trigger {
            self.apply_triggers_for_event(
                owner_id,
                card_id,
                Some(&card.name),
                &card.triggers,
                &TriggerEvent::Left,
                &[],
            );
            if let Some(owner) = self.players.get_mut(owner_id) {
                if self.format == GameFormat::Operator
                    && owner.operator_id.as_deref() == Some(card.id.as_str())
                {
                    let mut operator = card;
                    operator.attached_to = None;
                    operator.remembered_player_id = None;
                    owner.operator_zone.push(operator);
                } else {
                    let mut card = card;
                    card.remembered_player_id = None;
                    owner.exile.push(card);
                }
            }
        }
        self.detach_attachments(card_id);
    }

    pub fn remove_card_from_hand(player: &mut PlayerState, card_id: &str) -> Option<Card> {
        let idx = player.hand.iter().position(|card| card.id == card_id)?;
        Some(player.hand.remove(idx))
    }

    pub fn remove_from_battlefield(player: &mut PlayerState, card_id: &str) -> Option<Card> {
        let idx = player
            .battlefield
            .iter()
            .position(|card| card.id == card_id)?;
        Some(player.battlefield.remove(idx))
    }
}
