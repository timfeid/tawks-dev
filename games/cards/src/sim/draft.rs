//! Draft simulation for balance testing.
//!
//! Generates draft pools from the in-memory card catalog (no database required),
//! runs auto-draft with the hex-pick AI, and builds playable decks.

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::card::Card;
use crate::card_catalog;
use crate::helpers::get_card_color_identity;
use crate::state::{DRAFT_PACK_SIZE, DraftState};
use crate::types::{CardCategory, CardKind, CardRarity, CardSupertype, ManaType, UnitType};

const DRAFT_NUM_PACKS: usize = 8;

/// Weight for how often a card appears in draft packs.
/// Rarer cards appear proportionally less often (inverse of rarity_weight).
fn draft_pool_weight_rarity(rarity: &CardRarity) -> f32 {
    match rarity {
        CardRarity::Public => 8.0,
        CardRarity::Private => 4.0,
        CardRarity::Restricted => 2.0,
        CardRarity::Classified => 1.0,
        CardRarity::Singularity => 0.5,
    }
}

fn draft_pool_weight(card: &Card) -> f32 {
    draft_pool_weight_rarity(&card.rarity)
}

/// Sample an index from weighted candidates via reservoir-style selection.
fn weighted_pick<T>(
    items: &[T],
    weight_fn: impl Fn(&T) -> f32,
    rng: &mut impl Rng,
) -> Option<usize> {
    let total: f32 = items.iter().map(&weight_fn).sum();
    if total <= 0.0 {
        return None;
    }
    let mut r = rng.random::<f32>() * total;
    for (i, item) in items.iter().enumerate() {
        r -= weight_fn(item);
        if r <= 0.0 {
            return Some(i);
        }
    }
    Some(items.len() - 1)
}

