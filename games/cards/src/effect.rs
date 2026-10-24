use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatModifierEffect {
    pub id: String,
    pub target: EffectTarget,
    pub stat: StatType,
    pub amount: ModifierAmount,
    pub expires: ExpireContract,
    pub owner_id: String,
}

#[derive(Clone, Debug, Default)]
pub struct EffectManager {
    pub(crate) effects: Vec<StatModifierEffect>,
}

impl EffectManager {
    pub fn add_effect(&mut self, effect: StatModifierEffect) {
        self.effects.push(effect);
    }

    pub fn expire_end_of_turn(&mut self) {
        let mut next = Vec::with_capacity(self.effects.len());
        for effect in self.effects.drain(..) {
            match effect.expires {
                ExpireContract::Never => next.push(effect),
                ExpireContract::EndOfTurn => {}
                ExpireContract::Turns(0) => {}
                ExpireContract::Turns(turns) => {
                    let remaining = turns.saturating_sub(1);
                    if remaining > 0 {
                        next.push(StatModifierEffect {
                            expires: ExpireContract::Turns(remaining),
                            ..effect
                        });
                    }
                }
            }
        }
        self.effects = next;
    }

    pub fn remove_effects_for_card(&mut self, card_id: &str) {
        self.effects.retain(|effect| {
            if let EffectTarget::Card(id) = &effect.target {
                id != card_id
            } else {
                true
            }
        });
    }
}
