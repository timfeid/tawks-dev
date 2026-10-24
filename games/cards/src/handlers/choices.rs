//! Player choice action handlers (scry, triggers)

use crate::card::{Card, CardEffect};
use crate::state::CardGameState;
use crate::types::{TargetKind, TargetRef};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use std::collections::{HashMap, HashSet};

pub trait ChoiceHandlers {
    fn handle_scry_commit(
        &mut self,
        player_id: &str,
        top: Vec<String>,
        bottom: Vec<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_search_commit(&mut self, player_id: &str, card_id: String)
    -> Result<Vec<GameEffect>>;
    fn handle_search_cancel(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_trigger_choice(&mut self, player_id: &str, accept: bool) -> Result<Vec<GameEffect>>;
    fn handle_trigger_target_select(
        &mut self,
        player_id: &str,
        target: TargetRef,
    ) -> Result<Vec<GameEffect>>;
    fn handle_trigger_target_cancel(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_discard_commit(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_discard(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>>;
    fn handle_sacrifice(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>>;
}

impl ChoiceHandlers for CardGameState {
    fn handle_scry_commit(
        &mut self,
        player_id: &str,
        top: Vec<String>,
        bottom: Vec<String>,
    ) -> Result<Vec<GameEffect>> {
        let pending_effects = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            if player.scry.is_empty() {
                return Err(anyhow!("No scry pending"));
            }

            let total = top.len() + bottom.len();
            if total != player.scry.len() {
                return Err(anyhow!("Invalid scry order"));
            }

            let mut seen = HashSet::new();
            for id in top.iter().chain(bottom.iter()) {
                if !seen.insert(id.clone()) {
                    return Err(anyhow!("Duplicate scry card"));
                }
            }

            let mut scry_map: HashMap<String, Card> = player
                .scry
                .iter()
                .cloned()
                .map(|card| (card.id.clone(), card))
                .collect();

            if seen.len() != scry_map.len() {
                return Err(anyhow!("Invalid scry cards"));
            }

            for id in seen.iter() {
                if !scry_map.contains_key(id) {
                    return Err(anyhow!("Invalid scry cards"));
                }
            }

            let mut requested_bottom = Vec::new();
            for id in bottom {
                if let Some(card) = scry_map.remove(&id) {
                    requested_bottom.push(card);
                } else {
                    return Err(anyhow!("Invalid scry order"));
                }
            }

            let mut requested_top = Vec::new();
            for id in top {
                if let Some(card) = scry_map.remove(&id) {
                    requested_top.push(card);
                } else {
                    return Err(anyhow!("Invalid scry order"));
                }
            }

            let is_surveil = player.surveil_active;

            // Other queued effects can legally mutate the deck before the sim agent
            // commits a pending scry. Rebuild the order from any viewed cards that are
            // still present instead of dead-ending on a stale top-of-deck snapshot.
            let mut new_deck = Vec::new();
            for card in requested_bottom {
                if is_surveil {
                    // Surveil: "bottom" cards go to graveyard instead
                    if let Some(idx) = player
                        .deck
                        .iter()
                        .position(|deck_card| deck_card.id == card.id)
                    {
                        let removed = player.deck.remove(idx);
                        player.graveyard.push(removed);
                    }
                } else if let Some(idx) = player
                    .deck
                    .iter()
                    .position(|deck_card| deck_card.id == card.id)
                {
                    new_deck.push(player.deck.remove(idx));
                }
            }
            if !is_surveil {
                new_deck.extend(std::mem::take(&mut player.deck));
            } else {
                new_deck = std::mem::take(&mut player.deck);
            }
            for card in requested_top.into_iter().rev() {
                if let Some(idx) = new_deck
                    .iter()
                    .position(|deck_card| deck_card.id == card.id)
                {
                    let card = new_deck.remove(idx);
                    new_deck.push(card);
                }
            }

            player.deck = new_deck;
            player.scry.clear();
            player.surveil_active = false;
            std::mem::take(&mut player.pending_effects)
        };

        for (effect, targets) in pending_effects {
            self.apply_effect(player_id, &effect, &targets);
        }
        self.try_finish_end_step(player_id);
        Ok(vec![])
    }

    fn handle_search_commit(
        &mut self,
        player_id: &str,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        let (pending_effects, scavenge) = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            if !player.library_search_active {
                return Err(anyhow!("No search pending"));
            }
            if !player
                .library_search_results
                .iter()
                .any(|c| c.id == card_id)
                && !player.scry.iter().any(|c| c.id == card_id)
            {
                return Err(anyhow!("Card not in search results"));
            }
            if let Some(idx) = player.deck.iter().position(|c| c.id == card_id) {
                let card = player.deck.remove(idx);
                player.hand.push(card);
            }
            player.scry.clear();
            player.library_search_results.clear();
            player.library_search_active = false;
            player.card_browser_active = false;
            player.card_browser_title = None;
            player.card_browser_subtitle = None;
            (
                std::mem::take(&mut player.pending_effects),
                player.scavenge_pending.take(),
            )
        };

        // Scavenge: exile the selected card from graveyard and add counters
        if let Some((amount, targets)) = scavenge {
            if let Some(player) = self.players.get_mut(player_id) {
                if let Some(idx) = player.graveyard.iter().position(|c| c.id == card_id) {
                    let card = player.graveyard.remove(idx);
                    player.exile.push(card);
                }
            }
            self.apply_effect(player_id, &CardEffect::AddCounters(amount), &targets);
            for (effect, targets) in pending_effects {
                self.apply_effect(player_id, &effect, &targets);
            }
            return Ok(vec![]);
        }

        let mut selected_from_graveyard = false;
        if !self
            .players
            .get(player_id)
            .map(|player| player.hand.iter().any(|card| card.id == card_id))
            .unwrap_or(false)
        {
            let mut selected_card = None;
            for player in self.players.values_mut() {
                if let Some(idx) = player.graveyard.iter().position(|c| c.id == card_id) {
                    selected_card = Some(player.graveyard.remove(idx));
                    selected_from_graveyard = true;
                    break;
                }
            }
            if let Some(card) = selected_card {
                if let Some(owner) = self.players.get_mut(player_id) {
                    owner.hand.push(card);
                }
            }
        }

        if !selected_from_graveyard {
            self.shuffle_deck_for_player(player_id, "search_choice_miss");
        }

        for (effect, targets) in pending_effects {
            self.apply_effect(player_id, &effect, &targets);
        }
        Ok(vec![])
    }

    fn handle_search_cancel(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        let (searched_library, pending_effects) = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            if !player.library_search_active {
                return Err(anyhow!("No search pending"));
            }
            let searched_library = player
                .library_search_results
                .iter()
                .chain(player.scry.iter())
                .any(|card| player.deck.iter().any(|deck_card| deck_card.id == card.id));
            player.scry.clear();
            player.library_search_results.clear();
            player.library_search_active = false;
            player.card_browser_active = false;
            player.card_browser_title = None;
            player.card_browser_subtitle = None;
            player.scavenge_pending = None;
            (
                searched_library,
                std::mem::take(&mut player.pending_effects),
            )
        };

        if searched_library {
            self.shuffle_deck_for_player(player_id, "search_cancel");
        }

        for (effect, targets) in pending_effects {
            self.apply_effect(player_id, &effect, &targets);
        }
        Ok(vec![])
    }

    fn handle_trigger_choice(&mut self, player_id: &str, accept: bool) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let choice = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            player
                .take_pending_trigger_choice()
                .ok_or_else(|| anyhow!("No trigger choice pending"))?
        };

        let PendingAction::TriggerChoice {
            source_card_id,
            effect,
            targets,
            optional_cost,
            ..
        } = &choice
        else {
            unreachable!();
        };

        if accept {
            if let Some(cost) = optional_cost.as_ref() {
                let can_pay = {
                    let player = self.players.get(player_id).unwrap();
                    player.mana_pool.can_afford(&cost.get_mana()) && player.life > cost.get_life()
                };
                if !can_pay {
                    return Err(anyhow!("Cannot pay optional cost"));
                }
                let player = self.players.get_mut(player_id).unwrap();
                player.mana_pool.spend(&cost.get_mana());
                player.life -= cost.get_life();
            }
            let resolution_snapshot = self.capture_game_log_snapshot();
            self.apply_effect_with_source(player_id, Some(source_card_id), effect, targets);
            let source_name = self
                .find_card_name(source_card_id)
                .unwrap_or_else(|| source_card_id.clone());
            self.log_resolution_summary(
                crate::state::GameLogKind::Resolved,
                player_id,
                format!("{} resolves", source_name),
                &resolution_snapshot,
            );
        }
        Ok(vec![])
    }

    fn handle_trigger_target_cancel(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let pending = {
            let Some(player) = self.players.get(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            match player.pending_trigger_target() {
                Some(PendingAction::TriggerTargetSelection { optional: true, .. }) => {}
                Some(_) => {
                    return Err(anyhow!("Target required; cannot cancel this trigger"));
                }
                None => {
                    return Err(anyhow!("No trigger target selection pending"));
                }
            }
            self.players
                .get_mut(player_id)
                .unwrap()
                .take_pending_trigger_target()
                .unwrap()
        };

        let PendingAction::TriggerTargetSelection { source_card_id, .. } = &pending else {
            unreachable!();
        };

        let source_name = self
            .find_card_name(source_card_id)
            .unwrap_or_else(|| source_card_id.clone());
        self.log(
            crate::state::GameLogKind::Resolved,
            player_id,
            format!("{} declined", source_name),
        );
        Ok(vec![])
    }

    fn handle_trigger_target_select(
        &mut self,
        player_id: &str,
        target: TargetRef,
    ) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let pending = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            player
                .take_pending_trigger_target()
                .ok_or_else(|| anyhow!("No trigger target selection pending"))?
        };

        let PendingAction::TriggerTargetSelection {
            source_card_id,
            effect,
            target_kind,
            ..
        } = &pending
        else {
            unreachable!();
        };

        // Validate target matches the expected kind
        match (&target, target_kind) {
            (TargetRef::Player(pid), TargetKind::Player | TargetKind::PlayerOrUnit) => {
                if pid == player_id {
                    return Err(anyhow!("Cannot target yourself"));
                }
                if !self.players.contains_key(pid.as_str()) {
                    return Err(anyhow!("Invalid target player"));
                }
            }
            (
                TargetRef::Card(card_id),
                TargetKind::Unit | TargetKind::Card | TargetKind::PlayerOrUnit,
            ) => {
                if self.find_battlefield_card_owner(card_id).is_none() {
                    return Err(anyhow!("Target unit not found on battlefield"));
                }
            }
            _ => return Err(anyhow!("Invalid target type for this effect")),
        }

        let effect = effect.clone();
        let resolution_snapshot = self.capture_game_log_snapshot();
        self.apply_effect_with_source(player_id, Some(source_card_id), &effect, &[target]);
        let source_name = self
            .find_card_name(source_card_id)
            .unwrap_or_else(|| source_card_id.clone());
        self.log_resolution_summary(
            crate::state::GameLogKind::Resolved,
            player_id,
            format!("{} resolves", source_name),
            &resolution_snapshot,
        );
        Ok(vec![])
    }

