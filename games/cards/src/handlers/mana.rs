//! Mana and infrastructure action handlers (play land, tap card)

use crate::card::CardEffect;
use crate::helpers::add_mana_to_pool;
use crate::state::CardGameState;
use crate::types::{CardGameStage, CardKind, GridNode, ManaType};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;

pub trait ManaHandlers {
    fn handle_play_land(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>>;
    fn handle_deploy_node(
        &mut self,
        player_id: &str,
        mana_type: ManaType,
    ) -> Result<Vec<GameEffect>>;
    fn handle_tap_card(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>>;
}

impl ManaHandlers for CardGameState {
    fn handle_play_land(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Main && self.stage != CardGameStage::PostCombatMain {
            return Err(anyhow!("Infrastructures can only be played in main stages"));
        }
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can play infrastructures"));
        }
        // Sorcery-speed only — can't play an infrastructure while a spell
        // or showcase is pending on the stack.
        if !self.stack.is_empty() || self.showcase.is_some() {
            return Err(anyhow!(
                "Cannot play an infrastructure while a spell is on the stack"
            ));
        }

        let syndicate = self.ruleset == crate::types::Ruleset::Syndicate;
        let entered = {
            let Some(player) = self.active_player_mut() else {
                return Err(anyhow!("No active player"));
            };

            // Calculate max infrastructures allowed per turn (base 1 + ExtraInfrastructures from units)
            let max_infrastructures = 1 + player
                .battlefield
                .iter()
                .filter_map(|card| {
                    card.keywords.iter().find_map(|kw| {
                        if let crate::types::Keyword::ExtraInfrastructures(count) = kw {
                            Some(*count)
                        } else {
                            None
                        }
                    })
                })
                .sum::<u8>();

            if player.infrastructures_played_this_turn >= max_infrastructures {
                return Err(anyhow!("Infrastructure limit reached for this turn"));
            }

            let hand_ids: Vec<String> = player.hand.iter().map(|c| c.id.clone()).collect();
            tracing::info!(
                "[PLAY_LAND] Looking for card_id='{}' in hand with {} cards: {:?}",
                card_id,
                player.hand.len(),
                hand_ids,
            );
            let Some(card) = CardGameState::remove_card_from_hand(player, &card_id) else {
                return Err(anyhow!(
                    "Card not found in hand: '{}'. Hand contains: {:?}",
                    card_id,
                    hand_ids
                ));
            };

            if !matches!(card.kind, CardKind::Infrastructure) {
                return Err(anyhow!("Selected card is not an infrastructure"));
            }

            let mut card = card;
            if syndicate && !crate::syndicate::pay(player, &card.cost) {
                let cost = card.cost.clone();
                player.hand.push(card);
                return Err(anyhow!(
                    "Cannot afford this racket (${} with ${} credit)",
                    cost.total(),
                    crate::syndicate::DEBT_LIMIT
                ));
            }
            card.tapped = card.enters_tapped;
            card.summoning_sickness = false;
            card.haste = true;
            player.battlefield.push(card);
            player.infrastructures_played_this_turn += 1;

            player.battlefield.last().cloned()
        };
        if let Some(card) = entered {
            self.handle_card_played(player_id, &card, &[]);
            self.log(
                crate::state::GameLogKind::PlayLand,
                player_id,
                format!("plays {}", card.name),
            );
        }

        Ok(vec![])
    }

    fn handle_deploy_node(
        &mut self,
        player_id: &str,
        mana_type: ManaType,
    ) -> Result<Vec<GameEffect>> {
        if self.ruleset == crate::types::Ruleset::Syndicate {
            return Err(anyhow!("There is no grid in Syndicate"));
        }
        if self.stage != CardGameStage::Main && self.stage != CardGameStage::PostCombatMain {
            return Err(anyhow!("Nodes can only be deployed in main stages"));
        }
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can deploy nodes"));
        }
        // Deploying a node is a sorcery-speed action and cannot happen while
        // anything is pending resolution (stack items or showcase). The
        // player must either resolve / pass priority first.
        if !self.stack.is_empty() || self.showcase.is_some() {
            return Err(anyhow!(
                "Cannot deploy a node while a spell is on the stack"
            ));
        }

        {
            let Some(player) = self.active_player_mut() else {
                return Err(anyhow!("No active player"));
            };

            if player.nodes_deployed_this_turn >= 1 {
                return Err(anyhow!("Node limit reached for this turn"));
            }

            if mana_type != ManaType::Colorless && !player.color_identity.contains(&mana_type) {
                return Err(anyhow!("Cannot deploy a node outside your color identity"));
            }

            player.grid.push(GridNode {
                mana_type: mana_type.clone(),
            });
            player.nodes_deployed_this_turn += 1;

            // Immediately add this node's mana to the pool
            add_mana_to_pool(&mut player.mana_pool, &mana_type, 1);
        }
        self.log(
            crate::state::GameLogKind::DeployNode,
            player_id,
            format!("deploys a {:?} node", mana_type),
        );

        Ok(vec![])
    }

