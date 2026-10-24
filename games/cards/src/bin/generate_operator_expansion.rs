use anyhow::Result;
use cardgame::card::*;
use cardgame::repository::CardRepository;
use cardgame::types::*;
use sqlx::PgPool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let db_url = std::env::var("CARDGAME_DATABASE_URL")
        .expect("CARDGAME_DATABASE_URL must be set");

    let pool = PgPool::connect(&db_url).await?;
    let repo = CardRepository::new(Arc::new(pool));

    println!("Cleaning up existing cards and decks...");
    sqlx::query("TRUNCATE deck_cards, decks, cards CASCADE;")
        .execute(&*repo.pool())
        .await?;

    println!("Seeding COMPLEX MTG-style Operator expansion...");

    seed_chrome(&repo).await?;
    seed_slag(&repo).await?;
    seed_glitch(&repo).await?;
    seed_neon(&repo).await?;

    create_operator_deck(
        &repo,
        "deck_chrome_corporate",
        "The Corporate Hegemony",
        "chrome_operator_1",
        "chrome",
    )
    .await?;
    create_operator_deck(
        &repo,
        "deck_slag_scrapyard",
        "The Scrapheap Reclaimers",
        "slag_operator_1",
        "slag",
    )
    .await?;
    create_operator_deck(
        &repo,
        "deck_glitch_entropy",
        "The System Entropy",
        "glitch_operator_1",
        "glitch",
    )
    .await?;
    create_operator_deck(
        &repo,
        "deck_neon_velocity",
        "The Neon Slipstream",
        "neon_operator_1",
        "neon",
    )
    .await?;

    println!("Expansion seeded successfully with High-Depth cards!");
    Ok(())
}

async fn seed_chrome(repo: &CardRepository) -> Result<()> {
    println!("Generating High-Depth Chrome cards...");

    let cmd1 = CardBuilder::new("chrome_operator_1", "Overlord Valerius", CardKind::Unit(UnitType::Human))
        .supertype(CardSupertype::Apex).stats(2, 6)
        .cost(ManaCost { chrome: 2, colorless: 2, ..Default::default() })
        .description("Corporate Tax: Whenever a unit enters the battlefield under an opponent's control, they must pay {1} or you create a 1/1 Drone token.")
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::CreateToken(TokenSpec::new("Security Drone", 1, 1, UnitType::Drone)),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: Some(AbilityCost::new().with_mana(ManaCost { colorless: 1, ..Default::default() })),
            is_optional: false,
        })
        .build();
    repo.create_card(&cmd1).await?;

    for i in 4..=100 {
        let id = format!("chrome_card_{}", i);
        let mut builder = CardBuilder::new(&id, "Chrome Unit", CardKind::Unit(UnitType::Drone));
        let cost_val = (i % 5) + 1;
        builder = builder.cost(ManaCost {
            chrome: (i % 2) as i32 + 1,
            colorless: (cost_val - 1) as i32,
            ..Default::default()
        });

        match i % 6 {
            0 => {
                builder = builder
                    .name("Audit Sentry")
                    .stats(2, 2)
                    .keyword(Keyword::Vigilance)
                    .trigger(CardTrigger {
                        event: TriggerEvent::Entered,
                        effect: CardEffect::TapTarget,
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    });
            }
            1 => {
                builder = builder
                    .name("Stat-Blocker")
                    .keyword(Keyword::Indestructible)
                    .stats(1, 4);
            }
            2 => {
                builder = builder
                    .name("Logic Taxman")
                    .stats(1, 3)
                    .ability(CardAbility {
                        id: format!("{}_ab", id),
                        name: "Freeze System".into(),
                        cost: AbilityCost::new()
                            .with_mana(ManaCost {
                                chrome: 1,
                                ..Default::default()
                            })
                            .with_tap(),
                        effect: CardEffect::Freeze(ExpireContract::EndOfTurn),
                        target_spec: Some(TargetSpec::new(
                            TargetScope::Opponent,
                            TargetKind::Unit,
                            1,
                        )),
                        once_per_turn: true,
                        uses_stack: true,
                        tap_targets: false,
                        predicate: None,
                    });
            }
            3 => {
                builder = builder
                    .name("Buffer Array")
                    .stats(2, 3)
                    .trigger(CardTrigger {
                        event: TriggerEvent::TurnStart,
                        effect: CardEffect::ApplyStatModifier(StatModifierSpec {
                            stat: StatType::Toughness,
                            amount: ModifierAmount::Fixed(1),
                            expires: ExpireContract::EndOfTurn,
                            target: EffectTargetSpec::OwnerUnits(None),
                        }),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    });
            }
            4 => {
                builder = builder
                    .name("Compliance Officer")
                    .stats(2, 2)
                    .keyword(Keyword::Flash)
                    .effect(CardEffect::DrawCards(1), None);
            }
            _ => {
                builder = builder.name("Standard Drone").stats(1, 1).cost(ManaCost {
                    chrome: 1,
                    ..Default::default()
                });
            }
        }
        repo.create_card(&builder.build()).await?;
    }
    Ok(())
}

