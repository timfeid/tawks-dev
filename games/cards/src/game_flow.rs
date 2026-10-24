//! Game flow functions - turn management, drawing, and game progression

use crate::card::{Card, CardTrigger};
use crate::state::*;
use crate::types::*;

/// Game flow functions
impl CardGameState {
    pub fn draw_card(player: &mut PlayerState) {
        if let Some(card) = player.deck.pop() {
            player.hand.push(card);
        }
    }

    pub fn draw_card_for_player(&mut self, player_id: &str) -> bool {
        let drew = if let Some(player) = self.players.get_mut(player_id) {
            if let Some(card) = player.deck.pop() {
                player.hand.push(card);
                true
            } else {
                // Mill-out: attempting to draw from an empty library is fatal.
                // Set life to 0 and let the normal elimination logic in tick() handle it.
                player.life = 0;
                false
            }
        } else {
            false
        };

        if drew {
            let targets = vec![TargetRef::Player(player_id.to_string())];
            self.apply_battlefield_triggers_for_player_event(
                player_id,
                &TriggerEvent::DrewCard,
                &targets,
            );
            self.apply_battlefield_triggers_for_event_except_player(
                player_id,
                &TriggerEvent::OpponentDrew,
                &targets,
            );
        }

        drew
    }

    pub fn max_hand_size(&self) -> usize {
        match self.format {
            GameFormat::Classic => 7,
            GameFormat::Operator => 8,
        }
    }

    pub fn queue_end_step_discard_if_needed(&mut self, player_id: &str) -> bool {
        let max_hand_size = self.max_hand_size();
        let Some(player) = self.players.get_mut(player_id) else {
            return false;
        };

        let overflow = player.hand.len().saturating_sub(max_hand_size);
        if overflow == 0 {
            return false;
        }

        let source_description = format!(
            "Discard down to {} cards before ending your turn.",
            max_hand_size
        );

        if let Some(PendingAction::Discard {
            amount,
            source_description: pending_source,
            source_card_id,
        }) = player
            .pending_actions
            .iter_mut()
            .find(|action| matches!(action, PendingAction::Discard { .. }))
        {
            *amount = overflow;
            *pending_source = source_description;
            *source_card_id = None;
        } else {
            player.pending_actions.push(PendingAction::Discard {
                amount: overflow,
                source_description,
                source_card_id: None,
            });
        }

        self.stage = CardGameStage::End;
        true
    }

    pub fn try_finish_end_step(&mut self, player_id: &str) {
        if self.stage != CardGameStage::End || self.active_player_id.as_deref() != Some(player_id) {
            return;
        }

        let max_hand_size = self.max_hand_size();
        let Some(player) = self.players.get(player_id) else {
            return;
        };

        if player.has_any_pending() || !player.scry.is_empty() || player.library_search_active {
            return;
        }

        // If a discard trigger drew cards and pushed hand back above max,
        // re-queue the discard rather than leaving the player stuck.
        if player.hand.len() > max_hand_size {
            self.queue_end_step_discard_if_needed(player_id);
            return;
        }

        self.advance_turn();
    }

