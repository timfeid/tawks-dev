use crate::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, Cost,
    TargetSpec, TokenSpec,
};
use crate::types::{
    CardKind, CardRarity, CardSupertype, Condition, Keyword,
    ManaCost, ManaType, Predicate, TargetKind, TargetScope, TriggerEvent,
    TriggerLimit, UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // 6 OPERATORS (3 Apex Units + 3 Apex Agents)

    // 3 APEX UNIT OPERATORS
    cards.push((
        CardBuilder::new(
            "bitrot_trinket_thief",
            "Bitrot, Trinket Thief",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 2)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "bitrot_choose_player".to_string(),
            name: "Choose Player".to_string(),
            cost: AbilityCost::new(),
            effect: CardEffect::RememberTargetPlayer,
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Player, 1)),
            once_per_turn: false,
            uses_stack: false,
            tap_targets: false,
            predicate: None,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(
                Condition::TargetMatchesRememberedPlayer,
            )),
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DamagePlayer(3),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::All(vec![
                Predicate::Condition(Condition::TargetMatchesRememberedPlayer),
                Predicate::Condition(Condition::TargetPlayerHandSizeExactly(0)),
            ])),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/bitrot_trinket_thief_1772747638.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "corruptor_of_data",
            "Corruptor of Data",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 2,
            ..Default::default()
        })
        .keyword(Keyword::Menace)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/corruptor_of_data_1772747723.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "null_lord_of_the_void",
            "Null, Lord of the Void",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(6, 6)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Menace)
        .ability(CardAbility {
            id: "null_steal".to_string(),
            name: "Steal from Graveyard".to_string(),
            cost: AbilityCost::new().with_cost(Cost::Mana(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })),
            effect: CardEffect::ReanimateTargetUnit,
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Player, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/null_lord_of_the_void_1772748032.png")
        .build(),
        1,
    ));

    // 3 APEX AGENT OPERATORS
    cards.push((
        CardBuilder::new("entropy_weaver", "Entropy Weaver", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(3)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "entropy_veil_plus_1".to_string(),
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
                id: "entropy_veil_minus_1".to_string(),
                name: "[-1] Destroy Unit".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-1),
                    CardEffect::DestroyTarget,
                ]),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "entropy_veil_minus_8".to_string(),
                name: "[-8] Target Player Wipe".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-8),
                    CardEffect::DestroyTargetPlayerUnits,
                ]),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Player, 1)),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/liliana_veil_1772224122.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "entropy_dreadhorde_general",
            "Entropy, Datavoid General",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(5)
        .cost(ManaCost {
            glitch: 2,
            colorless: 3,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "entropy_dread_plus_1".to_string(),
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
            id: "entropy_dread_minus_4".to_string(),
            name: "[-3] Mass Discard".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::Discard(2),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "entropy_dread_minus_9".to_string(),
            name: "[-7] Husk Apocalypse".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-7),
                CardEffect::DamagePlayer(3),
                CardEffect::CreateToken(TokenSpec::new("Husk", 2, 2, UnitType::Virus)),
            ]),
            target_spec: Some(TargetSpec::new(
                TargetScope::Opponent,
                TargetKind::Player,
                1,
            )),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/entropy_dreadhorde_general_1772747803.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "entropy_last_resort",
            "Entropy, the Last Resort",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(4)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "entropy_lr_plus_1".to_string(),
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
            id: "entropy_lr_minus_3".to_string(),
            name: "[-3] Wheel".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::Wheel(5),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "entropy_lr_minus_6".to_string(),
            name: "[-6] Mill 12".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-6),
                CardEffect::Mill(12),
                CardEffect::DrawCards(2),
            ]),
            target_spec: Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Player, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/entropy_last_resort_1772747804.png")
        .build(),
        1,
    ));

    // 8 CHEAP CARDS (1-2 mana)
    cards.push((
        CardBuilder::new("memory_probe", "Memory Probe", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Discard(1),
                Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::Player,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/duress_1772223233.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "thought_corrupt",
            "Thought Corrupt",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/thoughtseize_1772225611.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "deep_inquisition",
            "Deep Inquisition",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 1)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/inquisition_1772223869.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "static_crime",
            "Static Crime",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 1)
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Deathtouch)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/raven_crime_1772224901.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "corruption_pulse",
            "Corruption Pulse",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 2)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DamagePlayerPerHandCard,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/corruption_pulse_1774307152.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("fatal_error", "Fatal Error", CardKind::Relic)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "fatal_error_kill".to_string(),
                name: "Fatal Error".to_string(),
                cost: AbilityCost::new().with_tap().with_cost(Cost::Discard(1)),
                effect: CardEffect::DestroyTarget,
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/fatal_push_1772223526.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("tragic_crash", "Kernel Panic", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Bounce, CardEffect::Discard(1)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/tragic_slip_1772225705.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("forbidden_query", "Forbidden Query", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::Search(Predicate::IsUnit), None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/forbidden_query_1774307199.png")
            .build(),
        1,
    ));

    // 56 OTHER CARDS - 3+ mana

    // 2 APEX UNITS (remaining)
    cards.push((
        CardBuilder::new(
            "malware_the_twisted",
            "Malware, the Twisted",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(4, 4)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/malware_the_twisted_1774307254.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "crash_nightmare_agent",
            "Crash, Nightmare Agent",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 2)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "chainer_reanimate".to_string(),
            name: "Reanimate and Draw".to_string(),
            cost: AbilityCost::new()
                .with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .with_cost(Cost::Discard(1)),
            effect: CardEffect::Sequence(vec![
                CardEffect::ReanimateTargetUnit,
                CardEffect::DrawCards(1),
            ]),
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Player, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/crash_nightmare_agent_1774307150.png")
        .build(),
        1,
    ));

    // 2 AGENTS (remaining)
    cards.push((
        CardBuilder::new("void_the_mirthless", "Void, the Hollow", CardKind::Agent)
            .loyalty(3)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "sorin_plus_1".to_string(),
                name: "[0] Grind Hand".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(0),
                    CardEffect::Discard(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "sorin_minus_2".to_string(),
                name: "[-2] Reanimate".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-2),
                    CardEffect::ReanimateTargetUnit,
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "sorin_minus_7".to_string(),
                name: "[-7] Wheel Lock".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-7),
                    CardEffect::Wheel(7),
                    CardEffect::Discard(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/sorin_grim_1772225338.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "decay_the_corruptor",
            "Decay, the Corruptor",
            CardKind::Agent,
        )
        .loyalty(4)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "vraska_plus_2".to_string(),
            name: "[+1] Filter Archive".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::Loot(2, 1),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "vraska_minus_3".to_string(),
            name: "[-3] Destroy".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DestroyTarget,
            ]),
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "vraska_minus_9".to_string(),
            name: "[-9] Reboot the Dead".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-9),
                CardEffect::BoardWipe,
                CardEffect::ReanimateTargetUnit,
                CardEffect::ReanimateTargetUnit,
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/decay_the_corruptor_1772747757.png")
        .build(),
        1,
    ));

    // 10 DISCARD ENGINES
    cards.push((
        CardBuilder::new("bottomless_cache", "Bottomless Cache", CardKind::Aura)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::Discard(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/bottomless_pit_1772143835.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("malware_mists", "Malware Mists", CardKind::Aura)
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::Discard(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/necrogen_mists_1772224459.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "system_oppression",
            "System Oppression",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::CastAny,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/system_oppression_1774307408.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "painful_exception",
            "Painful Exception",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::OpponentCast,
            effect: CardEffect::Sequence(vec![CardEffect::DamagePlayer(1), CardEffect::Discard(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/painful_exception_1774307279.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "entropy_caress",
            "Entropy's Caress",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DamagePlayer(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/liliana_caress_1772224066.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "data_hemorrhage",
            "Data Hemorrhage",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::AddMana(ManaType::Glitch, 1),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/megrim_1772224275.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "waste_not_want_not",
            "Waste Not, Want Not",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/waste_not_want_not_1774307451.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("shrieking_static", "Shrieking Static", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::CreateToken(TokenSpec::new("Phantom", 1, 1, UnitType::Virus)),
                limit: TriggerLimit::Unlimited,
                predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/shrieking_static_1774307362.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("the_vice", "The Vice", CardKind::Relic)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentDrew,
                effect: CardEffect::DamagePlayer(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/the_vice_1774307426.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("quest_for_null", "Quest for Null", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::DamagePlayer(4),
                limit: TriggerLimit::Unlimited,
                predicate: Some(Predicate::All(vec![
                    Predicate::Condition(Condition::TargetIsOpponentPlayer),
                    Predicate::Condition(Condition::TargetPlayerHandSizeExactly(0)),
                ])),
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/quest_for_null_1774307314.png")
            .build(),
        1,
    ));

    // 8 WHEEL EFFECTS (everyone discards, draws)
    cards.push((
        CardBuilder::new("wheel_of_entropy", "Entropy Protocol", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::Wheel(7), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/wheel_fortune_1772225948.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("data_cascade", "Data Cascade", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Wheel(5),
                    CardEffect::Surveil(2),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/windfall_1772225998.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("corrupt_deal", "Corrupt Deal", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::Wheel(6), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/dark_deal_1772223121.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("rewrite_memory", "Rewrite Memory", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Wheel(7), CardEffect::DrawCards(2)]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/reforge_soul_1772224992.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("wheel_of_static", "Static Protocol", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::Wheel(5), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/wheel_of_static_1774307453.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "magus_of_the_crash",
            "Adept of the Crash",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 1)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "magus_wheel_ability".to_string(),
            name: "Wheel Effect".to_string(),
            cost: AbilityCost::new().with_tap().with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Wheel(7),
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .keyword(Keyword::Deathtouch)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/magus_of_the_crash_1774307252.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "whispering_malware",
            "Whispering Malware",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Wheel(4), CardEffect::Discard(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/whispering_malware_1774307471.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "memory_overflow",
            "Memory Overflow",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Wheel(7),
                CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
                CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/incendiary_command_1772223851.png")
        .build(),
        1,
    ));

    // 10 TARGETED DISCARD
    cards.push((
        CardBuilder::new(
            "hymn_to_entropy",
            "Dirge of Entropy",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: Some(AbilityCost {
                costs: vec![Cost::Mana(ManaCost {
                    colorless: 2,
                    ..Default::default()
                })],
            }),
            is_optional: true,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/hymn_to_entropy_1774299281.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "wrench_process",
            "Wrench Process",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/wrench_mind_1772226084.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "memory_leak",
            "Memory Leak",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/mind_rot_1772224298.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "system_drain",
            "System Drain",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::DamagePlayer(2)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/unburden_1772225745.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "stack_fracture",
            "Stack Fracture",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Mill(3),
                CardEffect::Scry(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Stealth)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/stack_fracture_1774307393.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("data_rat", "Data Rat", CardKind::Unit(UnitType::Virus))
            .stats(2, 1)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Discard(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: Some(AbilityCost::new().with_mana(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })),
                is_optional: true,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/data_rat_1774307166.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ravenous_worms",
            "Ravenous Worms",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/ravenous_worms_1774307332.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "hypnotic_glitch",
            "Hypnotic Glitch",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 1)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/hypnotic_glitch_1772747923.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "entropy_spectre",
            "Entropy's Spectre",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/entropy_spectre_1772747818.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "virus_disciple",
            "Virus Disciple",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/virus_disciple_1772751835.png")
        .build(),
        1,
    ));

    // 10 REMOVAL
    cards.push((
        CardBuilder::new("terminate_process", "Terminate Process", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Discard(2),
                Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::Player,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/terminate-process_1774298369.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("system_downfall", "System's Downfall", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::Mill(2)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/hero_downfall_1772223761.png")
            .build(),
        1,
    ));
    // On-theme Assimilate: glitch infects the target with a memetic payload,
    // rewriting it as a Virus on your side. Forces opp to discard as collateral.
    cards.push((
        CardBuilder::new("memetic_override", "Memetic Override", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Assimilate(UnitType::Virus),
                    CardEffect::Discard(1),
                ]),
                Some(
                    TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)
                        .with_predicate(Predicate::Condition(Condition::TargetIsTapped)),
                ),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/doom_blade_1772223194.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("doom_code", "Doom Code", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Discard(2), CardEffect::DrawCards(1)]),
                Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::Player,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/doom_blade_1772223194.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("go_for_the_core", "Go for the Core", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::Mill(2)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/go_for_throat_1772325278.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("victim_of_static", "Victim of Static", CardKind::Instant)
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ExileTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/victim_night_1772225832.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("total_corruption", "Total Corruption", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::DestroyUnitsWithToughnessAtLeast(4), None)
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/damnation_1772325232.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "toxic_overflow",
            "Toxic Overflow",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 1)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/toxic_deluge_1772225681.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("void_zenith", "Void Zenith", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::BounceAllOpponentPermanents,
                    CardEffect::Discard(2),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/black_sun_1772325174.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "cascade_failure",
            "Cascade Failure",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Wheel(3),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/cascade_failure_1774307098.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("final_shutdown", "Final Shutdown", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::BounceAllUnits, CardEffect::Discard(2)]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/final_shutdown_1774307200.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("null_terminate", "Null Terminate", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Discard(2), CardEffect::GainLife(2)]),
                Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::Player,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/snuff_out_1772225325.png")
            .build(),
        1,
    ));

    // 8 RECURSION/GRAVEYARD
    cards.push((
        CardBuilder::new("reanimate", "Reanimate", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::ReanimateTargetUnit,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Player, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/reanimate_1772224938.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("boot_from_cache", "Boot from Cache", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::ReanimateTargetUnit,
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/animate_dead_1772143381.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("data_necromancy", "Data Necromancy", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::ReanimateTargetUnit,
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/data_necromancy_1774307150.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "recursive_reclamation",
            "Recursive Reclamation",
            CardKind::Aura,
        )
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "phyrexian_reclamation_ability".to_string(),
            name: "Reclaim Unit".to_string(),
            cost: AbilityCost::new().with_mana(ManaCost {
                glitch: 1,
                ..Default::default()
            }),
            effect: CardEffect::SearchGraveyard(Predicate::IsUnit),
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/recursive_reclamation_1774307326.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("living_reboot", "Living Reboot", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::BoardWipe,
                    CardEffect::ReanimateTargetUnit,
                    CardEffect::ReanimateTargetUnit,
                ]),
                None,
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/living_death_1772224135.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("rise_from_the_dump", "Rise from the Dump", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 2,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ReanimateTargetUnit,
                    CardEffect::DrawCards(2),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/rise_grave_1772225075.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "archived_alive",
            "Archived Alive",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Loot(2, 1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/buried_alive_1772325522.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("cache_burial", "Cache Burial", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .effect(CardEffect::Loot(2, 1), None)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/entomb_1772223346.png")
            .build(),
        1,
    ));

    // 5 CARD DRAW
    cards.push((
        CardBuilder::new("forbidden_access", "Forbidden Access", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "necropotence_draw".to_string(),
                name: "Pay Life to Draw".to_string(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .with_life(1),
                effect: CardEffect::DrawCards(1),
                target_spec: None,
                once_per_turn: false,
                uses_stack: false,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/necropotence_1772224488.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("corrupted_arena", "Corrupted Arena", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
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
            .image("https://cdn.tawks.app/cards/phyrexian_arena_1772224752.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "sign_in_void",
            "Sign in Void",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Loot(2, 1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Stealth)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/sign_blood_1772225234.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "read_the_logs",
            "Read the Logs",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Scry(2), CardEffect::DrawCards(2)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/read_bones_1772224926.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("glitch_surge", "Glitch Surge", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .effect(CardEffect::Surveil(2), None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/glitch_surge_1774307225.png")
            .build(),
        1,
    ));

    // No basic infrastructure — The Grid provides mana automatically

    // ── NEW CARDS ────────────────────────────────────────────────────────────

    // Paradox Worm — Singularity Apex Virus
    cards.push((
        CardBuilder::new(
            "paradox_worm",
            "Paradox Worm",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 1)
        .cost(ManaCost {
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/paradox_worm_1774299292.png")
        .build(),
        1,
    ));

    // Null Crawler — Wither virus with death-Entomb
    cards.push((
        CardBuilder::new(
            "null_crawler",
            "Null Crawler",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Entomb(Predicate::IsUnit),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/null_crawler_1774299279.png")
        .build(),
        1,
    ));

    // Memory Siphon — drain + surveil sorcery
    cards.push((
        CardBuilder::new(
            "memory_siphon",
            "Memory Siphon",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Siphon(3), CardEffect::Surveil(2)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/memory-siphon_1774298256.png")
        .build(),
        1,
    ));

    // Logic Bomb — cheap damage + discard instant
    cards.push((
        CardBuilder::new("logic_bomb", "Logic Bomb", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::DamageTarget(2), CardEffect::Surveil(1)]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/logic-bomb_1774298260.png")
            .build(),
        1,
    ));

    // Entropy Spore — cheap virus with discard ETB and death token
    cards.push((
        CardBuilder::new(
            "entropy_spore",
            "Entropy Spore",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/entropy-spore_1774298216.png")
        .build(),
        1,
    ));

    // NEON SPIKE - Neon Counter (Glitch)
    cards.push((
        CardBuilder::new("neon_spike", "Neon Spike", CardKind::Unit(UnitType::Virus))
            .stats(2, 2)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::DestroyTarget,
                Some(
                    TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1).with_predicate(
                        Predicate::Condition(Condition::TargetIsManaType(ManaType::Neon)),
                    ),
                ),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/neon-spike_1774298410.png")
            .build(),
        1,
    ));

    // ── Multi-color operators ────────────────────────────────────────────────

    // Datavoid Phantom (glitch/neon) — Unit
    cards.push((
        CardBuilder::new(
            "datavoid_phantom",
            "Datavoid Phantom",
            CardKind::Unit(UnitType::Virus),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 2)
        .cost(ManaCost {
            glitch: 1,
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/datavoid_phantom_1777083881.png")
        .build(),
        1,
    ));

    // Entropy Pyre (glitch/slag) — Agent
    cards.push((
        CardBuilder::new("entropy_pyre", "Entropy Cascade", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(5)
            .cost(ManaCost {
                glitch: 1,
                slag: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "entropy_pyre_plus_1".to_string(),
                name: "[+1] Discard".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::Discard(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "entropy_pyre_minus_3".to_string(),
                name: "[-4] Burn Discard".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-4),
                    CardEffect::Discard(1),
                    CardEffect::DamagePlayer(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "entropy_pyre_minus_8".to_string(),
                name: "[-6] Entropy Cascade".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-6),
                    CardEffect::DamagePlayer(5),
                    CardEffect::Discard(2),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Singularity)
            .image("https://cdn.tawks.app/cards/entropy_pyre_1777084251.png")
            .build(),
        1,
    ));

    cards
}