    fn handle_discard_commit(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let (pending_effects, discarded_count, discarded_names) = {
            let player = self
                .players
                .get(player_id)
                .ok_or_else(|| anyhow!("Player not found"))?;

            let amount = match player.pending_discard() {
                Some(PendingAction::Discard { amount, .. }) => *amount,
                _ => return Err(anyhow!("No discard pending")),
            };

            if card_ids.len() != amount {
                return Err(anyhow!("Must discard exactly {} card(s)", amount));
            }

            let mut seen = HashSet::new();
            for id in &card_ids {
                if !seen.insert(id.clone()) {
                    return Err(anyhow!("Duplicate card in discard selection"));
                }
                if !player.hand.iter().any(|c| &c.id == id) {
                    return Err(anyhow!("Card not in hand"));
                }
            }

            let player = self.players.get_mut(player_id).unwrap();
            let card_id_set: HashSet<&String> = card_ids.iter().collect();
            let discarded: Vec<Card> = player
                .hand
                .iter()
                .filter(|c| card_id_set.contains(&c.id))
                .cloned()
                .collect();
            let discarded_names: Vec<String> =
                discarded.iter().map(|card| card.name.clone()).collect();
            player.hand.retain(|c| !card_id_set.contains(&c.id));
            player.graveyard.extend(discarded);
            player.take_pending_discard();

            (
                std::mem::take(&mut player.pending_effects),
                card_ids.len(),
                discarded_names,
            )
        };

        if !discarded_names.is_empty() {
            self.log(
                crate::state::GameLogKind::CardDiscarded,
                player_id,
                format!("discards {}", CardGameState::human_list(&discarded_names)),
            );
        }

        for _ in 0..discarded_count {
            self.apply_all_battlefield_triggers_for_event(
                &crate::types::TriggerEvent::Discarded,
                &[crate::types::TargetRef::Player(player_id.to_string())],
            );
        }

        // Process any pending effects that were queued after the discard
        for (effect, targets) in pending_effects {
            self.apply_effect(player_id, &effect, &targets);
        }

        self.try_finish_end_step(player_id);

        Ok(vec![])
    }

