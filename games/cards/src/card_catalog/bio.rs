use crate::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, StatModifierSpec,
    TargetSpec, TokenSpec,
};
use crate::types::{
    CardKind, CardRarity, CardSupertype, Condition, EffectTargetSpec, ExpireContract, GrantTarget,
    Keyword, ManaCost, ManaType, ModifierAmount, Predicate, StatType, TargetKind, TargetScope,
    TriggerEvent, TriggerLimit, UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // 6 OPERATORS (3 Apex Units + 3 Apex Agents)

    // 3 APEX UNIT OPERATORS
    cards.push((
        CardBuilder::new(
            "xylo_operator",
            "Xylo, the Infinite Root",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(5, 5)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/xylo_operator_1772751873.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "verdant_warlord",
            "Verdant Warlord",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(2, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/verdant_warlord_1772751819.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "symbiote_channeler",
            "Symbiote Channeler",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(4, 5)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::ApplyStatModifiers(vec![
                StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::OwnerUnits(None),
                },
                StatModifierSpec {
                    stat: StatType::Toughness,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::OwnerUnits(None),
                },
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/symbiote_channeler_1772748285.png")
        .build(),
        1,
    ));

    // 3 APEX AGENT OPERATORS
    cards.push((
        CardBuilder::new(
            "sylva_voice_of_growth",
            "Sylva, Voice of Growth",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(6)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "nissa_plus_1".to_string(),
            name: "[+1] Create Spore".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "nissa_minus_2".to_string(),
            name: "[-2] Anthem".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                ]),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/sylva_voice_of_growth_1772748285.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "verdania_natures_fury",
            "Verdania, Nature's Fury",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(6)
        .cost(ManaCost {
            bio: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Megafauna", 2, 2, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "verdania_plus_2".to_string(),
            name: "[+1] Create Spore".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "verdania_minus_2".to_string(),
            name: "[-3] Destroy Target".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DestroyTarget,
            ]),
            target_spec: Some(TargetSpec {
                scope: TargetScope::Any,
                kind: TargetKind::Unit,
                max_targets: 1,
                optional: false,
                predicate: None,
            }),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/verdania_natures_fury_1772751806.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "briar_champion_of_wilds",
            "Briar, Champion of Wilds",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(5)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new(
                "Megafauna",
                1,
                1,
                UnitType::BioOrganism,
            )),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "vivien_plus_1".to_string(),
            name: "[+1] Create Megafauna".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Megafauna", 1, 1, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "vivien_minus_3".to_string(),
            name: "[-2] Summon Megafauna".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::CreateToken(TokenSpec::new("Megafauna", 3, 3, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "vivien_minus_5".to_string(),
            name: "[-5] Wild Surge".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-5),
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(2),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(2),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                ]),
                CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/briar_champion_of_wilds_1772747639.png")
        .build(),
        1,
    ));

    // 8 CHEAP CARDS (1-2 mana)
    cards.push((
        CardBuilder::new(
            "tendril_sapper",
            "Tendril Sapper",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 2)
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/tendril_sapper_1772751730.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "mycelium_weaver",
            "Mycelium Weaver",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/mycelium-weaver_1774298275.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("spore_sentinel", "Spore Sentinel", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DrawCards(1),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/spore_sentinel_1772748255.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "bloom_warden",
            "Bloom Warden",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::GainLife(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Interceptor)
        .keyword(Keyword::Absorb(1))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/bloom_warden_1772747641.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "nest_sprouter",
            "Nest Sprouter",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::GainLife(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/nest_sprouter_1772748003.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("symbiotic_shield", "Gene Bulwark", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/symbiotic_shield_1772748297.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("root_expansion", "Root Expansion", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::SearchToBattlefield(Predicate::Condition(Condition::TargetIsUnitType(
                    UnitType::BioOrganism,
                ))),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/natures_lore_1772224444.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("root_memory", "Root Memory", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::SearchGraveyard(Predicate::CardKind(CardKind::Unit(
                        UnitType::BioOrganism,
                    ))),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/root_memory_1774307344.png")
            .build(),
        1,
    ));

    // 56 OTHER CARDS (includes 5 more legends + 2 more agents + anthems, mana, tokens, finishers) - 3+ mana

    // 5 MORE APEX UNITS
    cards.push((
        CardBuilder::new(
            "spore_blade_captain",
            "Spore Blade Captain",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Attacked,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/spore_blade_captain_1774307388.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "rhizo_the_doubled",
            "Rhizo, the Doubled",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 1)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .tap_effect(CardEffect::CreateToken(TokenSpec::new(
            "Spore",
            1,
            1,
            UnitType::BioOrganism,
        )))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/rhizo_the_doubled_1772748126.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "flora_explorer",
            "Flora, Reclaimed Explorer",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/flora_explorer_1772747836.png")
        .build(),
        1,
    ));

    // 2 MORE AGENTS
    cards.push((
        CardBuilder::new(
            "thorn_primal_hunter",
            "Thorn, Primal Hunter",
            CardKind::Agent,
        )
        .loyalty(3)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "garruk_plus_1".to_string(),
            name: "[+1] Create Megafauna".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Megafauna", 1, 1, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "garruk_minus_3".to_string(),
            name: "[-3] Draw Cards".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DrawCards(2),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/thorn_primal_hunter_1772751745.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("gaia_who_awakens", "Gaia, Who Awakens", CardKind::Agent)
            .loyalty(4)
            .cost(ManaCost {
                bio: 3,
                colorless: 3,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "nissa_plus_1".to_string(),
                name: "[+1] Spore".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "nissa_minus_8".to_string(),
                name: "[-6] Wild Surge".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-6),
                    CardEffect::CreateToken(TokenSpec::new("Megafauna", 4, 4, UnitType::Megafauna)),
                    CardEffect::CreateToken(TokenSpec::new("Megafauna", 4, 4, UnitType::Megafauna)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/gaia_who_awakens_1772747896.png")
            .build(),
        1,
    ));

    // 8 ANTHEM UNITS - stack for massive buffs
    cards.push((
        CardBuilder::new(
            "verdant_matriarch",
            "Verdant Matriarch",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/verdant_matriarch_1772751800.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("mycelium_archdruid", "Mycelial Bloom", CardKind::Aura)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/mycelium_archdruid_1772747986.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "spore_warcaller",
            "Spore Warcaller",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::ApplyStatModifiers(vec![
                StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                },
                StatModifierSpec {
                    stat: StatType::Toughness,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                },
            ]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/spore_warcaller_1772748260.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "canopy_strategist",
            "Canopy Strategist",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 4,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/canopy_strategist_1772747658.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "thornveil_elder",
            "Thornveil Elder",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 4)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Interceptor)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(None),
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/thornveil_elder_1772751769.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "toxic_stalker",
            "Toxic Stalker",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/toxic_stalker_1772751764.png")
        .build(),
        1,
    ));
    // EVASION UNITS - carry anthem buffs through unblocked
    cards.push((
        CardBuilder::new(
            "spore_scout",
            "Spore Scout",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::AddMana(ManaType::Bio, 1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/spore_scout_1772748256.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "bark_protector",
            "Bark Protector",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 4)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/bark_protector_1775259987.png")
        .build(),
        1,
    ));

    // 4 MANA SCALERS - tap for Bio per unit
    cards.push((
        CardBuilder::new("priest_of_the_grove", "Viral Propagator", CardKind::Aura)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Spore",
                    1,
                    1,
                    UnitType::BioOrganism,
                )),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/priest_of_the_grove_1772748067.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "circle_of_roots",
            "Circle of Roots",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/circle_of_roots_1772747687.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "rootwire_channeler",
            "Rootwire Channeler",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 2)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/rootwire_channeler_1772748141.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "grove_elder",
            "Grove Elder",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticTapForMana(
            ManaType::Bio,
            1,
            GrantTarget::mine_of(UnitType::BioOrganism),
        ))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/grove_elder_1772747908.png")
        .build(),
        1,
    ));

    // 9 TOKEN GENERATORS
    cards.push((
        CardBuilder::new(
            "grove_huntmaster",
            "Grove Huntmaster",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Megafauna", 2, 2, UnitType::Megafauna)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/grove_huntmaster_1772747911.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "thornback_riders",
            "Thornback Riders",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .trigger(CardTrigger {
            event: TriggerEvent::Upkeep,
            effect: CardEffect::Sequence(vec![CardEffect::CreateToken(TokenSpec::new(
                "Spore",
                1,
                1,
                UnitType::BioOrganism,
            ))]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/thornback_riders_1772751752.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("spore_promenade", "Spore Promenade", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/spore_promenade_1772748243.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "presence_of_growth",
            "Presence of Growth",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .tap_effect(CardEffect::CreateToken(TokenSpec::new(
            "Spore",
            1,
            1,
            UnitType::BioOrganism,
        )))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/presence_of_growth_1772748066.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("spontaneous_budding", "Spore Caller", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/spontaneous_budding_1772748243.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("wild_propagator", "Wild Propagator", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/wild_propagator_1772751854.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("deep_grove_hermit", "Deep Grove Hermit", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/deep_grove_hermit_1772747759.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("beacon_of_bloom", "Beacon of Bloom", CardKind::Relic)
            .cost(ManaCost {
                bio: 2,
                colorless: 2,
                ..Default::default()
            })
            .tap_effect(CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                ]),
            ]))
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/beacon_of_bloom_1772747625.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "imperious_cultivar",
            "Imperious Cultivar",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .tap_effect(CardEffect::CreateToken(TokenSpec::new(
            "Spore",
            1,
            1,
            UnitType::BioOrganism,
        )))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/imperious_cultivar_1772747941.png")
        .build(),
        1,
    ));

    // 10 CARD DRAW / TUTORS
    cards.push((
        CardBuilder::new(
            "fauna_whisperer",
            "Fauna Whisperer",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(None),
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/fauna_whisperer_1772747818.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("regal_overgrowth", "Regal Overgrowth", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                    },
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/regal_overgrowth_1772748121.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "soul_of_the_canopy",
            "Soul of the Canopy",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(4, 4)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(None),
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/soul_of_the_canopy_1772748231.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("canopy_synthesis", "Canopy Synthesis", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DrawCards(2),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/harmonize_1772223748.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("deep_rooting", "Deep Rooting", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/cultivate_1772223073.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("chord_of_the_wild", "Chord of the Wild", CardKind::Instant)
            .cost(ManaCost {
                bio: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Search(Predicate::Any(vec![
                    Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism)),
                    Predicate::CardKind(CardKind::Unit(UnitType::Megafauna)),
                ])),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/chord_calling_1772144019.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("verdant_zenith", "Verdant Zenith", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Search(Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism))),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/verdant_zenith_1774307438.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("mycorrhizal_recall", "Mycorrhizal Recall", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::SearchGraveyard(Predicate::CardKind(CardKind::Unit(
                        UnitType::BioOrganism,
                    ))),
                    CardEffect::DrawCards(1),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/mycorrhizal_recall_1774307268.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("perennial_witness", "Perennial Witness", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::SearchGraveyard(Predicate::CardKind(CardKind::Unit(
                        UnitType::BioOrganism,
                    ))),
                    CardEffect::DrawCards(1),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/perennial_witness_1774307294.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("rootseeker", "Rootseeker", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Search(Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism))),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/rootseeker_1772748142.png")
            .build(),
        1,
    ));

    // 5 RAMP/MANA
    cards.push((
        CardBuilder::new(
            "pollen_tender",
            "Pollen Tender",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/pollen_tender_1772748067.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "oracle_of_deep_roots",
            "Oracle of Deep Roots",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/oracle_of_deep_roots_1772748031.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "azura_the_seeking",
            "Azura, the Seeking",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 2)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::StaticTapForMana(
            ManaType::Bio,
            1,
            GrantTarget::mine_of(UnitType::BioOrganism),
        ))
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/azura_the_seeking_1772747613.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "seedborn_awakener",
            "Rootborn Awakener",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 4)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::UntapDuringOtherTurns)
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/seedborn_awakener_1772748177.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "primeval_colossus",
            "Primal Colossus",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(5, 5)
        .cost(ManaCost {
            bio: 3,
            colorless: 4,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/primeval_colossus_1772748080.png")
        .build(),
        1,
    ));

    // 5 REMOVAL/UTILITY
    cards.push((
        CardBuilder::new(
            "consumed_by_growth",
            "Consumed by Growth",
            CardKind::Instant,
        )
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::Fight,
            Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 2)),
        )
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/consumed_by_growth_1774307136.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "reclamation_tendril",
            "Reclamation Tendril",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::DestroyTarget,
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            None,
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/reclamation_tendril_1774307329.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("terravore", "Viral Assimilation", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::Assimilate(UnitType::BioOrganism),
                Some(
                    TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)
                        .with_predicate(Predicate::Condition(Condition::TargetIsTapped)),
                ),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/terravore_1774298369.png")
            .build(),
        1,
    ));
    // FIGHT REMOVAL
    cards.push((
        CardBuilder::new("predatory_strike", "Predatory Strike", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Fight,
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(Some(UnitType::BioOrganism)),
                        },
                    ]),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 2)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/predatory_strike_1772224800.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("bane_of_circuitry", "Bane of Circuitry", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DestroyTarget,
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/bane_of_circuitry_1774299253.png")
            .build(),
        1,
    ));

    // 10 BIG FINISHERS
    cards.push((
        CardBuilder::new(
            "apex_stampede",
            "Apex Stampede",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(5, 5)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                ]),
                CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/apex_stampede_1774299254.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "end_growth_forerunners",
            "End-Growth Forerunners",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(4, 4)
        .cost(ManaCost {
            bio: 3,
            colorless: 5,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(1),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                ]),
                CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                CardEffect::GrantKeyword(Keyword::Vigilance, ExpireContract::EndOfTurn),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/end_growth_forerunners_1772747801.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "champion_of_the_thicket",
            "Mesh Guardian",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::StaticStealthGrant(GrantTarget::mine_of(
            UnitType::BioOrganism,
        )))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/champion_of_the_thicket_1772747674.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "avenger_of_the_grove",
            "Warden of the Grove",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(4, 4)
        .cost(ManaCost {
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Rhizome", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(TokenSpec::new("Rhizome", 1, 1, UnitType::BioOrganism)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/avenger_of_the_grove_1772747601.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "rampaging_rootbeasts",
            "Rampaging Rootbeasts",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(5, 5)
        .cost(ManaCost {
            bio: 3,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Megafauna", 2, 2, UnitType::Megafauna)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/rampaging_rootbeasts_1772748111.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "venomous_spore",
            "Venomous Spore",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .keyword(Keyword::Deathtouch)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/venomous_spore_1772751799.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("spore_shell", "Spore Shell", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(3),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                    ]),
                    CardEffect::GrantKeyword(Keyword::Camouflage, ExpireContract::EndOfTurn),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/spore_shell_1772225415.png")
            .build(),
        1,
    ));
    // PROTECTION INSTANTS - keep key units alive
    cards.push((
        CardBuilder::new("fungal_ward", "Fungal Ward", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                    ]),
                    CardEffect::GrantKeyword(Keyword::Camouflage, ExpireContract::EndOfTurn),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/fungal_ward_1774307226.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "verdant_titan",
            "Verdant Titan",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(5, 5)
        .cost(ManaCost {
            bio: 3,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Upkeep,
            effect: CardEffect::CreateToken(TokenSpec::new("Megafauna", 2, 2, UnitType::Megafauna)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/verdant_titan_1774307438.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("overgrowth_veil", "Overgrowth Veil", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(4),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(4),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::Targets,
                        },
                    ]),
                    CardEffect::GrantKeyword(Keyword::Camouflage, ExpireContract::EndOfTurn),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/overgrowth_veil_1772224678.png")
            .build(),
        1,
    ));

    // No basic infrastructure — The Grid provides mana automatically

    // ── NEW CARDS ────────────────────────────────────────────────────────────

    // Spore Sovereign — Singularity Apex Megafauna
    cards.push((
        CardBuilder::new(
            "spore_sovereign",
            "Spore Sovereign",
            CardKind::Unit(UnitType::Megafauna),
        )
        .supertype(CardSupertype::Apex)
        .stats(2, 3)
        .cost(ManaCost {
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Evolve)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/spore_sovereign_1774299293.png")
        .build(),
        1,
    ));

    // Mycorrhizal Web — Enchantment
    cards.push((
        CardBuilder::new("mycorrhizal_web", "Mycorrhizal Web", CardKind::Aura)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
                effect: CardEffect::Scavenge(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .ability(CardAbility {
                id: "mycorrhizal_search".to_string(),
                name: "Search".to_string(),
                cost: AbilityCost::new()
                    .with_tap()
                    .with_cost(crate::card::Cost::Sacrifice(1)),
                effect: CardEffect::Search(Predicate::IsUnit),
                target_spec: None,
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/mycorrhizal-web_1774298290.png")
            .build(),
        1,
    ));

    // Bramble Seedling — turn-1 wall for Bio. 1/3 Interceptor for 1
    // mana. Low power so not an efficient attacker; solid toughness and
    // natural Interceptor so Bio has a real early answer to Neon aggro.
    cards.push((
        CardBuilder::new(
            "bramble_seedling",
            "Bramble Seedling",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 2)
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .keyword(Keyword::Interceptor)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/canopy_veil_1774299266.png")
        .build(),
        3,
    ));

    // Canopy Veil — aura that grants Interceptor to bio creatures
    cards.push((
        CardBuilder::new(
            "canopy_veil",
            "Thornwood Guardian",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::SelfPowerPerKind(
            1,
            CardKind::Unit(UnitType::BioOrganism),
        ))
        .keyword(Keyword::SelfToughnessPerKind(
            1,
            CardKind::Unit(UnitType::BioOrganism),
        ))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/canopy_veil_1774299266.png")
        .build(),
        1,
    ));

    // Adaptive Polyp — small Evolve creature
    cards.push((
        CardBuilder::new(
            "adaptive_polyp",
            "Adaptive Polyp",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Evolve)
        .trigger(CardTrigger {
            event: TriggerEvent::ReceivedDamage,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/adaptive-polyp_1774298151.png")
        .build(),
        1,
    ));

    // Bloom Burst — token + scavenge sorcery
    cards.push((
        CardBuilder::new("bloom_burst", "Bloom Burst", CardKind::Ritual)
            .cost(ManaCost {
                bio: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 2, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 2, UnitType::BioOrganism)),
                    CardEffect::Scavenge(1),
                ]),
                Some(TargetSpec::new(
                    TargetScope::SelfPlayer,
                    TargetKind::Unit,
                    1,
                )),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/bloom-burst_1774298165.png")
            .build(),
        1,
    ));

    // Root Graft — combat trick instant
    cards.push((
        CardBuilder::new("root_graft", "Root Graft", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::AddCounters(2), CardEffect::Scavenge(1)]),
                Some(TargetSpec::new(
                    TargetScope::SelfPlayer,
                    TargetKind::Unit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/root-graft_1774298328.png")
            .build(),
        1,
    ));

    // ── Multi-color operators ────────────────────────────────────────────────

    // Rootweaver Drone (bio/chrome) — Unit
    cards.push((
        CardBuilder::new(
            "rootweaver_drone",
            "Rootweaver Drone",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 4)
        .cost(ManaCost {
            bio: 2,
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .keyword(Keyword::Vigilance)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/rootweaver_drone_1777083920.png")
        .build(),
        1,
    ));

    // Rot Sporelord (bio/glitch) — Agent
    cards.push((
        CardBuilder::new("rot_sporelord", "Rot Sporelord", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(5)
            .cost(ManaCost {
                bio: 2,
                glitch: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Spore",
                    1,
                    1,
                    UnitType::BioOrganism,
                )),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .ability(CardAbility {
                id: "rot_sporelord_plus_1".to_string(),
                name: "[+1] Create Spore".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "rot_sporelord_minus_3".to_string(),
                name: "[-3] Discard".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-3),
                    CardEffect::Discard(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Singularity)
            .image("https://cdn.tawks.app/cards/rot_sporelord_1777083919.png")
            .build(),
        1,
    ));

    cards
}
