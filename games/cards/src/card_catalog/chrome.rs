use crate::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, StatModifierSpec,
    TargetSpec, TokenSpec,
};
use crate::types::{
    CardKind, CardRarity, CardSupertype, CardZone, Condition, DynamicSource, EffectTargetSpec,
    ExpireContract, GrantTarget, Keyword, ManaCost, ManaType, ModifierAmount, PlayerScope,
    Predicate, RevealOutcome, StatType, TargetKind, TargetScope, TriggerEvent, TriggerLimit,
    UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // 6 OPERATORS (3 Apex Units + 3 Apex Agents)

    // Operator 1: Vyktor, Master Architect

    cards.push((
        CardBuilder::new(
            "urza_architect",
            "Vyktor, Master Architect",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::Trample)
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .tap_effect(CardEffect::AddMana(ManaType::Chrome, 1))
        .ability(CardAbility {
            id: "urza_artifact_tutor".to_string(),
            name: "Forge Artifact".to_string(),
            cost: AbilityCost::new().with_tap().with_mana(ManaCost {
                colorless: 2,
                ..Default::default()
            }),
            effect: CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/urza_architect_1772751781.png")
        .build(),
        1,
    ));

    // Operator 2: Kai, Drone Forgemaster

    cards.push((
        CardBuilder::new(
            "sai_forgemaster",
            "Kai, Drone Forgemaster",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 3)
        .cost(ManaCost {
            chrome: 2,
            colorless: 3,
            ..Default::default()
        })
        .tap_effect(CardEffect::CreateToken(TokenSpec::new(
            "Flicker",
            1,
            1,
            UnitType::Drone,
        )))
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/sai_forgemaster_1772748160.png")
        .build(),
        1,
    ));

    // Operator 3: Nyx, Data Salvager

    cards.push((
        CardBuilder::new(
            "emry_salvager",
            "Nyx, Data Salvager",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/emry_salvager_1772747789.png")
        .build(),
        1,
    ));

    // Operator 4: Zenith, Chrome Seeker (Agent)

    cards.push((
        CardBuilder::new("tezzeret_chrome", "Zenith, Chrome Seeker", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(6)
            .cost(ManaCost {
                chrome: 1,
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
                id: "zenith_plus_1_scry".to_string(),
                name: "[+1] Trace — Scry 2".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::Scry(2),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "zenith_plus_1".to_string(),
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
                id: "zenith_minus_2".to_string(),
                name: "[-4] Draw 3".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-4),
                    CardEffect::DrawCards(3),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/tezzeret_chrome_1772668693.png")
            .build(),
        1,
    ));

    // Operator 5: Lyra, Forge Adept (Agent)

    cards.push((
        CardBuilder::new("saheeli_adept", "Lyra, Forge Adept", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(2)
            .cost(ManaCost {
                chrome: 1,
                colorless: 4,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "lyra_plus_1".to_string(),
                name: "[+1] Create Subroutine".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::CreateToken(TokenSpec::new("Subroutine", 1, 1, UnitType::Drone)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "lyra_minus_2".to_string(),
                name: "[-3] Copy Unit".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-3),
                    CardEffect::CopyUnit,
                ]),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/saheeli_adept_1772668460.png")
            .build(),
        1,
    ));

    // Operator 6: Axis, Chrome Scion (Agent)

    cards.push((
        CardBuilder::new("karn_chrome", "Axis, Chrome Scion", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(5)
            .cost(ManaCost {
                chrome: 1,
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
                id: "axis_plus_1".to_string(),
                name: "[+1] Create Drone".to_string(),
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
                id: "axis_minus_3".to_string(),
                name: "[-3] Manifest Drone".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-3),
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "axis_minus_7".to_string(),
                name: "[-7] Drone Surge".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-7),
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(5),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(Some(UnitType::Drone)),
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(5),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(Some(UnitType::Drone)),
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
            .image("https://cdn.tawks.app/cards/karn_chrome_1772668175.png")
            .build(),
        1,
    ));

    // 8 CHEAP CARDS (1-2 mana)

    cards.push((
        CardBuilder::new("chrome_scanner", "Chrome Scanner", CardKind::Relic)
            .cost(ManaCost {
                colorless: 1,
                ..Default::default()
            })
            .tap_effect(CardEffect::RevealTopIf {
                predicate: Predicate::CardKind(CardKind::Relic),
                on_match: RevealOutcome::ToHand,
                on_fail: RevealOutcome::ToBottom,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/chrome_signet_1772667763.png")
            .build(),
        1,
    ));

    // 6 MANA ARTIFACTS

    cards.push((
        CardBuilder::new("sol_core", "Fusion Core", CardKind::Relic)
            .enters_tapped()
            .cost(ManaCost {
                colorless: 1,
                ..Default::default()
            })
            .tap_effect(CardEffect::AddMana(ManaType::Chrome, 1))
            .ability(CardAbility {
                id: "fusion_core_burn".to_string(),
                name: "Burn Out".to_string(),
                cost: AbilityCost::new().with_tap(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::AddMana(ManaType::Chrome, 3),
                    CardEffect::SacrificeThis,
                ]),
                target_spec: None,
                once_per_turn: false,
                uses_stack: false,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/sol2core_1774307376.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("grim_reactor", "Grim Reactor", CardKind::Relic)
            .cost(ManaCost {
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .tap_effect(CardEffect::AddMana(ManaType::Chrome, 2))
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/grim_reactor_1774307225.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("basalt_dynamo", "Basalt Dynamo", CardKind::Relic)
            .cost(ManaCost {
                colorless: 3,
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
            .tap_effect(CardEffect::AddMana(ManaType::Chrome, 2))
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/basalt_dynamo_1774307084.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("everflow_chalice", "Perpetual Chalice", CardKind::Relic)
            .cost(ManaCost {
                colorless: 3,
                ..Default::default()
            })
            .tap_effect(CardEffect::AddManaScaling(
                ManaType::Chrome,
                None,
                Some(CardKind::Relic),
            ))
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/everflow_chalice_1774307184.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("lotus_circuit", "Nexus Circuit", CardKind::Relic)
            .cost(ManaCost {
                colorless: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "nexus_circuit_untap".to_string(),
                name: "Untap Relic".to_string(),
                cost: AbilityCost::new().with_tap(),
                effect: CardEffect::UntapTarget,
                target_spec: Some(
                    TargetSpec::new(TargetScope::SelfPlayer, TargetKind::Card, 1)
                        .with_predicate(Predicate::CardKind(CardKind::Relic)),
                ),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/lotus_circuit_1774307240.png")
            .build(),
        1,
    ));

    // 6 COUNTERSPELLS

    cards.push((
        CardBuilder::new(
            "chrome_requisition",
            "Chrome Requisition",
            CardKind::Instant,
        )
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::Sequence(vec![CardEffect::DamageTarget(3), CardEffect::DrawCards(1)]),
            None,
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/disruption_pulse_1772667891.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("chrome_aegis", "Chrome Aegis", CardKind::Instant)
            .cost(ManaCost {
                chrome: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/null_field_1772668302.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("exile_protocol", "Exile Protocol", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ExileTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/pact_of_denial_1772668341.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("fierce_barrier", "Fierce Barrier", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(CardEffect::CounterTop, None)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/fierce_barrier_1774307187.png")
            .build(),
        1,
    ));

    // 5 NEW PROTECTION/TAX CARDS

    cards.push((
        CardBuilder::new(
            "chrome_sentinel",
            "Chrome Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/chrome_sentinel_1774307110.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "regulatory_framework",
            "Regulatory Framework",
            CardKind::Relic,
        )
        .cost(ManaCost {
            chrome: 2,
            colorless: 4,
            ..Default::default()
        })
        .keyword(Keyword::Throttle(2))
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/regulatory_framework_1774307351.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "safeguard_construct",
            "Safeguard Construct",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 4)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/safeguard_construct_1772748156.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("order_matrix", "Order Matrix", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentCast,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/order_matrix_1774307282.png")
            .build(),
        1,
    ));

    // Pulse Capacitor — chrome finisher relic: charges up on draws,
    // converts stored charge into burst damage. Every 3 draws ~= 1
    // activation, so chrome's card-advantage plan now closes games.
    cards.push((
        CardBuilder::new("pulse_capacitor", "Pulse Capacitor", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::DrewCard,
                effect: CardEffect::AddCounters(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .ability(CardAbility {
                id: "pulse_capacitor_face".to_string(),
                name: "Spend 3 Charge: 4 to face".to_string(),
                cost: AbilityCost::new().with_spend_charge(3),
                effect: CardEffect::DamagePlayer(4),
                target_spec: Some(TargetSpec::new(
                    TargetScope::Opponent,
                    TargetKind::Player,
                    1,
                )),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "pulse_capacitor_unit".to_string(),
                name: "Spend 3 Charge: 5 to unit".to_string(),
                cost: AbilityCost::new().with_spend_charge(3),
                effect: CardEffect::DamageTarget(5),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/order_matrix_1774307282.png")
            .build(),
        1,
    ));

    // Data Overload Protocol — chrome win condition: converts card
    // draws into combat presence. Each draw buffs your drones +1/+0
    // until end of turn, rewarding the color's high-draw plan.
    cards.push((
        CardBuilder::new(
            "data_overload_protocol",
            "Data Overload Protocol",
            CardKind::Aura,
        )
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DrewCard,
            effect: CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                stat: StatType::Power,
                amount: ModifierAmount::Fixed(1),
                expires: ExpireContract::EndOfTurn,
                target: EffectTargetSpec::OwnerUnits(Some(UnitType::Drone)),
            }]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/order_matrix_1774307282.png")
        .build(),
        1,
    ));

    // Drone Firewall Protocol — Aura. Your Drones have Firewall(1)
    // (opponents pay 1 more colorless to target them).
    cards.push((
        CardBuilder::new(
            "drone_firewall_protocol",
            "Drone Firewall Protocol",
            CardKind::Aura,
        )
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::StaticKeywordGrant(
            Box::new(Keyword::Firewall(2)),
            GrantTarget::mine_of(UnitType::Drone),
        ))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/order_matrix_1774307282.png")
        .build(),
        1,
    ));

    // 10 ARTIFACT SYNERGY UNITS

    cards.push((
        CardBuilder::new(
            "foundry_overseer",
            "Foundry Overseer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            chrome: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/foundry_overseer_1772747863.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "nexus_reconfigurer",
            "Nexus Reconfigurer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 4)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::SelfPowerPerKind(
            1,
            CardKind::Unit(UnitType::Drone),
        ))
        .tap_effect(CardEffect::Search(Predicate::CardKind(CardKind::Relic)))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/nexus_reconfigurer_1772748018.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "chrome_battlesphere",
            "Chrome Battlesphere",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(4, 7)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/chrome_battlesphere_1772747672.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "alloy_sculptor",
            "Alloy Sculptor",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
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
        .keyword(Keyword::Vigilance)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/alloy_sculptor_1772747584.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "circuit_relay",
            "Circuit Relay",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
                CardEffect::DrawCards(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/circuit_relay_1772747702.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "grid_sentinel",
            "Grid Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 4)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
                CardEffect::Scry(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Vigilance)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/grid_sentinel_1772747909.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "module_fabricator",
            "Module Fabricator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Flash)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/module_fabricator_1772747968.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "consecrated_engine",
            "Sanctified Engine",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 2)
        .cost(ManaCost {
            chrome: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .keyword(Keyword::SelfToughnessPerKind(
            1,
            CardKind::Unit(UnitType::Drone),
        ))
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/consecrated_engine_1772747718.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "assembly_automaton",
            "Assembly Automaton",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            chrome: 2,
            colorless: 2,
            ..Default::default()
        })
        .tap_effect(CardEffect::CreateToken(TokenSpec::new(
            "Drone",
            1,
            1,
            UnitType::Drone,
        )))
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/assembly_automaton_1772747598.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "phyrexian_reconstructor",
            "Synthesis Reconstructor",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 4)
        .cost(ManaCost {
            chrome: 2,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CopyUnit,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/phyrexian_reconstructor_1772748050.png")
        .build(),
        1,
    ));

    // 10 CARD DRAW / TUTORS

    cards.push((
        CardBuilder::new("rhystic_scanner", "Parasitic Scanner", CardKind::Aura)
            .cost(ManaCost {
                chrome: 2,
                colorless: 2,
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
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/rhystic_scanner_1772668447.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("whir_of_chrome", "Whir of Chrome", CardKind::Instant)
            .cost(ManaCost {
                chrome: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/whir_of_chrome_1772668789.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("matrix_download", "Matrix Download", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DrawCards(2),
                    CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/matrix_download_1772668222.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("fact_or_function", "Parse or Partition", CardKind::Instant)
            .cost(ManaCost {
                chrome: 3,
                colorless: 1,
                ..Default::default()
            })
            .effect(CardEffect::DrawCards(3), None)
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/fact_or_function_1772667992.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("treasure_protocol", "Treasure Protocol", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DrawCards(2),
                    CardEffect::CreateToken(TokenSpec::new("Shield Drone", 0, 4, UnitType::Drone)),
                ]),
                None,
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/treasure_protocol_1774307423.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("the_chrome_ring", "The Chrome Ring", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Sequence(vec![
                    CardEffect::DrawCards(2),
                    CardEffect::CreateToken(TokenSpec::new("Chassis", 3, 3, UnitType::Drone)),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/the_chrome_ring_1774307405.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "chronicle_engine",
            "Chronicle Engine",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .tap_effect(CardEffect::Sequence(vec![
            CardEffect::DrawCards(1),
            CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
        ]))
        .keyword(Keyword::Flash)
        .keyword(Keyword::SelfPowerPerKind(
            1,
            CardKind::Unit(UnitType::Drone),
        ))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/chronicle_engine_1774307122.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("acquisition_matrix", "Acquisition Matrix", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 2,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Search(Predicate::Any(vec![
                    Predicate::CardKind(CardKind::Unit(UnitType::Drone)),
                    Predicate::CardKind(CardKind::Relic),
                ])),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/acquisition_matrix_1772667555.png")
            .build(),
        1,
    ));

    // 6 REMOVAL / BOUNCE

    cards.push((
        CardBuilder::new("displacement_beam", "Displacement Beam", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DestroyTarget,
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/displacement_beam_1772667879.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("judgment_protocol", "Judgment Protocol", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 2,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::BoardWipe,
                    CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                    CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/cyclonic_protocol_1772667838.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("reality_breach", "Reality Breach", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::ExileTarget, CardEffect::DrawCards(1)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/reality_breach_1772668419.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("dispatch_protocol", "Dispatch Protocol", CardKind::Instant)
            .cost(ManaCost {
                chrome: 2,
                colorless: 4,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::DestroyTarget, CardEffect::DrawCards(1)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/dispatch_protocol_1774307166.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "bastion_engine",
            "Bastion Engine",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(7, 8)
        .cost(ManaCost {
            chrome: 3,
            colorless: 4,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::CastInstantOrRitual,
            effect: CardEffect::Sequence(vec![
                CardEffect::Freeze(ExpireContract::Turns(1)),
                CardEffect::DrawCards(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/bastion_engine_1774307084.png")
        .build(),
        1,
    ));

    // 8 FINISHERS

    cards.push((
        CardBuilder::new(
            "forge_colossus",
            "Forge Colossus",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(11, 11)
        .cost(ManaCost {
            chrome: 3,
            colorless: 8,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .keyword(Keyword::Indestructible)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Chassis", 4, 4, UnitType::Drone)),
                CardEffect::CreateToken(TokenSpec::new("Chassis", 4, 4, UnitType::Drone)),
                CardEffect::CreateToken(TokenSpec::new("Chassis", 4, 4, UnitType::Drone)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/forge_colossus_1774307212.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "spine_of_industry",
            "Spine of Industry",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(6, 6)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/spine_of_industry_1772748243.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "core_assembler_engine",
            "Core Assembler Engine",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(6, 6)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Deathtouch)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Crawler", 3, 3, UnitType::Drone)),
                CardEffect::CreateToken(TokenSpec::new("Crawler", 3, 3, UnitType::Drone)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/core_assembler_engine_1772747717.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "steel_titan",
            "Steel Titan",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(6, 6)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DestroyTarget,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: true,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/steel_titan_1772748272.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "platinum_sentinel",
            "Platinum Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 4,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .keyword(Keyword::Indestructible)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/platinum_sentinel_1772748047.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "chrome_colossus",
            "Chrome Colossus",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(10, 10)
        .cost(ManaCost {
            chrome: 3,
            colorless: 8,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::DrawCards(4),
                CardEffect::CreateToken(TokenSpec::new("Chassis", 2, 2, UnitType::Drone)),
                CardEffect::CreateToken(TokenSpec::new("Chassis", 2, 2, UnitType::Drone)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/chrome_colossus_1772747669.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "walking_colossus",
            "Walking Colossus",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(6, 6)
        .cost(ManaCost {
            chrome: 3,
            colorless: 5,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Search(Predicate::CardKind(CardKind::Relic)),
                CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/walking_colossus_1772751835.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "signal_siege",
            "Signal Siege",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
                CardEffect::CreateToken(TokenSpec::new("Drone", 2, 2, UnitType::Drone)),
                CardEffect::DrawCards(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/signal_siege_1774307374.png")
        .build(),
        1,
    ));

    // 10 ADDITIONAL SYNERGY

    cards.push((
        CardBuilder::new(
            "voltaic_sentinel",
            "Voltaic Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .tap_effect(CardEffect::AddMana(ManaType::Chrome, 1))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/voltaic_sentinel_1774307451.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "chrome_phantom",
            "Chrome Phantom",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
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
        .image("https://cdn.tawks.app/cards/chrome_phantom_1774307111.png")
        .build(),
        1,
    ));

    // Relic Sentinel — 0/2 that grows into a real threat as you build
    // out your relic suite.

    cards.push((
        CardBuilder::new(
            "spellskite_drone",
            "Relic Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 3)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Camouflage)
        .keyword(Keyword::SelfPowerPerKind(3, CardKind::Relic))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/spellskite_drone_1772748228.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("duplicator_core", "Duplicator Core", CardKind::Relic)
            .cost(ManaCost {
                chrome: 2,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Chassis", 2, 2, UnitType::Drone)),
                    CardEffect::CreateToken(TokenSpec::new("Chassis", 2, 2, UnitType::Drone)),
                    CardEffect::DrawCards(1),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/duplicator_core_1772747771.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "reconstruction_protocol",
            "Reconstruction Protocol",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            chrome: 2,
            colorless: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::SearchGraveyard(Predicate::CardKind(CardKind::Relic)),
            None,
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/reconstruction_protocol_1772748110.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new("surge_protocol", "Surge Protocol", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 3,
                colorless: 3,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Dynamic {
                            source: DynamicSource::CardsInZone {
                                zone: CardZone::Battlefield,
                                scope: PlayerScope::Owner,
                                kind_filter: Some(CardKind::Relic),
                                unit_type_filter: None,
                            },
                            multiplier: 1,
                            offset: 0,
                        },
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::Drone)),
                    },
                    StatModifierSpec {
                        stat: StatType::Toughness,
                        amount: ModifierAmount::Dynamic {
                            source: DynamicSource::CardsInZone {
                                zone: CardZone::Battlefield,
                                scope: PlayerScope::Owner,
                                kind_filter: Some(CardKind::Relic),
                                unit_type_filter: None,
                            },
                            multiplier: 1,
                            offset: 0,
                        },
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(Some(UnitType::Drone)),
                    },
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/surge_protocol_1774307393.png")
            .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "chrome_golem",
            "Chrome Golem",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 3,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/chrome_golem_1774307098.png")
        .build(),
        1,
    ));

    cards.push((
        CardBuilder::new(
            "arcbound_assembler",
            "Modular Assembler",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            chrome: 2,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Proxy", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/arcbound_assembler_1774307085.png")
        .build(),
        1,
    ));

    // === NEW CARDS ===

    // Bastion Prime — Singularity Apex Drone 3/6, Absorb(2), Vigilance, OnEquip: Draw 1, +0/+1 to Drones

    cards.push((
        CardBuilder::new(
            "bastion_prime",
            "Bastion Prime",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(5, 6)
        .cost(ManaCost {
            chrome: 2,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::Trample)
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/bastion_prime_1774299267.png")
        .build(),
        1,
    ));

    // Modular Sentinel — Unit(Drone) 0/0, Modular(3)

    cards.push((
        CardBuilder::new(
            "modular_sentinel",
            "Modular Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(0, 0)
        .cost(ManaCost {
            chrome: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Modular(3))
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/modular-sentinel_1774298274.png")
        .build(),
        1,
    ));

    // Transmute Relay — artifact-token engine for Chrome. ETB makes a
    // 1/1 Drone and tap produces another Drone each turn, feeding
    // Foundry Overseer / Chrome Bulwark / Prototype Accelerator.

    cards.push((
        CardBuilder::new("transmute_relay", "Transmute Relay", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
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
                id: "transmute_relay_spawn".to_string(),
                name: "Fabricate Drone".to_string(),
                cost: AbilityCost::new().with_tap(),
                effect: CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/transmute-relay_1774298371.png")
            .build(),
        1,
    ));

    // Plating Drone — Unit(Drone) 1/3, Interceptor, ETB: grant Firewall(1) until end of turn

    cards.push((
        CardBuilder::new(
            "plating_drone",
            "Alloy Construct",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Interceptor)
        .keyword(Keyword::SelfPowerPerKind(1, CardKind::Relic))
        .keyword(Keyword::SelfToughnessPerKind(1, CardKind::Relic))
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/plating-drone_1774298315.png")
        .build(),
        2,
    ));

    // Emergency Patch — Instant, grant Indestructible until end of turn + Scry(1)

    cards.push((
        CardBuilder::new("emergency_patch", "Emergency Patch", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
                    CardEffect::Surveil(1),
                ]),
                Some(TargetSpec::new(
                    TargetScope::SelfPlayer,
                    TargetKind::Unit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/emergency-patch_1774298205.png")
            .build(),
        1,
    ));

    // On-theme Assimilate: chrome reprograms the target's firmware, turning
    // it into a compliant Drone on your side.

    cards.push((
        CardBuilder::new(
            "corporate_reprogramming",
            "Corporate Reprogramming",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            chrome: 3,
            colorless: 4,
            ..Default::default()
        })
        .effect(
            CardEffect::DestroyTarget,
            Some(
                TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)
                    .with_predicate(Predicate::Condition(Condition::TargetIsTapped)),
            ),
        )
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/override-protocol_1774298301.png")
        .build(),
        1,
    ));

    // BALANCE BUFF CARDS
    // Override Protocol — removal (destroy-only, Gravecast recasts it once)

    cards.push((
        CardBuilder::new("override_protocol", "Override Protocol", CardKind::Instant)
            .keyword(Keyword::Gravecast(1))
            .cost(ManaCost {
                chrome: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::DestroyTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/override-protocol_1774298301.png")
            .build(),
        1,
    ));

    // Mass Fabrication — scaling tokens off your relic count.

    cards.push((
        CardBuilder::new("drone_convergence", "Mass Fabrication", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Drone", 3, 3, UnitType::Drone)),
                    CardEffect::CreateTokensPerBattlefieldKind(
                        TokenSpec::new("Drone", 1, 1, UnitType::Drone),
                        CardKind::Relic,
                    ),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/drone-convergence_1774298204.png")
            .build(),
        1,
    ));

    // Prototype Accelerator — token-to-draw engine (unlimited triggers)

    cards.push((
        CardBuilder::new(
            "prototype_accelerator",
            "Prototype Accelerator",
            CardKind::Relic,
        )
        .cost(ManaCost {
            chrome: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::TokenCreated,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/prototype-accelerator_1774298315.png")
        .build(),
        1,
    ));

    // Chrome Bulwark — beefy midrange unit that stabilizes

    cards.push((
        CardBuilder::new(
            "chrome_bulwark",
            "Chrome Bulwark",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(4, 6)
        .cost(ManaCost {
            chrome: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
        .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/chrome-bulwark_1774298165.png")
        .build(),
        1,
    ));

    // COMBO ENGINE — Artifact Storm
    // Overclock Matrix — tempo Relic, grants Overclock (Haste) for a turn

    // Spellforge Engine — instant/sorcery payoff: damage + draw

    cards.push((
        CardBuilder::new("spellforge_engine", "Spellforge Engine", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::CastRelic,
                effect: CardEffect::Sequence(vec![
                    CardEffect::DamagePlayer(3),
                    CardEffect::DrawCards(1),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/spellforge-engine_1774298357.png")
            .build(),
        1,
    ));

    // === A. TAX & GRIND ===
    // Compliance Field — opponents lose life when they cast spells (stacks with Throttle)

    cards.push((
        CardBuilder::new("compliance_field", "Compliance Field", CardKind::Aura)
            .cost(ManaCost {
                chrome: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentCast,
                effect: CardEffect::Siphon(2),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/compliance-field_1774298179.png")
            .build(),
        1,
    ));

    // === B. ASSEMBLY WIN ===
    // Convergence Strike — burn spell that scales with relic count

    cards.push((
        CardBuilder::new(
            "convergence_strike",
            "Convergence Strike",
            CardKind::Instant,
        )
        .cost(ManaCost {
            chrome: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::DamagePlayerPerBattlefieldKind(CardKind::Relic),
            None,
        )
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/convergence-strike_1774298178.png")
        .build(),
        1,
    ));

    // === C. TEMPO ARTIFACTS ===
    // Shock Module — cheap relic with ETB chip + cantrip. One-shot burn.

    cards.push((
        CardBuilder::new("shock_module", "Shock Module", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Sequence(vec![
                    CardEffect::DamagePlayer(4),
                    CardEffect::DrawCards(1),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/shock-module_1774298341.png")
            .build(),
        1,
    ));

    // Deployment Pod — cheap relic that creates board presence on entry

    cards.push((
        CardBuilder::new("deployment_pod", "Deployment Pod", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 1, UnitType::Drone)),
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 1, UnitType::Drone)),
                    CardEffect::CreateToken(TokenSpec::new("Drone", 2, 1, UnitType::Drone)),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/deployment-pod_1774298191.png")
            .build(),
        1,
    ));

    // ANTI-DISRUPTION — punish a chosen opponent's discard strategy
    // Data Recovery Matrix — lock onto a player, punish their discards

    cards.push((
        CardBuilder::new(
            "data_recovery_matrix",
            "Data Recovery Matrix",
            CardKind::Relic,
        )
        .cost(ManaCost {
            chrome: 2,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "data_recovery_matrix_choose_player".to_string(),
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
            effect: CardEffect::DamageTarget(3),
            limit: TriggerLimit::OncePerTurn,
            predicate: Some(Predicate::Condition(
                Condition::TargetMatchesRememberedPlayer,
            )),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/data-recovery-matrix_1774298190.png")
        .build(),
        1,
    ));

    // ANTI-DISRUPTION — threat that thrives with empty hand
    // Hardened Processor — cheap vigilant body that draws when it deals damage

    cards.push((
        CardBuilder::new(
            "hardened_processor",
            "Hardened Processor",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 4)
        .cost(ManaCost {
            chrome: 1,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/hardened-processor_1774298243.png")
        .build(),
        1,
    ));

    // ── Multi-color operators ────────────────────────────────────────────────

    // Rogue Protocol (chrome/glitch) — Agent
    cards.push((
        CardBuilder::new("rogue_protocol", "Rogue Protocol", CardKind::Agent)
            .supertype(CardSupertype::Apex)
            .loyalty(5)
            .cost(ManaCost {
                chrome: 1,
                glitch: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::CreateToken(TokenSpec::new("Skitterer", 1, 1, UnitType::Drone)),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .ability(CardAbility {
                id: "rogue_protocol_plus_1".to_string(),
                name: "[+1] Scry 2".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::Scry(2),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "rogue_protocol_minus_3".to_string(),
                name: "[-3] Damage 3".to_string(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-3),
                    CardEffect::DamagePlayer(3),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Singularity)
            .image("https://cdn.tawks.app/cards/rogue_protocol_1777083914.png")
            .build(),
        1,
    ));

    // Nexus Warden (chrome/neon) — Unit
    cards.push((
        CardBuilder::new(
            "nexus_warden",
            "Nexus Warden",
            CardKind::Unit(UnitType::Drone),
        )
        .supertype(CardSupertype::Apex)
        .stats(3, 4)
        .cost(ManaCost {
            chrome: 1,
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Aero)
        .rarity(CardRarity::Singularity)
        .image("https://cdn.tawks.app/cards/nexus_warden_1777083899.png")
        .build(),
        1,
    ));

    cards
}
