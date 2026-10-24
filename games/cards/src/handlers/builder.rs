//! Deck builder action handlers

use crate::card::{Card, mana_color_for_type};
use crate::card_catalog;
use crate::deck::DeckTemplateEntry;
use crate::helpers::{get_card_color_identity, validate_operator_deck_templates};
use crate::state::{CardBrowserSource, CardGameState};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use tawks_data::pagination::PaginationArgs;
use tawks_data::repository::{CursorDirection, Repository};
use ulid::Ulid;

use crate::types::{CardKind, CardRarity, CardSupertype, DeckFormat, GameFormat, ManaType};

pub trait BuilderHandlers {
    fn handle_builder_open(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_builder_close(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_builder_load_deck(
        &mut self,
        player_id: &str,
        deck_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_add_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_remove_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_add_sideboard_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_remove_sideboard_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_set_operator(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_clone_deck(
        &mut self,
        player_id: &str,
        deck_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_set_filter(
        &mut self,
        player_id: &str,
        kind: Option<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_set_sort(
        &mut self,
        player_id: &str,
        sort: Option<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_builder_create_deck(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_builder_autobuild(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_browser_open(
        &mut self,
        player_id: &str,
        target_player_id: Option<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_browser_close(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_browser_set_filter(
        &mut self,
        player_id: &str,
        kind: Option<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_browser_set_sort(
        &mut self,
        player_id: &str,
        sort: Option<String>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_browser_page(&mut self, player_id: &str, delta: i32) -> Result<Vec<GameEffect>>;
}

pub(crate) const COLLECTION_PAGE_SIZE: i32 = 40;
const CLASSIC_DECK_SIZE: i32 = 34;
const CLASSIC_MAX_COPIES: i32 = 3;
const OPERATOR_DECK_SIZE: i32 = 67;
const DRAFT_CLASSIC_SHELL_SIZE: i32 = 34;

fn draft_template_entries(drafted_cards: Vec<Card>) -> Vec<DeckTemplateEntry> {
    let mut by_card_id: HashMap<String, DeckTemplateEntry> = HashMap::new();
    for card in drafted_cards {
        let canonical_id = card.template_id.clone().unwrap_or_else(|| card.id.clone());
        let mut template_card = card.clone();
        template_card.id = canonical_id.clone();
        template_card.template_id = None;
        by_card_id
            .entry(canonical_id)
            .and_modify(|entry| entry.quantity += 1)
            .or_insert_with(|| DeckTemplateEntry {
                card: template_card,
                quantity: 1,
            });
    }
    by_card_id.into_values().collect()
}

fn mana_type_sort_key(mana: &ManaType) -> i32 {
    match mana {
        ManaType::Colorless => 0,
        ManaType::Chrome => 1,
        ManaType::Neon => 2,
        ManaType::Glitch => 3,
        ManaType::Bio => 4,
        ManaType::Slag => 5,
    }
}

fn rarity_sort_key(rarity: &CardRarity) -> i32 {
    match rarity {
        CardRarity::Public => 0,
        CardRarity::Private => 1,
        CardRarity::Restricted => 2,
        CardRarity::Classified => 3,
        CardRarity::Singularity => 4,
    }
}

fn kind_filter_key(kind: &CardKind) -> &'static str {
    match kind {
        CardKind::Infrastructure => "infrastructure",
        CardKind::Unit(_) => "unit",
        CardKind::Relic => "relic",
        CardKind::Instant => "instant",
        CardKind::Ritual => "ritual",
        CardKind::Aura => "aura",
        CardKind::Agent => "agent",
    }
}

fn mana_value(card: &Card) -> i32 {
    card.cost.total()
}

fn rarity_weight(rarity: &CardRarity) -> f32 {
    match rarity {
        CardRarity::Public => 1.0,
        CardRarity::Private => 1.35,
        CardRarity::Restricted => 1.9,
        CardRarity::Classified => 2.5,
        CardRarity::Singularity => 3.1,
    }
}

fn card_score(card: &Card) -> f32 {
    let mut score = rarity_weight(&card.rarity) * 7.0;
    score += (card.power.unwrap_or(0) + card.toughness.unwrap_or(0)) as f32 * 0.75;
    score += card.loyalty.unwrap_or(0) as f32 * 0.8;
    score += card.abilities.len() as f32 * 2.0;
    score += card.triggers.len() as f32 * 1.8;
    score += card.keywords.len() as f32 * 1.1;

    for category in &card.categories {
        score += match category {
            crate::types::CardCategory::Removal => 4.5,
            crate::types::CardCategory::CardDraw => 3.8,
            crate::types::CardCategory::ManaRamp => 3.0,
            crate::types::CardCategory::Aggro => 2.8,
            crate::types::CardCategory::Midrange => 2.4,
            crate::types::CardCategory::Control => 2.4,
            crate::types::CardCategory::Utility => 1.8,
            crate::types::CardCategory::Buff => 1.7,
            crate::types::CardCategory::Combo => 2.2,
            crate::types::CardCategory::Evasion => 2.6,
        };
    }

    score += match card.kind {
        CardKind::Infrastructure => 5.0,
        CardKind::Instant => 4.0,
        CardKind::Ritual => 3.2,
        CardKind::Aura => 3.0,
        CardKind::Agent => 3.4,
        CardKind::Relic => 2.8,
        CardKind::Unit(_) => 2.4,
    };

    if card.supertype == CardSupertype::Apex {
        score += 4.5;
    }
    if card.enters_tapped {
        score -= 0.7;
    }

    let cost = mana_value(card);
    score += match cost {
        0..=1 => 1.8,
        2..=3 => 2.1,
        4..=5 => 1.3,
        _ => 0.4,
    };

    score
}

fn is_operator_candidate(card: &Card) -> bool {
    card.supertype == CardSupertype::Apex
        && matches!(
            card.kind,
            CardKind::Unit(_) | CardKind::Agent | CardKind::Relic
        )
}

fn is_identity_compatible(card: &Card, palette: &HashSet<ManaType>) -> bool {
    let identity = get_card_color_identity(card);
    identity.is_empty() || identity.iter().all(|mana| palette.contains(mana))
}

fn classic_copy_cap(card: &Card, owned: i32) -> i32 {
    let curve_cap = match mana_value(card) {
        0..=2 => 3,
        3..=4 => 2,
        _ => 1,
    };
    owned.min(CLASSIC_MAX_COPIES).min(curve_cap)
}

fn add_entries_until_target(
    source: &[DeckTemplateEntry],
    selected: &mut HashMap<String, DeckTemplateEntry>,
    target: i32,
    copy_cap: impl Fn(&Card, i32) -> i32,
) -> i32 {
    let mut total = selected.values().map(|entry| entry.quantity).sum::<i32>();
    if total >= target {
        return total;
    }

    for entry in source {
        if total >= target {
            break;
        }

        let current = selected
            .get(&entry.card.id)
            .map(|card| card.quantity)
            .unwrap_or(0);
        let allowed = copy_cap(&entry.card, entry.quantity);
        if allowed <= current {
            continue;
        }

        let to_add = (allowed - current).min(target - total);
        if to_add <= 0 {
            continue;
        }

        selected
            .entry(entry.card.id.clone())
            .and_modify(|existing| existing.quantity += to_add)
            .or_insert_with(|| DeckTemplateEntry {
                card: entry.card.clone(),
                quantity: to_add,
            });
        total += to_add;
    }

    total
}

fn build_classic_for_palette(
    collection: &[DeckTemplateEntry],
    palette: &HashSet<ManaType>,
) -> Result<(Vec<DeckTemplateEntry>, f32)> {
    let mut spells = Vec::new();

    for entry in collection {
        if !is_identity_compatible(&entry.card, palette) {
            continue;
        }
        spells.push(entry.clone());
    }

    spells.sort_by(|a, b| {
        card_score(&b.card)
            .partial_cmp(&card_score(&a.card))
            .unwrap_or(Ordering::Equal)
    });

    let mut selected: HashMap<String, DeckTemplateEntry> = HashMap::new();
    add_entries_until_target(&spells, &mut selected, CLASSIC_DECK_SIZE, classic_copy_cap);
    add_entries_until_target(&spells, &mut selected, CLASSIC_DECK_SIZE, |_, owned| {
        owned.min(CLASSIC_MAX_COPIES)
    });

    let total = selected.values().map(|entry| entry.quantity).sum::<i32>();
    if total < CLASSIC_DECK_SIZE {
        return Err(anyhow!(
            "Not enough owned cards to build a full classic deck"
        ));
    }

    let mut out = selected.into_values().collect::<Vec<_>>();
    out.sort_by(|a, b| {
        card_score(&b.card)
            .partial_cmp(&card_score(&a.card))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.card.name.cmp(&b.card.name))
    });

    let score = out
        .iter()
        .map(|entry| card_score(&entry.card) * entry.quantity as f32)
        .sum::<f32>();
    Ok((out, score))
}

fn palette_hexes(palette: &HashSet<ManaType>) -> Vec<String> {
    let mut colors = palette.iter().cloned().collect::<Vec<_>>();
    colors.sort_by_key(mana_type_sort_key);
    colors
        .into_iter()
        .map(|mana| mana_color_for_type(&mana).to_string())
        .collect()
}

fn auto_build_classic(
    collection: &[DeckTemplateEntry],
) -> Result<(Vec<DeckTemplateEntry>, Vec<String>)> {
    let mut color_scores: HashMap<ManaType, f32> = HashMap::new();
    for entry in collection {
        let identity = get_card_color_identity(&entry.card);
        if identity.is_empty() {
            continue;
        }
        let share = card_score(&entry.card) * entry.quantity as f32 / identity.len() as f32;
        for mana in identity {
            if mana != ManaType::Colorless {
                *color_scores.entry(mana).or_insert(0.0) += share;
            }
        }
    }

    let colors = [
        ManaType::Neon,
        ManaType::Glitch,
        ManaType::Chrome,
        ManaType::Slag,
        ManaType::Bio,
    ];

    let mut best: Option<(Vec<DeckTemplateEntry>, Vec<String>, f32)> = None;
    for first in 0..colors.len() {
        for second in (first + 1)..colors.len() {
            for third in (second + 1)..colors.len() {
                let palette = HashSet::from([
                    colors[first].clone(),
                    colors[second].clone(),
                    colors[third].clone(),
                ]);
                if let Ok(candidate) = build_classic_for_palette(collection, &palette) {
                    let tri_bias = color_scores.get(&colors[first]).copied().unwrap_or(0.0)
                        + color_scores.get(&colors[second]).copied().unwrap_or(0.0)
                        + color_scores.get(&colors[third]).copied().unwrap_or(0.0)
                        + 6.0;
                    let adjusted = candidate.1 + tri_bias;
                    if best
                        .as_ref()
                        .map(|(_, _, score)| adjusted > *score)
                        .unwrap_or(true)
                    {
                        best = Some((candidate.0, palette_hexes(&palette), adjusted));
                    }
                }
            }
        }
    }

    best.map(|(deck, palette, _)| (deck, palette))
        .ok_or_else(|| anyhow!("Could not assemble a legal classic deck from owned cards"))
}

fn build_draft_classic_for_palette(
    collection: &[DeckTemplateEntry],
    palette: &HashSet<ManaType>,
) -> Result<(Vec<DeckTemplateEntry>, f32)> {
    let mut spells = collection
        .iter()
        .filter(|entry| is_identity_compatible(&entry.card, palette))
        .cloned()
        .collect::<Vec<_>>();

    spells.sort_by(|a, b| {
        card_score(&b.card)
            .partial_cmp(&card_score(&a.card))
            .unwrap_or(Ordering::Equal)
    });

    let mut selected: HashMap<String, DeckTemplateEntry> = HashMap::new();
    let target = DRAFT_CLASSIC_SHELL_SIZE.min(
        spells
            .iter()
            .map(|entry| entry.quantity.max(0))
            .sum::<i32>(),
    );

    if target <= 0 {
        return Err(anyhow!("No drafted spells available for a classic shell"));
    }

    add_entries_until_target(&spells, &mut selected, target, |_, owned| owned.min(1));

    let mut out = selected.into_values().collect::<Vec<_>>();
    if out.is_empty() {
        return Err(anyhow!(
            "Could not assemble a classic shell from drafted cards"
        ));
    }

    out.sort_by(|a, b| {
        card_score(&b.card)
            .partial_cmp(&card_score(&a.card))
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.card.name.cmp(&b.card.name))
    });

    let score = out
        .iter()
        .map(|entry| card_score(&entry.card) * entry.quantity as f32)
        .sum::<f32>();
    Ok((out, score))
}

fn auto_build_draft_classic(
    collection: &[DeckTemplateEntry],
) -> Result<(Vec<DeckTemplateEntry>, Vec<String>)> {
    let mut color_scores: HashMap<ManaType, f32> = HashMap::new();
    for entry in collection {
        let identity = get_card_color_identity(&entry.card);
        if identity.is_empty() {
            continue;
        }
        let share = card_score(&entry.card) * entry.quantity as f32 / identity.len() as f32;
        for mana in identity {
            if mana != ManaType::Colorless {
                *color_scores.entry(mana).or_insert(0.0) += share;
            }
        }
    }

    let colors = [
        ManaType::Neon,
        ManaType::Glitch,
        ManaType::Chrome,
        ManaType::Slag,
        ManaType::Bio,
    ];

    let mut best: Option<(Vec<DeckTemplateEntry>, Vec<String>, f32)> = None;
    for first in 0..colors.len() {
        for second in (first + 1)..colors.len() {
            for third in (second + 1)..colors.len() {
                let palette = HashSet::from([
                    colors[first].clone(),
                    colors[second].clone(),
                    colors[third].clone(),
                ]);
                if let Ok(candidate) = build_draft_classic_for_palette(collection, &palette) {
                    let tri_bias = color_scores.get(&colors[first]).copied().unwrap_or(0.0)
                        + color_scores.get(&colors[second]).copied().unwrap_or(0.0)
                        + color_scores.get(&colors[third]).copied().unwrap_or(0.0)
                        + 6.0;
                    let adjusted = candidate.1 + tri_bias;
                    if best
                        .as_ref()
                        .map(|(_, _, score)| adjusted > *score)
                        .unwrap_or(true)
                    {
                        best = Some((candidate.0, palette_hexes(&palette), adjusted));
                    }
                }
            }
        }
    }

    best.map(|(deck, palette, _)| (deck, palette))
        .ok_or_else(|| anyhow!("Could not assemble a legal classic deck from drafted cards"))
}

fn build_operator_for_candidate(
    collection: &[DeckTemplateEntry],
    operator_entry: &DeckTemplateEntry,
) -> Result<(Vec<DeckTemplateEntry>, f32)> {
    let palette = get_card_color_identity(&operator_entry.card);
    let mut candidates = collection
        .iter()
        .filter(|entry| entry.card.id != operator_entry.card.id)
        .filter(|entry| is_identity_compatible(&entry.card, &palette))
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        card_score(&b.card)
            .partial_cmp(&card_score(&a.card))
            .unwrap_or(Ordering::Equal)
    });

    let mut selected = vec![DeckTemplateEntry {
        card: operator_entry.card.clone(),
        quantity: 1,
    }];
    let mut total = 1;

    for entry in &candidates {
        if total >= OPERATOR_DECK_SIZE {
            break;
        }

        let quantity = if matches!(entry.card.kind, CardKind::Infrastructure) {
            1.min(entry.quantity)
        } else {
            1
        };
        if quantity <= 0 {
            continue;
        }
        selected.push(DeckTemplateEntry {
            card: entry.card.clone(),
            quantity,
        });
        total += quantity;
    }

    if total < OPERATOR_DECK_SIZE {
        for entry in &candidates {
            if total >= OPERATOR_DECK_SIZE {
                break;
            }
            if !matches!(entry.card.kind, CardKind::Infrastructure) {
                continue;
            }

            let already = selected
                .iter()
                .find(|picked| picked.card.id == entry.card.id)
                .map(|picked| picked.quantity)
                .unwrap_or(0);
            if entry.quantity <= already {
                continue;
            }

            let extra = (entry.quantity - already).min(OPERATOR_DECK_SIZE - total);
            if extra <= 0 {
                continue;
            }

            if let Some(existing) = selected
                .iter_mut()
                .find(|picked| picked.card.id == entry.card.id)
            {
                existing.quantity += extra;
            } else {
                selected.push(DeckTemplateEntry {
                    card: entry.card.clone(),
                    quantity: extra,
                });
            }
            total += extra;
        }
    }

    if total != OPERATOR_DECK_SIZE {
        return Err(anyhow!(
            "Not enough owned legal cards for a full operator deck"
        ));
    }

    validate_operator_deck_templates(&selected, &operator_entry.card.id)?;
    let score = selected
        .iter()
        .map(|entry| card_score(&entry.card) * entry.quantity as f32)
        .sum::<f32>()
        + get_card_color_identity(&operator_entry.card).len() as f32 * 4.0;
    Ok((selected, score))
}

fn auto_build_operator(
    collection: &[DeckTemplateEntry],
) -> Result<(Vec<DeckTemplateEntry>, String)> {
    let mut best: Option<(Vec<DeckTemplateEntry>, String, f32)> = None;
    for operator_entry in collection
        .iter()
        .filter(|entry| entry.quantity > 0 && is_operator_candidate(&entry.card))
    {
        let Ok((deck, score)) = build_operator_for_candidate(collection, operator_entry) else {
            continue;
        };
        if best
            .as_ref()
            .map(|(_, _, best_score)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((deck, operator_entry.card.id.clone(), score));
        }
    }

    best.map(|(deck, operator_id, _)| (deck, operator_id))
        .ok_or_else(|| anyhow!("Could not assemble a legal operator deck from owned cards"))
}

impl CardGameState {
    fn browser_filter_tokens(&self, player_id: &str) -> HashSet<String> {
        self.players
            .get(player_id)
            .and_then(|player| player.deck_builder_filter_kind.as_deref())
            .map(|raw| {
                raw.split(|ch| ch == ',' || ch == '|')
                    .map(|token| token.trim().to_ascii_lowercase())
                    .filter(|token| !token.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn apply_browser_filters_and_sort(&self, player_id: &str, cards: Vec<Card>) -> Vec<Card> {
        let filters = self.browser_filter_tokens(player_id);
        let mana_filters = filters
            .iter()
            .filter_map(|token| match token.as_str() {
                "neon" => Some(ManaType::Neon),
                "glitch" => Some(ManaType::Glitch),
                "chrome" => Some(ManaType::Chrome),
                "slag" => Some(ManaType::Slag),
                "bio" => Some(ManaType::Bio),
                "colorless" => Some(ManaType::Colorless),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let kind_filters = filters
            .iter()
            .filter(|token| {
                matches!(
                    token.as_str(),
                    "infrastructure"
                        | "unit"
                        | "artifact"
                        | "relic"
                        | "instant"
                        | "sorcery"
                        | "ritual"
                        | "enchantment"
                        | "aura"
                        | "agent"
                )
            })
            .cloned()
            .collect::<HashSet<_>>();
        let sort = self
            .players
            .get(player_id)
            .and_then(|player| player.deck_builder_sort_order.as_deref())
            .unwrap_or("color");

        let mut filtered = cards
            .into_iter()
            .filter(|card| {
                if !mana_filters.is_empty() {
                    let identity = get_card_color_identity(card);
                    let matches_mana = if identity.is_empty() {
                        mana_filters.contains(&ManaType::Colorless)
                    } else {
                        identity.iter().any(|mana| mana_filters.contains(mana))
                    };
                    if !matches_mana {
                        return false;
                    }
                }

                if !kind_filters.is_empty() && !kind_filters.contains(kind_filter_key(&card.kind)) {
                    return false;
                }

                true
            })
            .collect::<Vec<_>>();

        filtered.sort_by(|a, b| {
            let a_colors = get_card_color_identity(a);
            let b_colors = get_card_color_identity(b);
            let a_primary = a_colors.iter().map(mana_type_sort_key).min().unwrap_or(0);
            let b_primary = b_colors.iter().map(mana_type_sort_key).min().unwrap_or(0);
            let mut a_secondary = a_colors.iter().map(mana_type_sort_key).collect::<Vec<_>>();
            let mut b_secondary = b_colors.iter().map(mana_type_sort_key).collect::<Vec<_>>();
            a_secondary.sort_unstable();
            b_secondary.sort_unstable();
            let base = match sort {
                "name" => a.name.cmp(&b.name),
                "name_desc" => b.name.cmp(&a.name),
                "rarity" => rarity_sort_key(&a.rarity)
                    .cmp(&rarity_sort_key(&b.rarity))
                    .then_with(|| a_primary.cmp(&b_primary))
                    .then_with(|| a.name.cmp(&b.name)),
                "rarity_desc" => rarity_sort_key(&b.rarity)
                    .cmp(&rarity_sort_key(&a.rarity))
                    .then_with(|| a_primary.cmp(&b_primary))
                    .then_with(|| a.name.cmp(&b.name)),
                "obtained_desc" => b
                    .obtained_at
                    .cmp(&a.obtained_at)
                    .then_with(|| a.name.cmp(&b.name)),
                "obtained" => a
                    .obtained_at
                    .cmp(&b.obtained_at)
                    .then_with(|| a.name.cmp(&b.name)),
                "value_desc" => rarity_weight(&b.rarity)
                    .partial_cmp(&rarity_weight(&a.rarity))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.name.cmp(&b.name)),
                "value" => rarity_weight(&a.rarity)
                    .partial_cmp(&rarity_weight(&b.rarity))
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| a.name.cmp(&b.name)),
                "color_desc" => b_primary
                    .cmp(&a_primary)
                    .then_with(|| b_secondary.cmp(&a_secondary))
                    .then_with(|| rarity_sort_key(&b.rarity).cmp(&rarity_sort_key(&a.rarity)))
                    .then_with(|| a.name.cmp(&b.name)),
                _ => a_primary
                    .cmp(&b_primary)
                    .then_with(|| a_secondary.cmp(&b_secondary))
                    .then_with(|| rarity_sort_key(&a.rarity).cmp(&rarity_sort_key(&b.rarity)))
                    .then_with(|| a.name.cmp(&b.name)),
            };

            base.then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.id.cmp(&b.id))
        });

        filtered
    }

    fn refresh_draft_builder_library(&mut self, player_id: &str) {
        let drafted_cards = self
            .draft
            .as_ref()
            .and_then(|draft| draft.drafted.get(player_id))
            .cloned()
            .unwrap_or_default();
        let drafted_cards = self.apply_browser_filters_and_sort(player_id, drafted_cards);

        if let Some(player) = self.players.get_mut(player_id) {
            player.library_search_results = drafted_cards;
            player.collection_page = 0;
            player.collection_total = player.library_search_results.len() as i32;
        }
    }

    fn refresh_graveyard_browser_library(&mut self, player_id: &str, target_player_id: &str) {
        let target_name = self
            .players
            .get(target_player_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| "Unknown Player".to_string());
        let graveyard_cards = self
            .players
            .get(target_player_id)
            .map(|player| player.graveyard.clone())
            .unwrap_or_default();
        let graveyard_cards = self.apply_browser_filters_and_sort(player_id, graveyard_cards);

        if let Some(player) = self.players.get_mut(player_id) {
            player.card_browser_source = Some(CardBrowserSource::Graveyard {
                target_player_id: target_player_id.to_string(),
            });
            player.card_browser_title = Some(format!("{}'s Graveyard", target_name));
            player.card_browser_subtitle = Some("Browse discarded and destroyed cards".to_string());
            player.library_search_results = graveyard_cards;
            player.collection_page = 0;
            player.collection_total = player.library_search_results.len() as i32;
        }
    }

    fn local_catalog_cards(&self) -> Vec<Card> {
        let mut cards = Vec::new();
        for (card, _) in card_catalog::bio::all_cards() {
            cards.push(card);
        }
        for (card, _) in card_catalog::chrome::all_cards() {
            cards.push(card);
        }
        for (card, _) in card_catalog::glitch::all_cards() {
            cards.push(card);
        }
        for (card, _) in card_catalog::neon::all_cards() {
            cards.push(card);
        }
        for (card, _) in card_catalog::slag::all_cards() {
            cards.push(card);
        }
        for (card, _) in card_catalog::multicolor::all_cards() {
            cards.push(card);
        }
        cards
    }

    fn load_local_catalog_page(&mut self, player_id: &str, page: i32) {
        let filtered_cards =
            self.apply_browser_filters_and_sort(player_id, self.local_catalog_cards());
        let total = filtered_cards.len() as i32;
        let max_page = ((total as f64) / COLLECTION_PAGE_SIZE as f64).ceil() as i32 - 1;
        let clamped_page = page.clamp(0, max_page.max(0));
        let start = (clamped_page * COLLECTION_PAGE_SIZE) as usize;
        let end = (start + COLLECTION_PAGE_SIZE as usize).min(filtered_cards.len());
        let page_cards = if start < filtered_cards.len() {
            filtered_cards[start..end].to_vec()
        } else {
            Vec::new()
        };

        if let Some(player) = self.players.get_mut(player_id) {
            player.library_search_results = page_cards;
            player.collection_page = clamped_page;
            player.collection_total = total;
        }
    }

    fn load_collection_page(&mut self, player_id: &str, page: i32) {
        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            let pid = player_id.to_string();
            let offset = page * COLLECTION_PAGE_SIZE;
            let result = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    repo.find_user_collection(&pid, COLLECTION_PAGE_SIZE, offset)
                        .await
                })
            });
            if let Ok((cards, total)) = result {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.library_search_results = cards;
                    player.collection_page = page;
                    player.collection_total = total;
                }
            }
        }
    }

    fn load_catalog_page(&mut self, player_id: &str, page: i32) {
        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            let pid = player_id.to_string();
            let offset = page * COLLECTION_PAGE_SIZE;
            let take = COLLECTION_PAGE_SIZE;

            // Extract color/kind filters and sort from player state
            let (color_filters, kind_filters, sort) = self.extract_catalog_params(player_id);
            let sort_debug = sort.clone();

            let result = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    repo.find_catalog(
                        &pid,
                        take,
                        offset,
                        &color_filters,
                        &kind_filters,
                        sort.as_deref(),
                    )
                    .await
                })
            });
            match result {
                Ok((cards, total)) => {
                    tracing::info!(
                        target: "cardgame::catalog",
                        cards = cards.len(),
                        total,
                        page,
                        sort = ?sort_debug,
                        "load_catalog_page ok"
                    );
                    if cards.is_empty() && total == 0 {
                        tracing::warn!(
                            target: "cardgame::catalog",
                            "repository returned empty catalog, using local fallback"
                        );
                        self.load_local_catalog_page(player_id, page);
                    } else if let Some(player) = self.players.get_mut(player_id) {
                        player.library_search_results = cards;
                        player.collection_page = page;
                        player.collection_total = total;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "cardgame::catalog",
                        page,
                        error = ?e,
                        "load_catalog_page ERROR"
                    );
                    self.load_local_catalog_page(player_id, page);
                }
            }
        } else {
            tracing::warn!(target: "cardgame::catalog", "no repository available");
            self.load_local_catalog_page(player_id, page);
        }
    }