async fn seed_slag(repo: &CardRepository) -> Result<()> {
    println!("Generating High-Depth Slag cards...");

    let cmd1 = CardBuilder::new(
        "slag_operator_1",
        "Krugg, the Smelter",
        CardKind::Unit(UnitType::Megafauna),
    )
    .supertype(CardSupertype::Apex)
    .stats(5, 5)
    .cost(ManaCost {
        slag: 2,
        colorless: 4,
        ..Default::default()
    })
    .description("Recycle: Whenever a unit you control dies, Krugg deals 2 damage to any target.")
    .trigger(CardTrigger {
        event: TriggerEvent::Died,
        effect: CardEffect::DamageTarget(2),
        limit: TriggerLimit::Unlimited,
        predicate: None,
        optional_cost: None,
        is_optional: false,
    })
    .build();
    repo.create_card(&cmd1).await?;

    for i in 4..=100 {
        let id = format!("slag_card_{}", i);
        let mut builder = CardBuilder::new(&id, "Slag Unit", CardKind::Unit(UnitType::BioOrganism));
        let cost_val = (i % 6) + 1;
        builder = builder.cost(ManaCost {
            slag: (i % 2) as i32 + 1,
            colorless: (cost_val - 1) as i32,
            ..Default::default()
        });

        match i % 6 {
            0 => {
                builder = builder
                    .name("Rust Mauler")
                    .keyword(Keyword::Trample)
                    .stats(5, 2);
            }
            1 => {
                builder = builder
                    .name("Scrap Salvager")
                    .stats(2, 1)
                    .trigger(CardTrigger {
                        event: TriggerEvent::Died,
                        effect: CardEffect::AddMana(ManaType::Slag, 2),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    });
            }
            2 => {
                builder = builder
                    .name("Thermal Burner")
                    .stats(3, 2)
                    .ability(CardAbility {
                        id: format!("{}_ab", id),
                        name: "Smelt".into(),
                        cost: AbilityCost::new().with_life(2),
                        effect: CardEffect::DamageTarget(3),
                        target_spec: Some(TargetSpec::new(
                            TargetScope::Opponent,
                            TargetKind::Unit,
                            1,
                        )),
                        once_per_turn: false,
                        uses_stack: true,
                        tap_targets: false,
                        predicate: None,
                    });
            }
            3 => {
                builder = builder
                    .name("Waste Crawler")
                    .keyword(Keyword::Deathtouch)
                    .stats(1, 1);
            }
            4 => {
                builder = builder
                    .name("Industrial Titan")
                    .stats(7, 7)
                    .cost(ManaCost {
                        slag: 3,
                        colorless: 5,
                        ..Default::default()
                    })
                    .keyword(Keyword::Trample);
            }
            _ => {
                builder = builder.name("Scrap Bot").stats(1, 1).cost(ManaCost {
                    colorless: 1,
                    ..Default::default()
                });
            }
        }
        repo.create_card(&builder.build()).await?;
    }
    Ok(())
}

async fn seed_glitch(repo: &CardRepository) -> Result<()> {
    println!("Generating High-Depth Glitch cards...");

    let cmd1 = CardBuilder::new(
        "glitch_operator_1",
        "The Feedback Loop",
        CardKind::Unit(UnitType::Virus),
    )
    .supertype(CardSupertype::Apex)
    .stats(3, 3)
    .cost(ManaCost {
        glitch: 2,
        colorless: 2,
        ..Default::default()
    })
    .description(
        "Entropy: Whenever an opponent discards a card, they lose 2 life and you draw a card.",
    )
    .trigger(CardTrigger {
        event: TriggerEvent::Discarded,
        effect: CardEffect::Sequence(vec![CardEffect::DamagePlayer(2), CardEffect::DrawCards(1)]),
        limit: TriggerLimit::Unlimited,
        predicate: None,
        optional_cost: None,
        is_optional: false,
    })
    .build();
    repo.create_card(&cmd1).await?;

    for i in 4..=100 {
        let id = format!("glitch_card_{}", i);
        let mut builder = CardBuilder::new(&id, "Glitch Unit", CardKind::Unit(UnitType::Virus));
        let cost_val = (i % 4) + 1;
        builder = builder.cost(ManaCost {
            glitch: (i % 2) as i32 + 1,
            colorless: cost_val as i32,
            ..Default::default()
        });

        match i % 6 {
            0 => {
                builder = builder
                    .name("Logic Phantom")
                    .stats(2, 2)
                    .keyword(Keyword::Stealth)
                    .ability(CardAbility {
                        id: format!("{}_ab", id),
                        name: "Corrupt".into(),
                        cost: AbilityCost::new()
                            .with_mana(ManaCost {
                                glitch: 1,
                                ..Default::default()
                            })
                            .with_tap(),
                        effect: CardEffect::Discard(1),
                        target_spec: Some(TargetSpec::new(
                            TargetScope::Opponent,
                            TargetKind::Player,
                            1,
                        )),
                        once_per_turn: false,
                        uses_stack: true,
                        tap_targets: false,
                        predicate: None,
                    });
            }
            1 => {
                builder = builder
                    .name("Signal Jammer")
                    .stats(1, 3)
                    .keyword(Keyword::SignalJam);
            }
            2 => {
                builder = builder
                    .name("Data Leaker")
                    .stats(2, 1)
                    .trigger(CardTrigger {
                        event: TriggerEvent::DealtDamage,
                        effect: CardEffect::Loot(1, 1),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    });
            }
            3 => {
                builder = builder
                    .name("System Anomaly")
                    .stats(3, 3)
                    .ability(CardAbility {
                        id: format!("{}_ab", id),
                        name: "Scramble".into(),
                        cost: AbilityCost::new()
                            .with_mana(ManaCost {
                                colorless: 3,
                                ..Default::default()
                            })
                            .with_tap(),
                        effect: CardEffect::ShuffleLibrary(TargetScope::Opponent),
                        once_per_turn: false,
                        uses_stack: true,
                        tap_targets: false,
                        predicate: None,
                        target_spec: None,
                    });
            }
            4 => {
                builder = builder
                    .name("Void Pulse")
                    .kind(CardKind::Instant)
                    .effect(CardEffect::BoardWipe, None);
            }
            _ => {
                builder = builder.name("Null Bit").stats(1, 1).cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                });
            }
        }
        repo.create_card(&builder.build()).await?;
    }
    Ok(())
}

