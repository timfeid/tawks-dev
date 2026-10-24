use crate::card::*;
use crate::types::*;

pub struct AbilityFactory;

impl AbilityFactory {
    pub fn siphon(mana_type: ManaType, amount: i32) -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::AddMana(mana_type, amount),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn burnout(x: usize) -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Freeze(ExpireContract::Turns(1)),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetCountExactly(x))),
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn recursion(percentage_basis_points: i32) -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::RefundMana(percentage_basis_points),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn volatile(_target_spec: Option<TargetSpec>) -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DamageTarget(0), // Logic: Use power
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn stun_lock() -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Freeze(ExpireContract::Turns(1)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn ping(x: usize) -> CardEffect {
        CardEffect::Scry(x)
    }

    pub fn leach() -> CardTrigger {
        CardTrigger {
            event: TriggerEvent::Attacked,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        }
    }

    pub fn index(draw: usize, discard: usize) -> CardEffect {
        CardEffect::Loot(draw, discard)
    }

    pub fn scour(unit_type: UnitType) -> CardEffect {
        CardEffect::Search(Predicate::Condition(Condition::TargetIsUnitType(unit_type)))
    }

    pub fn reboot() -> CardEffect {
        CardEffect::Purify
    }

    pub fn upload() -> CardEffect {
        CardEffect::Bounce
    }

    pub fn hard_reset() -> CardEffect {
        CardEffect::BoardWipe
    }

    pub fn armor_up(amount: i32) -> CardEffect {
        CardEffect::ApplyStatModifier(StatModifierSpec {
            stat: StatType::Armor,
            amount: ModifierAmount::Fixed(amount),
            expires: ExpireContract::Never,
            target: EffectTargetSpec::Targets,
        })
    }

    pub fn empower(amount: i32) -> CardEffect {
        CardEffect::ApplyStatModifier(StatModifierSpec {
            stat: StatType::Power,
            amount: ModifierAmount::Fixed(amount),
            expires: ExpireContract::EndOfTurn,
            target: EffectTargetSpec::Targets,
        })
    }

    pub fn shield_pulse(amount: i32) -> CardEffect {
        CardEffect::ApplyStatModifier(StatModifierSpec {
            stat: StatType::Shield,
            amount: ModifierAmount::Fixed(amount),
            expires: ExpireContract::Turns(1),
            target: EffectTargetSpec::OwnerUnits(None),
        })
    }
}
