//! Debug setup handler for testing UI abilities
//!
//! Injects test cards into the active player's hand that exercise each UI interaction:
//! scry, search, discard, sacrifice, loot, trigger choice, bounce, destroy, exile, etc.
//! Also fills mana pool and places units on battlefield for targeting.

use crate::card::*;
use crate::deck::{DeckTemplates, build_deck_with_templates};
use crate::state::{CardGameState, PlayerState};
use crate::types::*;
use anyhow::{Result, anyhow};
use game_framework::GameEffect;

pub trait DebugHandlers {
    fn handle_debug_setup(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
    fn handle_debug_ai_concede(&mut self, player_id: &str) -> Result<Vec<GameEffect>>;
}

impl DebugHandlers for CardGameState {
    fn handle_debug_ai_concede(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if std::env::var("CARDGAME_DEBUG_ACTIONS").as_deref() != Ok("1") {
            return Err(anyhow!("Debug actions are off"));
        }
        if !self.players.get(player_id).is_some_and(|p| !p.is_ai) {
            return Err(anyhow!("Only a seated person can end the game"));
        }
        if !matches!(
            self.stage,
            CardGameStage::Main | CardGameStage::Combat | CardGameStage::PostCombatMain | CardGameStage::End
        ) {
            return Err(anyhow!("No game in progress"));
        }
        for player in self.players.values_mut().filter(|p| p.is_ai) {
            player.life = 0;
        }
        Ok(vec![])
    }

