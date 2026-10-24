//! Combat action handlers (attack, block, combat resolution)

use crate::card::CardEffect;
use crate::state::CardGameState;
use crate::types::{
    CardGameStage, CardKind, CombatPhase, GameFormat, Keyword, TargetRef, TriggerEvent,
};
use anyhow::{Result, anyhow};
use game_framework::GameEffect;
use serde_json::json;
use std::collections::{HashMap, HashSet};

pub trait CombatHandlers {
    fn handle_declare_attack(
        &mut self,
        player_id: &str,
        card_id: String,
        target: Option<TargetRef>,
    ) -> Result<Vec<GameEffect>>;
    fn handle_declare_block(
        &mut self,
        player_id: &str,
        attacker_id: String,
        blocker_id: String,
    ) -> Result<Vec<GameEffect>>;
    fn handle_next_stage(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_confirm_attackers(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_confirm_blockers(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
}

impl CombatHandlers for CardGameState {
    fn handle_declare_attack(
        &mut self,
        player_id: &str,
        card_id: String,
        target: Option<TargetRef>,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Combat {
            return Err(anyhow!("Attacks can only be declared in combat"));
        }
        if self.combat_phase != CombatPhase::DeclareAttackers {
            return Err(anyhow!(
                "Attacks can only be declared during the declare attackers phase"
            ));
        }
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can attack"));
        }
        if !self.stack.is_empty() || self.showcase.is_some() {
            return Err(anyhow!(
                "Cannot declare attacks while a spell is on the stack"
            ));
        }
        let Some(player) = self.players.get(player_id) else {
            return Err(anyhow!("Player not found"));
        };
        let Some(card) = player.battlefield.iter().find(|card| card.id == card_id) else {
            return Err(anyhow!("Card not found on battlefield"));
        };
        if !matches!(card.kind, CardKind::Unit(_)) {
            return Err(anyhow!("Only units can attack"));
        }
        if self.combat_attackers.contains(&card_id) {
            return Err(anyhow!("{} is already attacking", card.name));
        }
        if !self.can_attack_card(player_id, card) {
            return Err(anyhow!("{} cannot attack right now", card.name));
        }
        let target = self.resolve_attack_target(player_id, target)?;
        if !self.combat_attackers.contains(&card_id) {
            self.combat_attackers.push(card_id.clone());
        }
        self.combat_targets.insert(card_id.clone(), target.clone());

        let attacker_name = self
            .players
            .get(player_id)
            .and_then(|p| p.battlefield.iter().find(|c| c.id == card_id))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| card_id.clone());
        let target_name = match &target {
            crate::types::TargetRef::Player(pid) => self
                .players
                .get(pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| pid.clone()),
            crate::types::TargetRef::Card(cid) => self
                .players
                .values()
                .flat_map(|p| p.battlefield.iter())
                .find(|c| &c.id == cid)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| cid.clone()),
        };
        self.log(
            crate::state::GameLogKind::AttackDeclared,
            player_id,
            format!("{} attacks {}", attacker_name, target_name),
        );

        // Check vigilance before mutating
        let has_vigilance = {
            let Some(player) = self.players.get(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            let Some(card) = player.battlefield.iter().find(|c| c.id == card_id) else {
                return Err(anyhow!("Card not found"));
            };
            self.effective_keywords_for_card(player_id, card)
                .contains(&Keyword::Vigilance)
        };
        // Tap the attacker
        if !has_vigilance {
            let Some(player_mut) = self.players.get_mut(player_id) else {
                return Err(anyhow!("Player not found"));
            };
            let Some(card_mut) = player_mut.battlefield.iter_mut().find(|c| c.id == card_id) else {
                return Err(anyhow!("Card not found"));
            };
            card_mut.tapped = true;
        }
        Ok(vec![])
    }

    fn handle_declare_block(
        &mut self,
        player_id: &str,
        attacker_id: String,
        blocker_id: String,
    ) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Combat {
            return Err(anyhow!("Blocks can only be declared in combat"));
        }
        if self.combat_phase != CombatPhase::DeclareBlockers {
            return Err(anyhow!(
                "Blocks can only be declared during the declare blockers phase"
            ));
        }
        if self.is_active_player(player_id) {
            return Err(anyhow!("Only the defending player can block"));
        }
        if self.combat_blocker_id.as_deref() != Some(player_id) {
            return Err(anyhow!("It is not your turn to declare blockers"));
        }
        if !self.combat_attackers.contains(&attacker_id) {
            return Err(anyhow!("Attacker not found"));
        }
        let active_id = self
            .active_player_id
            .clone()
            .ok_or_else(|| anyhow!("No active player"))?;
        let target = self
            .combat_targets
            .get(&attacker_id)
            .cloned()
            .or_else(|| self.default_defender_id(&active_id).map(TargetRef::Player))
            .ok_or_else(|| anyhow!("Attack target not found"))?;
        let target_player_id = match target {
            TargetRef::Player(id) => id,
            TargetRef::Card(_) => {
                return Err(anyhow!("Cannot block attacks targeting units"));
            }
        };
        if target_player_id != player_id {
            return Err(anyhow!("Only the targeted player can block"));
        }
        if self
            .combat_blockers
            .values()
            .any(|ids| ids.contains(&blocker_id))
        {
            return Err(anyhow!("Blocker already assigned"));
        }
        let Some(player) = self.players.get(player_id) else {
            return Err(anyhow!("Player not found"));
        };
        let Some(card) = player.battlefield.iter().find(|card| card.id == blocker_id) else {
            return Err(anyhow!("Blocker not found on battlefield"));
        };
        if card.tapped {
            return Err(anyhow!("Tapped units cannot block"));
        }

        // Keyword Evasion Checks
        let active_player = self
            .players
            .get(&active_id)
            .ok_or_else(|| anyhow!("Active player not found"))?;
        let attacker_card = active_player
            .battlefield
            .iter()
            .find(|c| c.id == attacker_id)
            .ok_or_else(|| anyhow!("Attacker card not found"))?;

        // Resolve effective keywords (including grants from allies)
        let attacker_keywords = self.effective_keywords_for_card(&active_id, attacker_card);
        let blocker_keywords = self.effective_keywords_for_card(player_id, card);

        // Stealth
        if attacker_keywords.contains(&Keyword::Stealth) {
            return Err(anyhow!("Cannot block a unit with Stealth"));
        }

        // Aero
        if attacker_keywords.contains(&Keyword::Aero) {
            if !blocker_keywords.contains(&Keyword::Aero)
                && !blocker_keywords.contains(&Keyword::Interceptor)
            {
                return Err(anyhow!(
                    "Cannot block Aero unit without Aero or Interceptor"
                ));
            }
        }

        // Optics(Type)
        for keyword in &attacker_keywords {
            if let Keyword::Optics(unit_type) = keyword {
                let is_valid_type = match &card.kind {
                    CardKind::Unit(t) => t == unit_type,
                    _ => false,
                };
                if !is_valid_type {
                    return Err(anyhow!("Optics: Blocked only by matching unit type"));
                }
            }
        }

        let attacker_name_str = attacker_card.name.clone();
        let blocker_name_str = card.name.clone();
        self.combat_blockers
            .entry(attacker_id)
            .or_default()
            .push(blocker_id);
        self.log(
            crate::state::GameLogKind::BlockDeclared,
            player_id,
            format!("{} blocks {}", blocker_name_str, attacker_name_str),
        );
        Ok(vec![])
    }

