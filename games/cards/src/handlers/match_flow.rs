//! Best-of-N match flow: format selection and between-game sideboard swaps.
//!
//! The `match_state` on `CardGameState` is the source of truth for how many
//! games have been played and who is leading. When a game ends with the
//! series still undecided, `logic.rs` transitions the stage to
//! `SideboardSwap`; players then submit swaps via `SubmitSideboard`, and
//! once every human has submitted, the next game is spun up.

use crate::action::SideboardSwapEntry;
use crate::card::Card;
use crate::state::{CardGameState, PlayerState};
use crate::types::{CardGameStage, MatchFormat, MatchState};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

pub trait MatchHandlers {
    fn handle_set_match_format(
        &mut self,
        player_id: &str,
        format: MatchFormat,
    ) -> Result<Vec<GameEffect>>;

    fn handle_submit_sideboard(
        &mut self,
        player_id: &str,
        swaps: Vec<SideboardSwapEntry>,
    ) -> Result<Vec<GameEffect>>;
}

impl MatchHandlers for CardGameState {
    fn handle_set_match_format(
        &mut self,
        player_id: &str,
        format: MatchFormat,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Lobby {
            return Err(anyhow!("Match format can only be set in lobby"));
        }
        if self.host_id.as_deref() != Some(player_id) {
            return Err(anyhow!("Only the host can change match format"));
        }
        // Draft games already have their own end-of-match flow (Awards).
        // Bo3 doesn't compose with it — disable the toggle entirely.
        if self.draft.is_some() {
            return Err(anyhow!("Best-of-N is not available for draft matches"));
        }

        if matches!(format, MatchFormat::Bo1) {
            // Drop the match state entirely — Bo1 is the legacy path.
            self.match_state = None;
        } else {
            let ids = self.players.keys().cloned().collect::<Vec<_>>();
            self.match_state = Some(MatchState::new(format, ids));
        }
        Ok(vec![])
    }

    fn handle_submit_sideboard(
        &mut self,
        player_id: &str,
        swaps: Vec<SideboardSwapEntry>,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::SideboardSwap {
            return Err(anyhow!("Not in sideboard swap"));
        }

        // Apply the swaps to this player's main + sideboard, without
        // touching anyone else. Validation is strict: for every swap, the
        // player must own a main-deck copy of `main_out` AND a sideboard
        // copy of `side_in`.
        {
            let player = self
                .players
                .get_mut(player_id)
                .ok_or_else(|| anyhow!("Player not in match"))?;
            apply_swaps_to_player(player, &swaps)?;
        }

        // Mark this player's submission complete. Once every pending human
        // has submitted, start the next game.
        let all_submitted = {
            let Some(match_state) = self.match_state.as_mut() else {
                return Err(anyhow!("No match state"));
            };
            match_state.pending_sideboard.retain(|id| id != player_id);
            match_state.pending_sideboard.is_empty()
        };

        if all_submitted {
            self.start_next_game_in_match()?;
        }

        Ok(vec![])
    }
}

