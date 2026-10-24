//! Mana management and cost utility functions

use crate::card::*;
use crate::mana::ManaPool;
use crate::state::*;
use crate::types::*;

/// Extracts AddMana info from an effect, handling Sequence wrapping
fn extract_add_mana(effect: &CardEffect) -> Option<(ManaType, i32)> {
    match effect {
        CardEffect::AddMana(mana_type, amount) => Some((mana_type.clone(), *amount)),
        CardEffect::Sequence(effects) => effects.iter().find_map(|e| {
            if let CardEffect::AddMana(mana_type, amount) = e {
                Some((mana_type.clone(), *amount))
            } else {
                None
            }
        }),
        _ => None,
    }
}

/// Checks if a mana cost contains at least one mana of the specified type
pub fn mana_cost_matches_filter(cost: &ManaCost, filter: &str) -> bool {
    match filter {
        "neon" => cost.neon > 0,
        "glitch" => cost.glitch > 0,
        "chrome" => cost.chrome > 0,
        "slag" => cost.slag > 0,
        "bio" => cost.bio > 0,
        "colorless" => cost.colorless > 0,
        _ => false,
    }
}

/// Checks if a mana cost matches any of the specified filters
pub fn mana_cost_matches_any_filter(cost: &ManaCost, filters: &[&str]) -> bool {
    filters
        .iter()
        .any(|filter| mana_cost_matches_filter(cost, filter))
}

/// Helper function to add mana to a pool with optional scaling
/// Set CARDGAME_BIO_10X=true to enable 10x Bio mana scaling (testing only)
pub fn add_mana_to_pool(pool: &mut ManaPool, mana_type: &ManaType, amount: i32) {
    let bio_10x = std::env::var("CARDGAME_BIO_10X")
        .unwrap_or_default()
        .to_lowercase()
        == "true";

    let scaled = if matches!(mana_type, ManaType::Bio) && bio_10x {
        amount * 10
    } else {
        amount
    };

    match mana_type {
        ManaType::Neon => pool.neon += scaled,
        ManaType::Glitch => pool.glitch += scaled,
        ManaType::Chrome => pool.chrome += scaled,
        ManaType::Slag => pool.slag += scaled,
        ManaType::Bio => pool.bio += scaled,
        ManaType::Colorless => pool.colorless += scaled,
    }
}

/// Mana management functions
impl CardGameState {
    pub(crate) fn scaled_mana_amount(mana_type: &ManaType, amount: i32) -> i32 {
        let bio_10x = std::env::var("CARDGAME_BIO_10X")
            .unwrap_or_default()
            .to_lowercase()
            == "true";

        if matches!(mana_type, ManaType::Bio) && bio_10x {
            amount * 10
        } else {
            amount
        }
    }

    pub fn available_mana_for_player(&self, player_id: &str) -> ManaPool {
        let Some(player) = self.players.get(player_id) else {
            return ManaPool::default();
        };
        // Start from the current mana pool (what the player can actually
        // spend right now) and add what each UNTAPPED tap-for-mana source
        // could still contribute if tapped.
        let mut total = player.mana_pool.clone();

        for card in &player.battlefield {
            if card.tapped {
                continue;
            }
            // Skip non-infrastructure cards that are still Compiling
            if !matches!(card.kind, CardKind::Infrastructure)
                && card.summoning_sickness
                && !card.haste
                && card.keywords.contains(&Keyword::Compiling)
            {
                continue;
            }
            for ability in &card.abilities {
                if ability.cost.has_tap() && ability.cost.is_empty_except_tap() {
                    if let Some((mana_type, amount)) = extract_add_mana(&ability.effect) {
                        add_mana_to_pool(&mut total, &mana_type, amount);
                        break;
                    }
                }
            }
        }

        total
    }

