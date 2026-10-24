//! Game start action handler

use crate::state::CardGameState;
use crate::types::{CardGameStage, DeckFormat, GameFormat, ManaType};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;

pub trait GameStartHandlers {
    fn handle_start_game(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
}

impl GameStartHandlers for CardGameState {
    fn handle_start_game(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Game already started"));
        }

        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = false;
        }

        if self.players.len() < 2 {
            return Err(anyhow!("Need at least two players to start"));
        }
        if self.players.values().any(|player| player.deck_id.is_none()) {
            return Err(anyhow!("All players must choose a deck"));
        }

        // Sync match_state.wins with the final lobby roster. If players
        // joined after the host toggled Bo3, they don't have entries in
        // the wins HashMap yet — fix that here, and drop entries for
        // anyone who left. This also gracefully resets a stale
        // match_state if players cycle through the lobby.
        if let Some(match_state) = self.match_state.as_mut() {
            let current_ids: std::collections::HashSet<String> =
                self.players.keys().cloned().collect();
            match_state.wins.retain(|id, _| current_ids.contains(id));
            for id in &current_ids {
                match_state.wins.entry(id.clone()).or_insert(0);
            }
            match_state.series_winner = None;
            match_state.current_game = 1;
            match_state.pending_sideboard.clear();
        }

        let operator_selected = self
            .players
            .values()
            .any(|player| player.deck_format == Some(DeckFormat::Operator));

        if operator_selected {
            let all_operator = self
                .players
                .values()
                .all(|player| player.deck_format == Some(DeckFormat::Operator));
            if !all_operator {
                return Err(anyhow!("All players must use operator decks"));
            }
            self.format = GameFormat::Operator;
            for player in self.players.values_mut() {
                player.life = 40;
            }
        } else {
            self.format = GameFormat::Classic;
        }
        // A Syndicate deck at any seat puts the whole table under Syndicate rules.
        if self.players.values().any(|p| {
            p.deck_id
                .as_deref()
                .map(crate::types::Ruleset::for_deck_id)
                == Some(crate::types::Ruleset::Syndicate)
        }) {
            self.ruleset = crate::types::Ruleset::Syndicate;
        }
        if self.ruleset == crate::types::Ruleset::Syndicate {
            for player in self.players.values_mut() {
                player.mana_pool = crate::mana::ManaPool::default();
                player.mana_pool.colorless = crate::syndicate::STARTING_CASH;
                player.heat = 0;
            }
        }

        // Compute color identity for Grid node deployment
        for id in self.turn_order.clone() {
            if let Some(player) = self.players.get_mut(&id) {
                let mut identity = std::collections::HashSet::new();

                if self.format == GameFormat::Operator {
                    // Operator: derive from operator card's colors
                    for card in &player.operator_zone {
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
                    }
                } else {
                    // Classic: derive from all deck cards
                    for card in &player.deck {
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
                    }
                    for card in &player.hand {
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
                    }
                }

                let mut color_identity: Vec<ManaType> = identity.into_iter().collect();
                color_identity.push(ManaType::Colorless);
                player.color_identity = color_identity;
            }
        }

        for id in self.turn_order.clone() {
            self.open_hand(&id);
        }

        self.active_player_id = Some(self.turn_order[0].clone());
        self.round = 1;
        self.initial_player_count = self.players.len();

        if let Some(active_id) = self.active_player_id.clone() {
            // Start turn without firing triggers for the very first turn of the game
            if let Some(player) = self.players.get_mut(&active_id) {
                player.infrastructures_played_this_turn = 0;
                player.nodes_deployed_this_turn = 0;
                let syndicate = self.ruleset == crate::types::Ruleset::Syndicate;
                if !syndicate {
                    CardGameState::refresh_mana(player);
                }
                // Untap before filling — see comment in `game_flow::start_turn`.
                for card in player.battlefield.iter_mut() {
                    card.summoning_sickness = false;
                    card.tapped = false;
                }
                if syndicate {
                    crate::syndicate::collect_income(player);
                } else {
                    CardGameState::fill_mana_from_grid(player);
                    CardGameState::fill_mana_from_infrastructure(player);
                }
            }
            // Going-first rule: the first player skips their very first draw
            // step. Both players start with the same opening hand; p1 does
            // not draw on turn 1, so p2 has a one-card advantage to offset
            // the tempo of acting first.
        }

        self.stage = CardGameStage::Main;
        Ok(vec![])
    }
}
