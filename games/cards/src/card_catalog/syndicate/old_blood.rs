//! Old Blood: made men, protection, and punishing anyone who comes at the family.

use super::{OLD_BLOOD, crew, job, on, racket, trick};
use crate::card::TargetSpec;
use crate::card::{Card, CardEffect, StatModifierSpec};
use crate::types::{
    CardRarity, EffectTargetSpec, ExpireContract, Keyword, ModifierAmount, StatType, TargetKind,
    TargetScope, TriggerEvent,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_social_club", "Social Club", 2, 1, OLD_BLOOD)
                .description("Card games in the back. Nobody plays for fun.")
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            racket("syn_union_hall", "Union Hall", 4, 3, OLD_BLOOD)
                .description("Every paycheck on the docks has the family's name on it somewhere.")
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            racket("syn_casino", "Casino", 5, 3, OLD_BLOOD)
                .description("The house always wins. The house is us.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(1)))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            crew("syn_doorman", "Doorman", 2, 2, 3, OLD_BLOOD)
                .description("Nobody gets in. Nobody gets past. Nobody asks twice.")
                .keyword(Keyword::Vigilance)
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_made_man", "Made Man", 3, 2, 3, OLD_BLOOD)
                .description("You touch him, you touch the family. The family touches back.")
                .trigger(on(TriggerEvent::ReceivedDamage, CardEffect::DamagePlayer(2)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_consigliere", "Consigliere", 3, 2, 2, OLD_BLOOD)
                .description("He knows a judge. He knows every judge.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(-2)))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_bodyguard", "Bodyguard", 3, 2, 4, OLD_BLOOD)
                .description("Stands between the boss and everything else. Everything else loses.")
                .keyword(Keyword::ReactiveArmor(1))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_capo", "Capo", 4, 4, 4, OLD_BLOOD)
                .description("Take a swing at him and the bill goes to whoever sent you.")
                .keyword(Keyword::Vigilance)
                .trigger(on(TriggerEvent::ReceivedDamage, CardEffect::Reflect(3)))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_underboss", "Underboss", 5, 5, 4, OLD_BLOOD)
                .description("Runs the day to day. The day to day runs on fear.")
                .keyword(Keyword::Indestructible)
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(1)))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            crew("syn_the_don", "The Don", 6, 5, 5, OLD_BLOOD)
                .description("He does not raise his voice. He has never needed to.")
                .keyword(Keyword::Indestructible)
                .trigger(on(
                    TriggerEvent::Entered,
                    CardEffect::ApplyStatModifier(StatModifierSpec {
                        stat: StatType::Both,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(None),
                    }),
                ))
                .rarity(CardRarity::Classified)
                .build(),
            1,
        ),
        (
            job("syn_sit_down", "Sit-Down", 1, OLD_BLOOD)
                .description("Two coffees, one handshake, and the matter is closed.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::Heat(-3), CardEffect::GainLife(2)]),
                    None,
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            job("syn_make_an_example", "Make an Example", 4, OLD_BLOOD)
                .description("Somebody had to. It was arranged so that everybody would hear.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::Heat(2)]),
                    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            trick("syn_under_protection", "Under Protection", 2, OLD_BLOOD)
                .description("Word went out this morning. He is not to be touched.")
                .effect(
                    CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
                    Some(TargetSpec::new(TargetScope::SelfPlayer, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
    ]
}