    fn handle_next_stage(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can advance"));
        }
        // No stage transitions (especially main → combat or main → end)
        // while there's anything pending on the stack / showcase — those
        // must fully resolve before the game advances.
        if !self.stack.is_empty() || self.showcase.is_some() {
            return Err(anyhow!(
                "Cannot advance the stage while a spell is on the stack"
            ));
        }
        self.reset_priority();
        self.stage = match self.stage {
            CardGameStage::Main => {
                self.combat_phase = CombatPhase::DeclareAttackers;
                CardGameStage::Combat
            }
            CardGameStage::Combat => {
                return Err(anyhow!(
                    "Use Confirm Attackers / Confirm Blockers to progress combat"
                ));
            }
            CardGameStage::PostCombatMain => {
                if self.queue_end_step_discard_if_needed(player_id) {
                    return Ok(vec![]);
                }
                self.advance_turn();
                return Ok(vec![]);
            }
            CardGameStage::End => {
                if self.queue_end_step_discard_if_needed(player_id) {
                    return Ok(vec![]);
                }
                self.advance_turn();
                return Ok(vec![]);
            }
            CardGameStage::Lobby => CardGameStage::Lobby,
            CardGameStage::Draft => CardGameStage::Draft,
            CardGameStage::DraftBuilding => CardGameStage::DraftBuilding,
            CardGameStage::Awards => CardGameStage::Awards,
            CardGameStage::SideboardSwap => CardGameStage::SideboardSwap,
            CardGameStage::Finished => CardGameStage::Finished,
        };
        Ok(vec![])
    }

    fn handle_confirm_attackers(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Combat {
            return Err(anyhow!("Not in combat"));
        }
        if self.combat_phase != CombatPhase::DeclareAttackers {
            return Err(anyhow!("Not in declare attackers phase"));
        }
        if !self.is_active_player(player_id) {
            return Err(anyhow!("Only the active player can confirm attackers"));
        }
        if !self.stack.is_empty() || self.showcase.is_some() {
            return Err(anyhow!(
                "Cannot confirm attackers while a spell is on the stack"
            ));
        }

        // If no attackers were declared, skip blockers entirely
        if self.combat_attackers.is_empty() {
            self.combat_phase = CombatPhase::Idle;
            self.combat_blocker_id = None;
            self.stage = CardGameStage::PostCombatMain;
            return Ok(vec![]);
        }

        // Find the first non-active player who has potential blockers
        let active_id = self.active_player_id.clone().unwrap();
        let first_blocker = self.next_eligible_blocker(&active_id, None);

        if let Some(blocker_id) = first_blocker {
            self.combat_phase = CombatPhase::DeclareBlockers;
            self.combat_blocker_id = Some(blocker_id.clone());
            self.priority_holder = Some(blocker_id);
            self.consecutive_priority_passes = 0;
        } else {
            // No eligible defenders — resolve combat immediately
            self.reset_priority();
            self.resolve_combat();
            self.combat_phase = CombatPhase::Idle;
            self.combat_blocker_id = None;
            self.stage = CardGameStage::PostCombatMain;
        }

        Ok(vec![])
    }

    fn handle_confirm_blockers(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if self.stage != CardGameStage::Combat {
            return Err(anyhow!("Not in combat"));
        }
        if self.combat_phase != CombatPhase::DeclareBlockers {
            return Err(anyhow!("Not in declare blockers phase"));
        }
        if self.combat_blocker_id.as_deref() != Some(player_id) {
            return Err(anyhow!("It is not your turn to confirm blockers"));
        }

        let active_id = self.active_player_id.clone().unwrap();

        // Find the next eligible blocker after the current one
        let next_blocker = self.next_eligible_blocker(&active_id, Some(player_id));

        if let Some(next_id) = next_blocker {
            self.combat_blocker_id = Some(next_id.clone());
            self.priority_holder = Some(next_id);
            self.consecutive_priority_passes = 0;
        } else {
            // No more defenders — resolve combat
            self.reset_priority();
            self.resolve_combat();
            self.combat_phase = CombatPhase::Idle;
            self.combat_blocker_id = None;
            self.stage = CardGameStage::PostCombatMain;
        }

        Ok(vec![])
    }
}