    fn handle_debug_setup(&mut self, player_id: &str) -> Result<Vec<GameEffect>> {
        if !self.players.contains_key(player_id) {
            return Err(anyhow!("Player not found"));
        }

        // If in lobby, bootstrap the entire game first
        if self.stage == CardGameStage::Lobby {
            // Add a dummy bot opponent if only 1 player
            if self.players.len() < 2 {
                let bot_id = "debug-bot".to_string();
                self.players.insert(
                    bot_id.clone(),
                    PlayerState::new(
                        bot_id.clone(),
                        "Debug Bot".into(),
                        String::new(),
                        "debug-bot-instance".into(),
                        self.init.starting_life,
                    ),
                );
                self.turn_order.push(bot_id);
            }

            // Give both players a starter deck
            let templates = DeckTemplates::default();
            for id in self.turn_order.clone() {
                let deck =
                    build_deck_with_templates(&id, "bio", self.init.deck_seed, &templates, None);
                if let Some(player) = self.players.get_mut(&id) {
                    player.deck = deck;
                    player.deck_id = Some("bio".into());
                    player.deck_name = Some("Roots of Infinity".into());
                    player.deck_format = Some(DeckFormat::Classic);
                    // Keep debug sessions one hit from lethal so end-game bugs are easy to reproduce.
                    player.life = 1;
                }
            }

            // Start the game (mirrors handle_start_game logic)
            for id in self.turn_order.clone() {
                self.open_hand(&id);
            }
            self.active_player_id = Some(player_id.to_string());
            self.round = 1;
            if let Some(player) = self.players.get_mut(player_id) {
                player.infrastructures_played_this_turn = 0;
                player.nodes_deployed_this_turn = 0;
                CardGameState::refresh_mana(player);
                // Untap before filling — see comment in `game_flow::start_turn`.
                for card in player.battlefield.iter_mut() {
                    card.summoning_sickness = false;
                    card.tapped = false;
                }
                CardGameState::fill_mana_from_grid(player);
                CardGameState::fill_mana_from_infrastructure(player);
                CardGameState::draw_card(player);
            }
            self.stage = CardGameStage::Main;
        }

        let opponent_id = self
            .players
            .keys()
            .find(|id| *id != player_id)
            .cloned()
            .unwrap_or_default();

        // Place some units on battlefield for both players (targeting fodder)
        if let Some(player) = self.players.get_mut(player_id) {
            player.life = 1;
            for i in 0..3 {
                let mut unit = CardBuilder::new(
                    &format!("debug-unit-{}", i),
                    &format!("Test Drone {}", i + 1),
                    CardKind::Unit(UnitType::Drone),
                )
                .stats(2, 2)
                .build();
                unit.summoning_sickness = false;
                player.battlefield.push(unit);
            }

            // Fill mana pool
            player.mana_pool.neon = 99;
            player.mana_pool.bio = 99;
            player.mana_pool.slag = 99;
            player.mana_pool.glitch = 99;
            player.mana_pool.chrome = 99;
            player.mana_pool.colorless = 99;

            // Add cards to deck (needed for scry/search/draw)
            for i in 0..10 {
                player.deck.push(
                    CardBuilder::new(
                        &format!("debug-deck-{}", i),
                        &format!("Deck Card {}", i + 1),
                        CardKind::Unit(UnitType::BioOrganism),
                    )
                    .stats(1 + (i as i32 % 3), 1 + (i as i32 % 3))
                    .cost(ManaCost {
                        bio: 1 + (i as i32 % 3),
                        ..Default::default()
                    })
                    .build(),
                );
            }
        }

        // Give opponent some units too (for opponent-targeting effects)
        if !opponent_id.is_empty() {
            if let Some(opp) = self.players.get_mut(&opponent_id) {
                opp.life = 1;
                for i in 0..3 {
                    let mut unit = CardBuilder::new(
                        &format!("debug-opp-unit-{}", i),
                        &format!("Enemy Drone {}", i + 1),
                        CardKind::Unit(UnitType::Drone),
                    )
                    .stats(1, 1)
                    .build();
                    unit.summoning_sickness = false;
                    opp.battlefield.push(unit);
                }
            }
        }

        // Build test cards for each UI ability
        let test_cards = build_debug_cards();

        if let Some(player) = self.players.get_mut(player_id) {
            for card in test_cards {
                player.hand.push(card);
            }
        }

        Ok(vec![])
    }
}

fn build_debug_cards() -> Vec<Card> {
    vec![
        // 1. Scry — look at top 3 cards, rearrange top/bottom
        CardBuilder::new("debug-scry", "Test: Scry 3", CardKind::Ritual)
            .description("Scry 3.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(CardEffect::Scry(3), None)
            .build(),
        // 2. Search library — search for a unit
        CardBuilder::new("debug-search", "Test: Search Library", CardKind::Ritual)
            .description("Search library for a unit.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(CardEffect::Search(Predicate::IsUnit), None)
            .build(),
        // 3. Search graveyard
        CardBuilder::new(
            "debug-search-gy",
            "Test: Search Graveyard",
            CardKind::Ritual,
        )
        .description("Search graveyard for a unit.")
        .cost(ManaCost {
            neon: 1,
            ..Default::default()
        })
        .effect(CardEffect::SearchGraveyard(Predicate::IsUnit), None)
        .build(),
        // 4. Discard — opponent discards 2
        CardBuilder::new("debug-discard", "Test: Discard 2", CardKind::Ritual)
            .description("Target opponent discards 2 cards.")
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .effect(CardEffect::Discard(2), None)
            .build(),
        // 5. Loot — draw 3, discard 2
        CardBuilder::new("debug-loot", "Test: Loot (3/2)", CardKind::Ritual)
            .description("Draw 3, discard 2.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(CardEffect::Loot(3, 2), None)
            .build(),
        // 6. Sacrifice unit — ETB trigger requires sacrifice
        CardBuilder::new(
            "debug-sac",
            "Test: Sac Unit",
            CardKind::Unit(UnitType::Drone),
        )
        .description("When this enters, sacrifice a unit.")
        .stats(4, 4)
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .ability(CardAbility {
            id: "debug-sac-ability".into(),
            name: "Devour".into(),
            cost: AbilityCost::new().with_cost(Cost::Sacrifice(1)),
            effect: CardEffect::DrawCards(2),
            target_spec: None,
            once_per_turn: false,
            uses_stack: true,
            tap_targets: false,
            predicate: None,
        })
        .build(),
        // 7. Destroy target unit
        CardBuilder::new("debug-destroy", "Test: Destroy Target", CardKind::Instant)
            .description("Destroy target unit.")
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::DestroyTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 8. Exile target
        CardBuilder::new("debug-exile", "Test: Exile Target", CardKind::Instant)
            .description("Exile target unit.")
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ExileTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 9. Bounce target
        CardBuilder::new("debug-bounce", "Test: Bounce Target", CardKind::Instant)
            .description("Return target unit to its owner's hand.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Bounce,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 10. Board wipe
        CardBuilder::new("debug-wipe", "Test: Board Wipe", CardKind::Ritual)
            .description("Destroy all units.")
            .cost(ManaCost {
                slag: 2,
                ..Default::default()
            })
            .effect(CardEffect::BoardWipe, None)
            .build(),
        // 11. Create token
        CardBuilder::new("debug-token", "Test: Create Token", CardKind::Ritual)
            .description("Create a 3/3 Megafauna token.")
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::CreateToken(TokenSpec::new(
                    "Debug Megafauna",
                    3,
                    3,
                    UnitType::Megafauna,
                )),
                None,
            )
            .build(),
        // 12. Freeze target
        CardBuilder::new("debug-freeze", "Test: Freeze Target", CardKind::Instant)
            .description("Freeze target unit until end of turn.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Freeze(ExpireContract::EndOfTurn),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 13. Damage target (unit or player)
        CardBuilder::new("debug-damage", "Test: Deal 3 Damage", CardKind::Instant)
            .description("Deal 3 damage to any target.")
            .cost(ManaCost {
                slag: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::DamageTarget(3),
                Some(TargetSpec::new(
                    TargetScope::AnyPlayerOrUnit,
                    TargetKind::PlayerOrUnit,
                    1,
                )),
            )
            .build(),
        // 14. Trigger choice — unit with optional ETB trigger
        CardBuilder::new(
            "debug-trigger-choice",
            "Test: Trigger Choice",
            CardKind::Unit(UnitType::Drone),
        )
        .description("When enters, you may pay 1 life: draw 2 cards.")
        .stats(2, 3)
        .cost(ManaCost {
            neon: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::DrawCards(2),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: Some(AbilityCost::new().with_life(1)),
            is_optional: true,
        })
        .build(),
        // 15. Wheel — everyone discards hand, draws 5
        CardBuilder::new("debug-wheel", "Test: Data Cascade", CardKind::Ritual)
            .description("Each player discards their hand, then draws 5.")
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .effect(CardEffect::Wheel(5), None)
            .build(),
        // 16. Sequence — scry 2 then draw 1
        CardBuilder::new("debug-sequence", "Test: Scry 2 + Draw", CardKind::Ritual)
            .description("Scry 2, then draw a card.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::Sequence(vec![CardEffect::Scry(2), CardEffect::DrawCards(1)]),
                None,
            )
            .build(),
        // 17. Tap target
        CardBuilder::new("debug-tap", "Test: Tap Target", CardKind::Instant)
            .description("Tap target unit.")
            .cost(ManaCost {
                neon: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::TapTarget,
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 18. Mill
        CardBuilder::new("debug-mill", "Test: Mill 5", CardKind::Ritual)
            .description("Target player mills 5 cards.")
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .effect(CardEffect::Mill(5), None)
            .build(),
        // 19. Stat modifier — pump a unit
        CardBuilder::new("debug-pump", "Test: +3/+3 Until EOT", CardKind::Instant)
            .description("Target unit gets +3/+3 until end of turn.")
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifiers(vec![
                    StatModifierSpec {
                        stat: StatType::Power,
                        amount: ModifierAmount::Fixed(3),
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
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build(),
        // 20. Pump by mana — all units get +X/+X where X = total mana
        CardBuilder::new("debug-pump-mana", "Test: +X/+X (Mana)", CardKind::Ritual)
            .description("Your units get +X/+X until end of turn, where X is your total mana.")
            .cost(ManaCost {
                bio: 1,
                ..Default::default()
            })
            .effect(CardEffect::PumpByMana, None)
            .build(),
        // 21. Add counters — put +1/+1 counters on target unit
        CardBuilder::new(
            "debug-add-counters",
            "Test: Add 3 Counters",
            CardKind::Instant,
        )
        .description("Put three +1/+1 counters on target unit.")
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::AddCounters(3),
            Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
        )
        .build(),
        // 21. Remove counters — remove +1/+1 counters from target unit
        CardBuilder::new(
            "debug-remove-counters",
            "Test: Remove 2 Counters",
            CardKind::Instant,
        )
        .description("Remove two +1/+1 counters from target unit.")
        .cost(ManaCost {
            slag: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::RemoveCounters(2),
            Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
        )
        .build(),
        // 22. Unit that enters with +1/+1 counters
        CardBuilder::new(
            "debug-counter-unit",
            "Test: Counter Megafauna",
            CardKind::Unit(UnitType::Megafauna),
        )
        .description("Enters with three +1/+1 counters.")
        .stats(1, 1)
        .cost(ManaCost {
            bio: 1,
            ..Default::default()
        })
        .trigger(CardTrigger {
            event: TriggerEvent::Entered,
            effect: CardEffect::AddCounters(3),
            limit: TriggerLimit::Unlimited,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .build(),
        // 23. Agent — loyalty abilities (+1, -3, -7)
        CardBuilder::new("debug-agent", "Test: Debug Walker", CardKind::Agent)
            .description("Debug Walker. [+1]: Draw. [-3]: Destroy. [-7]: Board Wipe + Draw 3.")
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .loyalty(4)
            .ability(CardAbility {
                id: "debug-pw-plus".into(),
                name: "[+1]: Draw".into(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(1),
                    CardEffect::DrawCards(1),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "debug-pw-minus3".into(),
                name: "[-3]: Destroy".into(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-3),
                    CardEffect::DestroyTarget,
                ]),
                target_spec: Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .ability(CardAbility {
                id: "debug-pw-ult".into(),
                name: "[-7]: Ultimate".into(),
                cost: AbilityCost::default(),
                effect: CardEffect::Sequence(vec![
                    CardEffect::ModifyLoyalty(-7),
                    CardEffect::BoardWipe,
                    CardEffect::DrawCards(3),
                ]),
                target_spec: None,
                once_per_turn: true,
                uses_stack: true,
                tap_targets: false,
                predicate: None,
            })
            .build(),
    ]
}
