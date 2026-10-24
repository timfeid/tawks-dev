//! The Firm: lawyers, hackers, fixers and crooked cops. Cool heat, control the board.

use super::{FIRM, crew, job, on, racket, trick};
use crate::card::TargetSpec;
use crate::card::{Card, CardEffect};
use crate::types::{CardRarity, Keyword, TargetKind, TargetScope, TriggerEvent};

fn opponent_unit() -> Option<TargetSpec> {
    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1))
}

pub fn all_cards() -> Vec<(Card, i32)> {
    vec![
        (
            racket("syn_precinct_contact", "Precinct Contact", 2, 1, FIRM)
                .description("Reports filed. Reports lost. Invoice attached.")
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            racket("syn_law_office", "Law Office", 3, 2, FIRM)
                .description("Billable hours. Retainer due on signing.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(-2)))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            racket("syn_data_center", "Data Center", 3, 3, FIRM)
                .description("Everyone's records. Nobody's name on the lease.")
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_paralegal", "Paralegal", 2, 1, 4, FIRM)
                .description("Files the motion. Buries the evidence. Same drawer.")
                .trigger(on(TriggerEvent::Entered, CardEffect::Heat(-1)))
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            crew("syn_informant", "Informant", 2, 2, 3, FIRM)
                .description("Talks to everyone. Reports to us.")
                .trigger(on(TriggerEvent::DealtDamage, CardEffect::HeatOpponents(1)))
                .rarity(CardRarity::Public)
                .build(),
            3,
        ),
        (
            crew("syn_crooked_cop", "Crooked Cop", 3, 3, 4, FIRM)
                .description("Badge on the desk. Envelope under it.")
                .keyword(Keyword::Camouflage)
                .trigger(on(TriggerEvent::TurnStart, CardEffect::Heat(-1)))
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_black_hat", "Black Hat", 3, 2, 3, FIRM)
                .description("Your files were deleted. Per policy.")
                .effect(CardEffect::Discard(1), None)
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_fixer", "Fixer", 3, 3, 2, FIRM)
                .description("Arrives unannounced. Leaves a problem resolved.")
                .keyword(Keyword::Flash)
                .effect(CardEffect::TapTarget, opponent_unit())
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_senior_partner", "Senior Partner", 4, 5, 5, FIRM)
                .description("Name on the door. Never on the paperwork.")
                .keyword(Keyword::Vigilance)
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            crew("syn_the_closer", "The Closer", 5, 4, 5, FIRM)
                .description("Settlement reached. Party removed from proceedings.")
                .effect(CardEffect::Bounce, opponent_unit())
                .rarity(CardRarity::Restricted)
                .build(),
            1,
        ),
        (
            job("syn_frame_job", "Frame Job", 2, FIRM)
                .description("Wrong prints. Right suspect.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::HeatOpponents(2), CardEffect::Discard(1)]),
                    None,
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
        (
            job("syn_subpoena", "Subpoena", 3, FIRM)
                .description("Served. Detained. Case pending.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::Heat(1)]),
                    opponent_unit(),
                )
                .rarity(CardRarity::Private)
                .build(),
            2,
        ),
        (
            trick("syn_cease_and_desist", "Cease and Desist", 1, FIRM)
                .description("Effective immediately. See attached.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::TapTarget, CardEffect::Heat(-1)]),
                    opponent_unit(),
                )
                .rarity(CardRarity::Public)
                .build(),
            2,
        ),
    ]
}
