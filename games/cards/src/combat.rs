//! Combat logic functions

use crate::card::*;
use crate::state::*;
use crate::types::*;
use anyhow::{Result, anyhow};

/// Combat logic functions
impl CardGameState {
    pub fn can_attack_card(&self, player_id: &str, card: &Card) -> bool {
        if self.stage != CardGameStage::Combat {
            return false;
        }
        if self.combat_phase != CombatPhase::DeclareAttackers {
            return false;
        }
        if !self.is_active_player(player_id) {
            return false;
        }
        if !matches!(card.kind, CardKind::Unit(_)) {
            return false;
        }
        // Summoning sickness: units played this turn can't attack unless
        // they have Haste (intrinsic or granted by another permanent).
        if card.summoning_sickness {
            let has_haste = card.haste
                || self
                    .effective_keywords_for_card(player_id, card)
                    .contains(&Keyword::Haste)
                || self.has_keyword_for_card(&card.id, StatType::Haste);
            if !has_haste {
                return false;
            }
        }
        if self.modifier_amount_for_card(&card.id, StatType::Armor) == -999 {
            // Placeholder for Freeze
            return false;
        }
        !card.tapped
    }

    pub fn can_block_card(&self, player_id: &str, card: &Card) -> bool {
        if self.stage != CardGameStage::Combat {
            return false;
        }
        if self.combat_phase != CombatPhase::DeclareBlockers {
            return false;
        }
        if self.combat_blocker_id.as_deref() != Some(player_id) {
            return false;
        }
        if self.is_active_player(player_id) {
            return false;
        }
        if !matches!(card.kind, CardKind::Unit(_)) {
            return false;
        }
        if card.tapped {
            return false;
        }
        // Check if already assigned
        if self
            .combat_blockers
            .values()
            .any(|ids| ids.contains(&card.id))
        {
            return false;
        }

        // Also check if any attacker is targeting this player
        if self.combat_attackers.is_empty() {
            return false;
        }

        // Must be targeting this player
        let active_id = match &self.active_player_id {
            Some(id) => id,
            None => return false,
        };

        self.combat_attackers.iter().any(|attacker_id| {
            let target = self
                .combat_targets
                .get(attacker_id)
                .cloned()
                .or_else(|| self.default_defender_id(active_id).map(TargetRef::Player));
            match target {
                Some(TargetRef::Player(id)) => id == player_id,
                _ => false,
            }
        })
    }

    /// Default target selection for "attack an opponent" / "target opponent" with no
    /// explicit target. In 2-player this is the other player. In N-player pods (operator)
    /// picks the lowest-life living opponent; ties broken by most commander damage
    /// already dealt by this attacker (to finish off a commander-damage kill).
    pub fn default_defender_id(&self, attacker_id: &str) -> Option<String> {
        use crate::types::GameFormat;
        // Collect living opponents in turn order.
        let candidates: Vec<&String> = self
            .turn_order
            .iter()
            .filter(|id| id.as_str() != attacker_id)
            .filter(|id| {
                self.players
                    .get(id.as_str())
                    .map(|p| p.life > 0)
                    .unwrap_or(false)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Classic 1v1: just return the other player (preserves legacy behavior).
        if !matches!(self.format, GameFormat::Operator) || candidates.len() == 1 {
            return Some(candidates[0].clone());
        }

        // Operator pod: lowest life, ties broken by most commander damage from attacker.
        candidates
            .into_iter()
            .min_by(|a, b| {
                let pa = self.players.get(a.as_str());
                let pb = self.players.get(b.as_str());
                let life_a = pa.map(|p| p.life).unwrap_or(i32::MAX);
                let life_b = pb.map(|p| p.life).unwrap_or(i32::MAX);
                life_a.cmp(&life_b).then_with(|| {
                    // Higher commander damage from attacker = prefer (finish the kill).
                    let cd_a = pa
                        .and_then(|p| p.operator_damage.get(attacker_id).copied())
                        .unwrap_or(0);
                    let cd_b = pb
                        .and_then(|p| p.operator_damage.get(attacker_id).copied())
                        .unwrap_or(0);
                    cd_b.cmp(&cd_a) // reverse: higher cd wins (goes first)
                })
            })
            .cloned()
    }

    pub fn resolve_attack_target(
        &self,
        attacker_owner: &str,
        target: Option<TargetRef>,
    ) -> Result<TargetRef> {
        let target = match target {
            Some(target) => target,
            None => TargetRef::Player(
                self.default_defender_id(attacker_owner)
                    .ok_or_else(|| anyhow!("No defender available"))?,
            ),
        };

        match &target {
            TargetRef::Player(target_player_id) => {
                if target_player_id == attacker_owner {
                    return Err(anyhow!("Cannot attack yourself"));
                }
                if !self.players.contains_key(target_player_id) {
                    return Err(anyhow!("Invalid target"));
                }
            }
            TargetRef::Card(card_id) => {
                let Some((owner_id, kind)) = self.find_battlefield_card_owner(card_id) else {
                    return Err(anyhow!("Invalid target"));
                };
                if owner_id == attacker_owner {
                    return Err(anyhow!("Cannot attack your own permanent"));
                }
                if !matches!(kind, CardKind::Unit(_) | CardKind::Agent) {
                    return Err(anyhow!("Target is not a unit or agent"));
                }
            }
        }

        Ok(target)
    }
}
