use crate::card::*;
use crate::state::*;
use crate::types::*;
use anyhow::{Result, anyhow};

impl CardGameState {
    pub const DEFAULT_DECK_ID: &'static str = "bio";

    pub fn can_play_infrastructure(&self, player_id: &str, card: &Card) -> bool {
        if !self.is_active_player(player_id) {
            return false;
        }
        let Some(player) = self.players.get(player_id) else {
            return false;
        };
        if self.stage != CardGameStage::Main && self.stage != CardGameStage::PostCombatMain {
            return false;
        }
        // Sorcery-speed: can't play an infrastructure while a spell or
        // showcase is pending. Mirrors `handle_play_land`'s server-side gate
        // so the UI doesn't tease an action the server will reject.
        if !self.stack.is_empty() || self.showcase.is_some() {
            return false;
        }
        if card.kind != CardKind::Infrastructure {
            return false;
        }

        // Calculate max infrastructures allowed per turn (base 1 + ExtraInfrastructures from units)
        let max_infrastructures = 1 + player
            .battlefield
            .iter()
            .filter_map(|card| {
                card.keywords.iter().find_map(|kw| {
                    if let Keyword::ExtraInfrastructures(count) = kw {
                        Some(*count)
                    } else {
                        None
                    }
                })
            })
            .sum::<u8>();

        if player.infrastructures_played_this_turn >= max_infrastructures {
            return false;
        }

        if self.ruleset == crate::types::Ruleset::Syndicate
            && !crate::syndicate::can_pay(player, &card.cost)
        {
            return false;
        }

        // Must be in hand
        player.hand.iter().any(|c| c.id == card.id)
    }

    pub fn can_cast_card(&self, player_id: &str, card: &Card) -> bool {
        // 1. Infrastructure uses can_play_land instead
        if matches!(card.kind, CardKind::Infrastructure) {
            return false;
        }

        // 2. Player must exist
        if !self.players.contains_key(player_id) {
            return false;
        }

        // 3. Determine if this is instant-speed
        let has_flash = card.keywords.contains(&Keyword::Flash);
        let is_instant_speed = has_flash || matches!(card.kind, CardKind::Instant);

        // 4. Check timing restrictions
        let is_active = self.is_active_player(player_id);
        let is_main_phase =
            self.stage == CardGameStage::Main || self.stage == CardGameStage::PostCombatMain;

        if !is_instant_speed {
            // Sorcery-speed: must be active player during main phase
            // with nothing pending resolution. Showcase is the
            // pre-resolve display window for a unit you just played,
            // and it blocks further casts just like a non-empty stack
            // would — without this the hand shows `playable: true` for
            // cards you actually can't cast yet.
            if !is_active || !is_main_phase {
                return false;
            }
            if !self.stack.is_empty() || self.showcase.is_some() {
                return false;
            }
        } else {
            // Instant-speed: requires holding priority
            if self.effective_priority_holder() != Some(player_id) {
                return false;
            }
        }

        // 5. Testing mode bypass
        if std::env::var("CARDGAME_BIO_10X")
            .unwrap_or_default()
            .to_lowercase()
            == "true"
        {
            return true;
        }

        // 6. Check affordability (cash with credit under Syndicate, mana otherwise)
        let effective_cost = self.calculate_effective_cost(player_id, card);
        if self.ruleset == crate::types::Ruleset::Syndicate {
            let Some(player) = self.players.get(player_id) else {
                return false;
            };
            if !crate::syndicate::can_pay(player, &effective_cost) {
                return false;
            }
        } else {
            let available_mana = self.available_mana_for_player(player_id);
            if !available_mana.can_afford(&effective_cost) {
                return false;
            }
        }

        // 7. Check that valid targets exist if the spell requires targeting
        // Unit cards handle targeting at ETB time, not cast time
        // Exception: RememberTargetPlayer still needs cast-time targeting
        let spell_target_spec = if matches!(card.kind, CardKind::Unit(_)) {
            card.abilities
                .iter()
                .filter(|a| {
                    a.is_spell_effect_ability()
                        && matches!(a.effect, CardEffect::RememberTargetPlayer)
                })
                .find_map(|a| a.target_spec.as_ref())
        } else {
            card.abilities
                .iter()
                .filter(|a| a.is_spell_effect_ability())
                .find_map(|a| a.target_spec.as_ref())
        };

        if let Some(spec) = spell_target_spec {
            if !self.has_valid_targets(player_id, spec) {
                return false;
            }
        }

        true
    }