impl CardGameState {
    /// Find the next non-active player eligible to declare blockers.
    /// If `after` is None, starts from the beginning of turn_order.
    /// If `after` is Some(player_id), starts searching after that player.
    /// A player is eligible if they are defending against at least one player attack and
    /// still have something meaningful to do in the blockers window.
    fn next_eligible_blocker(&self, active_id: &str, after: Option<&str>) -> Option<String> {
        let start_index = match after {
            Some(pid) => {
                let idx = self.turn_order.iter().position(|id| id == pid).unwrap_or(0);
                idx + 1
            }
            None => 0,
        };

        // Search from start_index through the rest of turn_order (no wrapping)
        self.turn_order
            .iter()
            .skip(start_index)
            .find(|id| id.as_str() != active_id && self.player_has_potential_blockers(id))
            .cloned()
    }

    /// Check whether the defending player should receive a declare blockers window.
    pub fn player_has_potential_blockers(&self, player_id: &str) -> bool {
        let Some(active_id) = self.active_player_id.as_deref() else {
            return false;
        };
        if !self.player_is_under_attack(active_id, player_id) {
            return false;
        }

        let Some(player) = self.players.get(player_id) else {
            return false;
        };
        let has_untapped_blocker = player
            .battlefield
            .iter()
            .any(|card| matches!(card.kind, CardKind::Unit(_)) && !card.tapped);
        if has_untapped_blocker {
            return true;
        }

        self.player_has_response_available(player)
    }