    /// Sums Firewall(N) amounts from cards targeted by an opponent's spell
    /// or ability. Each targeted Firewall-bearing card adds its N to the
    /// caster's mana cost. Self-targets do not incur the tax. Also counts
    /// Firewall granted via StaticKeywordGrant from other battlefield cards.
    pub fn firewall_tax_for_targets(&self, caster_id: &str, targets: &[TargetRef]) -> i32 {
        let mut tax = 0i32;
        for target in targets {
            let card_id = match target {
                TargetRef::Card(id) => id,
                _ => continue,
            };
            for (owner_id, player) in &self.players {
                if owner_id == caster_id {
                    continue;
                }
                let Some(card) = player.battlefield.iter().find(|c| &c.id == card_id) else {
                    continue;
                };
                // Base-printed Firewall
                for keyword in &card.keywords {
                    if let Keyword::Firewall(amount) = keyword {
                        tax += *amount;
                    }
                }
                // Granted Firewall from StaticKeywordGrant on other battlefield cards
                let card_unit_type = match &card.kind {
                    CardKind::Unit(ut) => Some(ut.clone()),
                    _ => None,
                };
                for granter in &player.battlefield {
                    for keyword in &granter.keywords {
                        if let Keyword::StaticKeywordGrant(inner, target_spec) = keyword {
                            if let Keyword::Firewall(amount) = inner.as_ref() {
                                if target_spec.matches(owner_id, owner_id, card_unit_type.as_ref())
                                {
                                    tax += *amount;
                                }
                            }
                        }
                    }
                }
            }
        }
        tax
    }

    pub fn calculate_effective_cost(&self, player_id: &str, card: &Card) -> ManaCost {
        let mut total_cost = card.cost.clone();

        // Apply Operator Tax
        if let Some(player) = self.players.get(player_id) {
            if player.operator_zone.iter().any(|c| c.id == card.id) {
                total_cost.colorless += (player.operator_cast_count as i32) * 2;
            }
        }

        // Apply CostReduction keyword from own permanents
        if let Some(player) = self.players.get(player_id) {
            for card_in_play in &player.battlefield {
                for keyword in &card_in_play.keywords {
                    if let Keyword::CostReduction(amount, kind_filter) = keyword {
                        let applies = match kind_filter {
                            None => true,
                            Some(filter_kind) => card.kind == *filter_kind,
                        };
                        if applies {
                            total_cost.colorless = (total_cost.colorless - amount).max(0);
                        }
                    }
                }
            }
        }

        // Apply Throttle keyword from opponents
        for (pid, player) in &self.players {
            if pid != player_id {
                for card_in_play in &player.battlefield {
                    for keyword in &card_in_play.keywords {
                        if let Keyword::Throttle(amount) = keyword {
                            total_cost.colorless += amount;
                        }
                    }
                }
            }
        }
        total_cost
    }

    pub fn refresh_mana(player: &mut PlayerState) {
        player.mana_pool = ManaPool::default();
    }

    pub fn fill_mana_from_grid(player: &mut PlayerState) {
        for node in &player.grid {
            add_mana_to_pool(&mut player.mana_pool, &node.mana_type, 1);
        }
    }

    pub fn fill_mana_from_infrastructure(player: &mut PlayerState) {
        // Pre-tap each infrastructure as we pour its output into the pool.
        // Without this, `available_mana_for_player` later computes
        // max(untapped_infra_output, mana_pool) and ends up double-counting
        // the same infra mana — the player appears to have unlimited mana.
        // Callers must ensure this runs AFTER the start-of-turn untap step,
        // otherwise the subsequent untap will undo this and re-create the bug.
        for card in player.battlefield.iter_mut() {
            if !matches!(card.kind, CardKind::Infrastructure) {
                continue;
            }
            let mut produced = false;
            for ability in &card.abilities {
                if ability.cost.has_tap() {
                    if let CardEffect::AddMana(ref mana_type, amount) = ability.effect {
                        add_mana_to_pool(&mut player.mana_pool, mana_type, amount);
                        produced = true;
                    }
                }
            }
            if produced {
                card.tapped = true;
            }
        }
    }

    pub fn auto_tap_for_mana(player: &mut PlayerState, required: i32) {
        if required <= 0 {
            return;
        }
        let mut gained = 0;
        for card in player.battlefield.iter_mut() {
            if gained >= required {
                break;
            }
            if card.tapped || !matches!(card.kind, CardKind::Infrastructure) {
                continue;
            }
            let mana_gain = card
                .abilities
                .iter()
                .find(|a| a.cost.has_tap() && a.cost.is_empty_except_tap())
                .and_then(|a| match &a.effect {
                    CardEffect::AddMana(_, amount) => Some(*amount),
                    _ => None,
                })
                .unwrap_or(1);
            card.tapped = true;
            gained += mana_gain;
        }
        if gained > 0 {
            player.mana_pool.colorless += gained;
        }
    }

