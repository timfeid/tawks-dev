//! Cards every family can run.

use super::{crew, job, racket, trick};
use crate::card::{Card, CardEffect};
use crate::types::CardRarity;

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_fence", "Fence", 2, 1, "colorless")
                .description("A quiet shop that moves whatever comes through the back door.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_hired_gun", "Hired Gun", 3, 3, 3, "colorless")
                .description("Works for anyone who pays on time.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            job("syn_loan_sharks_visit", "Loan Shark's Visit", 0, "colorless")
                .description("Someone owes, and it is not you.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::CashOpponents(-3), CardEffect::DrawCards(1)]),
                    None,
                )
                .rarity(CardRarity::Private)
                .build(),
            1,
        ),
        (
            trick("syn_bribe", "Bribe", 2, "colorless")
                .description("An envelope changes hands and a file goes missing.")
                .effect(CardEffect::Heat(-4), None)
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            trick("syn_lawyer_up", "Lawyer Up", 1, "colorless")
                .description("Say nothing. Draw something.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::Heat(-2), CardEffect::DrawCards(1)]),
                    None,
                )
                .rarity(CardRarity::Public)
                .build(),
            1,
        ),
    ]
}
