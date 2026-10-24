use crate::card::{
    AbilityCost, AttachmentSpec, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, Cost,
    StatModifierSpec, TargetSpec, TokenSpec,
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
            "furnace_diva",
            "Furnace Diva",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/furnace_diva_1772747874.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ignitor_god_of_the_forge",
            "Ignitor, God of the Forge",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(6, 6)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/ignitor_god_of_the_forge_1772747923.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "mayhem_forgefiend",
            "Mayhem Forgefiend",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 3)
        .cost(ManaCost {
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/mayhem_forgefiend_1772747956.png")
        .build(),
        1,
    ));

    // 3 APEX AGENT OPERATORS

    cards.push((
        CardBuilder::new(
            "pyra_torch_of_defiance",
            "Pyra, Torch of Defiance",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(5)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "pyra_torch_plus_1".to_string(),
            name: "[+1] Scry".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![CardEffect::ModifyLoyalty(1), CardEffect::Scry(1)]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "pyra_torch_minus_3".to_string(),
            name: "[-3] Deal 5".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DamageTarget(5),
            ]),
            target_spec: Some(TargetSpec::new(
                TargetScope::Any,
                TargetKind::PlayerOrUnit,
                1,
            )),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "pyra_torch_minus_7".to_string(),
            name: "[-7] Cast Free".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-7),
                CardEffect::DrawCards(5),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/pyra_torch_of_defiance_1772748096.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "pyra_awakened_inferno",
            "Pyra, Awakened Inferno",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(5)
        .cost(ManaCost {
            slag: 2,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "pyra_awakened_plus_2".to_string(),
            name: "[+1] Create Ember".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "pyra_awakened_minus_3".to_string(),
            name: "[-3] Deal 1 to All".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DamagePlayer(1),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "pyra_awakened_minus_x".to_string(),
            name: "[-5] Deal 2 to Each".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-5),
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(-2),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::AllUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(-2),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::AllUnits(None),
                    },
                ]),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/pyra_awakened_inferno_1772748098.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "rivett_scrap_savant",
            "Rivett, Scrap Savant",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(4)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "daretti_plus_2".to_string(),
            name: "[+1] Surveil 2".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::Surveil(2),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "daretti_minus_2".to_string(),
            name: "[-2] Return Artifact".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::SearchGraveyard(Predicate::Any(vec![
                    Predicate::CardKind(CardKind::Relic),
                    Predicate::CardKind(CardKind::Relic),
                ])),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "daretti_minus_10".to_string(),
            name: "[-10] Animate Artifacts".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-10),
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(5),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OwnerUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(5),
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
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/rivett_scrap_savant_1772748139.png")
        .build(),
        1,
    ));

    // 8 CHEAP CARDS (1-2 mana)

    cards.push((
        CardBuilder::new("scrap_launcher", "Scrap Launcher", CardKind::Aura)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "sacrifice_for_damage".to_string(),
                name: "Sacrifice for Damage".to_string(),
                cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::DamageTarget(3),
                target_spec: Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/scrap_launcher_1772748174.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "slag_feeder",
            "Slag Feeder",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "sacrifice_for_buff".to_string(),
            name: "Sacrifice for +1/+1".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::ApplyStatModifiers(vec![
                StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                },
                StatModifierSpec {
                    stat: StatType::Toughness,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                },
            ]),
            target_spec: Some(TargetSpec::new(
                TargetScope::SelfPlayer,
                TargetKind::Unit,
                1,
            )),
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Wither)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/slag_feeder_1772748216.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "furnace_seer",
            "Furnace Seer",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "furnace_seer_pinger".to_string(),
            name: "Tap: Deal 1".to_string(),
            cost: AbilityCost::new().with_tap(),
            effect: CardEffect::DamageTarget(1),
            target_spec: Some(TargetSpec::new(
                TargetScope::Any,
                TargetKind::PlayerOrUnit,
                1,
            )),
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/furnace_seer_1772747875.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "siphon_welder",
            "Siphon Welder",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::DamagePlayer(1),
                CardEffect::GainLife(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/siphon_welder_1772748200.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "exhaust_cutthroat",
            "Exhaust Cutthroat",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/exhaust_cutthroat_1772747815.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("reckless_salvage", "Reckless Salvage", CardKind::Ritual)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .effect(CardEffect::Loot(2, 1), None)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/reckless-salvage_1774298315.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("arc_discharge", "Arc Discharge", CardKind::Instant)
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageTarget(3),
                    CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
                ]),
                Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/lightning_bolt_1772224045.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("jolt", "Jolt", CardKind::Instant)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                    stat: StatType::Both,
                    amount: ModifierAmount::Fixed(-1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                }]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/jolt_1774307238.png")
            .build(),
        1,
    ));

    // 56 OTHER CARDS - 3+ mana

    // 2 APEX UNITS (remaining)

    cards.push((
        CardBuilder::new(
            "ember_thane",
            "Ember Thane",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 4)
        .cost(ManaCost {
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::StaticPowerBuff(2, None))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/ember_thane_1774307183.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "scrapboss_mob_king",
            "Scrapboss, Mob King",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .tap_effect(CardEffect::Sequence(vec![
            CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
        ]))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/scrapboss_mob_king_1772748158.png")
        .build(),
        1,
    ));

    // 2 AGENTS (remaining)

    cards.push((
        CardBuilder::new(
            "vulcan_archduke_of_slag",
            "Vulcan, Archduke of Slag",
            CardKind::Agent,
        )
        .loyalty(3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "zariel_plus_1".to_string(),
            name: "[+1] Create Cinder".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Cinder", 1, 1, UnitType::Anomaly)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "zariel_minus_2".to_string(),
            name: "[-2] Buff & Haste".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::ApplyStatModifier(StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::OwnerUnits(None),
                }),
                CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "zariel_minus_6".to_string(),
            name: "[-6] Exile & Damage".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-6),
                CardEffect::DamagePlayer(5),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/vulcan_archduke_of_slag_1772751835.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "anvil_of_the_hammer",
            "Anvil of the Hammer",
            CardKind::Agent,
        )
        .loyalty(3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "koth_plus_1".to_string(),
            name: "[+1] Animate Land".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::AddMana(ManaType::Slag, 1),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "koth_minus_2".to_string(),
            name: "[-2] Deal Damage".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::DamageTarget(3),
            ]),
            target_spec: Some(TargetSpec::new(
                TargetScope::Any,
                TargetKind::PlayerOrUnit,
                1,
            )),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "koth_minus_5".to_string(),
            name: "[-5] Big Damage".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-5),
                CardEffect::DamageTarget(7),
            ]),
            target_spec: Some(TargetSpec::new(
                TargetScope::Any,
                TargetKind::PlayerOrUnit,
                1,
            )),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/koth_hammer_1772223992.png")
        .build(),
        1,
    ));

    // 10 DEATH TRIGGER PAYOFFS

    cards.push((
        CardBuilder::new("bastion_of_embers", "Bastion of Embers", CardKind::Aura)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Sacrificed,
                effect: CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/bastion_of_embers_1772747626.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "cruel_smelter",
            "Cruel Smelter",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::Sequence(vec![
                CardEffect::DamagePlayer(1),
                CardEffect::GainLife(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/cruel_smelter_1772747735.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "foundry_noble",
            "Foundry Noble",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(4, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/foundry_noble_1772747860.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "molten_tip_archer",
            "Molten-Tip Archer",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 4)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Deathtouch)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::DamageTarget(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/molten_tip_archer_1772747968.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "vindictive_furnaceman",
            "Vindictive Furnaceman",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::AddMana(ManaType::Slag, 1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/vindictive_furnaceman_1772751823.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syr_konrad_the_molten",
            "Konrad, the Molten",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(5, 4)
        .cost(ManaCost {
            slag: 3,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::DamagePlayer(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syr_konrad_the_molten_1772751731.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("impact_sparks", "Impact Sparks", CardKind::Aura)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(None),
                effect: CardEffect::DamageTarget(2),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/impact_tremors_1772223828.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("pandemonium_foundry", "Havoc Foundry", CardKind::Aura)
            .cost(ManaCost {
                slag: 2,
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(None),
                effect: CardEffect::DamagePlayer(3),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/pandemonium_1772224704.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("outpost_furnace", "Outpost Furnace", CardKind::Aura)
            .cost(ManaCost {
                slag: 2,
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AllyDied,
                effect: CardEffect::DamageTarget(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/outpost_furnace_1774307280.png")
            .build(),
        1,
    ));

    // 10 SACRIFICE OUTLETS

    cards.push((
        CardBuilder::new("smelting_altar", "Smelting Altar", CardKind::Relic)
            .cost(ManaCost {
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "ashnod_sac".to_string(),
                name: "Sacrifice for Mana".to_string(),
                cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::AddMana(ManaType::Colorless, 2),
                target_spec: None,
                once_per_turn: false,
                uses_stack: false,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "smelt_relic".to_string(),
                name: "Sacrifice to Smelt Relic".to_string(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::DestroyTarget,
                target_spec: Some(
                    TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)
                        .with_predicate(Predicate::CardKind(CardKind::Relic)),
                ),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/smelting_altar_1774307377.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("combustion_altar", "Combustion Altar", CardKind::Relic)
            .cost(ManaCost {
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "phyrexian_sac".to_string(),
                name: "Sacrifice to Burn".to_string(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::DamageTarget(2),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::PlayerOrUnit, 1)),
                once_per_turn: false,
                uses_stack: false,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/combustion_altar_1774307124.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("altar_of_slag", "Altar of Slag", CardKind::Relic)
            .cost(ManaCost {
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "altar_dementia_sac".to_string(),
                name: "Sacrifice to Burn".to_string(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::DamagePlayer(3),
                target_spec: Some(TargetSpec {
                    scope: TargetScope::Opponent,
                    kind: TargetKind::Player,
                    max_targets: 1,
                    optional: false,
                    predicate: None,
                }),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/altar_of_slag_1772747582.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "woe_stoker",
            "Woe Stoker",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Scrap", 0, 1, UnitType::Megafauna)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "woe_strider_scry".to_string(),
            name: "Sacrifice to Scry".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Scry(2),
            target_spec: None,
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/woe_stoker_1772751849.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "undying_forgemaster",
            "Undying Forgemaster",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "yahenni_sac".to_string(),
            name: "Sacrifice for Indestructible".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
            target_spec: None,
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/undying_forgemaster_1772751785.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "yawg_thran_engineer",
            "Yawg, Scrap Engineer",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "yawg_sac".to_string(),
            name: "Sacrifice to Weaken and Draw".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Sequence(vec![
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(-1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::Targets,
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(-1),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::Targets,
                    },
                ]),
                CardEffect::DrawCards(1),
            ]),
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/yawg_thran_engineer_1772751871.png")
        .build(),
        1,
    ));
    let mut slag_clamp = CardBuilder::new("slag_clamp", "Slag Clamp", CardKind::Relic)
        .cost(ManaCost {
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "skullclamp_equip".to_string(),
            name: "Equip".to_string(),
            cost: AbilityCost::new().with_mana(ManaCost {
                colorless: 1,
                ..Default::default()
            }),
            effect: CardEffect::Equip,
            target_spec: Some(TargetSpec::new(
                TargetScope::SelfPlayer,
                TargetKind::Unit,
                1,
            )),
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/slag_clamp_1772748203.png")
        .build();
    slag_clamp.attachment = Some(AttachmentSpec {
        modifiers: vec![
            StatModifierSpec {
                stat: StatType::Power,
                amount: ModifierAmount::Fixed(1),
                expires: ExpireContract::Never,
                target: EffectTargetSpec::Targets,
            },
            StatModifierSpec {
                stat: StatType::Toughness,
                amount: ModifierAmount::Fixed(-1),
                expires: ExpireContract::Never,
                target: EffectTargetSpec::Targets,
            },
        ],
        keywords: vec![],
    });
    slag_clamp.auto_description = slag_clamp.auto_description();
    slag_clamp.description = slag_clamp.auto_description.clone();
    cards.push((slag_clamp, 1));
    cards.push((
        CardBuilder::new("culling_crucible", "Culling Crucible", CardKind::Relic)
            .cost(ManaCost {
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "culling_crucible_burn".to_string(),
                name: "Cull for Burn".to_string(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .with_tap()
                    .with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::DamageTarget(3),
                target_spec: Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/culling_crucible_1772747735.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("forge_harvest", "Forge Harvest", CardKind::Aura)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "greater_good_sac".to_string(),
                name: "Sacrifice for Cards".to_string(),
                cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::Loot(3, 1),
                target_spec: None,
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/forge_harvest_1774307213.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("combustive_leap", "Combustive Leap", CardKind::Aura)
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "evolutionary_leap_ability".to_string(),
                name: "Sacrifice to Search".to_string(),
                cost: AbilityCost::new()
                    .with_cost(Cost::Sacrifice(1))
                    .with_mana(ManaCost {
                        slag: 1,
                        ..Default::default()
                    }),
                effect: CardEffect::Search(Predicate::IsUnit),
                target_spec: None,
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/combustive_leap_1774307138.png")
            .build(),
        1,
    ));

    // 10 RECURSIVE UNITS (come back from grave)

    cards.push((
        CardBuilder::new(
            "reassembling_scrap",
            "Reassembling Scrap",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 1)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(
                TokenSpec::new("Scrap Heap", 1, 2, UnitType::Anomaly).with_triggers(vec![
                    CardTrigger {
                        event: TriggerEvent::Upkeep,
                        effect: CardEffect::Sequence(vec![
                            CardEffect::SacrificeThis,
                            CardEffect::ReanimateNamedCard("Reassembling Scrap".to_string()),
                        ]),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    },
                ]),
            ),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/reassembling_scrap_1772748110.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ember_revenant",
            "Ember Revenant",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::GainLife(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/ember_revenant_1772747787.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "slag_crawler",
            "Slag Crawler",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 1)
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .keyword(Keyword::Deathtouch)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/slag_crawler_1772748217.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "furnace_traitor",
            "Furnace Traitor",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/furnace_traitor_1772747883.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "scrap_goblin_nabob",
            "Scrap Gremlin Nabob",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Upkeep,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/scrap_goblin_nabob_1772748173.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ember_phoenix",
            "Ember Phoenix",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DamagePlayer(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/ember_phoenix_1772747784.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "molten_phoenix",
            "Molten Phoenix",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/molten_phoenix_1772747968.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "rekindling_furnace_bird",
            "Rekindling Furnace Bird",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(4, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(
                TokenSpec::new("Anomaly", 0, 1, UnitType::Anomaly).with_triggers(vec![
                    CardTrigger {
                        event: TriggerEvent::Upkeep,
                        effect: CardEffect::Sequence(vec![
                            CardEffect::SacrificeThis,
                            CardEffect::ReanimateNamedCard("rekindling_furnace_bird".to_string()),
                        ]),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    },
                ]),
            ),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/rekindling_furnace_bird_1774307345.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "combustion_spirit",
            "Combustion Spirit",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::StaticPowerBuff(1, None))
        .keyword(Keyword::StaticHasteGrant(GrantTarget::mine()))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/combustion_spirit_1774307124.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "persist_recur",
            "Persist Unit",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(TokenSpec::new("Revenant", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/persist_recur_1774307293.png")
        .build(),
        1,
    ));

    // 10 TOKEN MAKERS

    cards.push((
        CardBuilder::new("forge_fodder", "Forge Fodder", CardKind::Ritual)
            .cost(ManaCost {
                slag: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/dragon_fodder_1772223209.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "scrapyard_command",
            "Scrapyard Command",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/scrapyard_command_1774307364.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "foundry_outburst",
            "Foundry Outburst",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/foundry_outburst_1774307213.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "forge_war_marshal",
            "Forge War Marshal",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/forge_war_marshal_1772747848.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "sling_gang_foreman",
            "Wreck-Gang Foreman",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "sling_gang_sacrifice".to_string(),
            name: "Sacrifice Gremlin".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Sequence(vec![
                CardEffect::DamagePlayer(1),
                CardEffect::GainLife(1),
            ]),
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/sling_gang_foreman_1772748217.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "siege_gang_overseer",
            "Raid-Gang Overseer",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 3)
        .cost(ManaCost {
            slag: 3,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "siege_gang_sac".to_string(),
            name: "Sacrifice for Damage".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::DamagePlayer(2),
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/siege_gang_overseer_1772748189.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "foundry_rabblemaster",
            "Foundry Ringleader",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::TurnStart,
            effect: CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/foundry_rabblemaster_1772747862.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "forge_instigator",
            "Forge Instigator",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 1)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/forge_instigator_1772747848.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ironback_chief",
            "Ironback Chief",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(1, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/ironback_chief_1772747935.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "forge_spawning",
            "Forge Spawning",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 2)
        .cost(ManaCost {
            slag: 2,
            colorless: 0,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Inferno", 3, 3, UnitType::Anomaly)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/forge_spawning_1772747849.png")
        .build(),
        1,
    ));

    // 6 DAMAGE/BURN

    cards.push((
        CardBuilder::new("meltdown_protocol", "Meltdown Protocol", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(-3),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::AllUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(-3),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::AllUnits(None),
                    },
                ]),
                None,
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/blasphemous_act_1772143738.png")
            .build(),
        2,
    ));

    // On-theme Assimilate: slag smelts an opponent's unit down and rebuilds
    // it as an Anomaly — brute-force recycling flavor.

    cards.push((
        CardBuilder::new("forge_conversion", "Forge Conversion", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Assimilate(UnitType::Anomaly),
                Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/chain_reaction_1772143953.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("chain_detonation", "Chain Detonation", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(-2),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OpponentUnits(None),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Fixed(-2),
                        expires: ExpireContract::Never,
                        target: EffectTargetSpec::OpponentUnits(None),
                    },
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/chain_reaction_1772143953.png")
            .build(),
        3,
    ));
    cards.push((
        CardBuilder::new("slag_storm", "Slag Storm", CardKind::Instant)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                    stat: StatType::Both,
                    amount: ModifierAmount::Fixed(-6),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                }]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/comet_storm_1772144068.png")
            .build(),
        2,
    ));
    cards.push((
        CardBuilder::new("forge_blast", "Forge Blast", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageTarget(4),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                ]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/banefire_1772143449.png")
            .build(),
        2,
    ));
    cards.push((
        CardBuilder::new(
            "bonfire_of_the_smelted",
            "Bonfire of the Smelted",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            slag: 3,
            colorless: 2,
            ..Default::default()
        })
        .effect(
            CardEffect::ApplyStatModifiers(vec![
                StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(-5),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::AllUnits(None),
                },
                StatModifierSpec {
                    stat: StatType::Toughness,
                    amount: ModifierAmount::Fixed(-5),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::AllUnits(None),
                },
            ]),
            None,
        )
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/bonfire_damned_1772325198.png")
        .build(),
        3,
    ));

    // 5 CARD DRAW

    cards.push((
        CardBuilder::new(
            "wheel_of_molten",
            "Meltdown Protocol",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 3)
        .cost(ManaCost {
            slag: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Wheel(7),
                CardEffect::DamagePlayer(2),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/wheel_fortune_red_1772225959.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("reforge_the_slag", "Reforge the Slag", CardKind::Ritual)
            .cost(ManaCost {
                slag: 3,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Wheel(7),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/reforge_soul_red_1772225005.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "cathartic_combustion",
            "Cathartic Combustion",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::DamageTarget(3),
            Some(TargetSpec::new(
                TargetScope::Any,
                TargetKind::PlayerOrUnit,
                1,
            )),
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/cathartic_combustion_1774307097.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "thrill_of_the_forge",
            "Thrill of the Forge",
            CardKind::Instant,
        )
        .cost(ManaCost {
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Loot(2, 1),
                CardEffect::CreateToken(TokenSpec::new("Ember", 1, 1, UnitType::Anomaly)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/thrill_of_the_forge_1774307819.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("seething_slag", "Seething Slag", CardKind::Instant)
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::DamageTarget(2),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::PlayerOrUnit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/seething_song_1772225159.png")
            .build(),
        1,
    ));

    // No basic infrastructure — The Grid provides mana automatically

    // === NEW CARDS ===

    // Meltdown Titan — Singularity Apex Anomaly 5/5, Haste, Menace, ETB: DamageSplit(4), OnPermanentSacrificed: DamagePlayer(2)

    cards.push((
        CardBuilder::new(
            "meltdown_titan",
            "Meltdown Titan",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(6, 6)
        .cost(ManaCost {
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Menace)
        .trigger(CardTrigger {
            event: TriggerEvent::Sacrificed,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/meltdown_titan_1774299279.png")
        .build(),
        1,
    ));

    // Volatile Gremlin — Unit(Anomaly) 1/1, Haste, Echo(S), OnDeath: DamageTarget(2)

    cards.push((
        CardBuilder::new(
            "volatile_gremlin",
            "Volatile Gremlin",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 1)
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DamageTarget(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/volatile-gremlin_1774298385.png")
        .build(),
        1,
    ));

    // Scrap Harvest — Sorcery, Sacrifice own unit, DamageEachOpponentUnit(2), Scavenge(1)

    cards.push((
        CardBuilder::new("scrap_harvest", "Scrap Harvest", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::SacrificeThis,
                    CardEffect::DamageEachOpponentUnit(2),
                    CardEffect::Scavenge(1),
                ]),
                Some(TargetSpec::new(
                    TargetScope::SelfPlayer,
                    TargetKind::Unit,
                    1,
                )),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/scrap-harvest_1774298326.png")
            .build(),
        1,
    ));

    // Slag Eruption — Sorcery, DamageSplit(4) + Create 2 Gremlin tokens

    cards.push((
        CardBuilder::new("slag_eruption", "Slag Eruption", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageTarget(4),
                    CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                ]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/slag-eruption_1774298342.png")
            .build(),
        1,
    ));

    // Furnace Vent — Instant, DamageSplit(3)

    cards.push((
        CardBuilder::new("furnace_vent", "Furnace Vent", CardKind::Instant)
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::DamageTarget(3),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/furnace-vent_1774298229.png")
            .build(),
        1,
    ));

    // 3 BALANCE BUFF CARDS
    // Slag Reactor — CMC 0 artifact, tap for Gremlin token

    cards.push((
        CardBuilder::new("slag_reactor", "Slag Reactor", CardKind::Relic)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .tap_effect(CardEffect::CreateToken(TokenSpec::new(
                "Gremlin",
                1,
                1,
                UnitType::Anomaly,
            )))
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/slag-reactor_1774298353.png")
            .build(),
        1,
    ));

    // Molten Insight — cheap draw spell for slag

    cards.push((
        CardBuilder::new("molten_insight", "Molten Insight", CardKind::Ritual)
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::DrawCards(2), None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/molten-insight_1774298271.png")
            .build(),
        1,
    ));

    // Inferno Colossus — big finisher with board clear

    cards.push((
        CardBuilder::new(
            "inferno_colossus",
            "Inferno Colossus",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(6, 6)
        .cost(ManaCost {
            slag: 3,
            colorless: 4,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/inferno-colossus_1774298245.png")
        .build(),
        1,
    ));

    // COMBO ENGINE — Sacrifice Storm
    // Blood Furnace Core — sac payoff: damage + mana

    cards.push((
        CardBuilder::new("blood_furnace_core", "Blood Furnace Core", CardKind::Relic)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Sacrificed,
                effect: CardEffect::Sequence(vec![
                    CardEffect::DamagePlayer(3),
                    CardEffect::AddMana(ManaType::Slag, 1),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/blood-furnace-core_1774298150.png")
            .build(),
        1,
    ));

    // Gremlin Propagation — sac fuel: sacrifice 1, get 2 tokens + draw

    cards.push((
        CardBuilder::new(
            "gremlin_propagation",
            "Gremlin Propagation",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::CreateToken(
                    TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly).enters_tapped(),
                ),
                CardEffect::CreateToken(
                    TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly).enters_tapped(),
                ),
                CardEffect::DrawCards(1),
            ]),
            None,
        )
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/gremlin-propagation_1774298230.png")
        .build(),
        1,
    ));

    // ACCELERATION — Rampage units that burn through blockers
    // Magma Charger — cheap hasty unit that burns the defender on attack

    cards.push((
        CardBuilder::new(
            "magma_charger",
            "Magma Charger",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/magma-charger_1774298257.png")
        .build(),
        1,
    ));

    // Siege Behemoth — big Rampage finisher

    cards.push((
        CardBuilder::new(
            "siege_behemoth",
            "Siege Behemoth",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(4, 4)
        .cost(ManaCost {
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .keyword(Keyword::Haste)
        .keyword(Keyword::Rampage(1))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/siege-behemoth_1774298342.png")
        .build(),
        1,
    ));

    // CHROME SPIKE - Chrome Counter (Slag)

    cards.push((
        CardBuilder::new(
            "chrome_spike",
            "Chrome Spike",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 3)
        .cost(ManaCost {
            slag: 2,
            ..Default::default()
        })
        .effect(
            CardEffect::DestroyTarget,
            Some(
                TargetSpec::new(TargetScope::Any, TargetKind::Card, 1).with_predicate(
                    Predicate::Any(vec![
                        Predicate::Condition(Condition::TargetIsManaType(ManaType::Chrome)),
                        Predicate::CardKind(CardKind::Relic),
                    ]),
                ),
            ),
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/chrome-spike_1774298420.png")
        .build(),
        1,
    ));

    // ── Multi-color operators ────────────────────────────────────────────────

    // Wildfire Spirit (bio/slag) — Unit
    cards.push((
        CardBuilder::new(
            "wildfire_spirit",
            "Ember Sprite",
            CardKind::Unit(UnitType::Anomaly),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 2)
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/wildfire_spirit_1777084250.png")
        .build(),
        1,
    ));

    // Forge Overseer (chrome/slag) — Agent
    cards.push((
        CardBuilder::new("forge_overseer", "Forge Overseer", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(4)
            .cost(ManaCost {
                chrome: 1,
                slag: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Forge Drone",
                    2,
                    2,
                    UnitType::Drone,
                )),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .ability(CardAbility {
                id: "forge_overseer_plus_1".to_string(),
                name: "[+1] Scry".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::Scry(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "forge_overseer_minus_3".to_string(),
                name: "[-4] Forge Bolt".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-4),
                    CardEffect::DamageTarget(2),
                ]),
                target_spec: Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "forge_overseer_minus_7".to_string(),
                name: "[-7] Haste Charge".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-7),
                    CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(3),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    }]),
                    CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Singularity)
            .image("https://cdn.tawks.app/cards/forge_overseer_1777083902.png")
            .build(),
        1,
    ));

    cards
}