/// Generate a draft pool from the in-memory card catalog.
/// Cards are sampled with probability proportional to rarity (common cards
/// appear more often, rare cards less), so the pool mirrors the real-pack
/// experience where rare effects are harder to draft.
/// Returns `total_packs * DRAFT_PACK_SIZE` cards shuffled randomly.
pub fn generate_sim_draft_pool(seed: u64) -> Vec<Card> {
    let mut rng = StdRng::seed_from_u64(seed);

    // Gather all non-operator, non-infrastructure cards, organized by primary color
    let color_names = ["neon", "glitch", "chrome", "slag", "bio"];
    let mut by_color: HashMap<String, Vec<Card>> = HashMap::new();
    let mut multicolor_cards: Vec<Card> = Vec::new();

    for entries in [
        card_catalog::neon::all_cards(),
        card_catalog::glitch::all_cards(),
        card_catalog::chrome::all_cards(),
        card_catalog::slag::all_cards(),
        card_catalog::bio::all_cards(),
        card_catalog::multicolor::all_cards(),
    ] {
        for (card, _) in entries {
            if card.supertype == CardSupertype::Apex {
                continue;
            }
            if matches!(card.kind, CardKind::Infrastructure) {
                continue;
            }
            let identity = get_card_color_identity(&card);
            let colored: Vec<_> = identity
                .iter()
                .filter(|m| **m != ManaType::Colorless)
                .collect();

            if colored.len() > 1 {
                multicolor_cards.push(card);
            } else if colored.len() == 1 {
                let color = match colored[0] {
                    ManaType::Neon => "neon",
                    ManaType::Glitch => "glitch",
                    ManaType::Chrome => "chrome",
                    ManaType::Slag => "slag",
                    ManaType::Bio => "bio",
                    _ => continue,
                };
                by_color.entry(color.to_string()).or_default().push(card);
            } else {
                // Colorless cards go to multicolor pool
                multicolor_cards.push(card);
            }
        }
    }

    let mut pool = Vec::with_capacity(DRAFT_NUM_PACKS * DRAFT_PACK_SIZE);
    let mut id_counter = 0usize;

    // Each pack: 3 cards per color (15) + 4 multicolor/random (19 total)
    let cards_per_color_per_pack = 3usize;
    let remaining_per_pack = DRAFT_PACK_SIZE - (color_names.len() * cards_per_color_per_pack);

    for _ in 0..DRAFT_NUM_PACKS {
        let mut used_this_pack = std::collections::HashSet::new();

        // Pick cards_per_color_per_pack from each color (weighted by rarity)
        for color in &color_names {
            if let Some(candidates) = by_color.get(*color) {
                for _ in 0..cards_per_color_per_pack {
                    let available: Vec<&Card> = candidates
                        .iter()
                        .filter(|c| !used_this_pack.contains(&c.id))
                        .collect();
                    if available.is_empty() {
                        continue;
                    }
                    if let Some(idx) = weighted_pick(&available, |c| draft_pool_weight(c), &mut rng)
                    {
                        let mut c = available[idx].clone();
                        used_this_pack.insert(c.id.clone());
                        id_counter += 1;
                        c.template_id = Some(c.id.clone());
                        c.id = format!("draft-{id_counter}");
                        pool.push(c);
                    }
                }
            }
        }

        // Fill remaining slots from multicolor pool + random colors (weighted)
        for i in 0..remaining_per_pack {
            let card = if !multicolor_cards.is_empty() && i < 2 {
                let available: Vec<&Card> = multicolor_cards
                    .iter()
                    .filter(|c| !used_this_pack.contains(&c.id))
                    .collect();
                if !available.is_empty() {
                    weighted_pick(&available, |c| draft_pool_weight(c), &mut rng)
                        .map(|idx| available[idx].clone())
                } else {
                    None
                }
            } else {
                None
            };

            let card = card.unwrap_or_else(|| {
                let color = color_names[rng.random_range(0..color_names.len())];
                if let Some(candidates) = by_color.get(color) {
                    let available: Vec<&Card> = candidates
                        .iter()
                        .filter(|c| !used_this_pack.contains(&c.id))
                        .collect();
                    if !available.is_empty() {
                        if let Some(idx) =
                            weighted_pick(&available, |c| draft_pool_weight(c), &mut rng)
                        {
                            return available[idx].clone();
                        }
                    }
                }
                for c_name in &color_names {
                    if let Some(candidates) = by_color.get(*c_name) {
                        let available: Vec<&Card> = candidates
                            .iter()
                            .filter(|c| !used_this_pack.contains(&c.id))
                            .collect();
                        if !available.is_empty() {
                            if let Some(idx) =
                                weighted_pick(&available, |c| draft_pool_weight(c), &mut rng)
                            {
                                return available[idx].clone();
                            }
                        }
                    }
                }
                multicolor_cards[rng.random_range(0..multicolor_cards.len())].clone()
            });

            let mut c = card;
            used_this_pack.insert(c.id.clone());
            id_counter += 1;
            c.template_id = Some(c.id.clone());
            c.id = format!("draft-{id_counter}");
            pool.push(c);
        }
    }

    pool.shuffle(&mut rng);
    pool
}

/// Run a full simulated draft between two AI players.
/// Returns (player_a_deck, player_b_deck) ready for match simulation.
pub fn run_sim_draft(seed: u64) -> (Vec<Card>, Vec<Card>) {
    let pool = generate_sim_draft_pool(seed);
    let player_ids = vec!["p1".to_string(), "p2".to_string()];
    let player_names: HashMap<String, String> = player_ids
        .iter()
        .map(|id| (id.clone(), id.clone()))
        .collect();

    let mut draft = DraftState::new(pool, &player_ids, player_names, DRAFT_NUM_PACKS);

    // Draft all packs
    for _ in 0..DRAFT_NUM_PACKS {
        if !draft.deal_next_pyramid(&player_ids) {
            break;
        }

        // Pick all 19 cards from the hex, alternating players
        loop {
            let picker = match &draft.current_picker {
                Some(p) => p.clone(),
                None => break,
            };

            let pick_index = match choose_auto_draft_pick(&draft, &picker) {
                Some(idx) => idx,
                None => break,
            };

            draft.pick(&picker, pick_index);

            // Advance picker
            let current_idx = player_ids.iter().position(|p| p == &picker).unwrap_or(0);
            let next_idx = (current_idx + 1) % player_ids.len();

            // Check if pyramid is empty
            let remaining = draft.pyramid.iter().filter(|s| s.is_some()).count();
            if remaining == 0 {
                draft.packs_completed += 1;
                draft.first_picker_index += 1;
                break;
            }

            draft.current_picker = Some(player_ids[next_idx].clone());
        }
    }

    // Build decks from drafted cards
    let deck_a = build_draft_deck("p1", &draft);
    let deck_b = build_draft_deck("p2", &draft);
    (deck_a, deck_b)
}

