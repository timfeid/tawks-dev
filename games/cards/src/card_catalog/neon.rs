use crate::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, StatModifierSpec,
    TargetSpec, TokenSpec,
};
use crate::types::{
    CardKind, CardRarity, CardSupertype, EffectTargetSpec, ExpireContract, GrantTarget, Keyword,
    ManaCost, ManaType, ModifierAmount, Predicate, StatType, TargetKind, TargetScope, TriggerEvent,
    TriggerLimit, UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // 6 OPERATORS (3 Apex Units + 3 Apex Agents)

    // 3 APEX UNIT OPERATORS

    cards.push((
        CardBuilder::new(
            "cipher_thief",
            "Cipher Thief, Data Acquisitor",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(6, 4)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/cipher_thief_1772747690.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "echo_signal",
            "Echo Signal, Intelligence Node",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(4, 5)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AnyAllyDealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/echo_signal_1772747770.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "grid_compiler",
            "Grid Compiler, Drone Fabricator",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(4, 5)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Glider", 2, 2, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AnyAllyDealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/grid_compiler_1772747897.png")
        .build(),
        1,
    ));

    // 3 APEX AGENT OPERATORS

    cards.push((
        CardBuilder::new(
            "cypher_data_architect",
            "Cypher, Data Architect",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(7)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "jace_plus_2".to_string(),
            name: "[+1] Trace — Scry 2".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![CardEffect::ModifyLoyalty(1), CardEffect::Scry(2)]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "jace_minus_1".to_string(),
            name: "[-3] Damage 3".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DamagePlayer(3),
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
        .ability(CardAbility {
            id: "jace_minus_10".to_string(),
            name: "[-6] Wash the Board".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-6),
                CardEffect::BounceAllOpponentPermanents,
                CardEffect::DrawCards(2),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/jace_beleren_1772223920.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "chronos_temporal_hacker",
            "Chronos, Temporal Hacker",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(5)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "chronos_plus_1".to_string(),
            name: "[+1] Tick — Tap Unit".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![CardEffect::ModifyLoyalty(1), CardEffect::TapTarget]),
            target_spec: Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "chronos_minus_1".to_string(),
            name: "[-1] Bounce".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![CardEffect::ModifyLoyalty(-1), CardEffect::Bounce]),
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "chronos_minus_5".to_string(),
            name: "[-5] Mass Bounce".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-5),
                CardEffect::BounceAllOpponentPermanents,
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/chronos_temporal_hacker_1772747686.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "filter_parter_of_streams",
            "Filter, Parter of Streams",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(3)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Firewall(2))
        .ability(CardAbility {
            id: "narset_plus_1".to_string(),
            name: "[+1] Filter Hand".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "narset_minus_3".to_string(),
            name: "[-3] Shock".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DamageTarget(3),
            ]),
            target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/filter_parter_of_streams_1772747830.png")
        .build(),
        1,
    ));

    // 8 CHEAP CARDS (1-2 mana)

    cards.push((
        CardBuilder::new(
            "data_wisp",
            "Echo Initiate",
            CardKind::Unit(UnitType::Hacker),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::CastInstantOrRitual,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/data_wisp_1772747756.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "spectral_hacker",
            "Spectral Hacker",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Absorb(1))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/spectral_hacker_1772748229.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "neon_delver",
            "Neon Delver",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 4)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Absorb(1))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/neon_delver_1772748001.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("data_surge", "Data Surge", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(2)]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/brainstorm_1772143847.png")
            .build(),
        1,
    ));

    // On-theme Assimilate: neon reroutes the target unit's control channel,
    // flipping it to your network as a Drone.

    cards.push((
        CardBuilder::new("derez", "Derez", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Bounce,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/unsummon_1772225761.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("deep_scan", "Deep Scan", CardKind::Ritual)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Scry(2), CardEffect::DrawCards(1)]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/ponder_1772224788.png")
            .build(),
        1,
    ));

    // 56 OTHER CARDS - 3+ mana

    // 2 APEX UNITS (remaining)

    cards.push((
        CardBuilder::new(
            "proxy_clone",
            "Proxy Clone, Identity Spoofer",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 1)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CopyUnit,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/proxy_clone_1774307298.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "firewall_chief",
            "Firewall Chief, Protocol Enforcer",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(1, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::CounteredSpell,
            effect: CardEffect::Sequence(vec![
                CardEffect::DrawCards(1),
                CardEffect::AddMana(ManaType::Neon, 1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/firewall_chief_1775261136.png")
        .build(),
        1,
    ));

    // 2 AGENTS (remaining)

    cards.push((
        CardBuilder::new(
            "vector_thought_architect",
            "Vector, Architect of Thought",
            CardKind::Agent,
        )
        .loyalty(3)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "jace_arch_plus_1".to_string(),
            name: "[+1] Weaken".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(1),
                CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(-1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::OpponentUnits(None),
                }]),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "jace_arch_minus_2".to_string(),
            name: "[-2] Scry & Draw".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-2),
                CardEffect::Scry(3),
                CardEffect::DrawCards(1),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "jace_arch_minus_8".to_string(),
            name: "[-8] Search".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-8),
                CardEffect::Search(Predicate::CardKind(CardKind::Unit(UnitType::Drone))),
                CardEffect::Search(Predicate::CardKind(CardKind::Instant)),
                CardEffect::DrawCards(1),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/vector_thought_architect_1772751783.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("nexus_the_seeker", "Nexus, the Seeker", CardKind::Agent)
            .loyalty(4)
            .cost(ManaCost {
                neon: 3,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "tezzeret_plus_1".to_string(),
                name: "[+1] Untap".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::AddMana(ManaType::Neon, 1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "tezzeret_minus_2".to_string(),
                name: "[-2] Search Artifact".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-2),
                    CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "tezzeret_minus_5".to_string(),
                name: "[-5] Animate Artifacts".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-5),
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(5),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(5),
                            expires: ExpireContract::EndOfTurn,
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
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/nexus_the_seeker_1772748018.png")
            .build(),
        1,
    ));

    // 8 ANTHEM/LORD UNITS

    cards.push((
        CardBuilder::new(
            "lord_of_the_grid",
            "Lord of the Grid",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 5)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticStealthGrant(GrantTarget::mine_of(
            UnitType::Drone,
        )))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/lord_of_the_grid_1772747936.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "master_of_signals",
            "Master of Signals",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 4)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/master_of_signals_1775260933.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "master_of_the_stream",
            "Master of the Stream",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 2)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AnyAllyDealtDamage,
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/master_of_the_stream_1775260949.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "naban_dean_of_loops",
            "Novan, Dean of Loops",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/naban_dean_of_loops_1772747981.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "proxy_warden",
            "Proxy Warden",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/proxy_warden_1772748079.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ping_cutthroat",
            "Ping, Data Cutthroat",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::CastDuringOpponentTurn,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/ping_cutthroat_1772748047.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "circuit_reejerey",
            "Circuit Resonator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 5)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::AddMana(ManaType::Neon, 1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/circuit_reejerey_1772747706.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "bandwidth_reducer",
            "Bandwidth Reducer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/bandwidth_reducer_1772747613.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "adaptive_protocol",
            "Adaptive Protocol",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AnyAllyDealtDamage,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/adaptive_protocol_1775260848.png")
        .build(),
        1,
    ));

    // 12 FLASH/INSTANT SPEED UNITS

    cards.push((
        CardBuilder::new(
            "stealth_infiltrator",
            "Stealth Infiltrator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/stealth_infiltrator_1772748272.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "data_siphon",
            "Data Siphon",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/data_siphon_1774306932.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "harbinger_of_static",
            "Harbinger of Static",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Bounce,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/harbinger_of_static_1772747924.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "nimble_jammer",
            "Nimble Jammer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 1)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::AddMana(ManaType::Neon, 1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/nimble_jammer_1772748015.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "cache_recall_drone",
            "Cache Recall Drone",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 1)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::DrawCards(1), CardEffect::Scry(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/cache_recall_drone_1775260862.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "vex_signal_shaper",
            "Vex, Signal Shaper",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Bounce,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/vex_signal_shaper_1772751819.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "protocol_viper",
            "Protocol Viper",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::CounterTop, CardEffect::Scry(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/protocol_viper_1772748080.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "flux_mystic",
            "Flux Mystic",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 2)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CounterTop,
                CardEffect::Scry(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/flux_mystic_1772747831.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "ambush_spike",
            "Ambush Spike",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Deathtouch)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/ambush_spike_1772747599.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "bandwidth_cutthroat",
            "Bandwidth Cutthroat",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::CastDuringOpponentTurn,
            effect: CardEffect::ApplyStatModifiers(vec![
                StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::Source,
                },
                StatModifierSpec {
                    stat: StatType::Toughness,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::EndOfTurn,
                    target: EffectTargetSpec::Source,
                },
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/bandwidth_cutthroat_1772747614.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "cunning_netrunner",
            "Cunning Netrunner",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::CastDuringOpponentTurn,
            effect: CardEffect::Sequence(vec![
                CardEffect::Scry(1),
                CardEffect::AddMana(ManaType::Neon, 1),
            ]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/cunning_netrunner_1772747743.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "shadow_process",
            "Shadow Process",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Freeze(ExpireContract::Turns(1)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/shadow_process_1772748188.png")
        .build(),
        1,
    ));

    // 10 UNBLOCKABLE UNITS

    cards.push((
        CardBuilder::new(
            "phantom_trace",
            "Phantom Trace",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/phantom_trace_1772748031.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "waveline_stalker",
            "Waveline Stalker",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 1)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/waveline_stalker_1772751855.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "signal_blade",
            "Signal Blade",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/signal_blade_1772748188.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "neon_infiltrator",
            "Neon Infiltrator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 1)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/neon_infiltrator_1772747997.png")
        .build(),
        2,
    ));
    cards.push((
        CardBuilder::new(
            "signal_intercept",
            "Signal Intercept",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 1)
        .cost(ManaCost {
            neon: 1,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/signal_intercept_1772748200.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("trident_of_data", "Trident of Data", CardKind::Relic)
            .cost(ManaCost {
                neon: 2,
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AnyAllyDealtDamage,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/trident_of_data_1772751769.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "surveillance_mission",
            "Surveillance Mission",
            CardKind::Aura,
        )
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AnyAllyDealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Scry(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/surveillance_mission_1772748286.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "thassa_god_of_data",
            "Axiom, God of Data",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(4, 4)
        .cost(ManaCost {
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .keyword(Keyword::Indestructible)
        .ability(CardAbility {
            id: "axiom_grant_stealth".to_string(),
            name: "Grant Unblockable".to_string(),
            cost: AbilityCost::new().with_mana(ManaCost {
                neon: 1,
                ..Default::default()
            }),
            effect: CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
            target_spec: Some(TargetSpec::new(
                TargetScope::SelfPlayer,
                TargetKind::Unit,
                1,
            )),
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/thassa_god_of_data_1772751745.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "upload_entity",
            "Upload Entity",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .keyword(Keyword::StaticStealthGrant(GrantTarget::mine_of(
            UnitType::Drone,
        )))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/upload_entity_1774307437.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "data_skulker",
            "Data Skulker",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 1)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DrawStep,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/data_skulker_1774307171.png")
        .build(),
        1,
    ));

    // 10 CARD DRAW/CANTRIPS

    cards.push((
        CardBuilder::new(
            "gateway_oracle",
            "Gateway Oracle",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/gateway_oracle_1772747894.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "stream_drifter",
            "Stream Drifter",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![CardEffect::Bounce, CardEffect::DrawCards(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/stream_drifter_1772748272.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("rhystic_analysis", "Parasitic Analysis", CardKind::Aura)
            .cost(ManaCost {
                neon: 2,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentCast,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/rhystic_study_1772225061.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("signal_leech", "Signal Leech", CardKind::Aura)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentCastNonUnit,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/signal_leech_1774307362.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "consecrated_server",
            "Sanctified Server",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(4, 6)
        .cost(ManaCost {
            neon: 3,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .trigger(CardTrigger {
            event: TriggerEvent::OpponentDrew,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/consecrated_server_1774307137.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("fact_or_noise", "Parse or Noise", CardKind::Instant)
            .cost(ManaCost {
                neon: 2,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Bounce,
                    CardEffect::Scry(1),
                    CardEffect::DrawCards(1),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/fact_or_noise_1775260983.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("stroke_of_insight", "Surge of Insight", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
                    CardEffect::DrawCards(1),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/stroke_genius_1772225454.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("neon_zenith", "Neon Zenith", CardKind::Instant)
            .cost(ManaCost {
                neon: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::DrawCards(4), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/blue_sun_1772143805.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("deep_decode", "Deep Decode", CardKind::Ritual)
            .cost(ManaCost {
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Scry(2),
                    CardEffect::DrawCards(2),
                ]),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/deep_decode_1775260874.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("data_cruise", "Data Cruise", CardKind::Ritual)
            .cost(ManaCost {
                neon: 3,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Scry(2),
                    CardEffect::DrawCards(3),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/treasure_cruise_1772225721.png")
            .build(),
        1,
    ));

    // 6 BOUNCE/TEMPO SPELLS

    cards.push((
        CardBuilder::new("system_purge", "System Purge", CardKind::Instant)
            .cost(ManaCost {
                neon: 3,
                colorless: 4,
                ..Default::default()
            })
            .effect(CardEffect::BounceAllOpponentPermanents, None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/cyclonic_rift_1772223099.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("mass_derez", "Mass Derez", CardKind::Instant)
            .cost(ManaCost {
                neon: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::BounceAllAttacking, None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/aetherize_1772143325.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("full_reboot", "Full Reboot", CardKind::Instant)
            .cost(ManaCost {
                neon: 3,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::BounceAllOpponentPermanents,
                    CardEffect::DrawCards(1),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/evacuation_1772223438.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("recursive_bounce", "Recursive Bounce", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Bounce,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/capsize_1772143905.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("blink_of_a_cycle", "Blink of a Cycle", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Bounce,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/blink_mischief_1772143750.png")
            .build(),
        1,
    ));

    // 5 COUNTERS/INTERACTION

    cards.push((
        CardBuilder::new("access_denied", "Access Denied", CardKind::Instant)
            .cost(ManaCost {
                neon: 2,
                ..Default::default()
            })
            .effect(CardEffect::CounterTop, None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/counterspell_1772144094.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("resource_drain", "Resource Drain", CardKind::Instant)
            .cost(ManaCost {
                neon: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CounterTop,
                    CardEffect::Scry(1),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/mana_drain_1772224204.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("packet_denial", "Packet Denial", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::CounterTop, CardEffect::DrawCards(1)]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/packet_denial_1775260965.png")
            .build(),
        1,
    ));

    // No basic infrastructure — The Grid provides mana automatically

    // ── NEW CARDS ────────────────────────────────────────────────────────────

    // Flux Oracle — Singularity Apex Drone

    cards.push((
        CardBuilder::new(
            "flux_oracle",
            "Flux Oracle",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(4, 6)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "flux_oracle_transmit".to_string(),
            name: "Transmit".to_string(),
            cost: AbilityCost::new().with_tap(),
            effect: CardEffect::Transmit(2),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DrewCard,
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/flux_oracle_1774299266.png")
        .build(),
        1,
    ));

    // Phase Skipper — Stealth drone with Phase trigger

    cards.push((
        CardBuilder::new(
            "phase_skipper",
            "Ghost Runner",
            CardKind::Unit(UnitType::Hacker),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Stealth)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/phase-skipper_1774298303.png")
        .build(),
        1,
    ));

    // Signal Cascade — Cascade sorcery

    cards.push((
        CardBuilder::new("signal_cascade", "Signal Cascade", CardKind::Ritual)
            .cost(ManaCost {
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Cascade)
            .effect(CardEffect::DrawCards(2), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/signal_cascade_1774299294.png")
            .build(),
        1,
    ));

    // Flicker Drone — cheap bounce ETB

    cards.push((
        CardBuilder::new(
            "flicker_drone",
            "Flicker Drone",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .effect(
            CardEffect::Phase,
            Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1).optional()),
        )
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/flicker-drone_1774298217.png")
        .build(),
        1,
    ));

    // Data Ripple — 1-mana Transmit instant

    cards.push((
        CardBuilder::new("data_ripple", "Data Ripple", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::Transmit(2), None)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/data-ripple_1774298189.png")
            .build(),
        1,
    ));

    // BALANCE BUFF CARDS
    // Phantom Relay — cheap flash threat that replaces itself

    cards.push((
        CardBuilder::new(
            "phantom_relay",
            "Phantom Relay",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/phantom-relay_1774298303.png")
        .build(),
        1,
    ));

    // Temporal Surge — draw 3 at instant speed

    cards.push((
        CardBuilder::new("temporal_surge", "Temporal Surge", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::DrawCards(3), None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/temporal-surge_1774298369.png")
            .build(),
        1,
    ));

    // Grid Phantom — resilient unblockable threat

    cards.push((
        CardBuilder::new(
            "grid_phantom",
            "Grid Phantom",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            neon: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/grid_phantom_1775260916.png")
        .build(),
        1,
    ));

    // COMBO ENGINE — Spell Chain
    // Feedback Loop — spellcast payoff: mana refund + burn

    cards.push((
        CardBuilder::new("feedback_loop", "Feedback Loop", CardKind::Aura)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::CastInstantOrRitual,
                effect: CardEffect::AddMana(ManaType::Neon, 1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/feedback-loop_1774298217.png")
            .build(),
        1,
    ));

    // Cascade Protocol — cheap cantrip that chains

    cards.push((
        CardBuilder::new("cascade_protocol", "Cascade Protocol", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(CardEffect::DrawCards(2), None)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/cascade-protocol_1774298162.png")
            .build(),
        1,
    ));

    // ACCELERATION — Gravecast instants for spell chain recursion
    // Echo Pulse — cheap draw spell that can be recast from graveyard

    cards.push((
        CardBuilder::new("echo_pulse", "Echo Pulse", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .keyword(Keyword::Gravecast(2))
            .effect(CardEffect::DrawCards(2), None)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/echo-pulse_1774298205.png")
            .build(),
        1,
    ));

    // Recursive Spike — cheap bounce with Gravecast

    cards.push((
        CardBuilder::new("recursive_spike", "Recursive Spike", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .keyword(Keyword::Gravecast(1))
            .effect(
                CardEffect::Bounce,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/recursive-spike_1774298329.png")
            .build(),
        1,
    ));

    // SIGNAL STABILIZER - Glitch Counter (Neon)

    cards.push((
        CardBuilder::new(
            "signal_stabilizer",
            "Signal Stabilizer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/signal-stabilizer_1774298400.png")
        .build(),
        1,
    ));

    // ANTI-BURN — sustain against slag's direct damage
    // Spell Shield — life gain on spell cast

    cards.push((
        CardBuilder::new("spell_shield", "Spell Shield", CardKind::Aura)
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::CastInstantOrRitual,
                effect: CardEffect::GainLife(3),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/spell_shield_1775261254.png")
            .build(),
        1,
    ));

    // ANTI-BIO — permanent removal instead of bounce
    // Nullify — exile-based removal so bio can't replay threats

    cards.push((
        CardBuilder::new("nullify", "Nullify", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .keyword(Keyword::Gravecast(1))
            .effect(
                CardEffect::ExileTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/nullify_1775261252.png")
            .build(),
        1,
    ));

    // Dimensional Rift — exile sweeper for when bio floods the board

    cards.push((
        CardBuilder::new("dimensional_rift", "Dimensional Rift", CardKind::Ritual)
            .cost(ManaCost {
                neon: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::DestroyUnitsWithToughnessAtLeast(4), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/dimensional_rift_1775261256.png")
            .build(),
        1,
    ));

    // ── Multi-color operators ────────────────────────────────────────────────

    // Burnphase Infiltrator (neon/slag) — Unit
    cards.push((
        CardBuilder::new(
            "burnphase_infiltrator",
            "Burnphase Infiltrator",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            slag: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/burnphase_infiltrator_1777083886.png")
        .build(),
        1,
    ));

    // Bioluminescent Oracle (bio/neon) — Agent
    cards.push((
        CardBuilder::new(
            "bioluminescent_oracle",
            "Bioluminescent Oracle",
            CardKind::Agent,
        )
        .supertype(CardSupertype::Apex)
        .loyalty(4)
        .cost(ManaCost {
            bio: 1,
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new(
                "Bio-Swarm",
                3,
                3,
                UnitType::BioOrganism,
            )),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "bioluminescent_oracle_plus_1".to_string(),
            name: "[+1] Scry 2".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![CardEffect::ModifyLoyalty(1), CardEffect::Scry(2)]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "bioluminescent_oracle_minus_3".to_string(),
            name: "[-3] Bloom".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-3),
                CardEffect::DrawCards(1),
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .ability(CardAbility {
            id: "bioluminescent_oracle_minus_7".to_string(),
            name: "[-7] Photosynthesis".to_string(),
            cost: AbilityCost::default(),
            effect: CardEffect::Sequence(vec![
                CardEffect::ModifyLoyalty(-7),
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
                CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/bioluminescent_oracle_1777083881.png")
        .build(),
        1,
    ));

    cards
}