/// Given the player's current state and a list of swaps, move cards between
/// their main deck and sideboard pool. Both sides are indexed by
/// template_id so instance ids don't matter.
///
/// Validation:
/// - Each swap is 1-for-1, so total main-deck count is preserved by
///   construction — we don't need to recount after applying.
/// - Per-card-template max 3 copies in the main deck post-swap (Classic
///   format constraint). We pre-compute the delta per template and
///   reject any swap set that would push any template over 3.
/// - Player must own the copies being moved, enforced by
///   `find_by_template` returning None.
fn apply_swaps_to_player(player: &mut PlayerState, swaps: &[SideboardSwapEntry]) -> Result<()> {
    // Net change per template id after applying all swaps.
    let mut delta: std::collections::HashMap<&str, i32> = std::collections::HashMap::new();
    for swap in swaps {
        *delta.entry(swap.main_out.as_str()).or_insert(0) -= 1;
        *delta.entry(swap.side_in.as_str()).or_insert(0) += 1;
    }

    // Current main-deck counts per template.
    let mut main_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for card in &player.deck {
        let tid = card.template_id.clone().unwrap_or_else(|| card.id.clone());
        *main_counts.entry(tid).or_insert(0) += 1;
    }

    // Reject if any template would exceed 3 copies after the swap.
    for (tid, d) in &delta {
        let post = main_counts.get(*tid).copied().unwrap_or(0) + *d;
        if post > 3 {
            return Err(anyhow!(
                "Cannot keep {} copies of '{}' in the main deck (max 3)",
                post,
                tid
            ));
        }
    }

    // Apply swaps one at a time now that we know they're all legal.
    for swap in swaps {
        let main_idx = find_by_template(&player.deck, &swap.main_out)
            .ok_or_else(|| anyhow!("Cannot swap out '{}': no copy in main deck", swap.main_out))?;
        let main_card = player.deck.remove(main_idx);

        let side_idx = find_by_template(&player.sideboard, &swap.side_in)
            .ok_or_else(|| anyhow!("Cannot swap in '{}': no copy in sideboard", swap.side_in))?;
        let side_card = player.sideboard.remove(side_idx);

        player.deck.push(side_card);
        player.sideboard.push(main_card);
    }
    Ok(())
}

fn find_by_template(cards: &[Card], template_id: &str) -> Option<usize> {
    cards
        .iter()
        .position(|c| c.template_id.as_deref() == Some(template_id) || c.id == template_id)
}

impl CardGameState {
    /// Auto-apply sideboard swaps for every AI in the match using the
    /// gauntlet-proven matchup tech (`classic_sideboard_ins` /
    /// `classic_sideboard_outs`). Called once when the match enters
    /// `SideboardSwap` so AIs adapt between games just like humans.
    pub fn apply_ai_sideboards_for_next_game(&mut self) {
        // Snapshot ids + deck ids so we don't hold a borrow across the
        // mutable apply below.
        let pairs: Vec<(String, String, String)> = self
            .turn_order
            .iter()
            .filter(|id| self.is_ai_player(id))
            .filter_map(|ai_id| {
                let ai_deck = self.players.get(ai_id).and_then(|p| p.deck_id.clone())?;
                // Pick any opponent (1v1 in practice). Use the first
                // non-self player with a deck.
                let opp = self
                    .turn_order
                    .iter()
                    .find(|pid| *pid != ai_id)
                    .and_then(|pid| self.players.get(pid).and_then(|p| p.deck_id.clone()))?;
                Some((ai_id.clone(), ai_deck, opp))
            })
            .collect();

        for (ai_id, ai_deck_id, opp_deck_id) in pairs {
            // Extract the simple deck-id root (strip `-{ulid}` from cloned
            // owner-specific decks so lookups match).
            let ai_root = deck_root_id(&ai_deck_id);
            let opp_root = deck_root_id(&opp_deck_id);
            let ins = crate::deck::classic_sideboard_ins(&ai_root, &opp_root);
            let outs = crate::deck::classic_sideboard_outs(&ai_root, &opp_root);

            if ins.is_empty() && outs.is_empty() {
                continue;
            }

            let Some(player) = self.players.get_mut(&ai_id) else {
                continue;
            };

            // Move OUTs from main -> sideboard, best-effort (skip if not
            // present). Then pull INs from sideboard -> main, ditto.
            for (card_id, qty) in outs {
                for _ in 0..qty {
                    if let Some(idx) = find_by_template(&player.deck, card_id) {
                        let card = player.deck.remove(idx);
                        player.sideboard.push(card);
                    }
                }
            }
            for (card_id, qty) in ins {
                for _ in 0..qty {
                    if let Some(idx) = find_by_template(&player.sideboard, card_id) {
                        let card = player.sideboard.remove(idx);
                        player.deck.push(card);
                    }
                }
            }
        }
    }