    fn browser_has_owned_filter(&self, player_id: &str) -> bool {
        self.players
            .get(player_id)
            .and_then(|p| p.deck_builder_filter_kind.as_deref())
            .map(|f| {
                f.split(|ch| ch == ',' || ch == '|')
                    .any(|entry| entry.trim().eq_ignore_ascii_case("owned"))
            })
            .unwrap_or(false)
    }

    fn extract_catalog_params(
        &self,
        player_id: &str,
    ) -> (Vec<String>, Vec<String>, Option<String>) {
        let player = self.players.get(player_id);
        let filter_str = player
            .and_then(|p| p.deck_builder_filter_kind.clone())
            .unwrap_or_default();
        let sort = player.and_then(|p| p.deck_builder_sort_order.clone());

        let mut color_filters: Vec<String> = Vec::new();
        let mut kind_filters: Vec<String> = Vec::new();
        for entry in filter_str.split(',').map(|s| s.trim().to_ascii_lowercase()) {
            if entry.is_empty() || entry == "owned" {
                continue;
            }
            if matches!(
                entry.as_str(),
                "neon" | "glitch" | "chrome" | "slag" | "bio" | "colorless"
            ) {
                color_filters.push(entry);
            } else {
                kind_filters.push(entry);
            }
        }
        (color_filters, kind_filters, sort)
    }
}

