//! Utility action handlers for miscellaneous game actions

use crate::card::Card;
use crate::state::{CardGameState, PingEvent};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use std::collections::HashMap;

pub trait UtilityHandlers {
    fn handle_ping(&mut self, player_id: &str, x: f32, z: f32) -> Result<Vec<GameEffect>>;
    fn handle_reorder_battlefield(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>>;
}

impl UtilityHandlers for CardGameState {
    fn handle_ping(&mut self, player_id: &str, x: f32, z: f32) -> Result<Vec<GameEffect>> {
        self.pings.push(PingEvent {
            x,
            z,
            player_id: player_id.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        });

        if self.pings.len() > 10 {
            self.pings.remove(0);
        }
        Ok(vec![])
    }

    fn handle_reorder_battlefield(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>> {
        let Some(player) = self.players.get_mut(player_id) else {
            return Err(anyhow!("Player not found"));
        };

        // Create a map of existing cards for quick lookup
        let mut card_map: HashMap<String, Card> = player
            .battlefield
            .drain(..)
            .map(|c| (c.id.clone(), c))
            .collect();

        // Rebuild the battlefield in the requested order
        for id in card_ids {
            if let Some(card) = card_map.remove(&id) {
                player.battlefield.push(card);
            }
        }

        // Put back any cards that weren't included in the reorder request
        for (_, card) in card_map {
            player.battlefield.push(card);
        }

        Ok(vec![])
    }
}
