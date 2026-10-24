use cardgame::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, Cost, StatModifierSpec,
    TargetSpec, TokenSpec,
};
use cardgame::types::{
    CardKind, CardRarity, Condition, EffectTargetSpec, ExpireContract, Keyword, ManaCost, ManaType,
    ModifierAmount, Predicate, StatType, TargetKind, TargetScope, TriggerEvent, TriggerLimit,
    UnitType,
};

fn token(name: &str, power: i32, toughness: i32, unit_type: UnitType) -> TokenSpec {
    TokenSpec::new(name, power, toughness, unit_type)
}

/// Generate multicolor combo cards across all 10 two-color pairs.
/// These are designed as synergy pieces that reward playing two colors together.
pub fn generate_multicolor_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // =========================================================================
    // COMBO 1: CHROME + NEON — "Flash Forge"
    // Strategy: Play at instant speed, build artifacts, then convert the board
    // into an evasive kill.
    // =========================================================================

    // Piece A: The engine
    cards.push((
        CardBuilder::new("resonance_relay", "Resonance Relay", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::CastDuringOpponentTurn,
                effect: CardEffect::Sequence(vec![
                    CardEffect::DrawCards(1),
                    CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                ]),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/resonance_relay_1772748123.png")
            .build(),
        1,
    ));

    // Piece B: The enabler — Flash unit that creates artifacts
    cards.push((
        CardBuilder::new(
            "quicksilver_fabricator",
            "Quicksilver Fabricator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::Drone)),
            effect: CardEffect::Scry(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/quicksilver_fabricator_1772748093.png")
        .build(),
        1,
    ));

    // Piece C: The finisher
    cards.push((
        CardBuilder::new(
            "slipstream_convergence",
            "Slipstream Convergence",
            CardKind::Instant,
        )
        .cost(ManaCost {
            chrome: 1,
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .description(
            "Draw 1 card. Create 2 1/1 Drone tokens. Your units gain Stealth until end of turn.",
        )
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::DrawCards(1),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
            ]),
            None,
        )
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/slipstream_convergence_1774306946.png")
        .build(),
        1,
    ));

    // =========================================================================
    // COMBO 2: NEON + GLITCH — "Mind Hack"
    // Strategy: evasive damage that converts into discard and card flow.
    // =========================================================================

    // Piece A: The combat engine
    cards.push((
        CardBuilder::new("breach_array", "Breach Array", CardKind::Relic)
            .cost(ManaCost {
                neon: 1,
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AnyAllyDealtDamage,
                effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::Scry(1)]),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/breach_array_1774306929.png")
            .build(),
        1,
    ));

    // Piece B: The evasive payoff body
    cards.push((
        CardBuilder::new(
            "ghost_protocol",
            "Ghost Protocol",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 1)
        .cost(ManaCost {
            neon: 1,
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::DrawCards(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/ghost_protocol_1774306928.png")
        .build(),
        1,
    ));

    // Piece C: Interaction that also disrupts
    cards.push((
        CardBuilder::new("neural_breach", "Neural Breach", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Counter target spell. Opponent discards 1 card. Scry 1.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CounterTop,
                    CardEffect::Discard(1),
                    CardEffect::Scry(1),
                ]),
                Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Card, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/neural_breach_1774307266.png")
            .build(),
        1,
    ));

    // =========================================================================
    // COMBO 3: SLAG + BIO — "Wildfire Bloom"
    // Strategy: turn disposable biomass into mana, board width, and a lethal swing.
    // =========================================================================

    // Piece A: The recursive token generator
    cards.push((
        CardBuilder::new(
            "pyrogenic_seedling",
            "Pyrogenic Seedling",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            slag: 1,
            bio: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::DamagePlayer(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/pyrogenic_seedling_1774307313.png")
        .build(),
        1,
    ));

    // Piece B: The sacrifice-for-value engine
    cards.push((
        CardBuilder::new("compost_furnace", "Compost Furnace", CardKind::Relic)
            .cost(ManaCost {
                slag: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "compost_furnace_sac".to_string(),
                name: "Burn & Grow".to_string(),
                cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::AddMana(ManaType::Bio, 1),
                    CardEffect::DamageTarget(1),
                ]),
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
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/compost_furnace_1772747702.png")
            .build(),
        1,
    ));

    // Piece C: The burst finisher
    cards.push((
        CardBuilder::new("overgrowth_pyre", "Overgrowth Pyre", CardKind::Ritual)
            .cost(ManaCost {
                slag: 2,
                bio: 2,
                ..Default::default()
            })
            .description("Create three 1/1 Spore tokens. Units you control get +2/+2 and gain Trample until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::ApplyStatModifiers(vec![
                        StatModifierSpec {
                            stat: StatType::Power,
                            amount: ModifierAmount::Fixed(2),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        },
                        StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(2),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        },
                    ]),
                    CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                ]),
                None,
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/overgrowth_pyre_1772747158.png")
            .build(),
        1,
    ));

    // =========================================================================
    // COMBO 4: GLITCH + SLAG — "Blood Ritual"
    // Strategy: sacrifice bodies to turn every death into discard, then cash the
    // discard in for damage.
    // =========================================================================

    // Piece A: The bridge card that connects both synergies
    cards.push((
        CardBuilder::new(
            "dreadforge_ritualist",
            "Dreadforge Ritualist",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 2,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DamagePlayer(2),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/dreadforge_ritualist_1772747774.png")
        .build(),
        1,
    ));

    // Piece B: The sacrifice outlet
    cards.push((
        CardBuilder::new(
            "carrion_smelter",
            "Carrion Smelter",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "carrion_smelter_sac".to_string(),
            name: "Smelt".to_string(),
            cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Sequence(vec![CardEffect::DamageTarget(2), CardEffect::Discard(1)]),
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
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/carrion_smelter_1772747652.png")
        .build(),
        1,
    ));

    // Piece C: The burst spell
    cards.push((
        CardBuilder::new("malice_pact", "Malice Pact", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .description(
                "Deal 3 damage to target player or unit. Opponent discards 1 card. Draw 1 card.",
            )
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageTarget(3),
                    CardEffect::Discard(1),
                    CardEffect::DrawCards(1),
                ]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/malice_pact_1774307253.png")
            .build(),
        1,
    ));

    // =========================================================================
    // COMBO 5: CHROME + BIO — "Living Machines"
    // Strategy: turn organic bodies and small artifacts into a unified board that
    // scales on both axes.
    // =========================================================================

    // Piece A: The counter engine
    cards.push((
        CardBuilder::new("growth_engine", "Growth Engine", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                bio: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "growth_engine_tap".to_string(),
                name: "Cultivate".to_string(),
                cost: AbilityCost::new().with_tap(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::AddMana(ManaType::Chrome, 1),
                ]),
                target_spec: None,
                once_per_turn: false,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/growth_engine_1774307237.png")
            .build(),
        1,
    ));

    // Piece B: The self-growing attacker
    cards.push((
        CardBuilder::new(
            "adaptive_construct",
            "Adaptive Construct",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::AddCounters(1),
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/adaptive_construct_1772747586.png")
        .build(),
        1,
    ));

    // Piece C: The team-wide payoff
    cards.push((
        CardBuilder::new("synthetic_symbiosis", "Synthetic Symbiosis", CardKind::Aura)
            .cost(ManaCost {
                chrome: 1,
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
            .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
            .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
            .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::BioOrganism)))
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(None),
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: Some(Predicate::Any(vec![
                    Predicate::CardKind(CardKind::Unit(UnitType::Drone)),
                    Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism)),
                ])),
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/synthetic_symbiosis_1772751694.png")
            .build(),
        1,
    ));

    cards
}