impl BuilderHandlers for CardGameState {
    fn handle_builder_open(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
            player.deck_builder_filter_kind =
                if self.stage == crate::types::CardGameStage::DraftBuilding {
                    None
                } else {
                    Some("owned".to_string())
                };
        }

        if self.stage == crate::types::CardGameStage::DraftBuilding {
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        // Re-trigger card listing to populate library
        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            let args = crate::repository::CardArgs {
                pagination: PaginationArgs {
                    before: None,
                    after: None,
                    first: Some(200),
                    last: None,
                },
                owner_id: Some(player_id.to_string()),
            };
            let result = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    repo.find(
                        None::<(CursorDirection, crate::repository::CardCursor)>,
                        args.pagination.first.unwrap_or(200),
                        &args,
                    )
                    .await
                })
            });
            if let Ok(cards) = result {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.library_search_results = cards;
                }
            }
        }
        Ok(vec![])
    }

    fn handle_builder_close(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = false;
            player.deck_builder_deck_id = None;
            player.deck_builder_name = None;
            player.deck_builder_description = None;
            player.deck_builder_format = None;
            player.deck_builder_operator_card_id = None;
            player.deck_builder_autobuild_colors = None;
            player.deck_builder_cards.clear();
            player.deck_builder_filter_kind = None;
            player.deck_builder_sort_order = None;
            player.library_search_results.clear();
        }
        Ok(vec![])
    }

    fn handle_builder_load_deck(
        &mut self,
        player_id: &str,
        deck_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        self.load_deck_builder_for_player(player_id, &deck_id)?;
        self.refresh_deck_choices_for(player_id);
        Ok(vec![])
    }

    fn handle_builder_add_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id_str = player_id.to_string();
        let player_id_for_task = player_id_str.clone();
        let deck_id_clone = deck_id.clone();
        let card_id_clone = card_id.clone();
        let draft_card_lookup = if self.stage == crate::types::CardGameStage::DraftBuilding {
            let mut by_instance = HashMap::new();
            let mut counts = HashMap::new();
            for card in self
                .draft
                .as_ref()
                .and_then(|draft| draft.drafted.get(player_id))
                .cloned()
                .unwrap_or_default()
            {
                let canonical_id = card.template_id.clone().unwrap_or_else(|| card.id.clone());
                by_instance.insert(card.id.clone(), canonical_id.clone());
                *counts.entry(canonical_id).or_insert(0) += 1;
            }
            Some((by_instance, counts))
        } else {
            None
        };
        let result: Result<()> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let draft_card_lookup = draft_card_lookup.clone();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id_clone).await?;
                if deck.owner_id.as_deref() != Some(&player_id_for_task) {
                    return Err(anyhow!("Deck does not belong to player"));
                }
                let normalized_card_id =
                    if let Some((instance_to_card, draft_owned_by_card)) = draft_card_lookup {
                        let canonical_card_id = instance_to_card
                            .get(&card_id_clone)
                            .cloned()
                            .unwrap_or_else(|| card_id_clone.clone());
                        let allowed = draft_owned_by_card
                            .get(&canonical_card_id)
                            .copied()
                            .unwrap_or(0);
                        if allowed <= 0 {
                            return Err(anyhow!("Card is not in your drafted pool"));
                        }
                        let current = repo
                            .load_deck_templates(&deck_id_clone)
                            .await?
                            .into_iter()
                            .find(|entry| entry.card.id == canonical_card_id)
                            .map(|entry| entry.quantity)
                            .unwrap_or(0);
                        if current >= allowed {
                            return Err(anyhow!(if allowed <= 1 {
                                "Draft decks can only include one copy of each drafted card"
                            } else {
                                "You can only add as many copies as you drafted"
                            }));
                        }
                        canonical_card_id
                    } else {
                        card_id_clone.clone()
                    };
                repo.adjust_deck_card_quantity(&deck_id_clone, &normalized_card_id, 1)
                    .await
            })
        });
        result?;
        self.load_deck_builder_for_player(player_id, &deck_id)?;
        Ok(vec![])
    }

    fn handle_builder_remove_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id_str = player_id.to_string();
        let player_id_for_task = player_id_str.clone();
        let deck_id_clone = deck_id.clone();
        let card_id_clone = card_id.clone();
        let result: Result<()> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id_clone).await?;
                if deck.owner_id.as_deref() != Some(&player_id_for_task) {
                    return Err(anyhow!("Deck does not belong to player"));
                }
                repo.adjust_deck_card_quantity(&deck_id_clone, &card_id_clone, -1)
                    .await
            })
        });
        result?;
        self.load_deck_builder_for_player(player_id, &deck_id)?;
        Ok(vec![])
    }

    fn handle_builder_add_sideboard_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id_for_task = player_id.to_string();
        let deck_id_clone = deck_id.clone();
        let card_id_clone = card_id.clone();
        let result: Result<()> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id_clone).await?;
                if deck.owner_id.as_deref() != Some(&player_id_for_task) {
                    return Err(anyhow!("Deck does not belong to player"));
                }
                // Enforce a soft 15-card sideboard cap — matches MTG convention
                // and keeps the swap UI readable.
                let current_sideboard = repo.load_sideboard_templates(&deck_id_clone).await?;
                let total: i32 = current_sideboard.iter().map(|e| e.quantity).sum();
                if total >= 15 {
                    return Err(anyhow!("Sideboard limit is 15 cards"));
                }
                repo.adjust_deck_card_quantity_with_zone(&deck_id_clone, &card_id_clone, 1, true)
                    .await
            })
        });
        result?;
        self.load_deck_builder_for_player(player_id, &deck_id)?;
        Ok(vec![])
    }

    fn handle_builder_remove_sideboard_card(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id_for_task = player_id.to_string();
        let deck_id_clone = deck_id.clone();
        let card_id_clone = card_id.clone();
        let result: Result<()> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id_clone).await?;
                if deck.owner_id.as_deref() != Some(&player_id_for_task) {
                    return Err(anyhow!("Deck does not belong to player"));
                }
                repo.adjust_deck_card_quantity_with_zone(&deck_id_clone, &card_id_clone, -1, true)
                    .await
            })
        });
        result?;
        self.load_deck_builder_for_player(player_id, &deck_id)?;
        Ok(vec![])
    }

    fn handle_builder_set_operator(
        &mut self,
        player_id: &str,
        deck_id: String,
        card_id: String,
    ) -> Result<Vec<GameEffect>> {
        let is_draft = self.draft.is_some();
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id = player_id.to_string();
        let result: Result<()> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let deck_id = deck_id.clone();
            let card_id = card_id.clone();
            let player_id = player_id.clone();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id).await?;
                if deck.owner_id.as_deref() != Some(&player_id) {
                    return Err(anyhow!("Deck does not belong to player"));
                }
                if deck.format != DeckFormat::Operator {
                    repo.set_deck_format(&deck_id, DeckFormat::Operator).await?;
                }
                let Some(card) = repo.get_card_by_id(&card_id).await? else {
                    return Err(anyhow!("Operator card not found"));
                };
                if !is_draft {
                    if card.supertype != CardSupertype::Apex
                        || !matches!(
                            card.kind,
                            CardKind::Unit(_) | CardKind::Agent | CardKind::Relic
                        )
                    {
                        return Err(anyhow!("Operator must be an apex unit, agent, or artifact"));
                    }
                }
                let templates = repo.load_deck_templates(&deck_id).await?;
                let in_deck = templates.iter().any(|entry| entry.card.id == card_id);
                if !in_deck {
                    repo.adjust_deck_card_quantity(&deck_id, &card_id, 1)
                        .await?;
                }
                repo.set_deck_operator(&deck_id, Some(&card_id)).await?;
                Ok(())
            })
        });
        result?;
        if let Some(player) = self.players.get_mut(&player_id) {
            player.deck_builder_active = true;
        }
        self.load_deck_builder_for_player(&player_id, &deck_id)?;
        Ok(vec![])
    }

    fn handle_builder_clone_deck(
        &mut self,
        player_id: &str,
        deck_id: String,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
        }
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let deck_id_clone = deck_id.clone();
        let player_id_str = player_id.to_string();
        let player_id_for_task = player_id_str.clone();
        let result: Result<crate::repository::DeckRecord> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id_clone).await?;
                let new_id = format!("{}-clone-{}", deck.id, Ulid::new());
                let new_name = format!("Copy of {}", deck.name);
                let base_deck_id = if deck.owner_id.is_none() {
                    Some(deck.id.as_str())
                } else {
                    deck.base_deck_id.as_deref()
                };
                repo.clone_deck_with_owner(
                    &deck.id,
                    &new_id,
                    &new_name,
                    &deck.description,
                    Some(&player_id_for_task),
                    base_deck_id,
                )
                .await
            })
        });
        let new_deck = result?;
        if let Some(player) = self.players.get_mut(&player_id_str) {
            player.deck_builder_active = true;
        }
        self.refresh_deck_choices_for(&player_id_str);
        self.load_deck_builder_for_player(&player_id_str, &new_deck.id)?;
        Ok(vec![])
    }

    fn handle_builder_set_filter(
        &mut self,
        player_id: &str,
        kind: Option<String>,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_active = true;
            player.deck_builder_filter_kind =
                kind.map(|value| value.replace('|', ",").to_ascii_lowercase());
        }

        if self.stage == crate::types::CardGameStage::DraftBuilding {
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        // Re-trigger card listing
        if let Some(repo) = &self.repository {
            let repo = repo.clone();
            let args = crate::repository::CardArgs {
                pagination: PaginationArgs {
                    before: None,
                    after: None,
                    first: Some(200),
                    last: None,
                },
                owner_id: Some(player_id.to_string()),
            };
            let result = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async move {
                    repo.find(
                        None::<(CursorDirection, crate::repository::CardCursor)>,
                        args.pagination.first.unwrap_or(200),
                        &args,
                    )
                    .await
                })
            });
            if let Ok(cards) = result {
                if let Some(player) = self.players.get_mut(player_id) {
                    player.library_search_results = cards;
                }
            }
        }
        Ok(vec![])
    }

    fn handle_builder_set_sort(
        &mut self,
        player_id: &str,
        sort: Option<String>,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_sort_order = sort;
        }
        if self.stage == crate::types::CardGameStage::DraftBuilding {
            self.refresh_draft_builder_library(player_id);
        }
        Ok(vec![])
    }

    fn handle_builder_create_deck(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let new_id = format!("custom-{}", Ulid::new());
        let short = new_id.split('-').nth(1).unwrap_or("Deck");
        let name = format!("Custom Deck {}", &short[..short.len().min(8)]);
        let description = "Custom deck".to_string();
        let player_id_str = player_id.to_string();
        let player_id_for_task = player_id_str.clone();
        let result: Result<crate::repository::DeckRecord> = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let new_id = new_id.clone();
            let name = name.clone();
            let description = description.clone();
            handle.block_on(async move {
                repo.create_deck_with_owner(
                    &new_id,
                    &name,
                    &description,
                    Some(&player_id_for_task),
                    None,
                )
                .await
            })
        });
        let new_deck = result?;
        if let Some(player) = self.players.get_mut(&player_id_str) {
            player.deck_builder_filter_kind = None;
            player.deck_builder_active = true;
        }
        self.refresh_deck_choices_for(&player_id_str);
        self.load_deck_builder_for_player(&player_id_str, &new_deck.id)?;
        Ok(vec![])
    }

    fn handle_builder_autobuild(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.is_ai_player(player_id) {
            let inferred_format = self
                .players
                .get(player_id)
                .and_then(|player| player.deck_builder_format.clone())
                .unwrap_or_else(|| match self.format {
                    GameFormat::Classic => DeckFormat::Classic,
                    GameFormat::Operator => DeckFormat::Operator,
                });

            let collection = if self.stage == crate::types::CardGameStage::DraftBuilding {
                let drafted_cards = self
                    .draft
                    .as_ref()
                    .and_then(|draft| draft.drafted.get(player_id))
                    .cloned()
                    .unwrap_or_default();
                draft_template_entries(drafted_cards)
            } else {
                return Err(anyhow!("AI autobuild is only supported for draft decks"));
            };

            if collection.is_empty() {
                return Err(anyhow!("No cards available for AI autobuild"));
            }

            let (templates, operator_card_id, autobuild_colors) = match inferred_format {
                DeckFormat::Classic => {
                    let (templates, colors) = auto_build_draft_classic(&collection)?;
                    (templates, None, Some(colors))
                }
                DeckFormat::Operator => {
                    let (templates, operator_card_id) = auto_build_operator(&collection)?;
                    (templates, Some(operator_card_id), None)
                }
            };

            let mut operator = None;
            let cards = templates
                .into_iter()
                .filter_map(|entry| {
                    if operator_card_id.as_deref() == Some(entry.card.id.as_str()) {
                        operator = Some(entry.card);
                        None
                    } else {
                        Some(crate::state::DeckBuilderCard {
                            card: entry.card,
                            quantity: entry.quantity,
                        })
                    }
                })
                .collect::<Vec<_>>();

            if let Some(player) = self.players.get_mut(player_id) {
                player.deck_builder_active = true;
                player.deck_builder_cards = cards;
                player.deck_builder_operator_card_id = operator_card_id;
                player.deck_builder_operator = operator;
                player.deck_builder_autobuild_colors = autobuild_colors;
            }
            return Ok(vec![]);
        }

        let repo = self
            .repository
            .as_ref()
            .ok_or_else(|| anyhow!("Cardgame repository not initialized"))?
            .clone();
        let player_id_str = player_id.to_string();
        let deck_id = self
            .players
            .get(player_id)
            .and_then(|player| player.deck_builder_deck_id.clone())
            .ok_or_else(|| anyhow!("Open a deck in the builder first"))?;
        let inferred_format = self
            .players
            .get(player_id)
            .and_then(|player| player.deck_builder_format.clone())
            .unwrap_or_else(|| match self.format {
                GameFormat::Classic => DeckFormat::Classic,
                GameFormat::Operator => DeckFormat::Operator,
            });
        let draft_collection = if self.stage == crate::types::CardGameStage::DraftBuilding {
            let drafted_cards = self
                .draft
                .as_ref()
                .and_then(|draft| draft.drafted.get(player_id))
                .cloned()
                .unwrap_or_default();
            Some(draft_template_entries(drafted_cards))
        } else {
            None
        };

        let autobuild_colors = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            let deck_id = deck_id.clone();
            let player_id = player_id_str.clone();
            let draft_collection = draft_collection.clone();
            handle.block_on(async move {
                let deck = repo.get_deck(&deck_id).await?;
                if deck.owner_id.as_deref() != Some(&player_id) {
                    return Err(anyhow!("Deck does not belong to player"));
                }

                let is_draft_building = draft_collection.is_some();
                let collection = if let Some(draft_collection) = draft_collection {
                    draft_collection
                } else {
                    repo.load_owned_card_templates(&player_id).await?
                };
                if collection.is_empty() {
                    return Err(anyhow!("No cards available for autobuild"));
                }

                let (templates, operator_card_id, format, autobuild_colors) = match inferred_format
                {
                    DeckFormat::Classic => {
                        let (templates, autobuild_colors) = if is_draft_building {
                            auto_build_draft_classic(&collection)?
                        } else {
                            auto_build_classic(&collection)?
                        };
                        (templates, None, DeckFormat::Classic, Some(autobuild_colors))
                    }
                    DeckFormat::Operator => {
                        let (templates, operator_card_id) = auto_build_operator(&collection)?;
                        (
                            templates,
                            Some(operator_card_id),
                            DeckFormat::Operator,
                            None,
                        )
                    }
                };

                repo.set_deck_format(&deck_id, format).await?;
                repo.replace_deck_templates(&deck_id, &templates).await?;
                repo.set_deck_operator(&deck_id, operator_card_id.as_deref())
                    .await?;
                Ok::<Option<Vec<String>>, anyhow::Error>(autobuild_colors)
            })
        })?;

        if let Some(player) = self.players.get_mut(&player_id_str) {
            player.deck_builder_active = true;
            player.deck_builder_autobuild_colors = None;
        }
        self.refresh_deck_choices_for(&player_id_str);
        self.load_deck_builder_for_player(&player_id_str, &deck_id)?;
        if let Some(player) = self.players.get_mut(&player_id_str) {
            player.deck_builder_autobuild_colors = autobuild_colors;
        }
        Ok(vec![])
    }

    fn handle_browser_open(
        &mut self,
        player_id: &str,
        target_player_id: Option<String>,
    ) -> Result<Vec<GameEffect>> {
        tracing::info!(
            target: "cardgame::catalog",
            player_id,
            ?target_player_id,
            stage = ?self.stage,
            "handle_browser_open"
        );
        if !self.players.contains_key(player_id) {
            use crate::state::PlayerState;
            self.players.insert(
                player_id.to_string(),
                PlayerState::new(
                    player_id.to_string(),
                    "Browser".to_string(),
                    String::new(),
                    String::new(),
                    self.init.starting_life,
                ),
            );
        }
        if let Some(player) = self.players.get_mut(player_id) {
            player.card_browser_active = true;
            player.deck_builder_active = true;
            player.deck_builder_filter_kind = None;
            player.card_browser_source = None;
            player.card_browser_title = None;
            player.card_browser_subtitle = None;
        }

        // A target_player_id only makes sense once a game is in progress — in Lobby
        // the graveyard is empty and the user expects the catalog. Ignore target in
        // pre-game stages and fall through to catalog browsing.
        if let Some(target_player_id) = target_player_id {
            let game_in_progress = !matches!(
                self.stage,
                crate::types::CardGameStage::Lobby
                    | crate::types::CardGameStage::Draft
                    | crate::types::CardGameStage::DraftBuilding
                    | crate::types::CardGameStage::Awards
                    | crate::types::CardGameStage::Finished
            );
            if game_in_progress && self.players.contains_key(&target_player_id) {
                self.refresh_graveyard_browser_library(player_id, &target_player_id);
                return Ok(vec![]);
            }
            // Target player not in game, or pre-game — fall through to catalog mode
        }

        if self.stage == crate::types::CardGameStage::DraftBuilding {
            if let Some(player) = self.players.get_mut(player_id) {
                player.card_browser_source = Some(CardBrowserSource::DraftPool);
                player.card_browser_title = Some("Draft Pool".to_string());
                player.card_browser_subtitle = Some("Browse your drafted cards".to_string());
            }
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        if let Some(player) = self.players.get_mut(player_id) {
            player.card_browser_source = Some(CardBrowserSource::Catalog);
            player.card_browser_title = Some("Card Collection".to_string());
            player.card_browser_subtitle = Some("Mana Clash • Browse All Cards".to_string());
        }

        // Default: load full catalog (all cards with owned flag)
        self.load_catalog_page(player_id, 0);
        Ok(vec![])
    }

    fn handle_browser_close(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.card_browser_active = false;
            player.deck_builder_active = false;
            player.deck_builder_deck_id = None;
            player.deck_builder_name = None;
            player.deck_builder_description = None;
            player.deck_builder_format = None;
            player.deck_builder_operator_card_id = None;
            player.deck_builder_autobuild_colors = None;
            player.deck_builder_cards.clear();
            player.deck_builder_filter_kind = None;
            player.deck_builder_sort_order = None;
            player.library_search_results.clear();
            player.card_browser_source = None;
            player.card_browser_title = None;
            player.card_browser_subtitle = None;
            player.collection_page = 0;
            player.collection_total = 0;
        }
        Ok(vec![])
    }

    fn handle_browser_set_filter(
        &mut self,
        player_id: &str,
        kind: Option<String>,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_filter_kind =
                kind.map(|value| value.replace('|', ",").to_ascii_lowercase());
        }

        let browser_source = self
            .players
            .get(player_id)
            .and_then(|player| player.card_browser_source.clone());

        if let Some(CardBrowserSource::Graveyard { target_player_id }) = browser_source.clone() {
            self.refresh_graveyard_browser_library(player_id, &target_player_id);
            return Ok(vec![]);
        }

        if matches!(browser_source, Some(CardBrowserSource::DraftPool))
            || self.stage == crate::types::CardGameStage::DraftBuilding
        {
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        // Switch data source: owned filter → individual instances, otherwise → full catalog
        if self.browser_has_owned_filter(player_id) {
            self.load_collection_page(player_id, 0);
        } else {
            self.load_catalog_page(player_id, 0);
        }
        Ok(vec![])
    }

    fn handle_browser_set_sort(
        &mut self,
        player_id: &str,
        sort: Option<String>,
    ) -> Result<Vec<GameEffect>> {
        if let Some(player) = self.players.get_mut(player_id) {
            player.deck_builder_sort_order = sort;
        }

        let browser_source = self
            .players
            .get(player_id)
            .and_then(|player| player.card_browser_source.clone());

        if let Some(CardBrowserSource::Graveyard { target_player_id }) = browser_source.clone() {
            self.refresh_graveyard_browser_library(player_id, &target_player_id);
            return Ok(vec![]);
        }

        if matches!(browser_source, Some(CardBrowserSource::DraftPool))
            || self.stage == crate::types::CardGameStage::DraftBuilding
        {
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        // Reload from page 0 with the new sort applied at the DB level
        if self.browser_has_owned_filter(player_id) {
            self.load_collection_page(player_id, 0);
        } else {
            self.load_catalog_page(player_id, 0);
        }
        Ok(vec![])
    }

    fn handle_browser_page(&mut self, player_id: &str, delta: i32) -> Result<Vec<GameEffect>> {
        let browser_source = self
            .players
            .get(player_id)
            .and_then(|player| player.card_browser_source.clone());

        if let Some(CardBrowserSource::Graveyard { target_player_id }) = browser_source.clone() {
            self.refresh_graveyard_browser_library(player_id, &target_player_id);
            return Ok(vec![]);
        }

        if matches!(browser_source, Some(CardBrowserSource::DraftPool))
            || self.stage == crate::types::CardGameStage::DraftBuilding
        {
            self.refresh_draft_builder_library(player_id);
            return Ok(vec![]);
        }

        let current_page = self
            .players
            .get(player_id)
            .map(|p| p.collection_page)
            .unwrap_or(0);
        let total = self
            .players
            .get(player_id)
            .map(|p| p.collection_total)
            .unwrap_or(0);
        let max_page = ((total as f64) / COLLECTION_PAGE_SIZE as f64).ceil() as i32 - 1;
        let next_page = (current_page + delta).clamp(0, max_page.max(0));
        if next_page != current_page {
            if self.browser_has_owned_filter(player_id) {
                self.load_collection_page(player_id, next_page);
            } else {
                self.load_catalog_page(player_id, next_page);
            }
        }
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::draft_template_entries;
    use crate::card::CardBuilder;
    use crate::types::{CardKind, UnitType};

    #[test]
    fn draft_template_entries_group_duplicate_instances_by_template() {
        let mut first = CardBuilder::new(
            "spore_template",
            "Spore",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .build();
        first.id = "draft-instance-1".to_string();
        first.template_id = Some("spore_template".to_string());

        let mut second = CardBuilder::new(
            "spore_template",
            "Spore",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .build();
        second.id = "draft-instance-2".to_string();
        second.template_id = Some("spore_template".to_string());

        let entries = draft_template_entries(vec![first, second]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].card.id, "spore_template");
        assert_eq!(entries[0].quantity, 2);
    }
}
