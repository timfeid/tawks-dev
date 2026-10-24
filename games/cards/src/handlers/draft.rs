use anyhow::{Result, anyhow};
use chrono::Utc;
use game_framework::GameEffect;
use rand::Rng;
use ulid::Ulid;

use crate::card::Card;
use crate::helpers::get_card_color_identity;
use crate::pack::generate_draft_pool;
use crate::state::{CardGameState, DRAFT_PACK_SIZE, DraftState};
use crate::types::{
    CardCategory, CardGameStage, CardKind, CardRarity, DeckFormat, GameFormat, ManaType, UnitType,
    VisualSpec,
};
use crate::user_card::UserCardInstance;

use super::game_start::GameStartHandlers;
use super::lobby::LobbyHandlers;

/// Classic draft pack count scales with player count so each player always
/// picks from a comparable pool (~3 packs worth of picks = 57-ish cards,
/// above the 36-card shell target). Floors at 6 for 1v1 parity with the
/// original constant.
fn classic_draft_num_packs(player_count: usize) -> usize {
    (3 * player_count).max(6)
}
/// Operator format uses singleton drafting so its per-player pool needs
/// to be richer.
fn operator_draft_num_packs(player_count: usize) -> usize {
    (5 * player_count).max(11)
}
pub const DRAFT_WINNER_PICK_COUNT: usize = 16;

fn reset_gameplay_card_state(card: &mut Card) {
    card.tapped = false;
    card.attached_to = None;
    card.remembered_player_id = None;
    card.attachment = None;
    card.summoning_sickness = false;
    card.burnout = false;
    card.counters = 0;
}

fn materialize_draft_game_cards(
    selected_cards: &[Card],
    drafted_pool: &[Card],
) -> Result<Vec<Card>> {
    let mut drafted_by_template: std::collections::HashMap<String, Vec<Card>> =
        std::collections::HashMap::new();

    for drafted_card in drafted_pool {
        let canonical_id = drafted_card
            .template_id
            .clone()
            .unwrap_or_else(|| drafted_card.id.clone());
        drafted_by_template
            .entry(canonical_id)
            .or_default()
            .push(drafted_card.clone());
    }

    for cards in drafted_by_template.values_mut() {
        cards.sort_by(|a, b| a.id.cmp(&b.id));
        cards.reverse();
    }

    let mut materialized = Vec::with_capacity(selected_cards.len());
    for selected_card in selected_cards {
        let canonical_id = selected_card
            .template_id
            .clone()
            .unwrap_or_else(|| selected_card.id.clone());
        let Some(mut drafted_card) = drafted_by_template
            .get_mut(&canonical_id)
            .and_then(|cards| cards.pop())
        else {
            return Err(anyhow!(
                "Selected draft deck includes card {} not present in drafted pool",
                canonical_id
            ));
        };
        reset_gameplay_card_state(&mut drafted_card);
        materialized.push(drafted_card);
    }

    Ok(materialized)
}

pub(crate) fn draft_card_base_score(card: &Card) -> f32 {
    let mut score = match card.rarity {
        CardRarity::Public => 1.0,
        CardRarity::Private => 2.2,
        CardRarity::Restricted => 3.6,
        CardRarity::Classified => 5.0,
        CardRarity::Singularity => 6.0,
    };

    score += (card.power.unwrap_or(0) + card.toughness.unwrap_or(0)) as f32 * 0.35;
    score += card.abilities.len() as f32 * 0.7;
    score += card.triggers.len() as f32 * 0.55;
    score += card.keywords.len() as f32 * 0.45;

    score += match card.kind {
        CardKind::Infrastructure => 1.4,
        CardKind::Instant => 1.2,
        CardKind::Ritual => 0.9,
        CardKind::Aura => 0.8,
        CardKind::Relic => 1.0,
        CardKind::Agent => 1.2,
        CardKind::Unit(_) => 1.1,
    };

    for category in &card.categories {
        score += match category {
            CardCategory::Removal => 1.4,
            CardCategory::CardDraw => 1.15,
            CardCategory::ManaRamp => 1.05,
            CardCategory::Aggro => 0.8,
            CardCategory::Midrange => 0.75,
            CardCategory::Control => 0.95,
            CardCategory::Utility => 0.6,
            CardCategory::Buff => 0.55,
            CardCategory::Combo => 0.9,
            CardCategory::Evasion => 0.85,
        };
    }

    score
}

