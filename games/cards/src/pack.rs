use crate::card::Card;
use crate::repository::CardRepository;
use crate::types::{CardCategory, CardKind, CardRarity, Keyword, ManaCost, UnitType, VisualSpec};
use crate::user_card::UserCardInstance;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackType {
    pub pack_type_id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub slots: Vec<PackSlot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackSlot {
    pub slot_id: String,
    pub pack_type_id: String,
    pub slot_index: i32,
    pub weight_public: i32,
    pub weight_private: i32,
    pub weight_restricted: i32,
    pub weight_classified: i32,
    pub weight_singularity: i32,
}

impl PackSlot {
    pub fn rarity_weights(&self) -> Vec<(CardRarity, i32)> {
        vec![
            (CardRarity::Public, self.weight_public),
            (CardRarity::Private, self.weight_private),
            (CardRarity::Restricted, self.weight_restricted),
            (CardRarity::Classified, self.weight_classified),
            (CardRarity::Singularity, self.weight_singularity),
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackInstance {
    pub pack_instance_id: String,
    pub pack_type_id: String,
    pub owner_id: String,
    pub source: String,
    pub source_ref: Option<String>,
    pub opened: bool,
    pub opened_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackOpeningResult {
    pub result_id: String,
    pub pack_instance_id: String,
    pub slot_index: i32,
    pub instance_id: String,
    pub card_id: String,
    pub rarity: CardRarity,
    pub card_name: String,
    pub card_image_url: Option<String>,
    // Full card display data for inspect panels
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cost: ManaCost,
    #[serde(default)]
    pub kind_tag: String,
    #[serde(default)]
    pub kind_label: String,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default)]
    pub toughness: Option<i32>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<Keyword>,
    #[serde(default)]
    pub frame_style: String,
    #[serde(default)]
    pub categories: Vec<CardCategory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_spec: Option<VisualSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loyalty: Option<i32>,
}

pub(crate) fn pack_kind_tag(kind: &CardKind) -> &'static str {
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

pub(crate) fn pack_kind_label(kind: &CardKind) -> &'static str {
    match kind {
        CardKind::Unit(ctype) => match ctype {
            UnitType::None => "unit",
            UnitType::BioOrganism => "unit • bio-organism",
            UnitType::Human => "unit • human",
            UnitType::Megafauna => "unit • megafauna",
            UnitType::Anomaly => "unit • anomaly",
            UnitType::Drone => "unit • drone",
            UnitType::Virus => "unit • virus",
            UnitType::Hacker => "unit • hacker",
            UnitType::Node => "unit • node",
        },
        CardKind::Infrastructure => "infrastructure",
        CardKind::Relic => "relic",
        CardKind::Instant => "instant",
        CardKind::Ritual => "ritual",
        CardKind::Aura => "aura",
        CardKind::Agent => "agent",
    }
}

pub(crate) fn pack_frame_style(kind: &CardKind) -> &'static str {
    match kind {
        CardKind::Infrastructure => "infra_hex",
        CardKind::Relic => "artifact_cube",
        CardKind::Aura => "enchantment_card",
        _ => "card",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackOpenResult {
    pub pack_instance_id: String,
    pub cards: Vec<PackOpeningResult>,
}

/// Select a rarity based on weighted random distribution
fn select_rarity(slot: &PackSlot, rng: &mut impl Rng) -> Option<CardRarity> {
    let weights = slot.rarity_weights();
    let total: i32 = weights.iter().map(|(_, w)| w).sum();
    if total <= 0 {
        return None;
    }

    let mut roll = rng.random_range(0..total);
    for (rarity, weight) in &weights {
        if *weight <= 0 {
            continue;
        }
        roll -= weight;
        if roll < 0 {
            return Some(rarity.clone());
        }
    }

    // Fallback to last non-zero weight
    weights
        .iter()
        .rev()
        .find(|(_, w)| *w > 0)
        .map(|(r, _)| r.clone())
}

/// Rarity ordering for fallback (higher index = rarer)
fn rarity_order(rarity: &CardRarity) -> usize {
    match rarity {
        CardRarity::Public => 0,
        CardRarity::Private => 1,
        CardRarity::Restricted => 2,
        CardRarity::Classified => 3,
        CardRarity::Singularity => 4,
    }
}

/// Get all rarities in order from most common to rarest
fn all_rarities_ordered() -> Vec<CardRarity> {
    vec![
        CardRarity::Public,
        CardRarity::Private,
        CardRarity::Restricted,
        CardRarity::Classified,
        CardRarity::Singularity,
    ]
}

/// Generate a purely random wear value between 0.0 and 1.0
fn random_wear(rng: &mut impl Rng) -> f32 {
    let base: f32 = rng.random_range(0.0..1.0_f32);
    (base * 100.0).round() / 100.0
}

/// Generate a draft pool with at least `target_cards` cards by simulating
/// enough draft-pack openings.
/// Returns ephemeral instance-style Card objects (unique `id`, canonical
/// `template_id`) without persisting user_card_instances.
/// Duplicate templates are allowed across packs but not within a single pack.
pub async fn generate_draft_pool(repo: &CardRepository, target_cards: usize) -> Result<Vec<Card>> {
    let pack_types = repo.list_pack_types().await?;
    let draft_pack = pack_types
        .iter()
        .find(|pt| pt.pack_type_id == "draft-pack")
        .or_else(|| {
            pack_types
                .iter()
                .find(|pt| pt.name.to_lowercase().contains("draft"))
        })
        .ok_or_else(|| anyhow!("Draft pack type not found"))?;

    if draft_pack.slots.is_empty() {
        return Err(anyhow!("Draft pack has no slots configured"));
    }

    // Pre-fetch all cards by rarity in one pass to avoid N+1 queries per slot
    let all_rarities = all_rarities_ordered();
    let mut cards_by_rarity: std::collections::HashMap<CardRarity, Vec<Card>> =
        std::collections::HashMap::new();
    for rarity in &all_rarities {
        let cards = repo.get_cards_by_rarity(rarity).await?;
        let non_infra: Vec<Card> = cards
            .into_iter()
            .filter(|c| !matches!(c.kind, CardKind::Infrastructure))
            .collect();
        cards_by_rarity.insert(rarity.clone(), non_infra);
    }

    let mut rng = rand::rng();
    let mut pool = Vec::new();
    let source_pack_count = target_cards.div_ceil(draft_pack.slots.len());

    for _ in 0..source_pack_count {
        let mut used_card_ids_this_pack = std::collections::HashSet::new();

        for slot in &draft_pack.slots {
            let selected_rarity = select_rarity(slot, &mut rng)
                .ok_or_else(|| anyhow!("Slot has no rarity weights"))?;

            let order = rarity_order(&selected_rarity);
            let all = all_rarities_ordered();
            let mut candidates: Vec<Card> = Vec::new();

            // Try the selected rarity first, then fall back downward
            for try_rarity in all.iter().take(order + 1).rev() {
                if let Some(cards) = cards_by_rarity.get(try_rarity) {
                    candidates = cards
                        .iter()
                        .filter(|c| !used_card_ids_this_pack.contains(&c.id))
                        .cloned()
                        .collect();
                    if !candidates.is_empty() {
                        break;
                    }
                }
            }

            // If still empty, try higher rarities
            if candidates.is_empty() {
                for try_rarity in all.iter().skip(order + 1) {
                    if let Some(cards) = cards_by_rarity.get(try_rarity) {
                        candidates = cards
                            .iter()
                            .filter(|c| !used_card_ids_this_pack.contains(&c.id))
                            .cloned()
                            .collect();
                        if !candidates.is_empty() {
                            break;
                        }
                    }
                }
            }

            // If still empty after all rarities, allow duplicates within the pack
            if candidates.is_empty() {
                let selected_rarity_dup = select_rarity(slot, &mut rng)
                    .ok_or_else(|| anyhow!("Slot has no rarity weights"))?;
                if let Some(cards) = cards_by_rarity.get(&selected_rarity_dup) {
                    candidates = cards.clone();
                }
            }
            if candidates.is_empty() {
                continue;
            }

            let mut card = candidates[rng.random_range(0..candidates.len())].clone();
            let template_id = card.id.clone();
            used_card_ids_this_pack.insert(template_id.clone());
            card.template_id = Some(template_id);
            card.id = Ulid::new().to_string();
            // Give each draft card a unique visual spec (seed/wear/holo)
            card.visual_spec = Some(VisualSpec {
                skin_id: "default".to_string(),
                seed: rng.random::<i32>(),
                wear: random_wear(&mut rng),
                holo: rng.random_range(0..100) == 0,
            });
            pool.push(card);
        }
    }

    // Shuffle the entire pool
    use rand::seq::SliceRandom;
    pool.shuffle(&mut rng);
    pool.truncate(target_cards);
    Ok(pool)
}

/// Open a pack: select cards for each slot, create instances, record results
pub async fn open_pack(
    repo: &CardRepository,
    pack_instance_id: &str,
    owner_id: &str,
) -> Result<PackOpenResult> {
    // Load pack instance
    let pack = repo
        .get_pack_instance(pack_instance_id)
        .await?
        .ok_or_else(|| anyhow!("Pack not found"))?;

    if pack.opened {
        return Err(anyhow!("Pack already opened"));
    }

    if pack.owner_id != owner_id {
        return Err(anyhow!("You don't own this pack"));
    }

    // Load pack type with slots
    let pack_type = repo
        .get_pack_type(&pack.pack_type_id)
        .await?
        .ok_or_else(|| anyhow!("Pack type not found"))?;

    if pack_type.slots.is_empty() {
        return Err(anyhow!("Pack type has no slots configured"));
    }

    let mut rng = rand::rng();
    let mut results = Vec::new();
    let mut used_card_ids = std::collections::HashSet::new();

    for slot in &pack_type.slots {
        let selected_rarity = select_rarity(slot, &mut rng)
            .ok_or_else(|| anyhow!("Slot {} has no rarity weights", slot.slot_index))?;

        // Try selected rarity, then fall back to lower rarities
        let order = rarity_order(&selected_rarity);
        let all = all_rarities_ordered();
        let mut candidates: Vec<Card> = Vec::new();

        // Try the selected rarity first, then fall back downward
        for try_rarity in all.iter().take(order + 1).rev() {
            let cards = repo.get_cards_by_rarity(try_rarity).await?;
            candidates = cards
                .into_iter()
                .filter(|c| !used_card_ids.contains(&c.id))
                .collect();
            if !candidates.is_empty() {
                break;
            }
        }

        // If still empty, try higher rarities
        if candidates.is_empty() {
            for try_rarity in all.iter().skip(order + 1) {
                let cards = repo.get_cards_by_rarity(try_rarity).await?;
                candidates = cards
                    .into_iter()
                    .filter(|c| !used_card_ids.contains(&c.id))
                    .collect();
                if !candidates.is_empty() {
                    break;
                }
            }
        }

        if candidates.is_empty() {
            return Err(anyhow!("No cards available for slot {}", slot.slot_index));
        }

        let card = &candidates[rng.random_range(0..candidates.len())];
        used_card_ids.insert(card.id.clone());

        let instance_id = Ulid::new().to_string();
        let visual_spec = VisualSpec {
            skin_id: "default".to_string(),
            seed: rng.random::<i32>(),
            wear: random_wear(&mut rng),
            holo: rng.random_range(0..100) == 0, // 1% chance
        };

        let result_visual_spec = visual_spec.clone();
        let card_instance = UserCardInstance {
            instance_id: instance_id.clone(),
            card_id: card.id.clone(),
            owner_id: owner_id.to_string(),
            visual_spec,
            is_tradeable: true,
            is_bound: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        repo.create_user_card_instance(
            &card_instance,
            owner_id,
            Some(serde_json::json!({
                "source": "pack_opening",
                "pack_instance_id": pack_instance_id,
            })),
        )
        .await?;

        let result = PackOpeningResult {
            result_id: Ulid::new().to_string(),
            pack_instance_id: pack_instance_id.to_string(),
            slot_index: slot.slot_index,
            instance_id: instance_id.clone(),
            card_id: card.id.clone(),
            rarity: selected_rarity.clone(),
            card_name: card.name.clone(),
            card_image_url: card.image_url.clone(),
            description: card.description.clone(),
            cost: card.cost.clone(),
            kind_tag: pack_kind_tag(&card.kind).to_string(),
            kind_label: pack_kind_label(&card.kind).to_string(),
            power: card.power,
            toughness: card.toughness,
            colors: card.colors.clone(),
            keywords: card.keywords.clone(),
            frame_style: pack_frame_style(&card.kind).to_string(),
            categories: card.categories.clone(),
            visual_spec: Some(result_visual_spec),
            loyalty: card.loyalty,
        };

        repo.create_pack_opening_result(&result).await?;
        results.push(result);
    }

    // Mark pack as opened
    repo.mark_pack_opened(pack_instance_id).await?;

    // Audit log
    repo.create_pack_audit(
        pack_instance_id,
        "opened",
        owner_id,
        Some(serde_json::json!({
            "cards_received": results.len(),
        })),
    )
    .await?;

    Ok(PackOpenResult {
        pack_instance_id: pack_instance_id.to_string(),
        cards: results,
    })
}