    pub fn start_turn(&mut self, player_id: &str) {
        // Untap active player's permanents
        if let Some(player) = self.players.get_mut(player_id) {
            player.infrastructures_played_this_turn = 0;
            player.nodes_deployed_this_turn = 0;
            let syndicate = self.ruleset == Ruleset::Syndicate;
            if !syndicate {
                Self::refresh_mana(player);
            }
            // Untap permanents BEFORE refilling the pool. `fill_mana_from_infrastructure`
            // marks each contributing infra as tapped so its output isn't counted twice
            // by `available_mana_for_player`; if untap ran after, those taps would be
            // undone and the bug would return.
            for card in player.battlefield.iter_mut() {
                card.summoning_sickness = false;
                // Frozen cards stay tapped and burn one "frozen turn" of
                // the freeze counter. They're untapped only once the
                // counter reaches 0.
                if card.frozen_turns > 0 {
                    card.frozen_turns -= 1;
                    continue;
                }
                if card.burnout {
                    card.burnout = false;
                } else {
                    card.tapped = false;
                }
            }
            if syndicate {
                crate::syndicate::collect_income(player);
            } else {
                Self::fill_mana_from_grid(player);
                Self::fill_mana_from_infrastructure(player);
            }
        }
        if self.ruleset == Ruleset::Syndicate {
            self.syndicate_repossess(player_id);
        }

        // First-player compensation: whoever goes second gets +1 colorless
        // mana on their very first turn, stacking with the going-first
        // skip-first-draw rule for the full compensation package.
        if self.round == 1 {
            let is_not_first = self
                .turn_order
                .first()
                .map(|first| first != player_id)
                .unwrap_or(false);
            if is_not_first {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.mana_pool.colorless += 1;
                }
            }
        }

        // Rootborn Awakener effect: Untap other players' permanents if they have UntapDuringOtherTurns
        for (pid, player) in self.players.iter_mut() {
            if pid == player_id {
                continue; // Skip active player (already handled above)
            }

            // Check if this player has any units with UntapDuringOtherTurns
            let has_seedborn = player
                .battlefield
                .iter()
                .any(|card| card.keywords.contains(&Keyword::UntapDuringOtherTurns));

            if has_seedborn {
                // Untap all permanents for this player (still honor freeze)
                for card in player.battlefield.iter_mut() {
                    if card.frozen_turns > 0 {
                        card.frozen_turns -= 1;
                        continue;
                    }
                    if card.burnout {
                        card.burnout = false;
                    } else {
                        card.tapped = false;
                    }
                }
            }
        }