    /// Reset and start the next game in an ongoing match. Each player's
    /// deck (main + sideboard already reflecting swaps) is reconstituted
    /// from all cards they own, shuffled, and the game state resets.
    pub fn start_next_game_in_match(&mut self) -> Result<()> {
        // Bump the game counter.
        if let Some(match_state) = self.match_state.as_mut() {
            match_state.current_game += 1;
            match_state.pending_sideboard.clear();
        }

        let starting_life = self.init.starting_life;
        let player_ids: Vec<String> = self.players.keys().cloned().collect();

        // Rebuild turn_order from scratch — some players may have been
        // eliminated during the prior game and popped out of `turn_order`.
        self.turn_order = player_ids.clone();

        // Seed one RNG for this game so shuffles are deterministic given
        // the match's `deck_seed` and which game this is.
        let base_seed = self.init.deck_seed.unwrap_or(0).wrapping_add(
            self.match_state
                .as_ref()
                .map(|m| m.current_game as u64)
                .unwrap_or(1)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15),
        );

        for (seat, pid) in player_ids.iter().enumerate() {
            let Some(player) = self.players.get_mut(pid) else {
                continue;
            };

            // Reconstitute the main deck from every zone the player's cards
            // might currently live in. Swaps already happened: sideboard
            // holds what should stay out.
            let mut pool: Vec<Card> = Vec::new();
            pool.append(&mut player.deck);
            pool.append(&mut player.hand);
            pool.append(&mut player.battlefield);
            pool.append(&mut player.graveyard);
            pool.append(&mut player.exile);

            // Drop tokens — they're not part of a real decklist.
            pool.retain(|c| !c.id.starts_with("token-") && !c.id.starts_with("temp-"));

            // Reset every card to its fresh state.
            for card in pool.iter_mut() {
                card.tapped = false;
                card.attached_to = None;
                card.attachment = None;
                card.summoning_sickness = false;
                card.burnout = false;
                card.remembered_player_id = None;
            }

            let mut rng =
                StdRng::seed_from_u64(base_seed.wrapping_add(seat as u64 * 0x165667B19E3779F9));
            pool.shuffle(&mut rng);
            player.deck = pool;

            // Reset everything else on the player.
            player.life = starting_life;
            player.mana_pool = Default::default();
            player.pending_actions.clear();
            player.pending_effects.clear();
            player.infrastructures_played_this_turn = 0;
            player.nodes_deployed_this_turn = 0;
            player.operator_cast_count = 0;
            player.operator_first_cast_round = None;
            player.operator_damage.clear();
            player.tokens_created = 0;
            player.loyalty_activations = 0;
            player.scry.clear();
            player.grid.clear();
        }

        // Board-level resets.
        self.stack.clear();
        self.combat_attackers.clear();
        self.combat_blockers.clear();
        self.combat_targets.clear();
        self.combat_phase = crate::types::CombatPhase::Idle;
        self.combat_blocker_id = None;
        self.history.clear();
        self.damage_dealt_this_turn.clear();
        self.pings.clear();
        self.forbidden_names.clear();
        self.triggers_played_this_turn.clear();
        self.deferred_effects.clear();
        self.showcase = None;
        self.revealed_cards.clear();
        self.trigger_log.clear();
        self.priority_holder = None;
        self.consecutive_priority_passes = 0;
        self.last_stack_push_at = None;

        for id in player_ids.iter() {
            self.open_hand(id);
        }

        self.active_player_id = self.turn_order.first().cloned();
        self.round = 1;
        self.stage = CardGameStage::Main;
        self.initial_player_count = self.players.len();

        if let Some(active_id) = self.active_player_id.clone() {
            if let Some(player) = self.players.get_mut(&active_id) {
                CardGameState::refresh_mana(player);
                for card in player.battlefield.iter_mut() {
                    card.summoning_sickness = false;
                    card.tapped = false;
                }
                CardGameState::fill_mana_from_grid(player);
                CardGameState::fill_mana_from_infrastructure(player);
            }
        }

        Ok(())
    }
}

/// Owner-cloned decks are saved as `{root}-{ulid}` (see
/// `handle_select_deck`). Strip the ulid to get the base deck id the
/// gauntlet sideboard tables know about.
fn deck_root_id(deck_id: &str) -> String {
    if let Some(idx) = deck_id.find('-') {
        deck_id[..idx].to_string()
    } else {
        deck_id.to_string()
    }
}