/// Build a playable deck from a player's drafted cards.
/// Uses all drafted cards plus auto-generated infrastructure.
fn build_draft_deck(player_id: &str, draft: &DraftState) -> Vec<Card> {
    let drafted = draft.drafted.get(player_id).cloned().unwrap_or_default();

    let cards: Vec<Card> = drafted
        .into_iter()
        .map(|mut c| {
            c.tapped = false;
            c.attached_to = None;
            c.remembered_player_id = None;
            c.attachment = None;
            c.summoning_sickness = false;
            c.burnout = false;
            c
        })
        .collect();

    // Mana comes from the per-turn deploy_node action — no dealt infra.
    let _ = player_id;

    cards
}

// ── Auto-pick AI (mirrors handlers/draft.rs logic) ──────────────────────────

fn draft_card_base_score(card: &Card) -> f32 {
    let mut score = match card.rarity {
        CardRarity::Public => 1.0,
        CardRarity::Private => 2.2,
        CardRarity::Restricted => 3.6,
        CardRarity::Classified => 5.0,
        CardRarity::Singularity => 6.0,
    };

    // Diminishing returns on stats — prevents huge creatures from dominating draft picks
    let total_stats = (card.power.unwrap_or(0) + card.toughness.unwrap_or(0)) as f32;
    score += total_stats.sqrt() * 0.9;
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

    // Penalize high mana costs — expensive cards are harder to cast in draft
    let total_mana = card.cost.total() as f32;
    if total_mana > 5.0 {
        score -= (total_mana - 5.0) * 0.4;
    }

    score
}