    fn handle_discard(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let (pending_effects, discarded_name) = {
            let player = self
                .players
                .get_mut(player_id)
                .ok_or_else(|| anyhow!("Player not found"))?;

            // Find and remove the card from hand
            let card_index = player
                .hand
                .iter()
                .position(|c| c.id == card_id)
                .ok_or_else(|| anyhow!("Card not in hand"))?;

            let card = player.hand.remove(card_index);
            let discarded_name = card.name.clone();
            player.graveyard.push(card);

            // If there's a pending discard, decrement it
            if let Some(idx) = player
                .pending_actions
                .iter()
                .position(|a| matches!(a, PendingAction::Discard { .. }))
            {
                if let PendingAction::Discard { ref mut amount, .. } = player.pending_actions[idx] {
                    *amount -= 1;
                    if *amount == 0 {
                        player.pending_actions.remove(idx);
                        (std::mem::take(&mut player.pending_effects), discarded_name)
                    } else {
                        (vec![], discarded_name)
                    }
                } else {
                    (vec![], discarded_name)
                }
            } else {
                (vec![], discarded_name)
            }
        };

        self.log(
            crate::state::GameLogKind::CardDiscarded,
            player_id,
            format!("discards {}", discarded_name),
        );

        self.apply_all_battlefield_triggers_for_event(
            &crate::types::TriggerEvent::Discarded,
            &[crate::types::TargetRef::Player(player_id.to_string())],
        );

        // Process any pending effects that were queued after the discard
        for (effect, targets) in pending_effects {
            self.apply_effect(player_id, &effect, &targets);
        }

        self.try_finish_end_step(player_id);

        Ok(vec![])
    }

