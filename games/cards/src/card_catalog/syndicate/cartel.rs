//! Cartel: rackets that print cash, then bombs nobody else can afford.

use super::{CARTEL, crew, job, on, racket, trick};
use crate::card::{Card, CardEffect, TokenSpec};
use crate::types::{CardRarity, Keyword, ManaType, TriggerEvent, UnitType};

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_stash_house", "Stash House", 3, 2, CARTEL)
                .description("Bricks in the walls. Everyone on the street knows.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(2)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            racket("syn_lab", "Lab", 3, 2, CARTEL)
                .description("Cook fast. Sell faster.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            racket("syn_shipping_front", "Shipping Front", 4, 3, CARTEL)
                .description("Bananas in, bananas out. The manifests are perfect.")
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_mule", "Mule", 1, 1, 1, CARTEL)
                .description("Swallowed it. Crossed. Got paid.")
                .trigger(on(TriggerEvent::Died, CardEffect::DrawCards(1)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_cobrador", "Cobrador", 2, 2, 1, CARTEL)
                .description("You pay him, or you pay him.")
                .trigger(on(TriggerEvent::DealtDamage, CardEffect::AddMana(ManaType::Colorless, 1)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_lavador", "Lavador", 3, 2, 2, CARTEL)
                .description("Dirty in the front, clean out the back.")
                .trigger(on(TriggerEvent::Entered, CardEffect::DrawCards(1)))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_plaza_boss", "Plaza Boss", 4, 4, 3, CARTEL)
                .description("The plaza is his. Ask anyone still breathing.")
                .keyword(Keyword::Menace)
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(1)))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_jefe_de_sicarios", "Jefe de Sicarios", 5, 3, 3, CARTEL)
                .description("He does not carry a gun. He carries two men who do.")
                .trigger(on(
                    TriggerEvent::Entered,
                    CardEffect::Sequence(vec![
                        CardEffect::CreateToken(TokenSpec::new("Sicario", 2, 2, UnitType::Human)),
                        CardEffect::CreateToken(TokenSpec::new("Sicario", 2, 2, UnitType::Human)),
                        CardEffect::Heat(1),
                    ]),
                ))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            crew("syn_kingpin", "Kingpin", 6, 6, 6, CARTEL)
                .description("El Patron does not visit. El Patron arrives.")
                .keyword(Keyword::Trample)
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(2)))
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            job("syn_plata_o_plomo", "Plata o Plomo", 3, CARTEL)
                .description("Silver or lead. Most choose silver.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::StealMana(ManaType::Colorless, 2),
                        CardEffect::Heat(1),
                    ]),
                    None,
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            job("syn_skim_the_take", "Skim the Take", 4, CARTEL)
                .description("Their count came up short. Yours came up long.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::StealMana(ManaType::Colorless, 2),
                        CardEffect::DrawCards(1),
                    ]),
                    None,
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            job("syn_pay_the_informant", "Pay the Informant", 3, CARTEL)
                .description("He talks to you. He talks to them too.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DrawCards(2), CardEffect::Heat(1)]),
                    None,
                )
                .rarity(CardRarity::Private)
                .build(),
            1,
        ),
        (
            trick("syn_mordida", "Mordida", 1, CARTEL)
                .description("A little bite for the comandante. The report goes away.")
                .effect(CardEffect::Heat(-3), None)
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
    ]
}
