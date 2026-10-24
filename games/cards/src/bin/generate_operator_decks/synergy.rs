use cardgame::card::{
    AbilityCost, Card, CardAbility, CardBuilder, CardEffect, CardTrigger, Cost, StatModifierSpec,
    TargetSpec, TokenSpec,
};
use cardgame::types::{
    CardKind, CardRarity, Condition, EffectTargetSpec, ExpireContract, GrantTarget, Keyword,
    ManaCost, ManaType, ModifierAmount, Predicate, StatType, TargetKind, TargetScope, TriggerEvent,
    TriggerLimit, UnitType,
};

/// 100 synergy cards: 10 per mono-color + 50 multicolor (5 per two-color pair).
pub fn generate_synergy_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();
    cards.extend(bio_cards());
    cards.extend(neon_cards());
    cards.extend(glitch_cards());
    cards.extend(slag_cards());
    cards.extend(chrome_cards());
    cards.extend(multicolor_cards());
    cards
}

fn token(name: &str, power: i32, toughness: i32, unit_type: UnitType) -> TokenSpec {
    TokenSpec::new(name, power, toughness, unit_type)
}

// =============================================================================
// BIO — Counters, tokens, anthems, ramp
// =============================================================================
fn bio_cards() -> Vec<(Card, i32)> {
    vec![
        // 1. Bloom Catalyst — ETB counter distributor
        (
            CardBuilder::new("syn_bloom_catalyst", "Bloom Catalyst", CardKind::Unit(UnitType::BioOrganism))
                .stats(2, 2)
                .cost(ManaCost { bio: 1, colorless: 2, ..Default::default() })
                .trigger(CardTrigger {
                    event: TriggerEvent::Entered,
                    effect: CardEffect::AddCounters(2),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_bloom_catalyst_1772748317.png")
                .build(),
            1,
        ),
        // 2. Mycorrhizal Network — counter payoff draw engine
        (
            CardBuilder::new("syn_mycorrhizal_network", "Mycorrhizal Network", CardKind::Aura)
                .cost(ManaCost { bio: 1, colorless: 2, ..Default::default() })
                .description("Whenever another Bio-Organism enters under your control, draw a card. (Once per turn)")
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::OncePerTurn,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_mycorrhizal_network_1772751512.png")
                .build(),
            1,
        ),
        // 3. Mutagenic Spray — cheap counter burst
        (
            CardBuilder::new("syn_mutagenic_spray", "Mutagenic Spray", CardKind::Instant)
                .cost(ManaCost { bio: 1, colorless: 1, ..Default::default() })
                .effect(
                    CardEffect::AddCounters(2),
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Public)
                .image("https://cdn.tawks.app/cards/syn_mutagenic_spray_1772751508.png")
                .build(),
            1,
        ),
        // 4. Spore Swarm — token mass production
        (
            CardBuilder::new("syn_spore_swarm", "Spore Swarm", CardKind::Ritual)
                .cost(ManaCost { bio: 2, colorless: 3, ..Default::default() })
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                        CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                        CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                        CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    ]),
                    None,
                )
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_spore_swarm_1772751679.png")
                .build(),
            1,
        ),
        // 5. Adaptive Predator — aggressive counter grower
        (
            CardBuilder::new("syn_adaptive_predator", "Adaptive Predator", CardKind::Unit(UnitType::BioOrganism))
                .stats(2, 2)
                .cost(ManaCost { bio: 1, colorless: 2, ..Default::default() })
                .keyword(Keyword::Haste)
                .trigger(CardTrigger {
                    event: TriggerEvent::DealtDamage,
                    effect: CardEffect::AddCounters(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_adaptive_predator_1772748302.png")
                .build(),
            1,
        ),
        // 6. Hive Mind Conductor — upkeep counter anthem
        (
            CardBuilder::new("syn_hive_mind_conductor", "Hive Mind Conductor", CardKind::Unit(UnitType::BioOrganism))
                .stats(2, 4)
                .cost(ManaCost { bio: 2, colorless: 2, ..Default::default() })
                .trigger(CardTrigger {
                    event: TriggerEvent::Upkeep,
                    effect: CardEffect::ApplyStatModifiers(vec![
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
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Classified)
                .image("https://cdn.tawks.app/cards/syn_hive_mind_conductor_1772751466.png")
                .build(),
            1,
        ),
        // 7. Fungal Colonization — death-to-tokens conversion
        (
            CardBuilder::new("syn_fungal_colonization", "Fungal Colonization", CardKind::Aura)
                .cost(ManaCost { bio: 2, colorless: 2, ..Default::default() })
                .description("Whenever a Bio-Organism dies, create a 1/1 Spore token.")
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyDied,
                    effect: CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    limit: TriggerLimit::Unlimited,
                    predicate: Some(Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism))),
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_fungal_colonization_1772751410.png")
                .build(),
            1,
        ),
        // 8. Evolution Vat — activated counter doubler
        (
            CardBuilder::new("syn_evolution_vat", "Evolution Vat", CardKind::Relic)
                .cost(ManaCost { bio: 1, colorless: 2, ..Default::default() })
                .ability(CardAbility {
                    id: "evolution_vat_ab".into(),
                    name: "Evolve".into(),
                    cost: AbilityCost::new()
                        .with_mana(ManaCost { bio: 1, colorless: 1, ..Default::default() })
                        .with_tap(),
                    effect: CardEffect::AddCounters(2),
                    target_spec: Some(TargetSpec::new(TargetScope::SelfPlayer, TargetKind::Unit, 1)),
                    once_per_turn: true,
                    uses_stack: true,
                    tap_targets: false,
                    predicate: None,
                })
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_evolution_vat_1772751397.png")
                .build(),
            1,
        ),
        // 9. Bioluminescent Guide — evasive card advantage
        (
            CardBuilder::new("syn_bioluminescent_guide", "Bioluminescent Guide", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .cost(ManaCost { bio: 1, colorless: 1, ..Default::default() })
                .keyword(Keyword::Stealth)
                .trigger(CardTrigger {
                    event: TriggerEvent::DealtDamage,
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::OncePerTurn,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Public)
                .image("https://cdn.tawks.app/cards/syn_bioluminescent_guide_1772748314.png")
                .build(),
            1,
        ),
        // 10. Primordial Surge — top-end finisher
        (
            CardBuilder::new("syn_primordial_surge", "Primordial Surge", CardKind::Ritual)
                .cost(ManaCost { bio: 3, colorless: 5, ..Default::default() })
                .description("Each unit you control gets +4/+4 and Trample until end of turn.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::ApplyStatModifier(StatModifierSpec {
                            stat: StatType::Both,
                            amount: ModifierAmount::Fixed(4),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        }),
                        CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                    ]),
                    None,
                )
                .rarity(CardRarity::Classified)
                .image("https://cdn.tawks.app/cards/syn_primordial_surge_1772751603.png")
                .build(),
            1,
        ),
    ]
}

// =============================================================================
// NEON — Tempo, flash, bounce, evasion, card draw
// =============================================================================
fn neon_cards() -> Vec<(Card, i32)> {
    vec![
        // 1. Phase Drifter — flash evasion body
        (
            CardBuilder::new(
                "syn_phase_drifter",
                "Phase Drifter",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(2, 1)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .keyword(Keyword::Flash)
            .keyword(Keyword::Stealth)
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_phase_drifter_1772751591.png")
            .build(),
            1,
        ),
        // 2. Echo Chamber — spell payoff
        (
            CardBuilder::new("syn_echo_chamber", "Echo Chamber", CardKind::Aura)
                .cost(ManaCost {
                    neon: 2,
                    colorless: 1,
                    ..Default::default()
                })
                .description("Whenever you cast an instant, draw a card.")
                .trigger(CardTrigger {
                    event: TriggerEvent::CastInstantOrRitual,
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::OncePerTurn,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_echo_chamber_1772751380.png")
                .build(),
            1,
        ),
        // 3. Quantum Flicker — reuse ETB
        (
            CardBuilder::new("syn_quantum_flicker", "Quantum Flicker", CardKind::Instant)
                .cost(ManaCost {
                    neon: 1,
                    ..Default::default()
                })
                .description("Bounce target unit you control, then draw a card.")
                .effect(
                    CardEffect::Sequence(vec![CardEffect::Bounce, CardEffect::DrawCards(1)]),
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Public)
                .image("https://cdn.tawks.app/cards/syn_quantum_flicker_1772751603.png")
                .build(),
            1,
        ),
        // 4. Surge Interceptor — flash blocker that bounces
        (
            CardBuilder::new(
                "syn_surge_interceptor",
                "Surge Interceptor",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(1, 3)
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
            .image("https://cdn.tawks.app/cards/syn_surge_interceptor_1772751680.png")
            .build(),
            1,
        ),
        // 5. Velocity Engine — mana advantage from spells
        (
            CardBuilder::new("syn_velocity_engine", "Velocity Engine", CardKind::Relic)
                .cost(ManaCost {
                    neon: 1,
                    colorless: 2,
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
                .image("https://cdn.tawks.app/cards/syn_velocity_engine_1772751715.png")
                .build(),
            1,
        ),
        // 6. Data Torrent — mass bounce
        (
            CardBuilder::new("syn_data_torrent", "Data Torrent", CardKind::Ritual)
                .cost(ManaCost {
                    neon: 2,
                    ..Default::default()
                })
                .effect(CardEffect::DrawCards(3), None)
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_data_torrent_1772751384.png")
                .build(),
            1,
        ),
        // 7. Refraction Agent — damage-to-draw
        (
            CardBuilder::new(
                "syn_refraction_agent",
                "Refraction Agent",
                CardKind::Unit(UnitType::Hacker),
            )
            .stats(3, 2)
            .cost(ManaCost {
                neon: 2,
                colorless: 1,
                ..Default::default()
            })
            .keyword(Keyword::Stealth)
            .trigger(CardTrigger {
                event: TriggerEvent::DealtDamage,
                effect: CardEffect::DrawCards(2),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_refraction_agent_1772751618.png")
            .build(),
            1,
        ),
        // 8. Signal Burst — cheap interaction + scry
        (
            CardBuilder::new("syn_signal_burst", "Signal Burst", CardKind::Instant)
                .cost(ManaCost {
                    neon: 1,
                    colorless: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DrawCards(2), None)
                .rarity(CardRarity::Public)
                .image("https://cdn.tawks.app/cards/syn_signal_burst_1772751632.png")
                .build(),
            1,
        ),
        // 9. Holographic Sentinel — flash lord
        (
            CardBuilder::new(
                "syn_holographic_sentinel",
                "Holographic Sentinel",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(2, 3)
            .cost(ManaCost {
                neon: 2,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Flash)
            .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Drone)))
            .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Drone)))
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_holographic_sentinel_1772751478.png")
            .build(),
            1,
        ),
        // 10. Neon Overload — finisher spell
        (
            CardBuilder::new("syn_neon_overload", "Neon Overload", CardKind::Instant)
                .cost(ManaCost {
                    neon: 3,
                    colorless: 3,
                    ..Default::default()
                })
                .description("Draw 5 cards. Units you control gain Stealth until end of turn.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::DrawCards(5),
                        CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
                    ]),
                    None,
                )
                .rarity(CardRarity::Classified)
                .image("https://cdn.tawks.app/cards/syn_neon_overload_1772751562.png")
                .build(),
            1,
        ),
    ]
}

// =============================================================================
// GLITCH — Discard, hand punishment, graveyard value
// =============================================================================
fn glitch_cards() -> Vec<(Card, i32)> {
    vec![
        // 1. Entropy Leech — discard-on-hit
        (
            CardBuilder::new(
                "syn_entropy_leech",
                "Entropy Leech",
                CardKind::Unit(UnitType::Virus),
            )
            .stats(2, 1)
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
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_entropy_leech_1772751397.png")
            .build(),
            1,
        ),
        // 2. Null Pointer — cheap targeted discard
        (
            CardBuilder::new("syn_null_pointer", "Null Pointer", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 1,
                    colorless: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DestroyTarget,
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .rarity(CardRarity::Public)
                .image("https://cdn.tawks.app/cards/syn_null_pointer_1772751588.png")
                .build(),
            1,
        ),
        // 3. Malware Propagator — discard payoff: tokens
        (
            CardBuilder::new(
                "syn_malware_propagator",
                "Malware Propagator",
                CardKind::Aura,
            )
            .cost(ManaCost {
                glitch: 2,
                colorless: 1,
                ..Default::default()
            })
            .description("Whenever an opponent discards, create a 1/1 Virus token.")
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::CreateToken(TokenSpec::new("Virus", 1, 1, UnitType::Virus)),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_malware_propagator_1772751481.png")
            .build(),
            1,
        ),
        // 4. Memory Siphon — wheel + pain
        (
            CardBuilder::new("syn_memory_siphon", "Memory Siphon", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 2,
                    colorless: 2,
                    ..Default::default()
                })
                .description("Each player discards their hand, then draws 4 cards.")
                .effect(CardEffect::Wheel(4), None)
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_memory_siphon_1772751494.png")
                .build(),
            1,
        ),
        // 5. Feedback Drone — discard payoff: damage
        (
            CardBuilder::new(
                "syn_feedback_drone",
                "Feedback Drone",
                CardKind::Unit(UnitType::Virus),
            )
            .stats(1, 3)
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::DamagePlayer(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_feedback_drone_1772751396.png")
            .build(),
            1,
        ),
        // 6. Corrupted Archive — recurring discard engine
        (
            CardBuilder::new(
                "syn_corrupted_archive",
                "Corrupted Archive",
                CardKind::Relic,
            )
            .cost(ManaCost {
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "corrupted_archive_ab".into(),
                name: "Corrupt".into(),
                cost: AbilityCost::new()
                    .with_mana(ManaCost {
                        glitch: 1,
                        ..Default::default()
                    })
                    .with_tap(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::Discard(1),
                    CardEffect::DrawCards(1),
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
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_corrupted_archive_1772751364.png")
            .build(),
            1,
        ),
        // 7. Void Stalker — evasive discard threat
        (
            CardBuilder::new(
                "syn_void_stalker",
                "Void Stalker",
                CardKind::Unit(UnitType::Virus),
            )
            .stats(3, 2)
            .cost(ManaCost {
                glitch: 2,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Stealth)
            .trigger(CardTrigger {
                event: TriggerEvent::DealtDamage,
                effect: CardEffect::Discard(2),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_void_stalker_1772751715.png")
            .build(),
            1,
        ),
        // 8. System Crash — removal + discard
        (
            CardBuilder::new("syn_system_crash", "System Crash", CardKind::Instant)
                .cost(ManaCost {
                    glitch: 2,
                    colorless: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DestroyTarget,
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Card, 1)),
                )
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_system_crash_1772751702.png")
                .build(),
            1,
        ),
        // 9. Data Parasite — growing discard threat
        (
            CardBuilder::new(
                "syn_data_parasite",
                "Data Parasite",
                CardKind::Unit(UnitType::Virus),
            )
            .stats(1, 1)
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::AddCounters(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_data_parasite_1772751368.png")
            .build(),
            1,
        ),
        // 10. Total System Failure — finisher wipe + discard
        (
            CardBuilder::new(
                "syn_total_system_failure",
                "Total System Failure",
                CardKind::Ritual,
            )
            .cost(ManaCost {
                glitch: 3,
                colorless: 4,
                ..Default::default()
            })
            .description("Destroy all units. Each opponent discards their hand.")
            .effect(
                CardEffect::Sequence(vec![CardEffect::BoardWipe, CardEffect::Discard(7)]),
                None,
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_total_system_failure_1772751697.png")
            .build(),
            1,
        ),
    ]
}

// =============================================================================
// SLAG — Sacrifice, death triggers, burn, tokens-as-fuel
// =============================================================================
fn slag_cards() -> Vec<(Card, i32)> {
    vec![
        // 1. Scrap Gremlin — cheap sac fodder that replaces
        (
            CardBuilder::new(
                "syn_scrap_goblin",
                "Scrap Gremlin",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(1, 1)
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Died,
                effect: CardEffect::CreateToken(TokenSpec::new("Scrap", 1, 1, UnitType::Anomaly)),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_scrap_goblin_1772751630.png")
            .build(),
            1,
        ),
        // 2. Furnace Altar — sac outlet for damage
        (
            CardBuilder::new("syn_furnace_altar", "Furnace Altar", CardKind::Relic)
                .cost(ManaCost {
                    slag: 1,
                    colorless: 1,
                    ..Default::default()
                })
                .ability(CardAbility {
                    id: "furnace_altar_ab".into(),
                    name: "Incinerate".into(),
                    cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)).with_tap(),
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
                .image("https://cdn.tawks.app/cards/syn_furnace_altar_1772751423.png")
                .build(),
            1,
        ),
        // 3. Rekindling Ember — recursive unit
        (
            CardBuilder::new(
                "syn_rekindling_ember",
                "Rekindling Ember",
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
                effect: CardEffect::AddMana(ManaType::Slag, 2),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_rekindling_ember_1772751629.png")
            .build(),
            1,
        ),
        // 4. Chain Detonator — death trigger amplifier
        (
            CardBuilder::new("syn_chain_detonator", "Chain Detonator", CardKind::Aura)
                .cost(ManaCost {
                    slag: 2,
                    colorless: 1,
                    ..Default::default()
                })
                .description("Whenever a unit you control dies, deal 2 damage to each opponent.")
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyDied,
                    effect: CardEffect::DamagePlayer(2),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_chain_detonator_1772748313.png")
                .build(),
            1,
        ),
        // 5. Molten Rebirth — sac + reanimate
        (
            CardBuilder::new("syn_molten_rebirth", "Molten Rebirth", CardKind::Instant)
                .cost(ManaCost {
                    slag: 1,
                    colorless: 2,
                    ..Default::default()
                })
                .description(
                    "Sacrifice a unit. Return target unit from your graveyard to the battlefield.",
                )
                .effect(CardEffect::ReanimateTargetUnit, None)
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_molten_rebirth_1772751497.png")
                .build(),
            1,
        ),
        // 6. Slag Broodmother — token factory
        (
            CardBuilder::new(
                "syn_slag_broodmother",
                "Slag Broodmother",
                CardKind::Unit(UnitType::Megafauna),
            )
            .stats(2, 4)
            .cost(ManaCost {
                slag: 2,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Slag Spawn",
                    1,
                    1,
                    UnitType::Anomaly,
                )),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_slag_broodmother_1772751668.png")
            .build(),
            1,
        ),
        // 7. Reckless Detonation — burn spell
        (
            CardBuilder::new(
                "syn_reckless_detonation",
                "Reckless Detonation",
                CardKind::Instant,
            )
            .cost(ManaCost {
                slag: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::DamageTarget(4),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_reckless_detonation_1772751615.png")
            .build(),
            1,
        ),
        // 8. Smelt Foreman — death trigger lord
        (
            CardBuilder::new(
                "syn_smelt_foreman",
                "Smelt Foreman",
                CardKind::Unit(UnitType::Human),
            )
            .stats(2, 3)
            .cost(ManaCost {
                slag: 2,
                colorless: 1,
                ..Default::default()
            })
            .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::Anomaly)))
            .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::Anomaly)))
            .trigger(CardTrigger {
                event: TriggerEvent::AllyDied,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_smelt_foreman_1772751680.png")
            .build(),
            1,
        ),
        // 9. Volatile Heap — sac for cards
        (
            CardBuilder::new("syn_volatile_heap", "Volatile Heap", CardKind::Relic)
                .cost(ManaCost {
                    slag: 1,
                    colorless: 1,
                    ..Default::default()
                })
                .ability(CardAbility {
                    id: "volatile_heap_ab".into(),
                    name: "Salvage".into(),
                    cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)).with_tap(),
                    effect: CardEffect::DrawCards(2),
                    target_spec: None,
                    once_per_turn: true,
                    uses_stack: true,
                    tap_targets: false,
                    predicate: None,
                })
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_volatile_heap_1772751717.png")
                .build(),
            1,
        ),
        // 10. Inferno Cascade — finisher burn
        (
            CardBuilder::new("syn_inferno_cascade", "Inferno Cascade", CardKind::Ritual)
                .cost(ManaCost {
                    slag: 3,
                    colorless: 4,
                    ..Default::default()
                })
                .description("Deal 4 damage to each opponent and each unit they control.")
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::DamagePlayer(4),
                        CardEffect::ApplyStatModifier(StatModifierSpec {
                            stat: StatType::Both,
                            amount: ModifierAmount::Fixed(-5),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OpponentUnits(None),
                        }),
                    ]),
                    None,
                )
                .rarity(CardRarity::Classified)
                .image("https://cdn.tawks.app/cards/syn_inferno_cascade_1772751478.png")
                .build(),
            1,
        ),
    ]
}

// =============================================================================
// CHROME — Artifacts, tax, protection, mana advantage
// =============================================================================
fn chrome_cards() -> Vec<(Card, i32)> {
    vec![
        // 1. Protocol Enforcer — tax body
        (
            CardBuilder::new(
                "syn_protocol_enforcer",
                "Protocol Enforcer",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(2, 3)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Throttle(1))
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_protocol_enforcer_1772751603.png")
            .build(),
            1,
        ),
        // 2. Fortified Chassis — cheap indestructible shield
        (
            CardBuilder::new(
                "syn_fortified_chassis",
                "Fortified Chassis",
                CardKind::Instant,
            )
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::GrantKeyword(Keyword::Indestructible, ExpireContract::EndOfTurn),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_fortified_chassis_1772751410.png")
            .build(),
            1,
        ),
        // 3. Assembly Line — artifact unit ETB tokens
        (
            CardBuilder::new("syn_assembly_line", "Assembly Line", CardKind::Ritual)
                .cost(ManaCost {
                    chrome: 1,
                    colorless: 2,
                    ..Default::default()
                })
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                        CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                    ]),
                    None,
                )
                .rarity(CardRarity::Private)
                .image("https://cdn.tawks.app/cards/syn_assembly_line_1774299309.png")
                .build(),
            1,
        ),
        // 4. Data Vault — draw engine off artifacts
        (
            CardBuilder::new("syn_data_vault", "Data Vault", CardKind::Relic)
                .cost(ManaCost {
                    chrome: 1,
                    colorless: 2,
                    ..Default::default()
                })
                .description(
                    "Whenever an artifact enters under your control, draw a card. (Once per turn)",
                )
                .trigger(CardTrigger {
                    event: TriggerEvent::Entered,
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::OncePerTurn,
                    predicate: Some(Predicate::CardKind(CardKind::Relic)),
                    optional_cost: None,
                    is_optional: false,
                })
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_data_vault_1772751380.png")
                .build(),
            1,
        ),
        // 5. Chrome Vanguard — aggressive artifact unit
        (
            CardBuilder::new(
                "syn_chrome_vanguard",
                "Chrome Vanguard",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(3, 2)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Vigilance)
            .trigger(CardTrigger {
                event: TriggerEvent::Attacked,
                effect: CardEffect::CreateToken(TokenSpec::new("Drone", 1, 1, UnitType::Drone)),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_chrome_vanguard_1772751349.png")
            .build(),
            1,
        ),
        // 6. Reinforced Barricade — defensive wall
        (
            CardBuilder::new(
                "syn_reinforced_barricade",
                "Reinforced Barricade",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(0, 6)
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .keyword(Keyword::Indestructible)
            .keyword(Keyword::Interceptor)
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_reinforced_barricade_1772751617.png")
            .build(),
            1,
        ),
        // 7. Precision Strike — targeted exile removal
        (
            CardBuilder::new(
                "syn_precision_strike",
                "Precision Strike",
                CardKind::Instant,
            )
            .cost(ManaCost {
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::DamageTarget(3),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_precision_strike_1772751590.png")
            .build(),
            1,
        ),
        // 8. Forge Colossus — finisher drone
        (
            CardBuilder::new(
                "syn_forge_colossus",
                "Forge Colossus",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(6, 6)
            .cost(ManaCost {
                chrome: 3,
                colorless: 4,
                ..Default::default()
            })
            .keyword(Keyword::Trample)
            .keyword(Keyword::Vigilance)
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::DrawCards(2),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_forge_colossus_1772751408.png")
            .build(),
            1,
        ),
        // 9. Mana Capacitor — ramp artifact
        (
            CardBuilder::new("syn_mana_capacitor", "Mana Capacitor", CardKind::Relic)
                .cost(ManaCost {
                    colorless: 3,
                    ..Default::default()
                })
                .tap_effect(CardEffect::AddMana(ManaType::Chrome, 2))
                .rarity(CardRarity::Restricted)
                .image("https://cdn.tawks.app/cards/syn_mana_capacitor_1772751492.png")
                .build(),
            1,
        ),
        // 10. Corporate Lockdown — stax finisher
        (
            CardBuilder::new(
                "syn_corporate_lockdown",
                "Corporate Lockdown",
                CardKind::Aura,
            )
            .cost(ManaCost {
                chrome: 2,
                colorless: 3,
                ..Default::default()
            })
            .description("Opponents' spells cost 2 more. Your Drones get +2/+2.")
            .keyword(Keyword::Throttle(2))
            .keyword(Keyword::StaticPowerBuff(2, Some(UnitType::Drone)))
            .keyword(Keyword::StaticToughnessBuff(2, Some(UnitType::Drone)))
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_corporate_lockdown_1772751361.png")
            .build(),
            1,
        ),
    ]
}

// =============================================================================
// MULTICOLOR — 5 cards per two-color pair × 10 pairs = 50 cards
// Existing 5 pairs (Chrome+Neon, Neon+Glitch, Slag+Bio, Glitch+Slag, Chrome+Bio)
// get 2 more cards each. New 5 pairs get 5 each. Total = 10 + 40 = 50.
// Actually: all 10 pairs get 5 cards = 50.
// =============================================================================
fn multicolor_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // =========================================================================
    // PAIR 1: CHROME + NEON — "Flash Forge" (5 new cards)
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_chrome_neon_1",
            "Chrono Fabricator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            neon: 1,
            colorless: 2,
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
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_chrome_neon_1_1772751302.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_neon_2", "Flashforge Nexus", CardKind::Aura)
            .cost(ManaCost {
                chrome: 1,
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Whenever you cast a spell during an opponent's turn, draw 1 card and create a 1/1 Drone token. (Once per turn)")
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
            .image("https://cdn.tawks.app/cards/syn_chrome_neon_2_1772751303.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_neon_3", "Photon Lattice", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                neon: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::CastInstantOrRitual,
                effect: CardEffect::Sequence(vec![
                    CardEffect::AddMana(ManaType::Chrome, 1),
                    CardEffect::AddMana(ManaType::Neon, 1),
                ]),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_chrome_neon_3_1772751315.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_neon_4",
            "Quicksilver Warden",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 4)
        .cost(ManaCost {
            chrome: 1,
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Vigilance)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/syn_chrome_neon_4_1772751320.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_neon_5",
            "Hypercycle Convergence",
            CardKind::Instant,
        )
        .cost(ManaCost {
            chrome: 2,
            neon: 2,
            colorless: 2,
            ..Default::default()
        })
        .description(
            "Draw 2 cards. Create 3 1/1 Drone tokens. Your units gain Stealth until end of turn.",
        )
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::DrawCards(2),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
            ]),
            None,
        )
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_chrome_neon_5_1772751319.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 2: NEON + GLITCH — "Mind Hack" (5 new cards)
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_neon_glitch_1",
            "Phantom Infiltrator",
            CardKind::Unit(UnitType::Hacker),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::Scry(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_neon_glitch_1_1772751536.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_glitch_2", "Synaptic Overwrite", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                glitch: 1,
                colorless: 1,
                ..Default::default()
            })
            .description(
                "Return target unit to its owner's hand. Opponent discards 1 card. Draw 1 card.",
            )
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Bounce,
                    CardEffect::Discard(1),
                    CardEffect::DrawCards(1),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_neon_glitch_2_1772751537.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_glitch_3",
            "Cognitive Drain",
            CardKind::Aura,
        )
        .cost(ManaCost {
            neon: 1,
            glitch: 1,
            colorless: 2,
            ..Default::default()
        })
        .description("Whenever you draw a card, opponent discards 1 card. Whenever an opponent discards, draw 1 card. Each ability triggers only once per turn.")
        .trigger(CardTrigger {
            event: TriggerEvent::DrewCard,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_neon_glitch_3_1772751549.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_glitch_4",
            "Flickering Virus",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 1)
        .cost(ManaCost {
            neon: 1,
            glitch: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/syn_neon_glitch_4_1772751550.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_glitch_5",
            "Mind Shatter Protocol",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            neon: 2,
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .description("Each player discards their hand, then draws 4 cards. Draw 2 cards. Your units gain Stealth until end of turn.")
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::Wheel(4),
                CardEffect::DrawCards(2),
                CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
            ]),
            None,
        )
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_neon_glitch_5_1772751551.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 3: SLAG + BIO — "Wildfire Bloom" (5 new cards)
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_slag_bio_1",
            "Ashbloom Shaman",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            slag: 1,
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::AllyDied,
            effect: CardEffect::AddCounters(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_slag_bio_1_1772751650.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_slag_bio_2", "Decomposition Engine", CardKind::Relic)
            .cost(ManaCost {
                slag: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "decomp_engine_ab".into(),
                name: "Decompose".into(),
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
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_slag_bio_2_1772751644.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_slag_bio_3",
            "Volcanic Sporemother",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(4, 4)
        .cost(ManaCost {
            slag: 2,
            bio: 2,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::DamagePlayer(3),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_slag_bio_3_1772751648.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_slag_bio_4", "Reclaim the Ashes", CardKind::Ritual)
            .cost(ManaCost {
                slag: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Create 2 1/1 Spore tokens. Units you control get +1/+1 and gain Trample until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
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
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_slag_bio_4_1772751665.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_slag_bio_5", "Eruption of Life", CardKind::Ritual)
            .cost(ManaCost {
                slag: 3,
                bio: 3,
                colorless: 2,
                ..Default::default()
            })
            .description("Destroy all units. Create 3 2/2 Spore Megafauna tokens. Your units gain Trample until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::BoardWipe,
                    CardEffect::CreateToken(token("Spore Megafauna", 2, 2, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore Megafauna", 2, 2, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore Megafauna", 2, 2, UnitType::BioOrganism)),
                    CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                ]),
                None,
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_slag_bio_5_1772751664.png")
            .build(),
        1,
    ));

    // =========================================================================
    // PAIR 4: GLITCH + SLAG — "Blood Ritual" (5 new cards)
    // =========================================================================
    cards.push((
        CardBuilder::new("syn_glitch_slag_1", "Torment Engine", CardKind::Aura)
            .cost(ManaCost {
                glitch: 1,
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .description(
                "Whenever a unit dies, opponent discards 1 card and loses 1 life. (Once per turn)",
            )
            .trigger(CardTrigger {
                event: TriggerEvent::AllyDied,
                effect: CardEffect::Sequence(vec![
                    CardEffect::Discard(1),
                    CardEffect::DamagePlayer(1),
                ]),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_glitch_slag_1_1772751450.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_glitch_slag_2",
            "Guttersnipe Hacker",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 1)
        .cost(ManaCost {
            glitch: 1,
            slag: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Sequence(vec![CardEffect::Discard(1), CardEffect::DamagePlayer(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Public)
        .image("https://cdn.tawks.app/cards/syn_glitch_slag_2_1772751450.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_glitch_slag_3", "Hemorrhage Ritual", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Deal 3 damage to target player or unit. Opponent discards 1 card.")
            .effect(
                CardEffect::Sequence(vec![CardEffect::DamageTarget(3), CardEffect::Discard(1)]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_glitch_slag_3_1772751451.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_glitch_slag_4",
            "Cinder Phantom",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 1)
        .cost(ManaCost {
            glitch: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![CardEffect::DamagePlayer(2), CardEffect::Discard(1)]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_glitch_slag_4_1772751466.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_glitch_slag_5",
            "Annihilation Protocol",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            glitch: 2,
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .description("Destroy all units. Opponent discards 2 cards. Return a unit from your graveyard to the battlefield.")
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::BoardWipe,
                CardEffect::Discard(2),
                CardEffect::ReanimateTargetUnit,
            ]),
            None,
        )
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_glitch_slag_5_1774299307.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 5: CHROME + BIO — "Living Machines" (5 new cards)
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_chrome_bio_1",
            "Biosynthetic Drone",
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
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_chrome_bio_1_1772748329.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_bio_2",
            "Augmented Growth",
            CardKind::Aura,
        )
        .cost(ManaCost {
            chrome: 1,
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .description("Whenever a Drone or Bio-Organism enters under your control, draw 1 card. (Once per turn)")
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
        .image("https://cdn.tawks.app/cards/syn_chrome_bio_2_1774299308.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_bio_3",
            "Cybernetic Hydra",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(4, 4)
        .cost(ManaCost {
            chrome: 2,
            bio: 2,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::AddCounters(4),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_chrome_bio_3_1772748331.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_bio_4", "Symbiotic Plating", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                bio: 1,
                ..Default::default()
            })
            .description("Target unit gets +3/+3 and gains Camouflage until end of turn. Create a 1/1 Spore token.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ApplyStatModifier(StatModifierSpec {
                        stat: StatType::Both,
                        amount: ModifierAmount::Fixed(3),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::Targets,
                    }),
                    CardEffect::GrantKeyword(Keyword::Camouflage, ExpireContract::EndOfTurn),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_chrome_bio_4_1772748344.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_bio_5",
            "Evolutionary Singularity",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            chrome: 2,
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .description("Create 2 2/2 Drone tokens and 2 1/1 Spore tokens. Units you control get +2/+2 until end of turn.")
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::CreateToken(token("Drone", 2, 2, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 2, 2, UnitType::Drone)),
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
            ]),
            None,
        )
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_chrome_bio_5_1772748346.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 6: CHROME + GLITCH — "Digital Oppression" (NEW)
    // Tax + Discard: Lock opponents out of resources from both directions.
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_chrome_glitch_1",
            "Censor Protocol",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(1, 4)
        .cost(ManaCost {
            chrome: 1,
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Throttle(1))
        .trigger(CardTrigger {
            event: TriggerEvent::OpponentCast,
            effect: CardEffect::Discard(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_chrome_glitch_1_1772748343.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_glitch_2", "Surveillance Grid", CardKind::Aura)
            .cost(ManaCost { chrome: 1, glitch: 1, colorless: 2, ..Default::default() })
            .description("Whenever an opponent draws a card, they lose 1 life. Whenever an opponent discards, you draw a card. (Once per turn)")
            .trigger(CardTrigger {
                event: TriggerEvent::OpponentDrew,
                effect: CardEffect::DamagePlayer(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_chrome_glitch_2_1772748357.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_glitch_3",
            "Compliance Virus",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            chrome: 1,
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
        .image("https://cdn.tawks.app/cards/syn_chrome_glitch_3_1772748361.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_glitch_4", "Blackout Mandate", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .description("Opponent discards 2 cards. Create 2 1/1 Drone tokens.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Discard(2),
                    CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                    CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_chrome_glitch_4_1772748363.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_glitch_5",
            "Totalitarian Override",
            CardKind::Aura,
        )
        .cost(ManaCost {
            chrome: 2,
            glitch: 2,
            colorless: 2,
            ..Default::default()
        })
        .description(
            "Opponents' spells cost 2 more. Whenever an opponent discards, create a 1/1 Drone.",
        )
        .keyword(Keyword::Throttle(2))
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::OpponentCast,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_chrome_glitch_5_1772751303.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 7: CHROME + SLAG — "War Machines" (NEW)
    // Artifacts + Sacrifice: Build artifacts, sacrifice them for value.
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_chrome_slag_1",
            "Scrapyard Overseer",
            CardKind::Unit(UnitType::Human),
        )
        .stats(3, 3)
        .cost(ManaCost {
            chrome: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "scrapyard_overseer_ab".into(),
            name: "Reclaim".into(),
            cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::Sequence(vec![
                CardEffect::DamagePlayer(2),
                CardEffect::DrawCards(1),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
            ]),
            target_spec: None,
            once_per_turn: true,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_chrome_slag_1_1772751333.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_slag_2",
            "Warforged Automaton",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(4, 3)
        .cost(ManaCost {
            chrome: 1,
            slag: 1,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::CreateToken(token("Scrap Drone", 2, 2, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_chrome_slag_2_1772751337.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_slag_3", "Forge Recycler", CardKind::Relic)
            .cost(ManaCost {
                chrome: 1,
                slag: 1,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::AllyDied,
                effect: CardEffect::Sequence(vec![
                    CardEffect::AddMana(ManaType::Chrome, 1),
                    CardEffect::AddMana(ManaType::Slag, 1),
                ]),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_chrome_slag_3_1772751334.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_chrome_slag_4", "Industrial Purge", CardKind::Instant)
            .cost(ManaCost {
                chrome: 1,
                slag: 1,
                colorless: 2,
                ..Default::default()
            })
            .description("Deal 4 damage to target unit. Create a 2/2 Drone token.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageTarget(4),
                    CardEffect::CreateToken(token("Drone", 2, 2, UnitType::Drone)),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_chrome_slag_4_1772751348.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_chrome_slag_5",
            "Doomsday Fabricator",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(5, 5)
        .cost(ManaCost {
            chrome: 2,
            slag: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Attacked,
            effect: CardEffect::DamagePlayer(3),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::DamagePlayer(3),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
                CardEffect::CreateToken(token("Drone", 1, 1, UnitType::Drone)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_chrome_slag_5_1772751349.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 8: NEON + BIO — "Quantum Evolution" (NEW)
    // Evasion + Counters: Unblockable units that grow.
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_neon_bio_1",
            "Biophotonic Runner",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(1, 1)
        .cost(ManaCost {
            neon: 1,
            bio: 1,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::AddCounters(2),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_neon_bio_1_1772751512.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_bio_2", "Phase Vine", CardKind::Aura)
            .cost(ManaCost {
                neon: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .description(
                "Whenever your unit deals combat damage, create a 1/1 Spore token. (Once per turn)",
            )
            .trigger(CardTrigger {
                event: TriggerEvent::AnyAllyDealtDamage,
                effect: CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_neon_bio_2_1772751525.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_bio_3",
            "Flicker Spore",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            neon: 1,
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_neon_bio_3_1772751525.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_bio_4", "Temporal Bloom", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Add 3 counters to target unit. It gains Stealth until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::AddCounters(3),
                    CardEffect::GrantKeyword(Keyword::Stealth, ExpireContract::EndOfTurn),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_neon_bio_4_1772751523.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_bio_5",
            "Quantum Apex Predator",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(5, 5)
        .cost(ManaCost {
            neon: 2,
            bio: 2,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Stealth)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::Sequence(vec![
                CardEffect::AddCounters(3),
                CardEffect::DrawCards(2),
            ]),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Classified)
        .image("https://cdn.tawks.app/cards/syn_neon_bio_5_1772751539.png")
        .build(),
        1,
    ));

    // =========================================================================
    // PAIR 9: NEON + SLAG — "Blitz Burn" (NEW)
    // Speed + Burn: Haste units that deal extra damage.
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_neon_slag_1",
            "Arc Runner",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 1)
        .cost(ManaCost {
            neon: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .trigger(CardTrigger {
            event: TriggerEvent::Attacked,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_neon_slag_1_1772751562.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_slag_2", "Voltaic Blitz", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Deal 3 damage to target unit or player. Draw a card.")
            .effect(
                CardEffect::Sequence(vec![CardEffect::DamageTarget(3), CardEffect::DrawCards(1)]),
                Some(TargetSpec::new(
                    TargetScope::Any,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .rarity(CardRarity::Public)
            .image("https://cdn.tawks.app/cards/syn_neon_slag_2_1772751562.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_neon_slag_3",
            "Shockwave Anomaly",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(3, 2)
        .cost(ManaCost {
            neon: 1,
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DamagePlayer(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/syn_neon_slag_3_1772751579.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_slag_4", "Overcharge", CardKind::Aura)
            .cost(ManaCost { neon: 1, slag: 1, colorless: 1, ..Default::default() })
            .description("Your units have Haste. Whenever your unit deals combat damage, deal 1 damage to target player. (Once per turn)")
            .keyword(Keyword::StaticHasteGrant(GrantTarget::mine()))
            .trigger(CardTrigger {
                event: TriggerEvent::AnyAllyDealtDamage,
                effect: CardEffect::DamagePlayer(1),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_neon_slag_4_1772751574.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_neon_slag_5", "Supernova Strike", CardKind::Ritual)
            .cost(ManaCost {
                neon: 2,
                slag: 2,
                colorless: 2,
                ..Default::default()
            })
            .description("Deal 4 damage to target player. Draw 2 cards. Units you control get +2/+0 until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamagePlayer(4),
                    CardEffect::DrawCards(2),
                    CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(2),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::OwnerUnits(None),
                    }]),
                ]),
                Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Player, 1)),
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_neon_slag_5_1772751574.png")
            .build(),
        1,
    ));

    // =========================================================================
    // PAIR 10: GLITCH + BIO — "Plague Spreaders" (NEW)
    // Discard + Tokens/Counters: Discard fuels organic growth.
    // =========================================================================
    cards.push((
        CardBuilder::new(
            "syn_glitch_bio_1",
            "Blight Cultivator",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(1, 3)
        .cost(ManaCost {
            glitch: 1,
            bio: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::CreateToken(token("Plague Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_glitch_bio_1_1772751425.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_glitch_bio_2", "Toxic Feedback", CardKind::Aura)
            .cost(ManaCost { glitch: 1, bio: 1, colorless: 1, ..Default::default() })
            .description("Whenever an opponent discards, your units get +1/+1 until end of turn. (Once per turn)")
            .trigger(CardTrigger {
                event: TriggerEvent::Discarded,
                effect: CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec { stat: StatType::Power, amount: ModifierAmount::Fixed(1), expires: ExpireContract::EndOfTurn, target: EffectTargetSpec::OwnerUnits(None) },
                    StatModifierSpec { stat: StatType::Toughness, amount: ModifierAmount::Fixed(1), expires: ExpireContract::EndOfTurn, target: EffectTargetSpec::OwnerUnits(None) },
                ]),
                limit: TriggerLimit::OncePerTurn,
                predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/syn_glitch_bio_2_1772751422.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_glitch_bio_3", "Fungal Corruption", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                bio: 1,
                colorless: 1,
                ..Default::default()
            })
            .description("Opponent discards 2 cards. Create 2 1/1 Spore tokens.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Discard(2),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/syn_glitch_bio_3_1772751438.png")
            .build(),
        1,
    ));
    cards.push((
        CardBuilder::new(
            "syn_glitch_bio_4",
            "Viral Overgrowth",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            bio: 1,
            colorless: 2,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Discarded,
            effect: CardEffect::AddCounters(2),
            limit: TriggerLimit::Unlimited,
            predicate: Some(Predicate::Condition(Condition::TargetIsOpponentPlayer)),
            optional_cost: None,
            is_optional: false,
        })
        .keyword(Keyword::Trample)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/syn_glitch_bio_4_1772751438.png")
        .build(),
        1,
    ));
    cards.push((
        CardBuilder::new("syn_glitch_bio_5", "Pandemic Protocol", CardKind::Ritual)
            .cost(ManaCost { glitch: 2, bio: 2, colorless: 3, ..Default::default() })
            .description("Opponent discards 3 cards. Create 2 3/3 Plague Megafauna tokens and 2 1/1 Spore tokens. Your units gain Trample until end of turn.")
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Discard(3),
                    CardEffect::CreateToken(token("Plague Megafauna", 3, 3, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Plague Megafauna", 3, 3, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(token("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                ]),
                None,
            )
            .rarity(CardRarity::Classified)
            .image("https://cdn.tawks.app/cards/syn_glitch_bio_5_1772751437.png")
            .build(),
        1,
    ));

    cards
}