    fn has_valid_targets(&self, caster_id: &str, spec: &TargetSpec) -> bool {
        let needs_unit = matches!(
            spec.kind,
            TargetKind::Unit | TargetKind::Card | TargetKind::PlayerOrUnit
        );
        let needs_player = matches!(spec.kind, TargetKind::Player | TargetKind::PlayerOrUnit);

        if needs_player {
            // There's always at least one valid player target
            return true;
        }

        if needs_unit {
            for (pid, player) in &self.players {
                let is_own = pid == caster_id;
                let scope_matches = match spec.scope {
                    TargetScope::SelfPlayer => is_own,
                    TargetScope::Opponent => !is_own,
                    TargetScope::Any | TargetScope::AnyPlayerOrUnit => true,
                };
                if scope_matches {
                    let has_unit = player
                        .battlefield
                        .iter()
                        .any(|c| matches!(c.kind, CardKind::Unit(_) | CardKind::Agent));
                    if has_unit {
                        return true;
                    }
                }
            }
            return false;
        }

        true
    }

    pub fn can_tap_card(&self, player_id: &str, card: &Card) -> bool {
        if !self.is_active_player(player_id) {
            return false;
        }
        if card.tapped {
            return false;
        }
        // Compiling: a freshly deployed unit with this keyword can't tap for
        // abilities its first turn (Haste bypasses Compiling, since Haste
        // implies "boot bypass").
        if card.summoning_sickness && !card.haste && card.keywords.contains(&Keyword::Compiling) {
            return false;
        }
        if self.modifier_amount_for_card(&card.id, StatType::Armor) == -999 {
            // Placeholder for Freeze
            return false;
        }
        card.abilities.iter().any(|a| a.cost.has_tap())
    }

    /// Returns None if the ability can be activated, or Some(reason) if it cannot.
    pub fn can_activate_ability(
        &self,
        owner_id: &str,
        card: &Card,
        ability: &CardAbility,
    ) -> Option<String> {
        if card.keywords.contains(&Keyword::Glitch) {
            return Some("Card has Glitch".into());
        }

        // SignalJam check
        for (pid, player) in &self.players {
            if pid != owner_id {
                for card_in_play in &player.battlefield {
                    if card_in_play.keywords.contains(&Keyword::SignalJam) {
                        return Some("Blocked by Signal Jam".into());
                    }
                }
            }
        }

        // Skip mana check in testing mode
        let bio_10x = std::env::var("CARDGAME_BIO_10X")
            .unwrap_or_default()
            .to_lowercase()
            == "true";
        if !bio_10x {
            if self.ruleset == crate::types::Ruleset::Syndicate {
                let short = self
                    .players
                    .get(owner_id)
                    .map(|p| !crate::syndicate::can_pay(p, &ability.cost.get_mana()))
                    .unwrap_or(true);
                if short {
                    return Some("Not enough cash".into());
                }
            } else {
                let available_mana = self.available_mana_for_player(owner_id);
                if !available_mana.can_afford(&ability.cost.get_mana()) {
                    return Some("Not enough mana".into());
                }
            }
        }
        if ability.cost.has_tap() && card.tapped {
            return Some("Card is tapped".into());
        }
        if ability.cost.has_tap()
            && card.summoning_sickness
            && !card.haste
            && card.keywords.contains(&Keyword::Compiling)
        {
            return Some("Compiling — abilities unavailable this turn".into());
        }
        // Spend Charge cost check
        let spend_charge = ability.cost.get_spend_charge();
        if spend_charge > 0 && card.counters < spend_charge {
            return Some("Not enough Charge".into());
        }
        // Sacrifice cost check
        let sacrifice_count = ability.cost.get_sacrifice_count();
        if sacrifice_count > 0 {
            let available = self
                .players
                .get(owner_id)
                .map(|p| {
                    p.battlefield
                        .iter()
                        .filter(|c| c.id != card.id && matches!(c.kind, CardKind::Unit(_)))
                        .count()
                })
                .unwrap_or(0);
            if available < sacrifice_count {
                return Some("Not enough units to sacrifice".into());
            }
        }
        // Agent loyalty check: ensure enough loyalty for negative costs
        if matches!(card.kind, CardKind::Agent) {
            if let Some(loyalty) = card.loyalty {
                let loyalty_cost = Self::extract_loyalty_cost(&ability.effect);
                if loyalty_cost < 0 && loyalty < loyalty_cost.unsigned_abs() as i32 {
                    return Some("Not enough loyalty".into());
                }
            }
        }

        // Agent: only one loyalty ability per turn (across all abilities on this card)
        if matches!(card.kind, CardKind::Agent) && ability.once_per_turn {
            let pw_key = format!("agent::{}", card.id);
            if self
                .triggers_played_this_turn
                .get(&pw_key)
                .copied()
                .unwrap_or(0)
                > 0
            {
                return Some("Already used a loyalty ability this turn".into());
            }
        }

        if ability.once_per_turn {
            let key = format!("ability::{}::{}", card.id, ability.id);
            if self
                .triggers_played_this_turn
                .get(&key)
                .copied()
                .unwrap_or(0)
                > 0
            {
                return Some("Already used this turn".into());
            }
        }
        if let Some(predicate) = &ability.predicate {
            if !self.evaluate_predicate_for_ui(owner_id, Some(&card.id), &[], predicate) {
                return Some("Condition not met".into());
            }
            if Self::predicate_contains_once_per_turn(predicate) {
                let key = Self::predicate_key(&ability.id, "ability");
                if self
                    .triggers_played_this_turn
                    .get(&key)
                    .copied()
                    .unwrap_or(0)
                    > 0
                {
                    return Some("Already used this turn".into());
                }
            }
        }
        None
    }

