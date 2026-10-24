//! Wolves: sacrifice, death value, and crews that thrive in debt.

use super::{WOLVES, crew, job, on, racket, trick};
use crate::card::TargetSpec;
use crate::card::{Card, CardEffect, StatModifierSpec, TokenSpec};
use crate::types::{
    CardRarity, EffectTargetSpec, ExpireContract, Keyword, ManaType, ModifierAmount, StatType,
    TargetKind, TargetScope, TriggerEvent, UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_pawn_shop", "Pawn Shop", 1, 1, WOLVES)
                .description("Everything in the window used to be somebody's.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            racket("syn_dog_fights", "Dog Fights", 2, 1, WOLVES)
                .description("The losers feed the winners.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(1)))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            racket("syn_junkyard", "Junkyard", 3, 2, WOLVES)
                .description("Nothing gets thrown away. It gets sold twice.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_stray", "Stray", 1, 1, 1, WOLVES)
                .description("Kick it out and something worse crawls in.")
                .trigger(on(
                    TriggerEvent::Died,
                    CardEffect::CreateToken(TokenSpec::new("Rat", 1, 1, UnitType::Human)),
                ))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_cutter", "Cutter", 3, 1, 1, WOLVES)
                .description("One blade, one lesson, no repeat customers.")
                .keyword(Keyword::Deathtouch)
                .trigger(on(TriggerEvent::Died, CardEffect::DrawCards(1)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_debt_hound", "Debt Hound", 2, 1, 2, WOLVES)
                .description("Buries the dead and bills the family.")
                .trigger(on(TriggerEvent::AllyDied, CardEffect::AddMana(ManaType::Colorless, 1)))
                .trigger(on(TriggerEvent::Died, CardEffect::AddMana(ManaType::Colorless, 1)))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_old_wolf", "Old Wolf", 4, 2, 2, WOLVES)
                .description("Every funeral makes him meaner.")
                .trigger(on(
                    TriggerEvent::AllyDied,
                    CardEffect::ApplyStatModifier(StatModifierSpec {
                        stat: StatType::Both,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::Source,
                    }),
                ))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_skinner", "Skinner", 3, 2, 2, WOLVES)
                .description("What he touches does not heal.")
                .keyword(Keyword::Wither)
                .trigger(on(TriggerEvent::Died, CardEffect::DamagePlayer(2)))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_widowmaker", "Widowmaker", 5, 3, 4, WOLVES)
                .description("Takes two to stop her. Takes a cut either way.")
                .keyword(Keyword::Menace)
                .keyword(Keyword::Wither)
                .trigger(on(TriggerEvent::Died, CardEffect::AddMana(ManaType::Colorless, 2)))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            crew("syn_mad_dog", "Mad Dog", 6, 5, 5, WOLVES)
                .description("Put him down and he still takes a piece of you.")
                .keyword(Keyword::Trample)
                .trigger(on(TriggerEvent::Died, CardEffect::DamagePlayer(3)))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            job("syn_arsonist", "Arsonist", 4, WOLVES)
                .description("Insurance never pays out on people.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::Heat(3)]),
                    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            job("syn_torch_job", "Torch Job", 5, WOLVES)
                .description("Lock the doors first.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DamageEachOpponentUnit(2), CardEffect::Heat(2)]),
                    None,
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            trick("syn_pick_the_bones", "Pick the Bones", 1, WOLVES)
                .description("Nothing on a corpse is sacred.")
                .effect(CardEffect::Loot(2, 1), None)
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
    ]
}
