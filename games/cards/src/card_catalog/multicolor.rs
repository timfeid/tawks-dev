use crate::card::{
    AbilityCost, AttachmentSpec, Card, CardAbility, CardBuilder, CardEffect, CardTrigger,
    StatModifierSpec, TargetSpec, TokenSpec,
};
use crate::types::{
    CardKind, CardRarity, EffectTargetSpec, ExpireContract, Keyword, ManaCost, ManaType,
    ModifierAmount, Predicate, StatType, TargetKind, TargetScope, TriggerEvent, TriggerLimit,
    UnitType,
};

pub fn all_cards() -> Vec<(Card, i32)> {
    let mut cards = Vec::new();

    // ── BIO + GLITCH (organic corruption) ───────────────────────────────────

    // Blight Symbiont — Wither unit, OnDamageDealt: create Spore, OnGraveyardEntry: AddCounters(1)
    cards.push((
        CardBuilder::new(
            "blight_symbiont",
            "Blight Symbiont",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 1,
            glitch: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Wither)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/blight_symbiont_1774304087.png")
        .build(),
        1,
    ));

    // Sporogenic Plague — Sorcery, Entomb(IsUnit) + Conscript + GainLife(3)
    cards.push((
        CardBuilder::new("sporogenic_plague", "Sporogenic Plague", CardKind::Ritual)
            .cost(ManaCost {
                bio: 1,
                glitch: 1,
                colorless: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Entomb(Predicate::IsUnit),
                    CardEffect::Conscript,
                    CardEffect::GainLife(3),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/sporogenic_plague_1774304167.png")
            .build(),
        1,
    ));

    // ── BIO + NEON (living data) ────────────────────────────────────────────

    // Photon Cultivator — Evolve unit, OnOtherUnitPlayed: Transmit(1)
    cards.push((
        CardBuilder::new(
            "photon_cultivator",
            "Photon Cultivator",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(2, 3)
        .cost(ManaCost {
            bio: 1,
            neon: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Evolve)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(None),
            effect: CardEffect::Transmit(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/photon_cultivator_1774304148.png")
        .build(),
        1,
    ));

    // Synapse Bloom — Enchantment, OnDrawCard: create Spore (once/turn), OnTokenCreated: Scry(1)
    cards.push((
        CardBuilder::new("synapse_bloom", "Synapse Bloom", CardKind::Aura)
            .cost(ManaCost {
                bio: 1,
                neon: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::DrewCard,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Spore",
                    1,
                    1,
                    UnitType::BioOrganism,
                )),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .trigger(CardTrigger {
                event: TriggerEvent::TokenCreated,
                effect: CardEffect::Surveil(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/synapse_bloom_1774304182.png")
            .build(),
        1,
    ));

    // ── BIO + CHROME (cyborg augmentation) ──────────────────────────────────

    // Graft Engine — Modular(2) unit, ETB: AddCounters(1) to target, OnDeath: GainLife(3)
    cards.push((
        CardBuilder::new(
            "graft_engine",
            "Graft Engine",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 4)
        .cost(ManaCost {
            bio: 1,
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Modular(2))
        .effect(
            CardEffect::AddCounters(1),
            Some(TargetSpec::new(
                TargetScope::SelfPlayer,
                TargetKind::Unit,
                1,
            )),
        )
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::GainLife(3),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/graft_engine_1774304117.png")
        .build(),
        1,
    ));

    // Living Plating — Equipment, grants +2/+2 and Absorb(1), OnEquip: GainLife(2)
    let mut living_plating = CardBuilder::new("living_plating", "Living Plating", CardKind::Relic)
        .cost(ManaCost {
            bio: 1,
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Equipped,
            effect: CardEffect::GainLife(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .ability(CardAbility {
            id: "living_plating_equip".to_string(),
            name: "Equip".to_string(),
            cost: AbilityCost::new().with_mana(ManaCost {
                colorless: 2,
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
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/living_plating_1774304129.png")
        .build();
    living_plating.attachment = Some(AttachmentSpec {
        modifiers: vec![
            StatModifierSpec {
                stat: StatType::Power,
                amount: ModifierAmount::Fixed(2),
                expires: ExpireContract::Never,
                target: EffectTargetSpec::Targets,
            },
            StatModifierSpec {
                stat: StatType::Toughness,
                amount: ModifierAmount::Fixed(2),
                expires: ExpireContract::Never,
                target: EffectTargetSpec::Targets,
            },
        ],
        keywords: vec![Keyword::Absorb(1)],
    });
    living_plating.auto_description = living_plating.auto_description();
    living_plating.description = living_plating.auto_description.clone();
    cards.push((living_plating, 1));

    // ── BIO + SLAG (volcanic life) ──────────────────────────────────────────

    // Magma Bloom — Megafauna, Trample, ETB: DamageEachOpponentUnit(1), OnDeath: 2 Spores
    cards.push((
        CardBuilder::new(
            "magma_bloom",
            "Magma Bloom",
            CardKind::Unit(UnitType::Megafauna),
        )
        .stats(3, 3)
        .cost(ManaCost {
            bio: 1,
            slag: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DamageEachOpponentUnit(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::Sequence(vec![
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/magma_bloom_1774304132.png")
        .build(),
        1,
    ));

    // Pyrogenic Spore — Instant, DamageEachOpponentUnit(1) + CreateToken(Spore)
    cards.push((
        CardBuilder::new("pyrogenic_spore", "Pyrogenic Spore", CardKind::Instant)
            .cost(ManaCost {
                bio: 1,
                slag: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::DamageEachOpponentUnit(1),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/pyrogenic_spore_1774304164.png")
            .build(),
        1,
    ));

    // ── GLITCH + NEON (digital chaos) ───────────────────────────────────────

    // Cascade Virus — Cascade, Stealth, OnDamageDealt: Discard(1)
    cards.push((
        CardBuilder::new(
            "cascade_virus",
            "Cascade Virus",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 3)
        .cost(ManaCost {
            glitch: 1,
            neon: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Cascade)
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
        .image("https://cdn.tawks.app/cards/cascade_virus_1775260166.png")
        .build(),
        1,
    ));

    // Glitch Storm — Sorcery, Surveil(3) + Discard(1) + DamagePlayer(2)
    cards.push((
        CardBuilder::new("glitch_storm", "Glitch Storm", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                neon: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::Surveil(3),
                    CardEffect::Discard(1),
                    CardEffect::DamagePlayer(2),
                ]),
                None,
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/glitch_storm_1774304116.png")
            .build(),
        1,
    ));

    // ── GLITCH + CHROME (corrupted order) ───────────────────────────────────

    // Malware Sentinel — Malware, Firewall(2), OnDamageDealt: MutateTarget -1/-1
    cards.push((
        CardBuilder::new(
            "malware_sentinel",
            "Malware Sentinel",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(3, 3)
        .cost(ManaCost {
            glitch: 1,
            chrome: 1,
            colorless: 2,
            ..Default::default()
        })
        .keyword(Keyword::Malware)
        .keyword(Keyword::Firewall(2))
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::MutateTarget(StatModifierSpec {
                stat: StatType::Power,
                amount: ModifierAmount::Fixed(-1),
                expires: ExpireContract::Never,
                target: EffectTargetSpec::Targets,
            }),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/malware_sentinel_1774304131.png")
        .build(),
        1,
    ));

    // Dark Protocol — Instant, Swap two targets + Discard(1)
    cards.push((
        CardBuilder::new("dark_protocol", "Dark Protocol", CardKind::Instant)
            .cost(ManaCost {
                glitch: 1,
                chrome: 1,
                colorless: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Swap, CardEffect::Discard(1)]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 2)),
            )
            .rarity(CardRarity::Private)
            .image("https://cdn.tawks.app/cards/dark_protocol_1774304084.png")
            .build(),
        1,
    ));

    // ── GLITCH + SLAG (entropic destruction) ────────────────────────────────

    // Meltdown Virus — Haste, Wither, OnDeath: DamageTarget(3)
    cards.push((
        CardBuilder::new(
            "meltdown_virus",
            "Meltdown Virus",
            CardKind::Unit(UnitType::Virus),
        )
        .stats(2, 2)
        .cost(ManaCost {
            glitch: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Haste)
        .keyword(Keyword::Wither)
        .trigger(CardTrigger {
            event: TriggerEvent::Died,
            effect: CardEffect::DamageTarget(3),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/meltdown_virus_1774304145.png")
        .build(),
        1,
    ));

    // Entropic Detonation — Sorcery, DestroyTarget + Siphon(3) + 2 Gremlin tokens
    cards.push((
        CardBuilder::new(
            "entropic_detonation",
            "Entropic Detonation",
            CardKind::Ritual,
        )
        .cost(ManaCost {
            glitch: 1,
            slag: 1,
            colorless: 3,
            ..Default::default()
        })
        .effect(
            CardEffect::Sequence(vec![
                CardEffect::DestroyTarget,
                CardEffect::Siphon(3),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
            ]),
            Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
        )
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/entropic_detonation_1774304103.png")
        .build(),
        1,
    ));

    // ── NEON + CHROME (digital fortress) ────────────────────────────────────

    // Prismatic Barrier — Flash, Interceptor, ETB: Reflect(3)
    cards.push((
        CardBuilder::new(
            "prismatic_barrier",
            "Prismatic Barrier",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            chrome: 1,
            colorless: 1,
            ..Default::default()
        })
        .keyword(Keyword::Flash)
        .keyword(Keyword::Interceptor)
        .effect(CardEffect::Reflect(3), None)
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/prismatic_barrier_1774304142.png")
        .build(),
        1,
    ));

    // Data Nexus — Artifact, Tap: Transmit(2), GrantKeyword(Aero) to target
    cards.push((
        CardBuilder::new("data_nexus", "Data Nexus", CardKind::Relic)
            .cost(ManaCost {
                neon: 1,
                chrome: 1,
                colorless: 2,
                ..Default::default()
            })
            .ability(CardAbility {
                id: "data_nexus_tap".to_string(),
                name: "Transmit Data".to_string(),
                cost: AbilityCost::new().with_tap(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::Transmit(2),
                    CardEffect::GrantKeyword(Keyword::Aero, ExpireContract::EndOfTurn),
                ]),
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
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/data_nexus_1774304099.png")
            .build(),
        1,
    ));

    // ── NEON + SLAG (volatile speed) ────────────────────────────────────────

    // Flashpoint Runner — Haste, Aero, OnDamageDealt: DamagePlayer(1), end of turn: sacrifice
    cards.push((
        CardBuilder::new(
            "flashpoint_runner",
            "Flashpoint Runner",
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
        .keyword(Keyword::Aero)
        .keyword(Keyword::Menace)
        .trigger(CardTrigger {
            event: TriggerEvent::DealtDamage,
            effect: CardEffect::DamagePlayer(1),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .trigger(CardTrigger {
            event: TriggerEvent::EndOfTurn,
            effect: CardEffect::SacrificeThis,
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/flashpoint_runner_1775260900.png")
        .build(),
        1,
    ));

    // Thermal Overclock — Instant, +3/+0 + Haste + Scavenge(1) + Draw 1
    cards.push((
        CardBuilder::new("thermal_overclock", "Thermal Overclock", CardKind::Instant)
            .cost(ManaCost {
                neon: 1,
                slag: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![
                    CardEffect::ApplyStatModifiers(vec![StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(3),
                        expires: ExpireContract::EndOfTurn,
                        target: EffectTargetSpec::Targets,
                    }]),
                    CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
                    CardEffect::Scavenge(1),
                    CardEffect::DrawCards(1),
                ]),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .rarity(CardRarity::Restricted)
            .image("https://cdn.tawks.app/cards/thermal_overclock_1774304182.png")
            .build(),
        1,
    ));

    // ── CHROME + SLAG (industrial forge) ────────────────────────────────────

    // Scrapforge Colossus — Trample, ETB: Polymorph target unit
    cards.push((
        CardBuilder::new(
            "scrapforge_colossus",
            "Scrapforge Colossus",
            CardKind::Unit(UnitType::Anomaly),
        )
        .stats(4, 4)
        .cost(ManaCost {
            chrome: 1,
            slag: 1,
            colorless: 3,
            ..Default::default()
        })
        .keyword(Keyword::Trample)
        .effect(
            CardEffect::Polymorph,
            Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
        )
        .rarity(CardRarity::Restricted)
        .image("https://cdn.tawks.app/cards/scrapforge_colossus_1774304167.png")
        .build(),
        1,
    ));

    // Industrial Reclaimer — ETB: Recycle + TransmuteMana + Draw 1
    cards.push((
        CardBuilder::new(
            "industrial_reclaimer",
            "Industrial Reclaimer",
            CardKind::Unit(UnitType::Drone),
        )
        .stats(2, 3)
        .cost(ManaCost {
            chrome: 1,
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::Sequence(vec![
                CardEffect::Recycle,
                CardEffect::TransmuteMana(ManaType::Slag, ManaType::Chrome, 1),
                CardEffect::DrawCards(1),
            ]),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .rarity(CardRarity::Private)
        .image("https://cdn.tawks.app/cards/industrial_reclaimer_1774304115.png")
        .build(),
        1,
    ));

    cards
}