impl CardGameState {
    /// Roll through draft turns that don't need a human decision: AI
    /// players, plus humans who have handed their remaining picks to the
    /// auto-drafter. Stops as soon as the picker is a human still choosing
    /// for themselves (or the draft leaves the Draft stage). Called after
    /// `handle_draft_start` and after every pick, so neither AIs nor
    /// auto-drafting players block the table waiting on their turn.
    pub(crate) fn auto_advance_ai_draft_picks(&mut self) -> Result<()> {
        // Safety guard: in a degenerate state the picker might never
        // advance. Cap iterations at (pool size × player count) so we
        // can't spin forever.
        let max_iterations = self
            .draft
            .as_ref()
            .map(|d| (d.pool.len() + d.pyramid.len() + 1) * self.players.len() + 1)
            .unwrap_or(0);

        for _ in 0..max_iterations {
            if self.stage != CardGameStage::Draft {
                return Ok(());
            }
            let pick = {
                let Some(draft) = self.draft.as_ref() else {
                    return Ok(());
                };
                let Some(current) = draft.current_picker.clone() else {
                    return Ok(());
                };
                if !self.is_ai_player(&current) && !draft.auto_draft.contains(&current) {
                    return Ok(());
                }
                let Some(idx) = choose_auto_draft_pick(draft, &current) else {
                    return Ok(());
                };
                (current, idx)
            };
            let (picker_id, idx) = pick;
            self.handle_draft_pick(&picker_id, idx)?;
        }
        Ok(())
    }
}