        // Echo: for each of the active player's permanents that came
        // under their control since the last upkeep (echo_pending=true),
        // auto-pay the echo cost from available mana; if they can't pay,
        // sacrifice it. Simplification: no player choice — always pay
        // when possible because sacrificing a unit is nearly always
        // worse than paying N mana.
        let echo_decisions: Vec<(String, ManaCost)> = self
            .players
            .get(player_id)
            .map(|p| {
                p.battlefield
                    .iter()
                    .filter(|c| c.echo_pending)
                    .filter_map(|c| {
                        c.keywords.iter().find_map(|k| match k {
                            Keyword::Echo(cost) => Some((c.id.clone(), cost.clone())),
                            _ => None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (card_id, echo_cost) in echo_decisions {
            let can_pay = self
                .players
                .get(player_id)
                .map(|p| p.mana_pool.can_afford(&echo_cost))
                .unwrap_or(false);
            if can_pay {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.mana_pool.spend(&echo_cost);
                    if let Some(card) = player.battlefield.iter_mut().find(|c| c.id == card_id) {
                        card.echo_pending = false;
                    }
                }
                self.log(
                    GameLogKind::Resolved,
                    player_id,
                    format!("paid Echo cost for {}", card_id),
                );
            } else {
                // Sacrifice
                self.log(
                    GameLogKind::Resolved,
                    player_id,
                    format!("sacrificed {} (unpaid Echo)", card_id),
                );
                self.remove_from_battlefield_and_attachments(player_id, &card_id);
            }
        }

        // Apply OnTurnStart triggers
        let battlefield_cards: Vec<(String, String, String, Vec<CardTrigger>)> = self
            .players
            .get(player_id)
            .map(|p| {
                p.battlefield
                    .iter()
                    .map(|c| {
                        (
                            player_id.to_string(),
                            c.id.clone(),
                            c.name.clone(),
                            c.triggers.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (pid, cid, name, triggers) in battlefield_cards {
            self.apply_triggers_for_event(
                &pid,
                &cid,
                Some(&name),
                &triggers,
                &TriggerEvent::TurnStart,
                &[],
            );
        }

        self.apply_targeted_turn_start_triggers(player_id);

        self.combat_attackers.clear();
        self.combat_blockers.clear();
        self.combat_phase = CombatPhase::Idle;
        self.combat_blocker_id = None;
        self.triggers_played_this_turn.clear();
        self.damage_dealt_this_turn.clear();
    }

    fn apply_battlefield_triggers_for_event(&mut self, player_id: &str, event: &TriggerEvent) {
        self.apply_battlefield_triggers_for_player_event(player_id, event, &[]);
    }

    fn apply_targeted_turn_start_triggers(&mut self, active_player_id: &str) {
        let battlefield_cards: Vec<(String, String, String, Vec<CardTrigger>)> = self
            .players
            .iter()
            .filter(|(pid, _)| pid.as_str() != active_player_id)
            .flat_map(|(pid, player)| {
                player
                    .battlefield
                    .iter()
                    .filter(|card| card.attached_to.as_deref() == Some(active_player_id))
                    .map(|card| {
                        (
                            pid.clone(),
                            card.id.clone(),
                            card.name.clone(),
                            card.triggers.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        let targets = vec![TargetRef::Player(active_player_id.to_string())];
        for (pid, cid, name, triggers) in battlefield_cards {
            self.apply_triggers_for_event(
                &pid,
                &cid,
                Some(&name),
                &triggers,
                &TriggerEvent::TurnStart,
                &targets,
            );
        }
    }

    pub fn reveal_top_if(
        &mut self,
        player_id: &str,
        predicate: &Predicate,
        on_match: RevealOutcome,
        on_fail: RevealOutcome,
    ) {
        let Some(player) = self.players.get_mut(player_id) else {
            return;
        };
        let Some(card) = player.deck.pop() else {
            return;
        };
        let matches = Self::card_matches_predicate(&card, predicate);
        let outcome = if matches { on_match } else { on_fail };
        let outcome_desc = match outcome {
            RevealOutcome::Deploy => "deploys it to the battlefield",
            RevealOutcome::ToHand => "adds it to hand",
            RevealOutcome::ToBottom => "puts it on the bottom",
            RevealOutcome::ToTop => "puts it back on top",
            RevealOutcome::ToGraveyard => "sends it to the graveyard",
        };
        self.log(
            crate::state::GameLogKind::Resolved,
            player_id,
            format!("reveals {} — {}", card.name, outcome_desc),
        );
        // Stash a center-display entry so all players actually see the
        // card. Trimmed to the latest few so state doesn't grow unbounded.
        self.revealed_cards.push(RevealedCard {
            card: card.clone(),
            owner_id: player_id.to_string(),
            outcome_label: outcome_desc.to_string(),
            revealed_at: chrono::Utc::now(),
        });
        const MAX_REVEALED: usize = 6;
        if self.revealed_cards.len() > MAX_REVEALED {
            let drop = self.revealed_cards.len() - MAX_REVEALED;
            self.revealed_cards.drain(0..drop);
        }
        self.apply_reveal_outcome(player_id, card, outcome);
    }

    fn apply_reveal_outcome(&mut self, player_id: &str, card: Card, outcome: RevealOutcome) {
        let is_permanent = !matches!(card.kind, CardKind::Instant | CardKind::Ritual);
        let Some(player) = self.players.get_mut(player_id) else {
            return;
        };
        match outcome {
            RevealOutcome::Deploy if is_permanent => {
                let mut card = card;
                card.summoning_sickness = true;
                player.battlefield.push(card);
                let entered = player.battlefield.last().cloned();
                if let Some(card) = entered {
                    self.handle_card_entered(player_id, &card, &[]);
                }
            }
            RevealOutcome::Deploy | RevealOutcome::ToTop => {
                // Non-permanents fall through to ToTop.
                player.deck.push(card);
            }
            RevealOutcome::ToHand => {
                player.hand.push(card);
            }
            RevealOutcome::ToBottom => {
                player.deck.insert(0, card);
            }
            RevealOutcome::ToGraveyard => {
                player.graveyard.push(card);
            }
        }
    }

    pub fn advance_turn(&mut self) {
        if self.turn_order.is_empty() {
            self.active_player_id = None;
            self.finish_table();
            return;
        }

        // Fire end-of-turn triggers for all battlefield cards
        self.apply_all_battlefield_triggers_for_event(&TriggerEvent::EndOfTurn, &[]);

        self.effect_manager.expire_end_of_turn();

        if self.ruleset == Ruleset::Syndicate {
            if let Some(active) = self.active_player_id.clone() {
                self.syndicate_raid_check(&active);
            }
        }

        // Run state-based actions BEFORE wiping damage so any unit with
        // lethal accumulated damage actually dies at end of turn. Otherwise
        // "take 3 damage on a 3-toughness unit" would reset to 0 at turn
        // end without the unit ever being removed.
        self.apply_passive_state_updates();

        // Clear marked damage on all surviving units (damage wears off at
        // end of turn). Absorb refreshes the same way — a new turn means
        // a unit can prevent up to its Absorb(N) capacity again.
        for player in self.players.values_mut() {
            for card in player.battlefield.iter_mut() {
                card.damage = 0;
                card.absorb_used_this_turn = 0;
            }
        }

        // Re-check after passive state updates: an elimination during
        // end-of-turn triggers can empty turn_order, which would panic on
        // the indexing below.
        if self.turn_order.is_empty() {
            self.active_player_id = None;
            self.finish_table();
            return;
        }

        let next_index = if let Some(active_id) = &self.active_player_id {
            let current_index = self
                .turn_order
                .iter()
                .position(|id| id == active_id)
                .unwrap_or(0);
            let next_index = (current_index + 1) % self.turn_order.len();
            if next_index == 0 {
                self.round = self.round.saturating_add(1);
            }
            next_index
        } else {
            0
        };

        let next_player_id = self.turn_order[next_index].clone();
        self.active_player_id = Some(next_player_id.clone());
        self.start_turn(&next_player_id);
        self.log(
            crate::state::GameLogKind::TurnStart,
            &next_player_id,
            "starts the turn",
        );

        self.apply_battlefield_triggers_for_event(&next_player_id, &TriggerEvent::Upkeep);

        // First-player rule: whoever goes first skips their very first draw
        // step. Offsets the tempo advantage of acting first.
        let skip_first_draw = self.round == 1
            && self
                .turn_order
                .first()
                .map(|first| first == &next_player_id)
                .unwrap_or(false);
        if !skip_first_draw {
            self.draw_card_for_player(&next_player_id);
        }

        self.apply_battlefield_triggers_for_event(&next_player_id, &TriggerEvent::DrawStep);

        self.stage = CardGameStage::Main;
        self.reset_priority();
    }

    pub fn open_hand(&mut self, player_id: &str) {
        let Some(player) = self.players.get_mut(player_id) else {
            return;
        };

        let hand_size = match self.format {
            GameFormat::Operator => 7,
            GameFormat::Classic => 5,
        };

        if player.deck.len() < hand_size {
            for _ in 0..hand_size.min(player.deck.len()) {
                Self::draw_card(player);
            }
            return;
        }

        // Pick 3 cheap spells (cost <= 2) then fill rest randomly from top of deck
        let mut hand_cards = Vec::new();
        let cheap_target = 3;
        let mut cheap_indices = Vec::new();
        for (i, card) in player.deck.iter().enumerate() {
            if cheap_indices.len() >= cheap_target {
                break;
            }
            if card.cost.total() <= 2 {
                cheap_indices.push(i);
            }
        }
        for &i in cheap_indices.iter().rev() {
            hand_cards.push(player.deck.remove(i));
        }
        while hand_cards.len() < hand_size && !player.deck.is_empty() {
            hand_cards.push(player.deck.remove(0));
        }
        player.hand.extend(hand_cards);
    }
}
