//! Lobby management action handlers (view, join, format, deck selection)

use crate::deck::{
    DeckTemplates, apply_instance_visual_specs, build_deck_from_templates,
    build_deck_with_templates, build_operator_deck_from_templates,
};
use crate::helpers::{game_format_for_deck, validate_operator_deck_templates};
use crate::state::CardGameState;
use crate::state::PlayerState;
use crate::types::{CardGameStage, DeckFormat, GameFormat, GameMode};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use std::collections::HashMap;
use ulid::Ulid;

pub trait LobbyHandlers {
    fn handle_view(&mut self, player_id: &str, instance_id: String) -> Result<Vec<GameEffect>>;
    fn handle_add_ai(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_join(
        &mut self,
        player_id: &str,
        name: String,
        avatar: String,
        instance_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_set_game_format(
        &mut self,
        player_id: &str,
        format: GameFormat,
    ) -> Result<Vec<GameEffect>>;
    fn handle_set_game_mode(&mut self, player_id: &str, mode: GameMode) -> Result<Vec<GameEffect>>;
    fn handle_select_deck(&mut self, player_id: &str, deck_id: String) -> Result<Vec<GameEffect>>;
}

impl LobbyHandlers for CardGameState {
    fn handle_view(&mut self, player_id: &str, instance_id: String) -> Result<Vec<GameEffect>> {
        self.record_instance_id(player_id, &instance_id);
        Ok(vec![])
    }

    fn handle_add_ai(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("AI can only be added in lobby"));
        }
        if self.host_id.as_deref() != Some(player_id) {
            return Err(anyhow!("Only the host can add AI"));
        }

        let mut seat = self.players.values().filter(|player| player.is_ai).count() + 1;
        let ai_id = loop {
            let candidate = format!("ai-{seat}");
            if !self.players.contains_key(&candidate) {
                break candidate;
            }
            seat += 1;
        };

        self.turn_order.push(ai_id.clone());
        self.players.insert(
            ai_id.clone(),
            PlayerState::new_ai(
                ai_id.clone(),
                format!("Practice AI {seat}"),
                String::new(),
                format!("{ai_id}-instance"),
                self.init.starting_life,
            ),
        );

        let deck_id = self
            .default_ai_deck_id(&ai_id)
            .ok_or_else(|| anyhow!("No AI deck available for this lobby"))?;
        self.assign_local_ai_deck(&ai_id, &deck_id)?;

        Ok(vec![])
    }