fn choose_auto_draft_pick(draft: &DraftState, player_id: &str) -> Option<usize> {
    let available = draft.available_indices();
    if available.is_empty() {
        return None;
    }

    let drafted_cards = draft.drafted.get(player_id).cloned().unwrap_or_default();
    let pick_number = drafted_cards.len();

    let mut color_counts: HashMap<ManaType, i32> = HashMap::new();
    let mut category_counts: HashMap<CardCategory, i32> = HashMap::new();
    let mut unit_type_counts: HashMap<UnitType, i32> = HashMap::new();
    let mut kind_counts: HashMap<&str, i32> = HashMap::new();
    let mut cost_buckets: HashMap<i32, i32> = HashMap::new();
    let mut keyword_counts: HashMap<String, i32> = HashMap::new();

    for card in &drafted_cards {
        for mana in get_card_color_identity(card) {
            if mana != ManaType::Colorless {
                *color_counts.entry(mana).or_insert(0) += 1;
            }
        }
        for category in &card.categories {
            *category_counts.entry(category.clone()).or_insert(0) += 1;
        }
        let total_cost = card.cost.total().min(7);
        *cost_buckets.entry(total_cost).or_insert(0) += 1;
        for kw in &card.keywords {
            let label = format!("{:?}", kw);
            let short = label.split('(').next().unwrap_or(&label).to_string();
            *keyword_counts.entry(short).or_insert(0) += 1;
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

    // Color commitment schedule:
    //   picks 0-3  : open, prefer strongest card but with a mild primary-color bonus
    //   picks 4-7  : lock to top color, strong bonus; off-primary allowed but docked
    //   picks 8+   : lock to top 2 colors, heavy penalty for anything outside
    let mut preferred_colors = color_counts.iter().collect::<Vec<_>>();
    preferred_colors.sort_by(|a, b| {
        b.1.cmp(a.1)
            .then_with(|| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)))
    });
    let primary_color = preferred_colors.first().map(|(m, _)| (*m).clone());
    let secondary_color = preferred_colors.get(1).map(|(m, _)| (*m).clone());
    let locked_colors: std::collections::HashSet<ManaType> = [&primary_color, &secondary_color]
        .iter()
        .filter_map(|opt| opt.as_ref().cloned())
        .collect();

    available.into_iter().max_by(|&a, &b| {
        let score_for = |index: usize| -> f32 {
            let Some(card) = draft.pyramid.get(index).and_then(|slot| slot.as_ref()) else {
                return f32::MIN;
            };

            let mut score = draft_card_base_score(card);
            let identity = get_card_color_identity(card);
            let non_colorless: Vec<_> = identity
                .iter()
                .filter(|mana| **mana != ManaType::Colorless)
                .cloned()
                .collect();

            // ── Color commitment ────────────────────────────────────────
            if non_colorless.is_empty() {
                score += 1.2; // splashable support
            } else {
                let is_primary = primary_color
                    .as_ref()
                    .map(|p| non_colorless.contains(p))
                    .unwrap_or(false);
                let is_secondary = secondary_color
                    .as_ref()
                    .map(|s| non_colorless.contains(s))
                    .unwrap_or(false);
                let in_locked = non_colorless.iter().any(|m| locked_colors.contains(m));

                if pick_number < 4 {
                    // Early: mild bonus for primary, slight penalty for multi-color splash
                    if is_primary {
                        score += 1.0;
                    }
                    if non_colorless.len() > 1 {
                        score -= 0.4;
                    }
                } else if pick_number < 8 {
                    // Mid: commit to primary, allow secondary
                    if is_primary {
                        score += 2.5;
                    } else if is_secondary {
                        score += 0.8;
                    } else {
                        score -= 2.5;
                    }
                } else {
                    // Late: only locked colors
                    if is_primary {
                        score += 3.0;
                    } else if is_secondary {
                        score += 1.2;
                    } else if !in_locked {
                        score -= 6.0 + non_colorless.len() as f32 * 0.8;
                    }
                }

                // Density: each additional matching-color card boosts value
                for mana in &non_colorless {
                    score += color_counts.get(mana).copied().unwrap_or(0) as f32 * 0.25;
                }
            }

            // ── Category density ────────────────────────────────────────
            for category in &card.categories {
                score += category_counts.get(category).copied().unwrap_or(0) as f32 * 0.4;
            }

            // ── Kind balance ────────────────────────────────────────────
            match &card.kind {
                CardKind::Unit(unit_type) => {
                    score += unit_type_counts.get(unit_type).copied().unwrap_or(0) as f32 * 0.5;
                    let unit_count = kind_counts.get("unit").copied().unwrap_or(0);
                    if unit_count < 12 {
                        score += 0.9;
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
                        score += 0.8;
                    }
                }
                _ => {}
            }

            // ── Mana curve ──────────────────────────────────────────────
            let total_cost = card.cost.total().min(7);
            let low_cost_count: i32 = (1..=3)
                .map(|c| cost_buckets.get(&c).copied().unwrap_or(0))
                .sum();
            let high_cost_count: i32 = (5..=7)
                .map(|c| cost_buckets.get(&c).copied().unwrap_or(0))
                .sum();
            if total_cost <= 3 && low_cost_count < 12 {
                score += 1.3;
            }
            if total_cost >= 5 && high_cost_count >= 8 {
                score -= 1.5;
            }

            // ── Keyword synergy ─────────────────────────────────────────
            for kw in &card.keywords {
                let label = format!("{:?}", kw);
                let short = label.split('(').next().unwrap_or(&label).to_string();
                let count = keyword_counts.get(&short).copied().unwrap_or(0);
                if count >= 2 {
                    score += (count as f32).min(4.0) * 0.4;
                }
            }

            score
        };

        score_for(a)
            .partial_cmp(&score_for(b))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.cmp(&a))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_draft_produces_two_decks() {
        let (deck_a, deck_b) = run_sim_draft(42);
        // Each player drafts from 8 packs of 19 cards = 152 total picks
        // 2 players alternating = ~47-48 cards each
        assert!(!deck_a.is_empty(), "player A deck should not be empty");
        assert!(!deck_b.is_empty(), "player B deck should not be empty");
        // Mana now comes from per-turn deploy_node actions rather than
        // dealt infrastructure cards, so infra is no longer in the deck.
    }

    #[test]
    fn sim_draft_is_deterministic() {
        let (a1, b1) = run_sim_draft(123);
        let (a2, b2) = run_sim_draft(123);
        assert_eq!(a1.len(), a2.len());
        assert_eq!(b1.len(), b2.len());
        for (c1, c2) in a1.iter().zip(a2.iter()) {
            assert_eq!(c1.template_id, c2.template_id);
        }
    }
}
