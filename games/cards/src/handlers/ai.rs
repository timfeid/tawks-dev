use anyhow::{Result, anyhow};

use crate::card::Card;
use crate::card_catalog;
use crate::deck::{
    DeckTemplateEntry, build_deck_catalog, build_deck_from_templates,
    build_operator_deck_from_templates, build_operator_sim_deck,
};
use crate::state::CardGameState;
use crate::types::{CardKind, CardSupertype, DeckFormat, GameFormat};

impl CardGameState {
    pub(crate) fn is_ai_player(&self, player_id: &str) -> bool {
        self.players
            .get(player_id)
            .map(|player| player.is_ai)
            .unwrap_or(false)
    }

    pub(crate) fn default_ai_deck_id(&self, player_id: &str) -> Option<String> {
        // An AI seat plays whatever ruleset the humans at the table picked, so
        // a Syndicate lobby never gets a Mana Clash bot and vice versa.
        let table_ruleset = self
            .players
            .values()
            .filter(|p| !p.is_ai)
            .filter_map(|p| p.deck_id.as_deref())
            .map(crate::types::Ruleset::for_deck_id)
            .find(|r| *r == crate::types::Ruleset::Syndicate)
            .unwrap_or(crate::types::Ruleset::ManaClash);
        let deck_ids = build_deck_catalog()
            .into_iter()
            .map(|deck| deck.id)
            .filter(|id| crate::types::Ruleset::for_deck_id(id) == table_ruleset)
            .collect::<Vec<_>>();
        if deck_ids.is_empty() {
            return None;
        }

        let ai_index = self
            .turn_order
            .iter()
            .filter(|pid| self.is_ai_player(pid))
            .position(|pid| pid == player_id)
            .unwrap_or(0);

        Some(deck_ids[ai_index % deck_ids.len()].clone())
    }

    pub(crate) fn ensure_ai_lobby_decks(&mut self) {
        if self.stage != crate::types::CardGameStage::Lobby {
            return;
        }

        let ai_ids = self
            .turn_order
            .iter()
            .filter(|pid| self.is_ai_player(pid))
            .cloned()
            .collect::<Vec<_>>();

        for player_id in ai_ids {
            let needs_deck = self
                .players
                .get(&player_id)
                .map(|player| player.deck_id.is_none())
                .unwrap_or(false);
            if !needs_deck {
                continue;
            }

            if let Some(deck_id) = self.default_ai_deck_id(&player_id) {
                let _ = self.assign_local_ai_deck(&player_id, &deck_id);
            }
        }
    }

    pub(crate) fn assign_local_ai_deck(&mut self, player_id: &str, deck_id: &str) -> Result<()> {
        let player = self
            .players
            .get(player_id)
            .ok_or_else(|| anyhow!("Player not found"))?;

        let builder_deck_id = player.deck_builder_deck_id.clone();
        let builder_name = player.deck_builder_name.clone();
        let builder_format = player.deck_builder_format.clone();
        let builder_operator_card_id = player.deck_builder_operator_card_id.clone();
        let builder_operator = player.deck_builder_operator.clone();
        let builder_cards = player.deck_builder_cards.clone();

        let local_builder_match = builder_deck_id.as_deref() == Some(deck_id)
            && (!builder_cards.is_empty() || builder_operator.is_some());

        let (deck_name, deck_format, deck, operator_zone, operator_id) = if local_builder_match {
            let mut templates = builder_cards
                .into_iter()
                .map(|entry| DeckTemplateEntry {
                    card: entry.card,
                    quantity: entry.quantity,
                })
                .collect::<Vec<_>>();

            if let Some(operator) = builder_operator {
                templates.push(DeckTemplateEntry {
                    card: operator.clone(),
                    quantity: 1,
                });
            }

            let format = builder_format.unwrap_or(DeckFormat::Classic);
            let name = builder_name.unwrap_or_else(|| "AI Draft Deck".to_string());

            match format {
                DeckFormat::Classic => (
                    name,
                    format,
                    build_deck_from_templates(player_id, self.init.deck_seed, &templates),
                    Vec::new(),
                    None,
                ),
                DeckFormat::Operator => {
                    let operator_card_id = builder_operator_card_id
                        .ok_or_else(|| anyhow!("AI operator deck is missing an operator"))?;
                    let (deck, operator) = build_operator_deck_from_templates(
                        player_id,
                        self.init.deck_seed,
                        &templates,
                        &operator_card_id,
                    )?;
                    (
                        name,
                        format,
                        deck,
                        vec![operator.clone()],
                        Some(operator.id),
                    )
                }
            }
        } else if self.format == GameFormat::Operator {
            let operator_card_id = default_operator_card_id(deck_id)
                .ok_or_else(|| anyhow!("No default operator found for deck {deck_id}"))?;
            let (deck, operator) = build_operator_sim_deck(
                player_id,
                deck_id,
                &operator_card_id,
                self.init.deck_seed,
            )?;
            (
                deck_display_name(deck_id),
                DeckFormat::Operator,
                deck,
                vec![operator.clone()],
                Some(operator.id),
            )
        } else {
            (
                deck_display_name(deck_id),
                DeckFormat::Classic,
                build_deck_from_templates(
                    player_id,
                    self.init.deck_seed,
                    &deck_catalog_templates(deck_id)?,
                ),
                Vec::new(),
                None,
            )
        };

        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| anyhow!("Player not found"))?;
        // Load the default sideboard for this deck so Bo3 AIs have a
        // pool to swap from. For local/built-catalog AIs (no repo) we
        // fall back to the sim-style curated default.
        let sideboard_cards = build_ai_sideboard_for_deck(player_id, deck_id);