    fn handle_sacrifice(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>> {
        use crate::state::PendingAction;

        let player = self
            .players
            .get(player_id)
            .ok_or_else(|| anyhow!("Player not found"))?;

        if !player.battlefield.iter().any(|c| c.id == card_id) {
            return Err(anyhow!("Card not on battlefield or not owned by you"));
        }

        self.remove_from_battlefield_and_attachments(player_id, &card_id);
        self.apply_all_battlefield_triggers_for_event(
            &crate::types::TriggerEvent::Sacrificed,
            &[crate::types::TargetRef::Player(player_id.to_string())],
        );

        // Check if this fulfills a pending sacrifice cost
        let resolved = {
            let player = self.players.get_mut(player_id).unwrap();
            if let Some(idx) = player
                .pending_actions
                .iter()
                .position(|a| matches!(a, PendingAction::Sacrifice { .. }))
            {
                if let PendingAction::Sacrifice {
                    ref mut remaining, ..
                } = player.pending_actions[idx]
                {
                    *remaining -= 1;
                    if *remaining == 0 {
                        Some(player.pending_actions.remove(idx))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(PendingAction::Sacrifice {
            source_card_id,
            ability,
            targets,
            uses_stack,
            owner_id,
            ..
        }) = resolved
        {
            if uses_stack {
                self.stack.push(crate::state::StackItem {
                    kind: crate::state::StackItemKind::Ability {
                        source_card_id,
                        ability,
                    },
                    owner_id,
                    targets,
                });
            } else {
                let resolution_snapshot = self.capture_game_log_snapshot();
                let source_name = self
                    .find_card_name(&source_card_id)
                    .unwrap_or_else(|| source_card_id.clone());
                let ability_name = ability.name.clone();
                self.apply_effect_with_source(
                    &owner_id,
                    Some(&source_card_id),
                    &ability.effect,
                    &targets,
                );
                self.log_resolution_summary(
                    crate::state::GameLogKind::Resolved,
                    &owner_id,
                    format!("{} — {} resolves", source_name, ability_name),
                    &resolution_snapshot,
                );
            }
        }

        Ok(vec![])
    }
}