pub(crate) fn choose_auto_draft_pick(draft: &DraftState, player_id: &str) -> Option<usize> {
    let available = draft.available_indices();
    if available.is_empty() {
        return None;
    }

    let drafted_cards = draft.drafted.get(player_id).cloned().unwrap_or_default();
    let mut color_counts: std::collections::HashMap<ManaType, i32> =
        std::collections::HashMap::new();
    let mut category_counts: std::collections::HashMap<CardCategory, i32> =
        std::collections::HashMap::new();
    let mut unit_type_counts: std::collections::HashMap<UnitType, i32> =
        std::collections::HashMap::new();
    let mut kind_counts: std::collections::HashMap<&'static str, i32> =
        std::collections::HashMap::new();

    for card in &drafted_cards {
        for mana in get_card_color_identity(card) {
            if mana != ManaType::Colorless {
                *color_counts.entry(mana).or_insert(0) += 1;
            }
        }
        for category in &card.categories {
            *category_counts.entry(category.clone()).or_insert(0) += 1;
        }
        match &card.kind {
            CardKind::Unit(unit_type) => {
                *unit_type_counts.entry(unit_type.clone()).or_insert(0) += 1;
                *kind_counts.entry("unit").or_insert(0) += 1;
            }
            CardKind::Infrastructure => *kind_counts.entry("infrastructure").or_insert(0) += 1,
            CardKind::Relic => *kind_counts.entry("relic").or_insert(0) += 1,
            CardKind::Instant => *kind_counts.entry("instant").or_insert(0) += 1,
            CardKind::Ritual => *kind_counts.entry("ritual").or_insert(0) += 1,
            CardKind::Aura => *kind_counts.entry("aura").or_insert(0) += 1,
            CardKind::Agent => *kind_counts.entry("agent").or_insert(0) += 1,
        }
    }

    let mut preferred_colors = color_counts.iter().collect::<Vec<_>>();
    preferred_colors.sort_by(|a, b| b.1.cmp(a.1));
    let locked_colors = preferred_colors
        .iter()
        .take(3)
        .map(|(mana, _)| (*mana).clone())
        .collect::<std::collections::HashSet<_>>();
    let has_established_plan = !locked_colors.is_empty() && drafted_cards.len() >= 6;

    available.into_iter().max_by(|&a, &b| {
        let score_for = |index: usize| -> f32 {
            let Some(card) = draft.pyramid.get(index).and_then(|slot| slot.as_ref()) else {
                return f32::MIN;
            };

            let mut score = draft_card_base_score(card);
            let identity = get_card_color_identity(card);
            let non_colorless = identity
                .iter()
                .filter(|mana| **mana != ManaType::Colorless)
                .cloned()
                .collect::<Vec<_>>();

            if non_colorless.is_empty() {
                score += 1.2;
            } else {
                let matching_colors = non_colorless
                    .iter()
                    .filter(|mana| locked_colors.contains(*mana))
                    .count() as f32;
                score += matching_colors * 2.3;

                for mana in &non_colorless {
                    score += color_counts.get(mana).copied().unwrap_or(0) as f32 * 0.35;
                }

                if has_established_plan && matching_colors == 0.0 {
                    score -= 4.5 + non_colorless.len() as f32 * 0.5;
                } else if !has_established_plan && locked_colors.len() < 3 {
                    score += 0.7;
                }
            }

            for category in &card.categories {
                score += category_counts.get(category).copied().unwrap_or(0) as f32 * 0.4;
            }

            match &card.kind {
                CardKind::Unit(unit_type) => {
                    score += unit_type_counts.get(unit_type).copied().unwrap_or(0) as f32 * 0.5;
                    let unit_count = kind_counts.get("unit").copied().unwrap_or(0);
                    if unit_count < 10 {
                        score += 0.8;
                    }
                }
                CardKind::Infrastructure => {
                    let infra_count = kind_counts.get("infrastructure").copied().unwrap_or(0);
                    if infra_count < 8 {
                        score += 1.8;
                    } else if infra_count > 13 {
                        score -= 1.2;
                    }
                }
                CardKind::Instant | CardKind::Ritual => {
                    let spell_count = kind_counts.get("instant").copied().unwrap_or(0)
                        + kind_counts.get("ritual").copied().unwrap_or(0);
                    if spell_count < 8 {
                        score += 0.7;
                    }
                }
                _ => {}
            }

            score
        };

        score_for(a)
            .partial_cmp(&score_for(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.cmp(&a))
    })
}

pub trait DraftHandlers {
    fn handle_draft_start(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_draft_pick(&mut self, player_id: &str, index: usize) -> Result<Vec<GameEffect>>;
    fn handle_draft_auto_pick(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_draft_confirm_pool(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_draft_keep_cards(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>>;
}

impl DraftHandlers for CardGameState {
    fn handle_draft_start(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.host_id.as_deref() != Some(player_id) {
            return Err(anyhow!("Only the host can start the draft"));
        }
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Draft can only start from lobby"));
        }
        if self.players.len() < 2 {
            return Err(anyhow!("Need at least 2 players"));
        }

        let player_count = self.players.len();
        let num_packs = if self.format == crate::types::GameFormat::Operator {
            operator_draft_num_packs(player_count)
        } else {
            classic_draft_num_packs(player_count)
        };

        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        let pool = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(generate_draft_pool(&repo, num_packs * DRAFT_PACK_SIZE))
        })?;

        let player_ids: Vec<String> = self.turn_order.clone();
        let player_names = self
            .players
            .iter()
            .map(|(pid, player)| (pid.clone(), player.name.clone()))
            .collect();
        let mut draft = DraftState::new(pool, &player_ids, player_names, num_packs);

        if !draft.deal_next_pyramid(&player_ids) {
            return Err(anyhow!("Not enough cards to start draft"));
        }

        self.draft = Some(draft);
        self.stage = CardGameStage::Draft;

        // If the first picker is an AI, roll through AI turns until a
        // human's up or the draft ends.
        self.auto_advance_ai_draft_picks()?;

        Ok(vec![])
    }

    fn handle_draft_pick(&mut self, player_id: &str, index: usize) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Draft {
            return Err(anyhow!("Not in draft stage"));
        }

        let player_ids = self.turn_order.clone();

        let draft = self
            .draft
            .as_mut()
            .ok_or_else(|| anyhow!("No active draft"))?;

        if draft.current_picker.as_deref() != Some(player_id) {
            return Err(anyhow!("Not your turn to pick"));
        }

        if draft.pick(player_id, index).is_none() {
            return Err(anyhow!("Invalid pick: card not available at that position"));
        }

        if draft.is_pyramid_empty() {
            draft.packs_completed += 1;
            draft.first_picker_index += 1;

            if !draft.deal_next_pyramid(&player_ids) {
                // All packs done — transition to deck building
                self.stage = CardGameStage::DraftBuilding;

                let repo = self
                    .repository
                    .as_ref()
                    .ok_or_else(|| anyhow!("Repository not initialized"))?
                    .clone();
                let draft_format = match self.format {
                    GameFormat::Classic => DeckFormat::Classic,
                    GameFormat::Operator => DeckFormat::Operator,
                };

                // Populate each player's deck builder library with their drafted cards.
                // The editable deck itself should start empty.
                let draft_ref = self.draft.as_ref().unwrap();
                for (pid, cards) in &draft_ref.drafted {
                    let (deck_id, deck_name, deck_description) = if self.is_ai_player(pid) {
                        let deck_id = format!("ai-draft-{}", Ulid::new());
                        let short = deck_id.split('-').nth(2).unwrap_or("deck").to_string();
                        let short = short[..short.len().min(8)].to_string();
                        (
                            deck_id,
                            format!("AI Draft Deck {short}"),
                            "AI draft deck".to_string(),
                        )
                    } else {
                        let pid_owned = pid.clone();
                        let draft_format_for_task = draft_format.clone();
                        tokio::task::block_in_place(|| {
                            let handle = tokio::runtime::Handle::current();
                            let repo = repo.clone();
                            handle.block_on(async move {
                                let deck_id = format!("draft-{}", Ulid::new());
                                let short = deck_id.split('-').nth(1).unwrap_or("deck");
                                let name = format!("Draft Deck {}", &short[..short.len().min(8)]);
                                let description = "Draft deck".to_string();
                                repo.create_deck_with_owner(
                                    &deck_id,
                                    &name,
                                    &description,
                                    Some(&pid_owned),
                                    None,
                                )
                                .await?;
                                repo.set_deck_format(&deck_id, draft_format_for_task)
                                    .await?;
                                Ok::<(String, String, String), anyhow::Error>((
                                    deck_id,
                                    name,
                                    description,
                                ))
                            })
                        })?
                    };

                    if let Some(player) = self.players.get_mut(pid) {
                        player.deck_builder_active = true;
                        player.deck_builder_deck_id = Some(deck_id);
                        player.deck_builder_name = Some(deck_name);
                        player.deck_builder_description = Some(deck_description);
                        player.deck_builder_format = Some(draft_format.clone());
                        player.deck_builder_operator_card_id = None;
                        player.deck_builder_autobuild_colors = None;
                        player.deck_builder_operator = None;
                        player.deck_builder_cards.clear();
                        player.deck_builder_filter_kind = None;
                        player.deck_builder_sort_order = None;
                        player.library_search_results = cards.clone();
                    }
                }
            }
        } else {
            // Switch to next player
            let current_idx = player_ids
                .iter()
                .position(|id| id == player_id)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % player_ids.len();
            let draft = self.draft.as_mut().unwrap();
            draft.current_picker = Some(player_ids[next_idx].clone());
        }

        // Whoever picks next — if they're an AI, pick for them now and
        // keep rolling until a human is up or the draft ends. Recursion
        // through handle_draft_pick is bounded by the pool size.
        self.auto_advance_ai_draft_picks()?;

        Ok(vec![])
    }

    fn handle_draft_auto_pick(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Draft {
            return Err(anyhow!("Not in draft stage"));
        }

        // Toggles auto-draft for the caller's remaining picks. Turning it
        // on takes their current pick immediately (if it's their turn) and
        // the advance loop keeps picking each time the turn comes back to
        // them. Only ever touches the caller's own picks.
        let draft = self
            .draft
            .as_mut()
            .ok_or_else(|| anyhow!("No active draft"))?;

        if !draft.auto_draft.insert(player_id.to_string()) {
            draft.auto_draft.remove(player_id);
            return Ok(vec![]);
        }

        self.auto_advance_ai_draft_picks()?;
        Ok(vec![])
    }

    fn handle_draft_confirm_pool(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::DraftBuilding {
            return Err(anyhow!("Not in draft building stage"));
        }

        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = false;
            player.card_browser_active = false;
        }

        let draft = self
            .draft
            .as_mut()
            .ok_or_else(|| anyhow!("No active draft"))?;

        draft.confirmed.insert(player_id.to_string());

        // Check if all players have confirmed
        let all_confirmed = self
            .turn_order
            .iter()
            .all(|pid| draft.confirmed.contains(pid));

        if all_confirmed {
            let draft_decks: Vec<(String, String)> = self
                .turn_order
                .iter()
                .map(|pid| {
                    let deck_id = self
                        .players
                        .get(pid)
                        .and_then(|player| player.deck_builder_deck_id.clone())
                        .ok_or_else(|| anyhow!("Missing draft deck for player {}", pid))?;
                    Ok((pid.clone(), deck_id))
                })
                .collect::<Result<_>>()?;

            self.stage = CardGameStage::Lobby;

            for (pid, deck_id) in draft_decks {
                self.handle_select_deck(&pid, deck_id)?;

                let drafted_pool = self
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.drafted.get(&pid))
                    .cloned()
                    .unwrap_or_default();
                let player = self
                    .players
                    .get_mut(&pid)
                    .ok_or_else(|| anyhow!("Missing player {}", pid))?;
                player.deck = materialize_draft_game_cards(&player.deck, &drafted_pool)?;
                if !player.operator_zone.is_empty() {
                    player.operator_zone =
                        materialize_draft_game_cards(&player.operator_zone, &drafted_pool)?;
                    player.operator_id = player.operator_zone.first().map(|card| card.id.clone());
                }
            }

            let starter_id = self
                .host_id
                .clone()
                .or_else(|| self.turn_order.first().cloned())
                .ok_or_else(|| anyhow!("No player available to start the drafted game"))?;
            self.handle_start_game(&starter_id)?;
        }

        Ok(vec![])
    }

    fn handle_draft_keep_cards(
        &mut self,
        player_id: &str,
        card_ids: Vec<String>,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Awards {
            return Err(anyhow!("Game must be in awards stage"));
        }

        let winner_id = self
            .turn_order
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("No winner available"))?;
        let is_winner = winner_id == player_id;
        let is_winner_ai = self.is_ai_player(player_id);

        if !is_winner {
            return Err(anyhow!("Only the winner selects cards"));
        }

        let (winner_cards, loser_remainders, completion_targets) = {
            let draft = self
                .draft
                .as_mut()
                .ok_or_else(|| anyhow!("No draft data available"))?;

            if draft.award_selections.contains_key(player_id) {
                return Err(anyhow!("Awards already submitted"));
            }

            let all_drafted_ids: std::collections::HashSet<_> = draft
                .drafted
                .values()
                .flat_map(|cards| cards.iter().map(|c| c.id.clone()))
                .collect();
            for cid in &card_ids {
                if !all_drafted_ids.contains(cid) {
                    return Err(anyhow!("Card {} is not in the draft pool", cid));
                }
            }

            let total_cards = all_drafted_ids.len();
            let pick_count = total_cards.min(DRAFT_WINNER_PICK_COUNT);
            if card_ids.len() != pick_count {
                return Err(anyhow!("Must select exactly {} cards", pick_count));
            }

            let selected_ids: std::collections::HashSet<String> =
                card_ids.iter().cloned().collect();

            let winner_cards: Vec<_> = draft
                .drafted
                .values()
                .flat_map(|cards| cards.iter())
                .filter(|card| selected_ids.contains(&card.id))
                .cloned()
                .collect();

            let loser_remainders = draft
                .drafted
                .iter()
                .filter(|(pid, _)| pid.as_str() != player_id)
                .map(|(pid, cards)| {
                    let remainder = cards
                        .iter()
                        .filter(|card| !selected_ids.contains(&card.id))
                        .cloned()
                        .collect::<Vec<_>>();
                    (pid.clone(), remainder)
                })
                .collect::<Vec<_>>();

            let completion_targets = draft.drafted.keys().cloned().collect::<Vec<_>>();

            draft
                .award_selections
                .insert(player_id.to_string(), card_ids.clone());

            (winner_cards, loser_remainders, completion_targets)
        };

        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Repository not initialized"))?
            .clone();

        if !is_winner_ai {
            let player_id_owned = player_id.to_string();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let mut rng = rand::rng();
                    for card in &winner_cards {
                        let instance_id = Ulid::new().to_string();
                        let visual_spec = card.visual_spec.clone().unwrap_or_else(|| VisualSpec {
                            skin_id: "default".to_string(),
                            seed: rng.random::<i32>(),
                            wear: {
                                let base: f32 = rng.random_range(0.0..1.0_f32);
                                (base * 100.0).round() / 100.0
                            },
                            holo: rng.random_range(0..100) == 0,
                        });
                        let canonical_card_id =
                            card.template_id.clone().unwrap_or_else(|| card.id.clone());

                        let card_instance = UserCardInstance {
                            instance_id,
                            card_id: canonical_card_id,
                            owner_id: player_id_owned.clone(),
                            visual_spec,
                            is_tradeable: true,
                            is_bound: false,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                        };

                        repo.create_user_card_instance(
                            &card_instance,
                            &player_id_owned,
                            Some(serde_json::json!({
                                "source": "draft_award",
                                "lobby_id": self.lobby_id,
                            })),
                        )
                        .await?;
                    }
                    Ok::<_, anyhow::Error>(())
                })
            })?;
        }

        // Persist remaining cards to their original owners (losers)
        for (loser_id, remainder) in &loser_remainders {
            if self.is_ai_player(loser_id) {
                continue;
            }
            let repo = repo.clone();
            let loser_id = loser_id.clone();
            let lobby_id = self.lobby_id.clone();
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let mut rng = rand::rng();
                    for card in remainder {
                        let instance_id = Ulid::new().to_string();
                        let visual_spec = card.visual_spec.clone().unwrap_or_else(|| VisualSpec {
                            skin_id: "default".to_string(),
                            seed: rng.random::<i32>(),
                            wear: (rng.random_range(0.0..1.0_f32) * 100.0).round() / 100.0,
                            holo: rng.random_range(0..100) == 0,
                        });
                        let canonical_card_id =
                            card.template_id.clone().unwrap_or_else(|| card.id.clone());
                        let card_instance = UserCardInstance {
                            instance_id,
                            card_id: canonical_card_id,
                            owner_id: loser_id.clone(),
                            visual_spec,
                            is_tradeable: true,
                            is_bound: false,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                        };
                        let _ = repo
                            .create_user_card_instance(
                                &card_instance,
                                &loser_id,
                                Some(serde_json::json!({
                                    "source": "draft_remainder",
                                    "lobby_id": lobby_id,
                                })),
                            )
                            .await;
                    }
                    Ok::<_, anyhow::Error>(())
                })
            })?;
        }

        self.finish_table();

        // Close the workflow for all players
        let mut effects = Vec::new();
        for pid in completion_targets {
            if self.is_ai_player(&pid) {
                continue;
            }
            effects.push(GameEffect::CompleteWorkflow {
                user_id: pid,
                message: "Game complete! Cards have been awarded.".to_string(),
                responses: std::collections::HashMap::new(),
                close_active_modal: true,
            });
        }
        return Ok(effects);
    }
}

#[cfg(test)]
mod tests {
    use super::materialize_draft_game_cards;
    use crate::card::CardBuilder;
    use crate::types::{CardKind, UnitType, VisualSpec};

    #[test]
    fn materialize_draft_game_cards_preserves_instance_visuals() {
        let mut drafted_card = CardBuilder::new(
            "draft-instance-1",
            "Spore Blade Captain",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .build();
        drafted_card.template_id = Some("spore_blade_captain".to_string());
        drafted_card.visual_spec = Some(VisualSpec {
            skin_id: "drafted".to_string(),
            seed: 17,
            wear: 0.42,
            holo: false,
        });

        let mut selected_card = CardBuilder::new(
            "p1-spore_blade_captain-1",
            "Spore Blade Captain",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .build();
        selected_card.template_id = Some("spore_blade_captain".to_string());

        let materialized =
            materialize_draft_game_cards(&[selected_card], &[drafted_card.clone()]).unwrap();

        assert_eq!(materialized.len(), 1);
        assert_eq!(materialized[0].id, drafted_card.id);
        assert_eq!(materialized[0].template_id, drafted_card.template_id);
        assert_eq!(materialized[0].visual_spec, drafted_card.visual_spec);
    }
}