    fn player_is_under_attack(&self, active_id: &str, player_id: &str) -> bool {
        self.combat_attackers.iter().any(|attacker_id| {
            self.combat_targets
                .get(attacker_id)
                .cloned()
                .or_else(|| self.default_defender_id(active_id).map(TargetRef::Player))
                == Some(TargetRef::Player(player_id.to_string()))
        })
    }

    pub fn player_has_response_available(&self, player: &crate::state::PlayerState) -> bool {
        let player_id = &player.id;

        // Check if any instant-speed card in hand could be cast if
        // priority were passed to this player. We don't use the full
        // `can_cast_card` check because that requires the player to
        // already hold priority — here we're deciding *whether* to pass
        // them priority at all, so only mana availability matters.
        let available = self.available_mana_for_player(player_id);
        let has_castable_instant = player.hand.iter().any(|card| {
            let is_instant_speed =
                matches!(card.kind, CardKind::Instant) || card.keywords.contains(&Keyword::Flash);
            if !is_instant_speed {
                return false;
            }
            let cost = self.calculate_effective_cost(player_id, card);
            available.can_afford(&cost)
        });
        if has_castable_instant {
            return true;
        }

        // Check if any activated ability on the battlefield can be used
        player.battlefield.iter().any(|card| {
            if card.tapped {
                return false;
            }
            // Summoning sickness: freshly deployed units can't use activated
            // abilities the turn they enter unless they have Haste.
            if card.summoning_sickness && !card.haste {
                return false;
            }
            card.abilities.iter().any(|ability| {
                // Mana abilities don't count as a "response" — tapping for
                // mana alone doesn't do anything to the spell on the stack
                // unless there's also an instant/flash to cast with it
                // (which is checked separately above).
                let is_mana_ability = matches!(
                    ability.effect,
                    CardEffect::AddMana(..) | CardEffect::AddManaScaling(..)
                );
                !is_mana_ability
                    && !ability.is_spell_effect_ability()
                    && ability.cost.has_tap()
                    && ability.cost.is_empty_except_tap()
            })
        })
    }

