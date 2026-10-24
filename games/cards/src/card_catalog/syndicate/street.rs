//! Street: cheap crews that go wide and fast, and run hot doing it.

use super::{STREET, crew, job, on, racket, trick};
use crate::card::{CardEffect, StatModifierSpec, TokenSpec, Card};
use crate::types::{
    CardRarity, EffectTargetSpec, ExpireContract, Keyword, ManaType, ModifierAmount, StatType,
    TargetKind, TargetScope, TriggerEvent, UnitType,
};
use crate::card::TargetSpec;

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_corner", "Corner", 1, 1, STREET)
                .description("Every block has one. This one is yours.")
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            racket("syn_chop_shop", "Chop Shop", 3, 2, STREET)
                .description("Cars come in whole and leave in boxes.")
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_lookout", "Lookout", 1, 1, 1, STREET)
                .description("Sees the cops before the cops see anything.")
                .keyword(Keyword::Haste)
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_bagman", "Bagman", 2, 1, 2, STREET)
                .description("Collects. Skims. Collects again.")
                .trigger(on(TriggerEvent::DealtDamage, CardEffect::AddMana(ManaType::Colorless, 2)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_brawler", "Brawler", 2, 3, 3, STREET)
                .description("Loud entrance. Louder exit.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(1)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_corner_boys", "Corner Boys", 3, 3, 2, STREET)
                .description("Never just one of them.")
                .trigger(on(
                    TriggerEvent::Entered,
                    CardEffect::CreateToken(TokenSpec::new("Goon", 1, 1, UnitType::Human)),
                ))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_getaway_driver", "Getaway Driver", 3, 2, 1, STREET)
                .description("Gone before the alarm finishes ringing.")
                .keyword(Keyword::Haste)
                .keyword(Keyword::Stealth)
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_enforcer", "Enforcer", 4, 4, 4, STREET)
                .description("The conversation is over when he stands up.")
                .keyword(Keyword::Trample)
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            job("syn_drive_by", "Drive-By", 2, STREET)
                .description("Three shots. Two hit. The whole block heard it.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DamageTarget(3), CardEffect::Heat(1)]),
                    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::PlayerOrUnit, 1)),
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            job("syn_rally_the_block", "Rally the Block", 3, STREET)
                .description("Everyone out. Now.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::ApplyStatModifier(StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        }),
                        CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
                        CardEffect::Heat(1),
                    ]),
                    None,
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            trick("syn_ambush", "Ambush", 1, STREET)
                .description("They never see the second guy.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::ApplyStatModifier(StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(3),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        }),
                        CardEffect::Heat(1),
                    ]),
                    Some(TargetSpec::new(TargetScope::SelfPlayer, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
    ]
}