    fn handle_join(
        &mut self,
        player_id: &str,
        name: String,
        avatar: String,
        instance_id: String,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Game already started"));
        }
        if self.host_id.is_none() {
            self.host_id = Some(player_id.to_string());
        }
        if !self.players.contains_key(player_id) {
            self.turn_order.push(player_id.to_string());
        }
        self.players.insert(
            player_id.to_string(),
            PlayerState::new(
                player_id.to_string(),
                name,
                avatar,
                instance_id,
                self.init.starting_life,
            ),
        );
        Ok(vec![])
    }

    fn handle_set_game_format(
        &mut self,
        player_id: &str,
        format: GameFormat,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Game format can only be changed in lobby"));
        }
        let is_host = self.host_id.as_deref() == Some(player_id);
        if !is_host {
            return Err(anyhow!("Only the host can change game format"));
        }
        if self.format != format {
            self.format = format;
            for player in self.players.values_mut() {
                player.deck_id = None;
                player.deck_name = None;
                player.deck_format = None;
                player.deck.clear();
                player.hand.clear();
                player.battlefield.clear();
                player.graveyard.clear();
                player.exile.clear();
                player.operator_zone.clear();
                player.pending_actions.clear();
                player.operator_id = None;
                player.operator_cast_count = 0;
                player.operator_first_cast_round = None;
                player.operator_damage.clear();
                player.tokens_created = 0;
                player.loyalty_activations = 0;
            }
            self.deck_choices_loaded.clear();
            self.deck_choices_by_player.clear();

            // Force immediate reload for all existing players
            let player_ids: Vec<String> = self.players.keys().cloned().collect();
            for pid in player_ids {
                self.ensure_deck_choices_loaded_for(&pid);
            }
            self.ensure_ai_lobby_decks();
        }
        Ok(vec![])
    }

    fn handle_set_game_mode(&mut self, player_id: &str, mode: GameMode) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Game mode can only be changed in lobby"));
        }
        let is_host = self.host_id.as_deref() == Some(player_id);
        if !is_host {
            return Err(anyhow!("Only the host can change game mode"));
        }
        if self.mode != mode {
            self.mode = mode;
            // Reset all player deck selections when mode changes
            for player in self.players.values_mut() {
                player.deck_id = None;
                player.deck_name = None;
                player.deck_format = None;
                player.deck.clear();
                player.hand.clear();
                player.battlefield.clear();
                player.graveyard.clear();
                player.exile.clear();
                player.operator_zone.clear();
                player.pending_actions.clear();
                player.operator_id = None;
                player.operator_cast_count = 0;
                player.operator_first_cast_round = None;
                player.operator_damage.clear();
                player.tokens_created = 0;
                player.loyalty_activations = 0;
            }
            self.deck_choices_loaded.clear();
            self.deck_choices_by_player.clear();

            let player_ids: Vec<String> = self.players.keys().cloned().collect();
            for pid in player_ids {
                self.ensure_deck_choices_loaded_for(&pid);
            }
            self.ensure_ai_lobby_decks();
        }
        Ok(vec![])
    }

    fn handle_select_deck(&mut self, player_id: &str, deck_id: String) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Decks can only be selected in lobby"));
        }

        if self.is_ai_player(player_id) {
            self.assign_local_ai_deck(player_id, &deck_id)?;
            return Ok(vec![]);
        }

        for player in self.players.values_mut() {
            player.deck_builder_active = false;
        }

        self.ensure_deck_choices_loaded_for(player_id);
        let choices = self
            .deck_choices_by_player
            .get(player_id)
            .cloned()
            .unwrap_or_default();
        let deck_choice = choices.iter().find(|choice| choice.id == deck_id);

        let is_competitive = self.mode == GameMode::Competitive;

        if let Some(repo) = self.repository.as_ref() {
            let repo = repo.clone();
            let deck_id = deck_id.clone();
            let player_id = player_id.to_string();
            let player_id_for_task = player_id.clone();
            let (resolved, templates, sideboard_templates, instances) =
                tokio::task::block_in_place(|| {
                    let handle = tokio::runtime::Handle::current();
                    handle.block_on(async move {
                    let deck = repo.get_deck(&deck_id).await?;
                    let resolved = if deck.owner_id.is_none() {
                        if let Some(existing) = repo
                            .get_user_deck_for_base(&player_id_for_task, &deck.id)
                            .await?
                        {
                            existing
                        } else {
                            let new_id = format!("{}-{}", deck.id, Ulid::new());
                            let new_name = format!("{} (Custom)", deck.name);
                            repo.clone_deck_with_owner(
                                &deck.id,
                                &new_id,
                                &new_name,
                                &deck.description,
                                Some(&player_id_for_task),
                                Some(&deck.id),
                            )
                            .await?
                        }
                    } else if deck.owner_id.as_deref() == Some(&player_id_for_task) {
                        deck
                    } else {
                        return Err(anyhow!("Deck does not belong to player"));
                    };

                    let templates = repo.load_deck_templates(&resolved.id).await?;
                    let sideboard_templates = repo.load_sideboard_templates(&resolved.id).await?;

                    // In competitive mode, load user card instances for this deck
                    let instances = if is_competitive {
                        let inst = repo.get_deck_instances(&resolved.id).await?;
                        let required: i32 = templates.iter().map(|t| t.quantity).sum();
                        if (inst.len() as i32) < required {
                            return Err(anyhow!(
                                "Competitive deck requires {} cards but only {} instances are assigned",
                                required,
                                inst.len()
                            ));
                        }
                        Some(inst)
                    } else {
                        None
                    };

                    Ok((resolved, templates, sideboard_templates, instances))
                })
                })?;

            if templates.is_empty() {
                return Err(anyhow!("Deck {} has no cards", resolved.id));
            }

            if !self.format_allows_deck(&resolved.format) {
                let is_host = self.host_id.as_deref() == Some(player_id.as_str());
                if is_host {
                    self.format = game_format_for_deck(&resolved.format);
                    for player in self.players.values_mut() {
                        player.deck_id = None;
                        player.deck_name = None;
                        player.deck_format = None;
                        player.deck.clear();
                        player.hand.clear();
                        player.battlefield.clear();
                        player.graveyard.clear();
                        player.exile.clear();
                        player.operator_zone.clear();
                        player.operator_id = None;
                        player.operator_cast_count = 0;
                        player.operator_first_cast_round = None;
                        player.operator_damage.clear();
                        player.tokens_created = 0;
                        player.loyalty_activations = 0;
                        player.pending_actions.clear();
                    }
                    self.deck_choices_loaded.clear();
                    self.deck_choices_by_player.clear();
                } else {
                    return Err(anyhow!("Deck format does not match lobby format"));
                }
            }

            let Some(player) = self.players.get_mut(player_id.as_str()) else {
                return Err(anyhow!("Player not found"));
            };

            player.deck_id = Some(resolved.id.clone());
            player.deck_name = Some(resolved.name.clone());
            player.deck_format = Some(resolved.format.clone());
            self.ruleset = crate::types::Ruleset::for_deck_id(&resolved.id);
            player.operator_zone.clear();
            player.operator_id = None;
            player.operator_cast_count = 0;
            player.operator_first_cast_round = None;
            player.operator_damage.clear();
            player.tokens_created = 0;
            player.loyalty_activations = 0;

            if resolved.format == DeckFormat::Operator {
                let operator_card_id = resolved
                    .operator_card_id
                    .as_deref()
                    .ok_or_else(|| anyhow!("Operator deck missing operator card"))?;
                validate_operator_deck_templates(&templates, operator_card_id)?;
                let (mut deck, mut operator) = build_operator_deck_from_templates(
                    player_id.as_str(),
                    self.init.deck_seed,
                    &templates,
                    operator_card_id,
                )?;

                // Apply user card instance visual_specs in competitive mode
                if let Some(inst) = instances {
                    let mut by_card: HashMap<String, Vec<_>> = HashMap::new();
                    for i in inst {
                        by_card.entry(i.card_id.clone()).or_default().push(i);
                    }
                    apply_instance_visual_specs(&mut deck, &mut by_card);
                    // Apply to operator too
                    if let Some(tmpl_id) = &operator.template_id {
                        if let Some(list) = by_card.get_mut(tmpl_id.as_str()) {
                            if let Some(i) = list.pop() {
                                operator.visual_spec = Some(i.visual_spec);
                            }
                        }
                    }
                }

                player.deck = deck;
                player.operator_zone = vec![operator.clone()];
                player.operator_id = Some(operator.id.clone());
                self.format = GameFormat::Operator;
            } else {
                let mut deck =
                    build_deck_from_templates(player_id.as_str(), self.init.deck_seed, &templates);

                // Apply user card instance visual_specs in competitive mode
                if let Some(inst) = instances {
                    let mut by_card: HashMap<String, Vec<_>> = HashMap::new();
                    for i in inst {
                        by_card.entry(i.card_id.clone()).or_default().push(i);
                    }
                    apply_instance_visual_specs(&mut deck, &mut by_card);
                }

                player.deck = deck;
            }

            // Sideboard: build from its own templates, but don't shuffle —
            // keeping stable order so the swap UI shows a consistent list.
            let sideboard_cards =
                build_deck_from_templates(player_id.as_str(), None, &sideboard_templates);
            player.sideboard = sideboard_cards;

            self.refresh_deck_choices_for(player_id.as_str());
        } else {
            if self.format == GameFormat::Operator {
                return Err(anyhow!("Operator decks require database-backed decks"));
            }
            let chosen = deck_choice.ok_or_else(|| anyhow!("Invalid deck selection"))?;
            let Some(player) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            player.deck_id = Some(chosen.id.clone());
            player.deck_name = Some(chosen.name.clone());
            player.deck_format = Some(DeckFormat::Classic);
            player.operator_zone.clear();
            player.operator_id = None;
            player.operator_cast_count = 0;
            player.operator_first_cast_round = None;
            player.operator_damage.clear();
            player.tokens_created = 0;
            player.loyalty_activations = 0;
            player.deck = build_deck_with_templates(
                player_id,
                &chosen.id,
                self.init.deck_seed,
                &DeckTemplates::default(),
                None,
            );
        }
        Ok(vec![])
    }
}