        player.deck_id = Some(deck_id.to_string());
        player.deck_name = Some(deck_name);
        player.deck_format = Some(deck_format);
        player.deck = deck;
        player.sideboard = sideboard_cards;
        player.hand.clear();
        player.battlefield.clear();
        player.graveyard.clear();
        player.exile.clear();
        player.pending_actions.clear();
        player.operator_zone = operator_zone;
        player.operator_id = operator_id;
        player.operator_cast_count = 0;
        player.operator_first_cast_round = None;
        player.operator_damage.clear();
        player.tokens_created = 0;
        player.loyalty_activations = 0;
        Ok(())
    }
}

/// Build a sideboard pool for an AI from the default 15-card list (falls
/// back to empty for unknown deck ids). Cards are freshly cloned from
/// the color catalog and assigned owner-scoped instance ids.
fn build_ai_sideboard_for_deck(player_id: &str, deck_id: &str) -> Vec<crate::card::Card> {
    let ids = crate::deck::classic_default_sideboard_card_ids(deck_id);
    if ids.is_empty() {
        return Vec::new();
    }

    let catalog = match deck_id {
        "neon" => card_catalog::neon::all_cards(),
        "glitch" => card_catalog::glitch::all_cards(),
        "chrome" => card_catalog::chrome::all_cards(),
        "slag" => card_catalog::slag::all_cards(),
        "bio" => card_catalog::bio::all_cards(),
        _ => return Vec::new(),
    };

    let by_id: std::collections::HashMap<String, crate::card::Card> = catalog
        .into_iter()
        .map(|(card, _)| (card.id.clone(), card))
        .collect();

    let mut out = Vec::new();
    let mut counter = 0usize;
    for (card_id, qty) in ids {
        let Some(template) = by_id.get(card_id) else {
            continue;
        };
        for _ in 0..qty {
            counter += 1;
            let mut card = template.clone();
            card.template_id = Some(template.id.clone());
            card.id = format!("{player_id}-sb-{}-{counter}", template.id);
            card.tapped = false;
            card.summoning_sickness = false;
            card.burnout = false;
            out.push(card);
        }
    }
    out
}

fn deck_catalog_templates(deck_id: &str) -> Result<Vec<DeckTemplateEntry>> {
    build_deck_catalog()
        .into_iter()
        .find(|deck| deck.id == deck_id)
        .map(|deck| deck.cards)
        .ok_or_else(|| anyhow!("Unknown AI deck {deck_id}"))
}

fn deck_display_name(deck_id: &str) -> String {
    build_deck_catalog()
        .into_iter()
        .find(|deck| deck.id == deck_id)
        .map(|deck| deck.name)
        .unwrap_or_else(|| deck_id.to_string())
}

fn default_operator_card_id(deck_id: &str) -> Option<String> {
    let mut cards = match deck_id {
        "neon" => card_catalog::neon::all_cards(),
        "glitch" => card_catalog::glitch::all_cards(),
        "chrome" => card_catalog::chrome::all_cards(),
        "slag" => card_catalog::slag::all_cards(),
        "bio" => card_catalog::bio::all_cards(),
        _ => return None,
    };
    cards.extend(card_catalog::multicolor::all_cards());

    cards
        .into_iter()
        .map(|(card, _)| card)
        .filter(|card| {
            card.supertype == CardSupertype::Apex
                && matches!(
                    card.kind,
                    CardKind::Unit(_) | CardKind::Agent | CardKind::Relic
                )
        })
        .map(|card: Card| card.id)
        .min()
}