    /// Extracts the loyalty cost from an ability's effect (e.g., ModifyLoyalty(-3) returns -3)
    fn extract_loyalty_cost(effect: &CardEffect) -> i32 {
        match effect {
            CardEffect::ModifyLoyalty(amount) => *amount,
            CardEffect::Sequence(effects) => effects
                .iter()
                .find_map(|e| {
                    if let CardEffect::ModifyLoyalty(amount) = e {
                        Some(*amount)
                    } else {
                        None
                    }
                })
                .unwrap_or(0),
            _ => 0,
        }
    }

    pub fn attach_equipment(
        &mut self,
        owner_id: &str,
        equipment_id: &str,
        target_id: &str,
    ) -> Result<()> {
        let Some((target_owner_id, target_kind)) = self.find_battlefield_card_owner(target_id)
        else {
            return Err(anyhow!("Invalid target"));
        };
        if target_owner_id != owner_id {
            return Err(anyhow!("Invalid target"));
        }
        if !matches!(target_kind, CardKind::Unit(_)) {
            return Err(anyhow!("Invalid target"));
        }

        let (equipment_name, equipment_triggers) = {
            let Some(owner) = self.players.get_mut(owner_id) else {
                return Err(anyhow!("Invalid owner"));
            };
            let Some(equipment) = owner
                .battlefield
                .iter_mut()
                .find(|card| card.id == equipment_id)
            else {
                return Err(anyhow!("Equipment not on battlefield"));
            };
            if !matches!(equipment.kind, CardKind::Relic) {
                return Err(anyhow!("Not an equipment"));
            }
            equipment.attached_to = Some(target_id.to_string());
            (equipment.name.clone(), equipment.triggers.clone())
        };

        // Fire the Equipped trigger on the equipment so "When equipped, ..."
        // abilities resolve. Effects on the attached unit (stat buffs via
        // GrantKeyword / ApplyStatModifier) are handled elsewhere via the
        // attachment resolver — this is just the one-shot ETB-style trigger.
        self.apply_triggers_for_event(
            owner_id,
            equipment_id,
            Some(&equipment_name),
            &equipment_triggers,
            &TriggerEvent::Equipped,
            &[TargetRef::Card(target_id.to_string())],
        );
        Ok(())
    }

    pub fn resolve_attachment(&mut self, owner_id: &str, mut card: Card, targets: &[TargetRef]) {
        let Some(TargetRef::Card(target_id)) = targets.first() else {
            return;
        };
        let Some((_target_owner, target_kind)) = self.find_battlefield_card_owner(target_id) else {
            return;
        };
        if !matches!(target_kind, CardKind::Unit(_)) {
            return;
        }
        card.attached_to = Some(target_id.clone());
        if let Some(owner) = self.players.get_mut(owner_id) {
            owner.battlefield.push(card);
            let entered = owner.battlefield.last().cloned();
            if let Some(card) = entered {
                self.handle_card_played(owner_id, &card, targets);
            }
        }
    }
}