    /// Resolve combat damage — extracted from the old handle_next_stage Combat arm.
    fn resolve_combat(&mut self) {
        let Some(active_id) = self.active_player_id.clone() else {
            return;
        };

        let mut damage_to_defenders: HashMap<String, i32> = HashMap::new();
        let mut operator_damage_to_defenders: HashMap<String, i32> = HashMap::new();
        let mut loyalty_hits: HashMap<String, i32> = HashMap::new();
        let mut dead_attackers: Vec<String> = Vec::new();
        let mut dead_blockers: Vec<String> = Vec::new();
        let mut damage_dealt_ids: HashSet<String> = HashSet::new();
        let mut damage_received_ids: HashSet<String> = HashSet::new();
        // Track accumulated damage to each blocker across multiple attackers
        let mut blocker_damage_taken: HashMap<String, i32> = HashMap::new();
        // Track which attackers contributed how much damage to each defender (for game log)
        let mut damage_attribution: HashMap<String, Vec<(String, i32)>> = HashMap::new();
        // Track unit IDs that should become malware-infected after combat resolves
        // (any unit that took damage from a Malware-keyworded source)
        let mut malware_infected_ids: HashSet<String> = HashSet::new();

        let combat_attackers_snapshot = self.combat_attackers.clone();
        for attacker_id in &combat_attackers_snapshot {
            let (
                attacker_power,
                attacker_toughness,
                attacker_armor,
                attacker_shield,
                attacker_deathtouch,
                attacker_indestructible,
                attacker_has_trample,
            ) = self
                .players
                .get(&active_id)
                .and_then(|p| p.battlefield.iter().find(|c| &c.id == attacker_id))
                .map(|card| self.card_combat_stats(card))
                .unwrap_or((0, 0, 0, 0, false, false, false));
            let rampage_damage = self
                .players
                .get(&active_id)
                .and_then(|p| p.battlefield.iter().find(|c| &c.id == attacker_id))
                .map(|card| {
                    card.keywords
                        .iter()
                        .filter_map(|k| {
                            if let Keyword::Rampage(n) = k {
                                Some(*n)
                            } else {
                                None
                            }
                        })
                        .sum::<i32>()
                })
                .unwrap_or(0);

            let attacker_has_malware = self
                .players
                .get(&active_id)
                .and_then(|p| p.battlefield.iter().find(|c| &c.id == attacker_id))
                .map(|card| {
                    self.effective_keywords_for_card(&active_id, card)
                        .contains(&Keyword::Malware)
                })
                .unwrap_or(false);

            let is_operator_attacker = self.format == GameFormat::Operator
                && self
                    .players
                    .get(&active_id)
                    .and_then(|player| player.operator_id.as_ref())
                    .map(|id| id == attacker_id)
                    .unwrap_or(false);

            let target = self
                .combat_targets
                .get(attacker_id)
                .cloned()
                .or_else(|| self.default_defender_id(&active_id).map(TargetRef::Player));
            let Some(target) = target else {
                continue;
            };

            match target {
                TargetRef::Player(defender_id) => {
                    let defender_key = defender_id.clone();
                    if let Some(blocker_ids) = self.combat_blockers.get(attacker_id) {
                        let mut total_blocker_power = 0;
                        let mut remaining_damage = attacker_power;
                        let mut has_blocker_deathtouch = false;
                        for blocker_id in blocker_ids {
                            let (
                                blocker_power,
                                blocker_toughness,
                                blocker_armor,
                                blocker_shield,
                                blocker_deathtouch,
                                blocker_indestructible,
                                _,
                            ) = self
                                .players
                                .get(&defender_id)
                                .and_then(|p| p.battlefield.iter().find(|c| &c.id == blocker_id))
                                .map(|card| self.card_combat_stats(card))
                                .unwrap_or((0, 0, 0, 0, false, false, false));
                            total_blocker_power += blocker_power;
                            if blocker_deathtouch && blocker_power > 0 {
                                has_blocker_deathtouch = true;
                            }
                            if attacker_power > 0 {
                                damage_dealt_ids.insert(attacker_id.clone());
                                damage_received_ids.insert(blocker_id.clone());
                            }
                            if blocker_power > 0 {
                                damage_dealt_ids.insert(blocker_id.clone());
                                damage_received_ids.insert(attacker_id.clone());
                            }

                            let lethal_toughness =
                                blocker_toughness + blocker_armor + blocker_shield;
                            let damage_to_assign = if attacker_deathtouch && remaining_damage > 0 {
                                1 // Deathtouch only needs 1 damage
                            } else {
                                remaining_damage.min(lethal_toughness)
                            };
                            if damage_to_assign > 0 {
                                *blocker_damage_taken.entry(blocker_id.clone()).or_insert(0) +=
                                    damage_to_assign;
                                if attacker_has_malware {
                                    malware_infected_ids.insert(blocker_id.clone());
                                }
                                if attacker_deathtouch {
                                    // Mark for deathtouch kill unless indestructible
                                    if !blocker_indestructible {
                                        dead_blockers.push(blocker_id.clone());
                                    }
                                }
                                remaining_damage = (remaining_damage - damage_to_assign).max(0);
                            }

                            // Blocker damages attacker — if blocker has Malware, infect attacker
                            if blocker_power > 0 {
                                let blocker_has_malware = self
                                    .players
                                    .get(&defender_id)
                                    .and_then(|p| {
                                        p.battlefield.iter().find(|c| &c.id == blocker_id)
                                    })
                                    .map(|card| {
                                        self.effective_keywords_for_card(&defender_id, card)
                                            .contains(&Keyword::Malware)
                                    })
                                    .unwrap_or(false);
                                if blocker_has_malware {
                                    malware_infected_ids.insert(attacker_id.clone());
                                }
                            }
                        }

                        // Apply blocker damage to the attacker (persists until end of turn).
                        // Route through `damage_unit` so Absorb is honored.
                        if total_blocker_power > 0 {
                            let attacker_id_owned = attacker_id.clone();
                            self.damage_unit(&active_id, &attacker_id_owned, total_blocker_power);
                        }
                        // Deathtouch kills regardless of toughness; state-based
                        // actions handle lethal-damage kills via card.damage.
                        if has_blocker_deathtouch && !attacker_indestructible {
                            dead_attackers.push(attacker_id.clone());
                        }
                        let _ = (attacker_toughness, attacker_armor, attacker_shield);

                        if attacker_has_trample && remaining_damage > 0 {
                            *damage_to_defenders.entry(defender_id.clone()).or_insert(0) +=
                                remaining_damage;
                            damage_attribution
                                .entry(defender_id.clone())
                                .or_default()
                                .push((attacker_id.clone(), remaining_damage));
                            if is_operator_attacker {
                                *operator_damage_to_defenders.entry(defender_id).or_insert(0) +=
                                    remaining_damage;
                            }
                        }
                    } else {
                        *damage_to_defenders.entry(defender_key.clone()).or_insert(0) +=
                            attacker_power;
                        if attacker_power > 0 {
                            damage_attribution
                                .entry(defender_key.clone())
                                .or_default()
                                .push((attacker_id.clone(), attacker_power));
                        }
                        if is_operator_attacker && attacker_power > 0 {
                            *operator_damage_to_defenders
                                .entry(defender_key.clone())
                                .or_insert(0) += attacker_power;
                        }

                        if attacker_power > 0 {
                            damage_dealt_ids.insert(attacker_id.clone());
                        }
                    }

                    // Rampage: deal extra damage to defending player regardless of blockers
                    if rampage_damage > 0 {
                        *damage_to_defenders.entry(defender_key.clone()).or_insert(0) +=
                            rampage_damage;
                        damage_attribution
                            .entry(defender_key.clone())
                            .or_default()
                            .push((attacker_id.clone(), rampage_damage));
                    }
                }
                TargetRef::Card(target_card_id) => {
                    let owner_info = self.find_battlefield_card_owner(&target_card_id);
                    let Some((owner_id, kind)) = owner_info else {
                        continue;
                    };

                    if kind == CardKind::Agent {
                        let mut blocked = false;
                        let mut remaining_damage = attacker_power;
                        if let Some(blocker_ids) = self.combat_blockers.get(attacker_id) {
                            if !blocker_ids.is_empty() {
                                blocked = true;
                                let mut total_blocker_power = 0;
                                let mut has_blocker_deathtouch = false;
                                for blocker_id in blocker_ids {
                                    let (
                                        blocker_power,
                                        blocker_toughness,
                                        blocker_armor,
                                        blocker_shield,
                                        blocker_deathtouch,
                                        blocker_indestructible,
                                        _,
                                    ) = self
                                        .players
                                        .get(&owner_id)
                                        .and_then(|p| {
                                            p.battlefield.iter().find(|c| &c.id == blocker_id)
                                        })
                                        .map(|card| self.card_combat_stats(card))
                                        .unwrap_or((0, 0, 0, 0, false, false, false));
                                    total_blocker_power += blocker_power;
                                    if blocker_deathtouch && blocker_power > 0 {
                                        has_blocker_deathtouch = true;
                                    }
                                    if attacker_power > 0 {
                                        damage_dealt_ids.insert(attacker_id.clone());
                                        damage_received_ids.insert(blocker_id.clone());
                                    }
                                    if blocker_power > 0 {
                                        damage_dealt_ids.insert(blocker_id.clone());
                                        damage_received_ids.insert(attacker_id.clone());
                                    }
                                    let lethal_toughness =
                                        blocker_toughness + blocker_armor + blocker_shield;
                                    let is_lethal = if attacker_deathtouch {
                                        remaining_damage > 0
                                    } else {
                                        remaining_damage >= lethal_toughness
                                    };
                                    if is_lethal && !blocker_indestructible {
                                        dead_blockers.push(blocker_id.clone());
                                        let consumed = if attacker_deathtouch {
                                            1
                                        } else {
                                            lethal_toughness
                                        };
                                        remaining_damage -= consumed;
                                    } else {
                                        remaining_damage = 0;
                                    }
                                }
                                if ((has_blocker_deathtouch && total_blocker_power > 0)
                                    || total_blocker_power
                                        >= (attacker_toughness + attacker_armor + attacker_shield))
                                    && !attacker_indestructible
                                {
                                    dead_attackers.push(attacker_id.clone());
                                }
                            }
                        }

                        if !blocked || (attacker_has_trample && remaining_damage > 0) {
                            *loyalty_hits.entry(target_card_id).or_insert(0) += if blocked {
                                remaining_damage
                            } else {
                                attacker_power
                            };
                            if attacker_power > 0 {
                                damage_dealt_ids.insert(attacker_id.clone());
                            }
                        }
                    } else {
                        let (
                            target_power,
                            target_toughness,
                            target_armor,
                            target_shield,
                            target_deathtouch,
                            target_indestructible,
                            _,
                        ) = self
                            .players
                            .get(&owner_id)
                            .and_then(|p| p.battlefield.iter().find(|c| c.id == target_card_id))
                            .map(|card| self.card_combat_stats(card))
                            .unwrap_or((0, 0, 0, 0, false, false, false));

                        if attacker_power > 0 {
                            damage_dealt_ids.insert(attacker_id.clone());
                            damage_received_ids.insert(target_card_id.clone());
                        }
                        if target_power > 0 {
                            damage_dealt_ids.insert(target_card_id.clone());
                            damage_received_ids.insert(attacker_id.clone());
                        }

                        if ((attacker_deathtouch && attacker_power > 0)
                            || attacker_power >= (target_toughness + target_armor + target_shield))
                            && !target_indestructible
                        {
                            dead_blockers.push(target_card_id);
                        }
                        if ((target_deathtouch && target_power > 0)
                            || target_power
                                >= (attacker_toughness + attacker_armor + attacker_shield))
                            && !attacker_indestructible
                        {
                            dead_attackers.push(attacker_id.clone());
                        }
                    }
                }
            }
        }

        // Apply blocker damage to each blocker's card.damage so it persists
        // until end of turn. Lethal damage kills are handled by state-based
        // actions (apply_passive_state_updates). Route through
        // `damage_unit` so Absorb is honored.
        for (blocker_id, total_damage) in &blocker_damage_taken {
            if let Some((blocker_owner, _)) = self.find_battlefield_card_owner(blocker_id) {
                self.damage_unit(&blocker_owner, blocker_id, *total_damage);
            }
        }

        // Apply all hits and track damage dealt. Any uncontested damage
        // to a player (unblocked attacker getting through) also resets
        // the display timer so the client has a 5-second window to play
        // the hit animation before the game auto-advances.
        let any_player_damage = damage_to_defenders.values().any(|&d| d > 0);
        if any_player_damage {
            self.last_stack_push_at = Some(chrono::Utc::now());
        }
        for (target_id, damage) in &damage_to_defenders {
            if let Some(target) = self.players.get_mut(target_id) {
                target.life = (target.life - damage).max(0);
            }
            let target_name = self
                .players
                .get(target_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| target_id.clone());

            // Build per-attacker breakdown so the log shows the source(s)
            let attribution = damage_attribution
                .get(target_id)
                .cloned()
                .unwrap_or_default();
            let source_clause = if attribution.is_empty() {
                String::new()
            } else {
                let parts: Vec<String> = attribution
                    .iter()
                    .map(|(attacker_id, dmg)| {
                        let name = self
                            .players
                            .get(&active_id)
                            .and_then(|p| p.battlefield.iter().find(|c| &c.id == attacker_id))
                            .map(|c| c.name.clone())
                            .unwrap_or_else(|| attacker_id.clone());
                        format!("{} ({})", name, dmg)
                    })
                    .collect();
                format!(" — from {}", parts.join(", "))
            };
            self.log(
                crate::state::GameLogKind::CombatDamage,
                &active_id,
                format!(
                    "dealt {} damage to {}{}",
                    damage, target_name, source_clause
                ),
            );
        }
        let total_combat_damage: i32 = damage_to_defenders.values().sum();
        if total_combat_damage > 0 {
            *self
                .damage_dealt_this_turn
                .entry(active_id.clone())
                .or_insert(0) += total_combat_damage;

            // Emit combat_damage_dealt milestone event
            let has_operator_damage = self.format == GameFormat::Operator
                && operator_damage_to_defenders.values().any(|&d| d > 0);
            let mut meta = HashMap::new();
            meta.insert("damage".into(), json!(total_combat_damage));
            meta.insert(
                "format".into(),
                json!(match self.format {
                    GameFormat::Classic => "classic",
                    GameFormat::Operator => "operator",
                }),
            );
            meta.insert("killed_by_operator".into(), json!(has_operator_damage));
            self.deferred_effects.push(GameEffect::MilestoneEvent {
                game_id: "card_game".into(),
                event_key: "combat_damage_dealt".into(),
                user_id: active_id.clone(),
                lobby_id: String::new(),
                meta,
            });
        }
        if self.format == GameFormat::Operator {
            for (target_id, damage) in operator_damage_to_defenders {
                if let Some(target) = self.players.get_mut(&target_id) {
                    let entry = target.operator_damage.entry(active_id.clone()).or_insert(0);
                    *entry += damage;
                    if *entry >= 21 {
                        target.life = 0;
                    }
                }
            }
        }

        for (card_id, damage) in loyalty_hits {
            if let Some((owner_id, _)) = self.find_battlefield_card_owner(&card_id) {
                if let Some(player) = self.players.get_mut(&owner_id) {
                    if let Some(card) = player.battlefield.iter_mut().find(|c| c.id == card_id) {
                        if let Some(loyalty) = &mut card.loyalty {
                            *loyalty = (*loyalty - damage).max(0);
                        }
                    }
                }
            }
        }

        let any_unit_damage_events: Vec<(String, String)> = damage_dealt_ids
            .iter()
            .filter_map(|card_id| {
                self.find_battlefield_card_owner(card_id)
                    .and_then(|(owner_id, kind)| {
                        matches!(kind, CardKind::Unit(_)).then_some((owner_id, card_id.clone()))
                    })
            })
            .collect();

        let mut trigger_queue: Vec<(
            String,
            String,
            String,
            Vec<crate::card::CardTrigger>,
            TriggerEvent,
        )> = Vec::new();
        for card_id in damage_dealt_ids {
            if let Some((owner_id, _)) = self.find_battlefield_card_owner(&card_id) {
                if let Some(card) = self
                    .players
                    .get(&owner_id)
                    .and_then(|p| p.battlefield.iter().find(|c| c.id == card_id))
                {
                    trigger_queue.push((
                        owner_id.clone(),
                        card_id.clone(),
                        card.name.clone(),
                        card.triggers.clone(),
                        TriggerEvent::DealtDamage,
                    ));
                }
            }
        }

        for card_id in damage_received_ids {
            if let Some((owner_id, _)) = self.find_battlefield_card_owner(&card_id) {
                if let Some(card) = self
                    .players
                    .get(&owner_id)
                    .and_then(|p| p.battlefield.iter().find(|c| c.id == card_id))
                {
                    trigger_queue.push((
                        owner_id.clone(),
                        card_id.clone(),
                        card.name.clone(),
                        card.triggers.clone(),
                        TriggerEvent::ReceivedDamage,
                    ));
                }
            }
        }

        for (owner_id, card_id, name, triggers, event) in trigger_queue {
            self.apply_triggers_for_event(&owner_id, &card_id, Some(&name), &triggers, &event, &[]);
        }

        for (owner_id, card_id) in any_unit_damage_events {
            self.apply_battlefield_triggers_for_player_event(
                &owner_id,
                &TriggerEvent::AnyAllyDealtDamage,
                &[TargetRef::Card(card_id)],
            );
        }

        // Apply Malware infection: strip abilities + triggers + non-keyword text
        // from any unit that took damage from a Malware-keyworded source.
        for infected_id in &malware_infected_ids {
            // Skip units that died in combat — no point infecting a corpse.
            let died_in_combat = dead_attackers.iter().any(|id| id == infected_id)
                || dead_blockers.iter().any(|id| id == infected_id);
            if died_in_combat {
                continue;
            }
            let mut infected_name: Option<String> = None;
            let mut owner: Option<String> = None;
            for (pid, player) in self.players.iter_mut() {
                if let Some(card) = player.battlefield.iter_mut().find(|c| &c.id == infected_id) {
                    if !card.malware_infected {
                        card.malware_infected = true;
                        card.abilities.clear();
                        card.triggers.clear();
                        infected_name = Some(card.name.clone());
                        owner = Some(pid.clone());
                    }
                    break;
                }
            }
            if let (Some(name), Some(owner)) = (infected_name, owner) {
                self.log(
                    crate::state::GameLogKind::Other,
                    &owner,
                    format!("{} is infected by Malware (abilities lost)", name),
                );
            }
        }

        if self.players.contains_key(&active_id) {
            for attacker_id in dead_attackers {
                let dead_name = self
                    .players
                    .get(&active_id)
                    .and_then(|p| p.battlefield.iter().find(|c| c.id == attacker_id))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| attacker_id.clone());
                self.log(
                    crate::state::GameLogKind::UnitDied,
                    &active_id,
                    format!("{} died in combat", dead_name),
                );
                self.remove_from_battlefield_and_attachments(&active_id, &attacker_id);
            }
        }

        for blocker_id in dead_blockers {
            if let Some((owner_id, _)) = self.find_battlefield_card_owner(&blocker_id) {
                let dead_name = self
                    .players
                    .get(&owner_id)
                    .and_then(|p| p.battlefield.iter().find(|c| c.id == blocker_id))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| blocker_id.clone());
                self.log(
                    crate::state::GameLogKind::UnitDied,
                    &owner_id,
                    format!("{} died in combat", dead_name),
                );
                self.remove_from_battlefield_and_attachments(&owner_id, &blocker_id);
            }
        }

        self.combat_attackers.clear();
        self.combat_blockers.clear();
        self.combat_targets.clear();
    }
}