    pub fn auto_tap_mana(&mut self, player_id: &str, cost: &ManaCost) -> bool {
        // Skip mana check in testing mode
        let bio_10x = std::env::var("CARDGAME_BIO_10X")
            .unwrap_or_default()
            .to_lowercase()
            == "true";
        if bio_10x {
            return true;
        }

        if self.ruleset == crate::types::Ruleset::Syndicate {
            return self
                .players
                .get_mut(player_id)
                .map(|p| crate::syndicate::pay(p, cost))
                .unwrap_or(false);
        }

        let available = self.available_mana_for_player(player_id);
        if !available.can_afford(cost) {
            return false;
        }

        let Some(player) = self.players.get_mut(player_id) else {
            return false;
        };

        let mut remaining = cost.clone();

        // 1. Spend from mana pool first (filled from Grid at turn start)
        player.mana_pool.spend_available(&mut remaining);

        // 2. Tap battlefield cards with tap-for-mana abilities for specific colors
        for card in player.battlefield.iter_mut() {
            if remaining.total() == 0 {
                break;
            }
            if card.tapped {
                continue;
            }
            // Skip Compiling cards on the turn they were deployed
            if card.summoning_sickness && !card.haste && card.keywords.contains(&Keyword::Compiling)
            {
                continue;
            }

            if let Some(ability) = card
                .abilities
                .iter()
                .find(|a| a.cost.has_tap() && a.cost.is_empty_except_tap())
            {
                if let Some((mana_type, amount)) = extract_add_mana(&ability.effect) {
                    let scaled = Self::scaled_mana_amount(&mana_type, amount);
                    match mana_type {
                        ManaType::Neon if remaining.neon > 0 => {
                            card.tapped = true;
                            remaining.neon = (remaining.neon - scaled).max(0);
                        }
                        ManaType::Glitch if remaining.glitch > 0 => {
                            card.tapped = true;
                            remaining.glitch = (remaining.glitch - scaled).max(0);
                        }
                        ManaType::Chrome if remaining.chrome > 0 => {
                            card.tapped = true;
                            remaining.chrome = (remaining.chrome - scaled).max(0);
                        }
                        ManaType::Slag if remaining.slag > 0 => {
                            card.tapped = true;
                            remaining.slag = (remaining.slag - scaled).max(0);
                        }
                        ManaType::Bio if remaining.bio > 0 => {
                            card.tapped = true;
                            remaining.bio = (remaining.bio - scaled).max(0);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 3. Use excess colored mana for colorless costs
        let excess = (-remaining.neon).max(0)
            + (-remaining.glitch).max(0)
            + (-remaining.chrome).max(0)
            + (-remaining.slag).max(0)
            + (-remaining.bio).max(0);
        remaining.colorless -= excess;

        // 4. Tap remaining cards for colorless
        for card in player.battlefield.iter_mut() {
            if remaining.colorless <= 0 {
                break;
            }
            if card.tapped {
                continue;
            }
            // Skip Compiling cards on the turn they were deployed
            if card.summoning_sickness && !card.haste && card.keywords.contains(&Keyword::Compiling)
            {
                continue;
            }

            if let Some(ability) = card
                .abilities
                .iter()
                .find(|a| a.cost.has_tap() && a.cost.is_empty_except_tap())
            {
                if let Some((mana_type, amount)) = extract_add_mana(&ability.effect) {
                    let scaled = Self::scaled_mana_amount(&mana_type, amount);
                    card.tapped = true;
                    remaining.colorless -= scaled;
                }
            }
        }

        // 5. Return over-tapped mana to pool
        if remaining.neon < 0 {
            player.mana_pool.neon += -remaining.neon;
        }
        if remaining.glitch < 0 {
            player.mana_pool.glitch += -remaining.glitch;
        }
        if remaining.chrome < 0 {
            player.mana_pool.chrome += -remaining.chrome;
        }
        if remaining.slag < 0 {
            player.mana_pool.slag += -remaining.slag;
        }
        if remaining.bio < 0 {
            player.mana_pool.bio += -remaining.bio;
        }
        if remaining.colorless < 0 {
            player.mana_pool.colorless += -remaining.colorless;
        }

        remaining.neon <= 0
            && remaining.glitch <= 0
            && remaining.chrome <= 0
            && remaining.slag <= 0
            && remaining.bio <= 0
            && remaining.colorless <= 0
    }
}