async fn seed_neon(repo: &CardRepository) -> Result<()> {
    println!("Generating High-Depth Neon cards...");

    let cmd1 = CardBuilder::new(
        "neon_operator_1",
        "Flash, Mirror-Edge",
        CardKind::Unit(UnitType::Human),
    )
    .supertype(CardSupertype::Apex)
    .stats(4, 2)
    .cost(ManaCost {
        neon: 2,
        colorless: 1,
        ..Default::default()
    })
    .keyword(Keyword::Haste)
    .description("Flicker: {U}: Exile target unit you control, then return it to the battlefield.")
    .ability(CardAbility {
        id: "neon_cmd_1_flicker".into(),
        name: "Phase Shift".into(),
        cost: AbilityCost::new().with_mana(ManaCost {
            neon: 1,
            ..Default::default()
        }),
        effect: CardEffect::Bounce,
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
    .build();
    repo.create_card(&cmd1).await?;

    for i in 4..=100 {
        let id = format!("neon_card_{}", i);
        let mut builder = CardBuilder::new(&id, "Neon Unit", CardKind::Unit(UnitType::Human));
        let cost_val = (i % 4) + 1;
        builder = builder.cost(ManaCost {
            neon: (i % 2) as i32 + 1,
            colorless: (cost_val - 1) as i32,
            ..Default::default()
        });

        match i % 6 {
            0 => {
                builder = builder
                    .name("Grid Runner")
                    .stats(2, 1)
                    .keyword(Keyword::Haste);
            }
            1 => {
                builder = builder
                    .name("Holo-Striker")
                    .stats(3, 2)
                    .keyword(Keyword::Stealth)
                    .trigger(CardTrigger {
                        event: TriggerEvent::Attacked,
                        effect: CardEffect::DrawCards(1),
                        limit: TriggerLimit::Unlimited,
                        predicate: None,
                        optional_cost: None,
                        is_optional: false,
                    });
            }
            2 => {
                builder = builder.name("Pulse Blade").kind(CardKind::Instant).effect(
                    CardEffect::Bounce,
                    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
                );
            }
            3 => {
                builder = builder
                    .name("Cyber Shadow")
                    .stats(3, 1)
                    .keyword(Keyword::Ethereal);
            }
            4 => {
                builder = builder
                    .name("Velocity Injector")
                    .kind(CardKind::Instant)
                    .effect(
                        CardEffect::GrantKeyword(Keyword::Haste, ExpireContract::EndOfTurn),
                        Some(TargetSpec::new(
                            TargetScope::SelfPlayer,
                            TargetKind::Unit,
                            1,
                        )),
                    );
            }
            _ => {
                builder = builder
                    .name("Neon Spark")
                    .stats(1, 1)
                    .cost(ManaCost {
                        colorless: 1,
                        ..Default::default()
                    })
                    .keyword(Keyword::Haste);
            }
        }
        repo.create_card(&builder.build()).await?;
    }
    Ok(())
}

async fn create_operator_deck(
    repo: &CardRepository,
    id: &str,
    name: &str,
    operator_id: &str,
    color: &str,
) -> Result<()> {
    repo.create_deck_with_owner(id, name, "A tactical high-depth deck.", None, None)
        .await?;
    repo.set_deck_format(id, DeckFormat::Operator).await?;
    repo.set_deck_operator(id, Some(operator_id)).await?;
    for i in 4..=100 {
        repo.adjust_deck_card_quantity(id, &format!("{}_card_{}", color, i), 1)
            .await?;
    }
    Ok(())
}