    fn handle_tap_card(&mut self, player_id: &str, card_id: String) -> Result<Vec<GameEffect>> {
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can tap cards"));
        }

        // Tap abilities that add mana are gated on stack/showcase resolution
        // — this applies to Infrastructure AND units with a tap-for-mana
        // ability. Abilities that do something else (draw, damage, etc.) can
        // still be activated at instant speed.
        let produces_mana = self
            .players
            .get(player_id)
            .and_then(|p| p.battlefield.iter().find(|c| c.id == card_id))
            .map(|c| {
                if matches!(c.kind, CardKind::Infrastructure) {
                    return true;
                }
                // Any tap ability whose effect adds mana (directly or via a
                // Sequence containing an AddMana step).
                c.abilities.iter().any(|ab| {
                    ab.cost.has_tap()
                        && match &ab.effect {
                            CardEffect::AddMana(..) | CardEffect::AddManaScaling(..) => true,
                            CardEffect::Sequence(effs) => effs.iter().any(|e| {
                                matches!(
                                    e,
                                    CardEffect::AddMana(..) | CardEffect::AddManaScaling(..)
                                )
                            }),
                            _ => false,
                        }
                })
            })
            .unwrap_or(false);
        if produces_mana && (!self.stack.is_empty() || self.showcase.is_some()) {
            return Err(anyhow!(
                "Cannot generate mana while a spell is on the stack"
            ));
        }

        // Extract all needed data first to avoid borrow conflicts
        let (tap_abilities, triggers, card_name, _is_infrastructure) = {
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };

            let card_index = player
                .battlefield
                .iter()
                .position(|card| card.id == card_id)
                .ok_or_else(|| anyhow!("Card not found on battlefield"))?;

            if player.battlefield[card_index].tapped {
                return Err(anyhow!("Card is already tapped"));
            }

            let has_tap_ability = player.battlefield[card_index]
                .abilities
                .iter()
                .any(|a| a.cost.has_tap());
            if !has_tap_ability {
                return Err(anyhow!("Card has no tap effect"));
            }

            // Summoning sick units/agents cannot tap this turn (unless Haste).
            // Infrastructure enters with haste so it's unaffected by this check.
            let card_ref = &player.battlefield[card_index];
            if card_ref.summoning_sickness && !card_ref.haste {
                return Err(anyhow!(
                    "This card has summoning sickness and cannot tap yet"
                ));
            }

            let card = &mut player.battlefield[card_index];
            card.tapped = true;

            let tap_abilities: Vec<_> = card
                .abilities
                .iter()
                .filter(|a| a.cost.has_tap() && a.cost.is_empty_except_tap())
                .cloned()
                .collect();
            let triggers = card.triggers.clone();
            let card_name = card.name.clone();
            let is_infrastructure = matches!(card.kind, CardKind::Infrastructure);

            (tap_abilities, triggers, card_name, is_infrastructure)
        };

        // Now handle the tap abilities without holding a mutable borrow
        if !tap_abilities.is_empty() {
            for ability in tap_abilities {
                // Use the general effect handler for all effects (including Sequence)
                self.apply_effect_with_source(player_id, Some(&card_id), &ability.effect, &[]);
            }
        }

        // Handle tap triggers (OnTap event)
        if !triggers.is_empty() {
            self.apply_triggers_for_event(
                player_id,
                &card_id,
                Some(&card_name),
                &triggers,
                &crate::types::TriggerEvent::Tapped,
                &[],
            );
        }

        Ok(vec![])
    }
}
