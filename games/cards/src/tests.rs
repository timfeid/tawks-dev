use crate::action::CardGameAction;
use crate::card::*;
use crate::card_catalog;
use crate::handlers::BuilderHandlers;
use crate::handlers::ChoiceHandlers;
use crate::handlers::DraftHandlers;
use crate::handlers::{SpellHandlers, StackHandlers};
use crate::mana::ManaPool;
use crate::state::*;
use crate::studio::*;
use crate::types::*;
use game_framework::GameLogic;
use std::collections::{BTreeSet, HashMap};

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn join_player(state: &mut CardGameState, id: &str) {
    state
        .update(
            id,
            CardGameAction::Join {
                name: format!("Player {}", id),
                avatar: "https://example.com/avatar.png".into(),
                instance_id: format!("instance-{}", id),
            },
        )
        .unwrap();
}

fn select_deck_with_id(state: &mut CardGameState, id: &str, deck_id: &str) {
    state
        .update(
            id,
            CardGameAction::SelectDeck {
                deck_id: deck_id.to_string(),
            },
        )
        .unwrap();
}

fn select_deck(state: &mut CardGameState, id: &str) {
    select_deck_with_id(state, id, CardGameState::DEFAULT_DECK_ID);
}

fn draft_player_names(state: &CardGameState) -> HashMap<String, String> {
    state
        .turn_order
        .iter()
        .map(|pid| {
            let name = state
                .players
                .get(pid)
                .map(|player| player.name.clone())
                .unwrap_or_else(|| pid.clone());
            (pid.clone(), name)
        })
        .collect()
}

/// Sets up a 2-player game in Main phase with p1 active.
fn setup_game() -> CardGameState {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "p1");
    join_player(&mut state, "p2");
    select_deck(&mut state, "p1");
    select_deck(&mut state, "p2");
    state.update("p1", CardGameAction::StartGame).unwrap();
    assert_eq!(state.stage, CardGameStage::Main);
    state
}

/// Advance from Main -> Combat -> PostCombatMain -> Main (next player's turn).
fn pass_turn(state: &mut CardGameState, player_id: &str) {
    state.update(player_id, CardGameAction::NextStage).unwrap(); // Main -> Combat (DeclareAttackers)
    confirm_all_blockers(state, player_id); // ConfirmAttackers + any eligible blockers
    // Now in PostCombatMain
    state.update(player_id, CardGameAction::NextStage).unwrap(); // PostCombatMain -> next player's Main
}

/// Confirm attackers, then each eligible defender confirms blockers until combat resolves.
/// Handles the case where no attackers → skips blockers entirely.
fn end_combat(state: &mut CardGameState, active_player_id: &str) {
    confirm_all_blockers(state, active_player_id);
}

/// Confirm attackers, declare specified blocks, then confirm all remaining blockers.
fn end_combat_with_blocks(
    state: &mut CardGameState,
    active_player_id: &str,
    blocks: &[(&str, &str, &str)], // (defender_id, attacker_id, blocker_id)
) {
    state
        .update(active_player_id, CardGameAction::ConfirmAttackers)
        .unwrap();
    // Walk through each blocker the engine asks for
    while state.combat_phase == CombatPhase::DeclareBlockers {
        let blocker_id = state.combat_blocker_id.clone().unwrap();
        // Declare any blocks for this defender
        for (bid, attacker_id, card_id) in blocks {
            if *bid == blocker_id.as_str() {
                state
                    .update(
                        &blocker_id,
                        CardGameAction::DeclareBlock {
                            attacker_id: attacker_id.to_string(),
                            blocker_id: card_id.to_string(),
                        },
                    )
                    .unwrap();
            }
        }
        state
            .update(&blocker_id, CardGameAction::ConfirmBlockers)
            .unwrap();
    }
}

/// Helper: ConfirmAttackers then walk through all eligible blockers.
fn confirm_all_blockers(state: &mut CardGameState, active_player_id: &str) {
    state
        .update(active_player_id, CardGameAction::ConfirmAttackers)
        .unwrap();
    while state.combat_phase == CombatPhase::DeclareBlockers {
        let blocker_id = state.combat_blocker_id.clone().unwrap();
        state
            .update(&blocker_id, CardGameAction::ConfirmBlockers)
            .unwrap();
    }
}

fn all_catalog_cards() -> Vec<Card> {
    let mut cards = Vec::new();
    for (card, _) in card_catalog::bio::all_cards() {
        cards.push(card);
    }
    for (card, _) in card_catalog::chrome::all_cards() {
        cards.push(card);
    }
    for (card, _) in card_catalog::glitch::all_cards() {
        cards.push(card);
    }
    for (card, _) in card_catalog::neon::all_cards() {
        cards.push(card);
    }
    for (card, _) in card_catalog::slag::all_cards() {
        cards.push(card);
    }
    for (card, _) in card_catalog::multicolor::all_cards() {
        cards.push(card);
    }
    cards
}

#[test]
fn debug_setup_starts_players_at_one_life() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "p1");

    state.update("p1", CardGameAction::DebugSetup).unwrap();

    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.players.get("p1").unwrap().life, 1);

    let opponent_id = state
        .turn_order
        .iter()
        .find(|id| id.as_str() != "p1")
        .expect("debug setup should create an opponent")
        .clone();
    assert_eq!(state.players.get(&opponent_id).unwrap().life, 1);
}

fn collect_effect_kind_names(effect: &CardEffect, out: &mut BTreeSet<&'static str>) {
    match effect {
        CardEffect::RememberTargetPlayer => {
            out.insert("remember_target_player");
        }
        CardEffect::DamagePlayer(_) => {
            out.insert("damage_player");
        }
        CardEffect::DamageUnit(_) => {
            out.insert("damage_unit");
        }
        CardEffect::DamageTarget(_) => {
            out.insert("damage_target");
        }
        CardEffect::DrawCards(_) => {
            out.insert("draw_cards");
        }
        CardEffect::GainLife(_) => {
            out.insert("gain_life");
        }
        CardEffect::Heat(_) => {
            out.insert("heat");
        }
        CardEffect::HeatOpponents(_) => {
            out.insert("heat_opponents");
        }
        CardEffect::CashOpponents(_) => {
            out.insert("cash_opponents");
        }
        CardEffect::CounterTop => {
            out.insert("counter_top");
        }
        CardEffect::AddMana(_, _) => {
            out.insert("add_mana");
        }
        CardEffect::CreateToken(_) => {
            out.insert("create_token");
        }
        CardEffect::CreateInfrastructure(_) => {
            out.insert("create_infrastructure");
        }
        CardEffect::ApplyStatModifier(_) => {
            out.insert("apply_stat_modifier");
        }
        CardEffect::ApplyStatModifiers(_) => {
            out.insert("apply_stat_modifiers");
        }
        CardEffect::Sequence(effects) => {
            out.insert("sequence");
            for nested in effects {
                collect_effect_kind_names(nested, out);
            }
        }
        CardEffect::RevealTopIf { .. } => {
            out.insert("reveal_top_if");
        }
        CardEffect::Freeze(_) => {
            out.insert("freeze");
        }
        CardEffect::RefundMana(_) => {
            out.insert("refund_mana");
        }
        CardEffect::Loot(_, _) => {
            out.insert("loot");
        }
        CardEffect::Wheel(_) => {
            out.insert("wheel");
        }
        CardEffect::Scry(_) => {
            out.insert("scry");
        }
        CardEffect::Search(_) => {
            out.insert("search");
        }
        CardEffect::SearchToBattlefield(_) => {
            out.insert("search_to_battlefield");
        }
        CardEffect::Bounce => {
            out.insert("bounce");
        }
        CardEffect::BoardWipe => {
            out.insert("board_wipe");
        }
        CardEffect::DestroyTargetPlayerUnits => {
            out.insert("destroy_target_player_units");
        }
        CardEffect::DestroyUnitsWithToughnessAtLeast(_) => {
            out.insert("destroy_units_with_toughness_at_least");
        }
        CardEffect::CopyLastCast => {
            out.insert("copy_last_cast");
        }
        CardEffect::Purify => {
            out.insert("purify");
        }
        CardEffect::GrantKeyword(_, _) => {
            out.insert("grant_keyword");
        }
        CardEffect::Discard(_) => {
            out.insert("discard");
        }
        CardEffect::StealMana(_, _) => {
            out.insert("steal_mana");
        }
        CardEffect::ShuffleLibrary(_) => {
            out.insert("shuffle_library");
        }
        CardEffect::ForbidName => {
            out.insert("forbid_name");
        }
        CardEffect::Fight => {
            out.insert("fight");
        }
        CardEffect::TapTarget => {
            out.insert("tap_target");
        }
        CardEffect::DestroyTarget => {
            out.insert("destroy_target");
        }
        CardEffect::ExileTarget => {
            out.insert("exile_target");
        }
        CardEffect::Equip => {
            out.insert("equip");
        }
        CardEffect::AddManaScaling(_, _, _) => {
            out.insert("add_mana_scaling");
        }
        CardEffect::ModifyLoyalty(_) => {
            out.insert("modify_loyalty");
        }
        CardEffect::Emblem(_, _) => {
            out.insert("emblem");
        }
        CardEffect::SacrificeThis => {
            out.insert("sacrifice_this");
        }
        CardEffect::ReanimateNamedCard(_) => {
            out.insert("reanimate_named_card");
        }
        CardEffect::CopyUnit => {
            out.insert("copy_unit");
        }
        CardEffect::Mill(_) => {
            out.insert("mill");
        }
        CardEffect::BounceAllUnits => {
            out.insert("bounce_all_units");
        }
        CardEffect::BounceAllOpponentPermanents => {
            out.insert("bounce_all_opponent_permanents");
        }
        CardEffect::BounceAllAttacking => {
            out.insert("bounce_all_attacking");
        }
        CardEffect::SearchGraveyard(_) => {
            out.insert("search_graveyard");
        }
        CardEffect::ReanimateTargetUnit => {
            out.insert("reanimate_target_unit");
        }
        CardEffect::AddCounters(_) => {
            out.insert("add_counters");
        }
        CardEffect::RemoveCounters(_) => {
            out.insert("remove_counters");
        }
        CardEffect::PumpByMana => {
            out.insert("pump_by_mana");
        }
        CardEffect::DamagePlayerPerHandCard => {
            out.insert("damage_player_per_hand_card");
        }
        // New effects
        CardEffect::Siphon(_) => {
            out.insert("siphon");
        }
        CardEffect::TransmuteMana(_, _, _) => {
            out.insert("transmute_mana");
        }
        CardEffect::Swap => {
            out.insert("swap");
        }
        CardEffect::Phase => {
            out.insert("phase");
        }
        CardEffect::MutateTarget(_) => {
            out.insert("mutate_target");
        }
        CardEffect::Polymorph => {
            out.insert("polymorph");
        }
        CardEffect::Surveil(_) => {
            out.insert("surveil");
        }
        CardEffect::Transmit(_) => {
            out.insert("transmit");
        }
        CardEffect::Entomb(_) => {
            out.insert("entomb");
        }
        CardEffect::Recycle => {
            out.insert("recycle");
        }
        CardEffect::DamageSplit(_) => {
            out.insert("damage_split");
        }
        CardEffect::DamageEachOpponentUnit(_) => {
            out.insert("damage_each_opponent_unit");
        }
        CardEffect::Reflect(_) => {
            out.insert("reflect");
        }
        CardEffect::Conscript => {
            out.insert("conscript");
        }
        CardEffect::Scavenge(_) => {
            out.insert("scavenge");
        }
        CardEffect::DamagePlayerPerBattlefieldKind(_) => {
            out.insert("damage_player_per_battlefield_kind");
        }
        CardEffect::CreateTokensPerBattlefieldKind(_, _) => {
            out.insert("create_tokens_per_battlefield_kind");
        }
        CardEffect::DiscardSelfRandom(_) => {
            out.insert("discard_self_random");
        }
        CardEffect::UntapTarget => {
            out.insert("untap_target");
        }
        CardEffect::Assimilate(_) => {
            out.insert("assimilate");
        }
    }
}

fn assert_no_empty_sequences(effect: &CardEffect, context: &str) {
    match effect {
        CardEffect::Sequence(effects) => {
            assert!(
                !effects.is_empty(),
                "{context} contains an empty Sequence effect"
            );
            for nested in effects {
                assert_no_empty_sequences(nested, context);
            }
        }
        _ => {}
    }
}

#[test]
fn catalog_cards_do_not_use_empty_sequence_effects() {
    for card in all_catalog_cards() {
        for ability in &card.abilities {
            assert_no_empty_sequences(
                &ability.effect,
                &format!("ability {} on {}", ability.id, card.name),
            );
        }
        for trigger in &card.triggers {
            assert_no_empty_sequences(
                &trigger.effect,
                &format!("trigger {:?} on {}", trigger.event, card.name),
            );
        }
    }
}

#[test]
fn migrate_script_covers_all_catalog_effect_kinds() {
    let mut kinds = BTreeSet::new();
    for card in all_catalog_cards() {
        for ability in &card.abilities {
            collect_effect_kind_names(&ability.effect, &mut kinds);
        }
        for trigger in &card.triggers {
            collect_effect_kind_names(&trigger.effect, &mut kinds);
        }
    }

    let migrate_script = include_str!("bin/cards-migrate.rs");
    for kind in kinds {
        assert!(
            migrate_script.contains(&format!("'{kind}'")),
            "cards-migrate.rs is missing card_effect_kind enum value '{kind}'"
        );
    }
}

#[test]
fn effect_draft_parser_supports_recent_persisted_regressions() {
    let damage_per_hand = crate::build_effect(&crate::EffectDraft {
        kind: Some("damage_player_per_hand_card".to_string()),
        ..Default::default()
    })
    .unwrap();
    assert!(matches!(
        damage_per_hand,
        Some(CardEffect::DamagePlayerPerHandCard)
    ));

    let destroy_by_toughness = crate::build_effect(&crate::EffectDraft {
        kind: Some("destroy_units_with_toughness_at_least".to_string()),
        amount: Some(4),
        ..Default::default()
    })
    .unwrap();
    assert!(matches!(
        destroy_by_toughness,
        Some(CardEffect::DestroyUnitsWithToughnessAtLeast(4))
    ));
}

#[test]
fn draft_building_browser_actions_stay_scoped_to_drafted_pool() {
    let mut state = setup_game();
    state.stage = CardGameStage::DraftBuilding;

    let drafted_card = CardBuilder::new("draft_only_card", "Draft Only Card", CardKind::Ritual)
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .build();
    let catalog_like_card =
        CardBuilder::new("not_drafted", "Not Drafted", CardKind::Ritual).build();

    if let Some(player) = state.players.get_mut("p1") {
        player.library_search_results = vec![catalog_like_card.clone()];
    }

    state.draft = Some(DraftState::new(
        vec![],
        &state.turn_order.clone(),
        draft_player_names(&state),
        5,
    ));
    state
        .draft
        .as_mut()
        .unwrap()
        .drafted
        .insert("p1".to_string(), vec![drafted_card.clone()]);

    state.handle_browser_open("p1", None).unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].id, drafted_card.id);

    state
        .handle_browser_set_filter("p1", Some("glitch".to_string()))
        .unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].id, drafted_card.id);

    state.handle_browser_page("p1", 1).unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].id, drafted_card.id);
}

#[test]
fn draft_building_filters_drafted_pool_locally() {
    let mut state = setup_game();
    state.stage = CardGameStage::DraftBuilding;

    let glitch_card = CardBuilder::new("draft_glitch", "Draft Glitch", CardKind::Ritual)
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .build();
    let chrome_card = CardBuilder::new("draft_chrome", "Draft Chrome", CardKind::Relic)
        .cost(ManaCost {
            chrome: 1,
            ..Default::default()
        })
        .build();

    state.draft = Some(DraftState::new(
        vec![],
        &state.turn_order.clone(),
        draft_player_names(&state),
        5,
    ));
    state.draft.as_mut().unwrap().drafted.insert(
        "p1".to_string(),
        vec![glitch_card.clone(), chrome_card.clone()],
    );

    state.handle_browser_open("p1", None).unwrap();
    state
        .handle_browser_set_filter("p1", Some("glitch".to_string()))
        .unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].id, glitch_card.id);

    state
        .handle_browser_set_filter("p1", Some("relic".to_string()))
        .unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].id, chrome_card.id);
}

#[test]
fn draft_building_defaults_to_color_then_rarity_sort() {
    let mut state = setup_game();
    state.stage = CardGameStage::DraftBuilding;

    let colorless_public =
        CardBuilder::new("colorless_public", "Colorless Public", CardKind::Relic).build();
    let chrome_public = CardBuilder::new("chrome_public", "Chrome Public", CardKind::Ritual)
        .cost(ManaCost {
            chrome: 1,
            ..Default::default()
        })
        .rarity(CardRarity::Public)
        .build();
    let chrome_classified =
        CardBuilder::new("chrome_classified", "Chrome Classified", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .rarity(CardRarity::Classified)
            .build();

    state.draft = Some(DraftState::new(
        vec![],
        &state.turn_order.clone(),
        draft_player_names(&state),
        5,
    ));
    state.draft.as_mut().unwrap().drafted.insert(
        "p1".to_string(),
        vec![
            chrome_classified.clone(),
            chrome_public.clone(),
            colorless_public.clone(),
        ],
    );

    state.handle_browser_open("p1", None).unwrap();
    let library = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    let ids = library.into_iter().map(|card| card.id).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![colorless_public.id, chrome_public.id, chrome_classified.id,]
    );
}

#[test]
fn browser_can_open_another_players_graveyard() {
    let mut state = setup_game();

    let graveyard_card = CardBuilder::new("graveyard_card", "Graveyard Card", CardKind::Ritual)
        .cost(ManaCost {
            glitch: 1,
            ..Default::default()
        })
        .build();
    let other_card = CardBuilder::new("other_card", "Other Card", CardKind::Ritual)
        .cost(ManaCost {
            chrome: 1,
            ..Default::default()
        })
        .build();

    state.players.get_mut("p2").unwrap().graveyard = vec![graveyard_card.clone()];
    state.players.get_mut("p1").unwrap().graveyard = vec![other_card];

    state
        .handle_browser_open("p1", Some("p2".to_string()))
        .unwrap();

    let player = state.players.get("p1").unwrap();
    assert!(player.card_browser_active);
    assert_eq!(player.library_search_results.len(), 1);
    assert_eq!(player.library_search_results[0].id, graveyard_card.id);
    assert_eq!(
        player.card_browser_title.as_deref(),
        Some("Player p2's Graveyard")
    );

    state
        .handle_browser_set_filter("p1", Some("glitch".to_string()))
        .unwrap();
    let filtered = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, graveyard_card.id);

    state.handle_browser_page("p1", 1).unwrap();
    let paged = state
        .players
        .get("p1")
        .unwrap()
        .library_search_results
        .clone();
    assert_eq!(paged.len(), 1);
    assert_eq!(paged[0].id, graveyard_card.id);
}

#[test]
fn browser_view_omits_deck_builder_extras_and_reports_page_bounds() {
    let mut state = setup_game();

    let browser_card_one =
        CardBuilder::new("browser_card_one", "Browser Card One", CardKind::Ritual)
            .cost(ManaCost {
                glitch: 1,
                ..Default::default()
            })
            .build();
    let browser_card_two =
        CardBuilder::new("browser_card_two", "Browser Card Two", CardKind::Ritual)
            .cost(ManaCost {
                chrome: 1,
                ..Default::default()
            })
            .build();
    let deck_card = CardBuilder::new("deck_card", "Deck Card", CardKind::Ritual).build();
    let operator = CardBuilder::new(
        "browser_operator",
        "Browser Operator",
        CardKind::Unit(UnitType::Hacker),
    )
    .supertype(CardSupertype::Apex)
    .stats(4, 4)
    .build();
    let battlefield_unit = CardBuilder::new(
        "browser_targetable_unit",
        "Browser Targetable Unit",
        CardKind::Unit(UnitType::Drone),
    )
    .stats(2, 2)
    .build();

    if let Some(player) = state.players.get_mut("p1") {
        player.card_browser_active = true;
        player.deck_builder_active = true;
        player.collection_page = 1;
        player.collection_total = 120;
        player.library_search_results = vec![browser_card_one.clone(), browser_card_two.clone()];
        player.deck_builder_cards = vec![DeckBuilderCard {
            card: deck_card,
            quantity: 2,
        }];
        player.deck_builder_operator = Some(operator);
    }

    state.players.get_mut("p2").unwrap().battlefield = vec![battlefield_unit];

    let view = <CardGameState as GameLogic>::view_for(&state, "p1");

    assert!(view.cards.is_empty());
    assert!(view.targetable_units.is_empty());
    assert!(view.deck_builder.cards.is_empty());
    assert!(view.deck_builder.operator.is_none());
    assert_eq!(view.deck_builder.library.len(), 2);
    assert!(view.deck_builder.collection_has_prev);
    assert!(view.deck_builder.collection_has_next);
}

#[test]
fn draft_confirm_pool_starts_game_once_everyone_is_ready() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "p1");
    join_player(&mut state, "p2");
    select_deck(&mut state, "p1");
    select_deck(&mut state, "p2");

    state.stage = CardGameStage::DraftBuilding;
    state.draft = Some(DraftState::new(
        vec![],
        &state.turn_order.clone(),
        draft_player_names(&state),
        5,
    ));

    for pid in ["p1", "p2"] {
        let deck_id = state
            .players
            .get(pid)
            .and_then(|player| player.deck_id.clone())
            .expect("selected deck id");
        let player = state.players.get_mut(pid).unwrap();
        player.deck_builder_deck_id = Some(deck_id);
    }

    for pid in ["p1", "p2"] {
        let drafted_pool = state.players.get(pid).unwrap().deck.clone();
        state
            .draft
            .as_mut()
            .unwrap()
            .drafted
            .insert(pid.to_string(), drafted_pool);
    }

    state.handle_draft_confirm_pool("p1").unwrap();
    assert_eq!(state.stage, CardGameStage::DraftBuilding);

    state.handle_draft_confirm_pool("p2").unwrap();
    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.active_player_id.as_deref(), Some("p1"));
    assert!(state.players.values().all(|player| !player.hand.is_empty()));
}

#[test]
fn quest_for_null_hits_targeted_player_on_empty_hand_turn() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("quest_for_null", "Quest for Null", CardKind::Aura)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .ability(CardAbility {
                    id: "quest_for_null_target".into(),
                    name: "Choose target opponent".into(),
                    cost: AbilityCost::new(),
                    effect: CardEffect::Sequence(vec![]),
                    target_spec: Some(TargetSpec::new(
                        TargetScope::Opponent,
                        TargetKind::Player,
                        1,
                    )),
                    once_per_turn: false,
                    uses_stack: false,
                    tap_targets: false,
                    predicate: None,
                })
                .trigger(CardTrigger {
                    event: TriggerEvent::TurnStart,
                    effect: CardEffect::DamagePlayer(5),
                    limit: TriggerLimit::Unlimited,
                    predicate: Some(Predicate::Condition(
                        Condition::TargetPlayerHandSizeExactly(0),
                    )),
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );
        p1.mana_pool.glitch = 1;
    }
    state.players.get_mut("p2").unwrap().hand.clear();

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "quest_for_null".into(),
                targets: vec![TargetRef::Player("p2".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let quest = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|card| card.id == "quest_for_null")
        .unwrap();
    assert_eq!(quest.attached_to.as_deref(), Some("p2"));

    pass_turn(&mut state, "p1");

    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
    assert_eq!(state.players.get("p2").unwrap().life, 15);
    assert_eq!(state.players.get("p2").unwrap().hand.len(), 1);
}

#[test]
fn quest_for_null_does_not_hit_if_target_has_cards_in_hand() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("quest_for_null", "Quest for Null", CardKind::Aura)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .ability(CardAbility {
                    id: "quest_for_null_target".into(),
                    name: "Choose target opponent".into(),
                    cost: AbilityCost::new(),
                    effect: CardEffect::Sequence(vec![]),
                    target_spec: Some(TargetSpec::new(
                        TargetScope::Opponent,
                        TargetKind::Player,
                        1,
                    )),
                    once_per_turn: false,
                    uses_stack: false,
                    tap_targets: false,
                    predicate: None,
                })
                .trigger(CardTrigger {
                    event: TriggerEvent::TurnStart,
                    effect: CardEffect::DamagePlayer(5),
                    limit: TriggerLimit::Unlimited,
                    predicate: Some(Predicate::Condition(
                        Condition::TargetPlayerHandSizeExactly(0),
                    )),
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );
        p1.mana_pool.glitch = 1;
    }

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        p2.hand
            .push(CardBuilder::new("spare_card", "Spare Card", CardKind::Ritual).build());
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "quest_for_null".into(),
                targets: vec![TargetRef::Player("p2".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    pass_turn(&mut state, "p1");

    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
    assert_eq!(state.players.get("p2").unwrap().life, 20);
}

// ──────────────────────────────────────────────────────────────────────────────
// Original tests (fixed for current API: no Draw stage)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn classic_end_step_forces_discard_down_to_seven() {
    let mut state = setup_game();
    state.update("p1", CardGameAction::NextStage).unwrap();
    end_combat(&mut state, "p1");
    assert_eq!(state.stage, CardGameStage::PostCombatMain);

    let hand: Vec<Card> = (0..8)
        .map(|i| {
            CardBuilder::new(
                &format!("classic-hand-{}", i),
                &format!("Classic Hand {}", i),
                CardKind::Ritual,
            )
            .build()
        })
        .collect();
    state.players.get_mut("p1").unwrap().hand = hand;

    state.update("p1", CardGameAction::NextStage).unwrap();

    assert_eq!(state.stage, CardGameStage::End);
    assert_eq!(state.active_player_id.as_deref(), Some("p1"));
    assert_eq!(
        state.players.get("p1").unwrap().pending_discard_amount(),
        Some(1)
    );

    let discard_id = state.players.get("p1").unwrap().hand[0].id.clone();
    state
        .update(
            "p1",
            CardGameAction::Discard {
                card_id: discard_id,
            },
        )
        .unwrap();

    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
    assert_eq!(state.players.get("p1").unwrap().hand.len(), 7);
    assert!(!state.players.get("p1").unwrap().has_pending_discard());
}

#[test]
fn operator_end_step_allows_eight_cards_in_hand() {
    let mut state = setup_game();
    state.format = GameFormat::Operator;
    state.update("p1", CardGameAction::NextStage).unwrap();
    end_combat(&mut state, "p1");
    assert_eq!(state.stage, CardGameStage::PostCombatMain);

    let hand: Vec<Card> = (0..8)
        .map(|i| {
            CardBuilder::new(
                &format!("operator-hand-{}", i),
                &format!("Operator Hand {}", i),
                CardKind::Ritual,
            )
            .build()
        })
        .collect();
    state.players.get_mut("p1").unwrap().hand = hand;

    state.update("p1", CardGameAction::NextStage).unwrap();

    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
    assert!(!state.players.get("p1").unwrap().has_pending_discard());
}

#[test]
fn apex_rule_keeps_newest() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "p1");

    let legend_one = CardBuilder::new("legend-1", "Singular Echo", CardKind::Unit(UnitType::Drone))
        .supertype(CardSupertype::Apex)
        .stats(2, 2)
        .build();
    let legend_two = CardBuilder::new("legend-2", "Singular Echo", CardKind::Unit(UnitType::Drone))
        .supertype(CardSupertype::Apex)
        .stats(3, 3)
        .build();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.push(legend_one.clone());
    }
    state.handle_card_entered("p1", &legend_one, &[]);

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.push(legend_two.clone());
    }
    state.handle_card_entered("p1", &legend_two, &[]);

    let player = state.players.get("p1").unwrap();
    assert_eq!(player.battlefield.len(), 1);
    assert_eq!(player.graveyard.len(), 1);
    assert_eq!(player.battlefield[0].id, "legend-2");
}

#[test]
fn requires_deck_selection_before_start() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "p1");
    join_player(&mut state, "p2");

    assert!(state.update("p1", CardGameAction::StartGame).is_err());

    select_deck(&mut state, "p1");
    assert!(state.update("p1", CardGameAction::StartGame).is_err());

    select_deck(&mut state, "p2");
    state.update("p1", CardGameAction::StartGame).unwrap();
    assert_eq!(state.stage, CardGameStage::Main);
}

#[test]
fn casting_spell_applies_effect_after_resolve() {
    let mut state = setup_game();

    let card = CardBuilder::new("spell-1", "Test Bolt", CardKind::Instant)
        .description("Deal 2 damage.")
        .cost(ManaCost {
            colorless: 1,
            ..Default::default()
        })
        .effect(CardEffect::DamagePlayer(2), None)
        .build();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.hand.clear();
        player.hand.push(card);
        player.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "spell-1".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let opponent = state.players.get("p2").unwrap();
    assert_eq!(opponent.life, 18);
}

#[test]
fn turn_phases_advance_and_swap_active_player() {
    let mut state = setup_game();
    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.active_player_id.as_deref(), Some("p1"));

    // Give p1 an attacker and p2 a blocker so the full flow is exercised
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("atk", "Attacker", CardKind::Unit(UnitType::None))
                .stats(1, 1)
                .keyword(Keyword::Haste)
                .build(),
        );
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.push(
            CardBuilder::new("blk", "Blocker", CardKind::Unit(UnitType::None))
                .stats(1, 1)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    assert_eq!(state.stage, CardGameStage::Combat);
    assert_eq!(state.combat_phase, CombatPhase::DeclareAttackers);

    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk".into(),
                target: None,
            },
        )
        .unwrap();

    state
        .update("p1", CardGameAction::ConfirmAttackers)
        .unwrap();
    assert_eq!(state.combat_phase, CombatPhase::DeclareBlockers);
    assert_eq!(state.combat_blocker_id.as_deref(), Some("p2"));

    state.update("p2", CardGameAction::ConfirmBlockers).unwrap();
    assert_eq!(state.stage, CardGameStage::PostCombatMain);

    state.update("p1", CardGameAction::NextStage).unwrap();
    assert_eq!(state.stage, CardGameStage::Main);
    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
}

#[test]
fn only_units_able_to_attack_can_be_declared_once_each() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("watch", "Watcher", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .keyword(Keyword::Vigilance)
                .build(),
        );
        p1.battlefield.push(
            CardBuilder::new("fresh", "Fresh", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .build(),
        );
        p1.battlefield.last_mut().unwrap().summoning_sickness = true;
        p1.battlefield.push(
            CardBuilder::new("tired", "Tired", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
        p1.battlefield.last_mut().unwrap().tapped = true;
    }
    state.update("p1", CardGameAction::NextStage).unwrap();
    let attack = |id: &str| CardGameAction::DeclareAttack { card_id: id.into(), target: None };

    state.update("p1", attack("watch")).unwrap();
    assert!(state.update("p1", attack("watch")).is_err(), "a vigilant attacker stays untapped but attacks once");
    assert!(state.update("p1", attack("fresh")).is_err(), "summoning sick");
    assert!(state.update("p1", attack("tired")).is_err(), "tapped");
    assert_eq!(state.combat_attackers, vec!["watch".to_string()]);
    let declared = state
        .game_log
        .iter()
        .filter(|e| e.message.contains("Watcher attacks"))
        .count();
    assert_eq!(declared, 1);
}

/// Plays AI against AI the way the server does: each tick's planned actions
/// are applied in order, and a spell sits on the stack until its display
/// window has passed. Every planned action should be legal when it lands.
#[test]
fn ai_plans_only_moves_the_real_game_accepts() {
    for seed_game in 0..10 {
        let mut state = CardGameState::new(CardGameInitArgs::default());
        join_player(&mut state, "p1");
        select_deck(&mut state, "p1");
        state.update("p1", CardGameAction::AddAi).unwrap();
        state.update("p1", CardGameAction::StartGame).unwrap();
        state.players.get_mut("p1").unwrap().is_ai = true;

        let mut rejected = Vec::new();
        let mut ticks = 0;
        while !matches!(state.stage, CardGameStage::Finished | CardGameStage::Awards) && ticks < 4000 {
            ticks += 1;
            if state.last_stack_push_at.is_some() {
                state.last_stack_push_at = Some(chrono::Utc::now() - chrono::Duration::seconds(10));
            }
            for action in state.tick() {
                let label = format!("{action:?}");
                if let Err(err) = state.update("system", action) {
                    rejected.push(format!("{err}: {}", &label[..label.len().min(160)]));
                }
            }
        }
        assert!(
            matches!(state.stage, CardGameStage::Finished | CardGameStage::Awards),
            "game {seed_game} did not finish in {ticks} ticks, stuck in {:?}",
            state.stage
        );
        assert!(rejected.is_empty(), "game {seed_game}: {} rejected, first: {:?}", rejected.len(), rejected.first());
    }
}

#[test]
fn confirm_attackers_with_no_attackers_skips_blockers() {
    let mut state = setup_game();
    state.update("p1", CardGameAction::NextStage).unwrap();
    assert_eq!(state.stage, CardGameStage::Combat);

    // No attackers declared — ConfirmAttackers should skip to PostCombatMain
    state
        .update("p1", CardGameAction::ConfirmAttackers)
        .unwrap();
    assert_eq!(state.stage, CardGameStage::PostCombatMain);
    assert_eq!(state.combat_phase, CombatPhase::Idle);
    assert_eq!(state.combat_blocker_id, None);
}

#[test]
fn vigilance_attacker_does_not_tap_when_declared() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new(
                "vigil",
                "Watchful Sentinel",
                CardKind::Unit(UnitType::Human),
            )
            .stats(3, 3)
            .keyword(Keyword::Vigilance)
            .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    assert_eq!(state.stage, CardGameStage::Combat);
    assert_eq!(state.combat_phase, CombatPhase::DeclareAttackers);

    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "vigil".into(),
                target: None,
            },
        )
        .unwrap();

    let vigilant = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|card| card.id == "vigil")
        .unwrap();
    assert!(
        !vigilant.tapped,
        "vigilance attacker should remain untapped after attacking"
    );
}

#[test]
fn defender_can_cast_targeted_instant_during_blockers_with_infrastructure_mana() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("atk", "Attacker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.mana_pool = ManaPool::default();
        p2.battlefield.push(
            CardBuilder::new("forge", "Glitch Forge", CardKind::Infrastructure)
                .tap_effect(CardEffect::AddMana(ManaType::Glitch, 1))
                .build(),
        );
        p2.hand.push(
            CardBuilder::new("murder", "Terminate Process", CardKind::Instant)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DestroyTarget,
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk".into(),
                target: None,
            },
        )
        .unwrap();
    state
        .update("p1", CardGameAction::ConfirmAttackers)
        .unwrap();

    assert_eq!(state.stage, CardGameStage::Combat);
    assert_eq!(state.combat_phase, CombatPhase::DeclareBlockers);
    assert_eq!(state.combat_blocker_id.as_deref(), Some("p2"));
    assert_eq!(state.effective_priority_holder(), Some("p2"));

    let view = <CardGameState as GameLogic>::view_for(&state, "p2");
    let viewer = view.viewer.unwrap();
    let murder_view = viewer.hand.iter().find(|c| c.id == "murder").unwrap();
    assert!(
        murder_view.can_cast,
        "Targeted instant should be castable during declare blockers"
    );
    assert!(
        murder_view.playable,
        "Targeted instant should be marked playable during declare blockers"
    );

    state
        .update(
            "p2",
            CardGameAction::CastSpell {
                card_id: "murder".into(),
                targets: vec![TargetRef::Card("atk".into())],
            },
        )
        .unwrap();

    assert_eq!(state.stack.len(), 1);
    assert!(
        state
            .players
            .get("p2")
            .unwrap()
            .battlefield
            .iter()
            .find(|card| card.id == "forge")
            .map(|card| card.tapped)
            .unwrap_or(false),
        "Infrastructure should be tapped to pay for the response spell"
    );
}

#[test]
fn full_game_flow_start_to_second_round() {
    let mut state = setup_game();
    assert_eq!(state.round, 1);

    {
        let player = state.players.get_mut("p1").unwrap();
        player.hand.clear();
        player.hand.push(
            CardBuilder::new("land-1", "Test Land", CardKind::Infrastructure)
                .description("A land for testing.")
                .tap_effect(CardEffect::AddMana(ManaType::Colorless, 1))
                .build(),
        );
        player.hand.push(
            CardBuilder::new("bolt-1", "Test Bolt", CardKind::Instant)
                .description("Deal 2 damage.")
                .cost(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DamagePlayer(2), None)
                .build(),
        );
    }

    state
        .update(
            "p1",
            CardGameAction::PlayLand {
                card_id: "land-1".into(),
            },
        )
        .unwrap();
    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "land-1".into(),
            },
        )
        .unwrap();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bolt-1".into(),
                targets: vec![],
            },
        )
        .unwrap();
    assert_eq!(state.stack.len(), 1);

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.stack.len(), 0);
    assert_eq!(state.players.get("p2").unwrap().life, 18);

    pass_turn(&mut state, "p1");
    assert_eq!(state.active_player_id.as_deref(), Some("p2"));
    assert_eq!(state.round, 1);

    pass_turn(&mut state, "p2");
    assert_eq!(state.active_player_id.as_deref(), Some("p1"));
    assert_eq!(state.round, 2);
}

#[test]
fn combat_unblocked_deals_damage_to_defender() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("atk-1", "Attacker", CardKind::Unit(UnitType::None))
                .stats(3, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    assert_eq!(state.stage, CardGameStage::Combat);

    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk-1".into(),
                target: None,
            },
        )
        .unwrap();
    end_combat(&mut state, "p1");
    assert_eq!(state.stage, CardGameStage::PostCombatMain);
    assert_eq!(state.players.get("p2").unwrap().life, 17);
}

#[test]
fn combat_blocked_trades_and_no_player_damage() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("atk-1", "Attacker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }
    {
        let player = state.players.get_mut("p2").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("blk-1", "Blocker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk-1".into(),
                target: None,
            },
        )
        .unwrap();
    end_combat_with_blocks(&mut state, "p1", &[("p2", "atk-1", "blk-1")]);

    assert_eq!(state.stage, CardGameStage::PostCombatMain);
    assert_eq!(state.players.get("p2").unwrap().life, 20);
    assert!(state.players.get("p1").unwrap().battlefield.is_empty());
    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
}

#[test]
fn combat_multiple_blockers_split_damage() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("atk-1", "Attacker", CardKind::Unit(UnitType::None))
                .stats(3, 3)
                .keyword(Keyword::Haste)
                .build(),
        );
    }
    {
        let player = state.players.get_mut("p2").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("blk-1", "Blocker One", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
        player.battlefield.push(
            CardBuilder::new("blk-2", "Blocker Two", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk-1".into(),
                target: None,
            },
        )
        .unwrap();
    end_combat_with_blocks(
        &mut state,
        "p1",
        &[("p2", "atk-1", "blk-1"), ("p2", "atk-1", "blk-2")],
    );

    assert_eq!(state.players.get("p2").unwrap().life, 20);
    assert!(state.players.get("p1").unwrap().battlefield.is_empty());
    let p2_bf = &state.players.get("p2").unwrap().battlefield;
    assert_eq!(p2_bf.len(), 1);
    assert_eq!(p2_bf[0].id, "blk-2");
}

#[test]
fn combat_attacker_survives_smaller_blocker() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("atk-1", "Attacker", CardKind::Unit(UnitType::None))
                .stats(4, 4)
                .keyword(Keyword::Haste)
                .build(),
        );
    }
    {
        let player = state.players.get_mut("p2").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("blk-1", "Blocker", CardKind::Unit(UnitType::None))
                .stats(1, 1)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk-1".into(),
                target: None,
            },
        )
        .unwrap();
    end_combat_with_blocks(&mut state, "p1", &[("p2", "atk-1", "blk-1")]);

    let p1_bf = &state.players.get("p1").unwrap().battlefield;
    assert_eq!(p1_bf.len(), 1);
    assert_eq!(p1_bf[0].id, "atk-1");
    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
}

#[test]
fn combat_tapped_blocker_skips_defender() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("atk-1", "Attacker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }
    {
        let player = state.players.get_mut("p2").unwrap();
        player.battlefield.clear();
        let mut tapped_blocker =
            CardBuilder::new("blk-1", "Blocker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .build();
        tapped_blocker.tapped = true;
        player.battlefield.push(tapped_blocker);
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "atk-1".into(),
                target: None,
            },
        )
        .unwrap();

    // p2 only has a tapped unit — they should be skipped entirely
    state
        .update("p1", CardGameAction::ConfirmAttackers)
        .unwrap();
    assert_eq!(
        state.stage,
        CardGameStage::PostCombatMain,
        "Defender with no untapped units should be skipped, resolving combat immediately"
    );
    // Unblocked attacker dealt 2 damage
    assert_eq!(state.players.get("p2").unwrap().life, 18);
}

#[test]
fn targeted_spell_requires_valid_target() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.hand.clear();
        player.hand.push(
            CardBuilder::new("spell-1", "Snipe", CardKind::Instant)
                .description("Deal 2 damage to a unit.")
                .cost(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DamageTarget(2),
                    Some(TargetSpec::new(TargetScope::Opponent, TargetKind::Unit, 1)),
                )
                .build(),
        );
        player.mana_pool.colorless = 1;
    }
    {
        let player = state.players.get_mut("p2").unwrap();
        player.battlefield.clear();
        player.battlefield.push(
            CardBuilder::new("unit-1", "Blocker", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    let err = state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "spell-1".into(),
                targets: vec![],
            },
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("target"));

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "spell-1".into(),
                targets: vec![TargetRef::Card("unit-1".into())],
            },
        )
        .unwrap();
    assert_eq!(state.stack.len(), 1);

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
}

#[test]
fn bio_warmaster_triggers_on_other_bio() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.hand.clear();
        player.hand.push(
            CardBuilder::new(
                "warmaster-1",
                "Spore Warmaster",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 2)
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Bio Token",
                    1,
                    1,
                    UnitType::BioOrganism,
                )),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
        player.hand.push(
            CardBuilder::new(
                "elf-1",
                "Lichen Scouts",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .build(),
        );
        player.mana_pool.colorless = 3;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "warmaster-1".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "elf-1".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let player = state.players.get("p1").unwrap();
    assert_eq!(player.battlefield.len(), 3);
    assert!(
        player
            .battlefield
            .iter()
            .any(|card| card.name == "Bio Token")
    );
}

#[test]
fn enchantment_attaches_and_grants_stats() {
    let mut state = setup_game();

    {
        let player = state.players.get_mut("p1").unwrap();
        player.hand.clear();
        player.hand.push(
            CardBuilder::new("unit-1", "Test Bio", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );

        let mut aura = CardBuilder::new("aura-1", "Bio Charm", CardKind::Aura)
            .description("Attach to unit. It gets +2/+2 and deathtouch.")
            .cost(ManaCost {
                bio: 2,
                ..Default::default()
            })
            .effect(
                CardEffect::ApplyStatModifier(StatModifierSpec {
                    stat: StatType::Power,
                    amount: ModifierAmount::Fixed(2),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                }),
                Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
            )
            .build();
        aura.attachment = Some(AttachmentSpec {
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
                StatModifierSpec {
                    stat: StatType::Deathtouch,
                    amount: ModifierAmount::Fixed(1),
                    expires: ExpireContract::Never,
                    target: EffectTargetSpec::Targets,
                },
            ],
            keywords: vec![Keyword::Deathtouch],
        });
        player.hand.push(aura);
        player.mana_pool.colorless = 3;
        player.mana_pool.bio = 2;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unit-1".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "aura-1".into(),
                targets: vec![TargetRef::Card("unit-1".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let player = state.players.get("p1").unwrap();
    let unit = player
        .battlefield
        .iter()
        .find(|card| card.id == "unit-1")
        .unwrap();
    let (power, toughness, _, _, deathtouch, _, _) = state.card_combat_stats(unit);
    assert_eq!(power, 3);
    assert_eq!(toughness, 3);
    assert!(deathtouch);

    state.remove_from_battlefield_and_attachments("p1", "unit-1");
    let player = state.players.get("p1").unwrap();
    assert!(player.battlefield.iter().all(|card| card.id != "aura-1"));
}

#[test]
fn throttle_increases_opponent_spell_cost() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardStudio::begin("taxer", "Tax Collector", CardKind::Unit(UnitType::Human))
                .stats(2, 2)
                .keyword(Keyword::Throttle(1))
                .build(),
        );
    }

    pass_turn(&mut state, "p1");

    // Now it's p2's turn
    let bolt = CardBuilder::new("bolt", "Bolt", CardKind::Instant)
        .cost(ManaCost {
            colorless: 1,
            ..Default::default()
        })
        .build();
    let bolt_id = bolt.id.clone();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.push(bolt);
        p2.mana_pool.colorless = 1;
    }

    // Should fail because cost is now 2
    assert!(
        state
            .update(
                "p2",
                CardGameAction::CastSpell {
                    card_id: bolt_id.clone(),
                    targets: vec![],
                }
            )
            .is_err()
    );

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.mana_pool.colorless = 2;
    }
    state
        .update(
            "p2",
            CardGameAction::CastSpell {
                card_id: bolt_id,
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p2", CardGameAction::ResolveStackTop).unwrap();
}

#[test]
fn signal_jam_prevents_activated_abilities() {
    let mut state = setup_game();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.push(
            CardStudio::begin("jammer", "Jammer", CardKind::Unit(UnitType::Drone))
                .keyword(Keyword::SignalJam)
                .build(),
        );
    }

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("bot", "Bot", CardKind::Unit(UnitType::Drone))
                .ability(CardAbility {
                    id: "act".into(),
                    name: "Act".into(),
                    cost: AbilityCost::default(),
                    effect: CardEffect::DamagePlayer(1),
                    target_spec: None,
                    once_per_turn: false,
                    uses_stack: true,
                    tap_targets: false,
                    predicate: None,
                })
                .build(),
        );
    }

    assert!(
        state
            .update(
                "p1",
                CardGameAction::ActivateAbility {
                    card_id: "p1-bot-0".into(),
                    ability_id: "act".into(),
                    targets: vec![],
                }
            )
            .is_err()
    );
}

#[test]
fn freeze_prevents_attacking_and_tapping() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("frozen", "Frozen Bot", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    let card_id = "p1-frozen-0".to_string();
    state.add_stat_modifier(
        "p1",
        EffectTarget::Card(card_id.clone()),
        &StatModifierSpec {
            stat: StatType::Armor,
            amount: ModifierAmount::Fixed(-999),
            expires: ExpireContract::EndOfTurn,
            target: EffectTargetSpec::Targets,
        },
    );

    state.update("p1", CardGameAction::NextStage).unwrap(); // To Combat
    assert!(
        state
            .update(
                "p1",
                CardGameAction::DeclareAttack {
                    card_id: card_id.clone(),
                    target: None
                }
            )
            .is_err()
    );
}

#[test]
fn siphon_generates_mana_on_combat_damage() {
    let mut state = setup_game();

    let card_id = {
        let p1 = state.players.get_mut("p1").unwrap();
        let card = CardStudio::begin("leech", "Energy Leech", CardKind::Unit(UnitType::Drone))
            .stats(1, 1)
            .keyword(Keyword::Haste)
            .with_ability(AbilityModule::new(
                Triggers::on_damage_dealt(),
                Actions::add_mana(ManaType::Neon, 1),
                Constraints::none(),
            ))
            .build();
        let id = card.id.clone();
        p1.battlefield.push(card);
        p1.mana_pool.neon = 0;
        id
    };

    state.update("p1", CardGameAction::NextStage).unwrap(); // To Combat
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id,
                target: None,
            },
        )
        .unwrap();
    end_combat(&mut state, "p1"); // Resolve Damage

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.neon, 1);
}

#[test]
fn recursion_refunds_mana_on_death() {
    let mut state = setup_game();

    let card_id = {
        let p1 = state.players.get_mut("p1").unwrap();
        let card = CardBuilder::new("scrap", "Scrap Bot", CardKind::Unit(UnitType::Drone))
            .cost(ManaCost {
                colorless: 4,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Died,
                effect: CardEffect::RefundMana(5000), // 50%
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build();
        let id = card.id.clone();
        p1.battlefield.push(card);
        p1.mana_pool.colorless = 0;
        id
    };

    state.remove_from_battlefield_and_attachments("p1", &card_id);

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.colorless, 2);
}

#[test]
fn slag_feeder_can_target_a_unit_for_its_sacrifice_buff() {
    let mut state = setup_game();

    let feeder_id = {
        let feeder = card_catalog::slag::all_cards()
            .into_iter()
            .find_map(|(card, _)| (card.id == "slag_feeder").then_some(card))
            .expect("slag feeder exists");
        let id = feeder.id.clone();
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(feeder);
        id
    };

    let fodder_id = {
        let fodder = CardBuilder::new("fodder", "Fodder", CardKind::Unit(UnitType::Anomaly))
            .stats(1, 1)
            .build();
        let id = fodder.id.clone();
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(fodder);
        // Sacrifice-for-value abilities now require 1 colorless on top
        // of the sacrifice itself.
        p1.mana_pool.colorless = 1;
        id
    };

    state
        .update(
            "p1",
            CardGameAction::ActivateAbility {
                card_id: feeder_id.clone(),
                ability_id: "sacrifice_for_buff".into(),
                targets: vec![TargetRef::Card(feeder_id.clone())],
            },
        )
        .unwrap();

    assert!(state.players.get("p1").unwrap().has_pending_sacrifice());

    state
        .update(
            "p1",
            CardGameAction::Sacrifice {
                card_id: fodder_id.clone(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert!(!p1.has_pending_sacrifice());
    assert!(p1.graveyard.iter().any(|card| card.id == fodder_id));

    let view = <CardGameState as GameLogic>::view_for(&state, "p1");
    let feeder_view = view
        .players
        .iter()
        .find(|player| player.id == "p1")
        .and_then(|player| {
            player
                .battlefield_units
                .iter()
                .find(|card| card.id == feeder_id)
        })
        .expect("slag feeder visible in battlefield view");
    assert_eq!(feeder_view.power, Some(2));
    assert_eq!(feeder_view.toughness, Some(2));
}

// ──────────────────────────────────────────────────────────────────────────────
// ETB (Enter the Battlefield) Trigger Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn etb_draw_cards_spore_frog() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new(
                "spore_frog",
                "Spore Frog",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .cost(ManaCost {
                bio: 1,
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
            .build(),
        );
        p1.mana_pool.bio = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "spore_frog".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Started with 1 card, played it (-1), drew 1 from ETB (+1) = 1
    assert_eq!(p1.hand.len(), 1);
    assert!(p1.battlefield.iter().any(|c| c.id == "spore_frog"));
}

#[test]
fn etb_gain_life_essence_warden() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.push(
            CardBuilder::new(
                "essence_warden",
                "Essence Warden",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .cost(ManaCost {
                bio: 1,
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
            .build(),
        );
        p1.mana_pool.bio = 1;
    }

    let life_before = state.players.get("p1").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "essence_warden".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.players.get("p1").unwrap().life, life_before + 2);
}

#[test]
fn etb_create_token_nest_invader() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.push(
            CardBuilder::new(
                "nest_invader",
                "Nest Invader",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 2)
            .cost(ManaCost {
                bio: 1,
                colorless: 1,
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
            .build(),
        );
        p1.mana_pool.bio = 1;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "nest_invader".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 2);
    assert!(p1.battlefield.iter().any(|c| c.name == "Spore"));
}

#[test]
fn etb_create_multiple_tokens_spore_caller() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.push(
            CardBuilder::new(
                "spore_caller",
                "Spore Caller",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(3, 3)
            .cost(ManaCost {
                bio: 2,
                colorless: 3,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
        p1.mana_pool.bio = 2;
        p1.mana_pool.colorless = 3;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "spore_caller".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 4); // Spore Caller + 3 Spores
    assert_eq!(
        p1.battlefield.iter().filter(|c| c.name == "Spore").count(),
        3
    );
}

#[test]
fn etb_discard_burglar_rat() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.push(
            CardBuilder::new(
                "burglar_rat",
                "Burglar Rat",
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
            .build(),
        );
        p1.mana_pool.glitch = 1;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "burglar_rat".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    // Opponent should have pending discard
    let p2 = state.players.get("p2").unwrap();
    assert!(p2.has_pending_discard());
    assert_eq!(p2.pending_discard_amount(), Some(1));
}

#[test]
fn etb_draw_two_cards_eternal_witness() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new(
                "eternal_witness",
                "Eternal Witness",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 1)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
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
            .build(),
        );
        p1.mana_pool.bio = 1;
        p1.mana_pool.colorless = 2;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "eternal_witness".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Started with 1 card, played it (-1), ETB drew 2 (+2) = 2
    assert_eq!(p1.hand.len(), 2);
}

#[test]
fn etb_bounce_harbinger_of_tides() {
    let mut state = setup_game();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("target_unit", "Enemy", CardKind::Unit(UnitType::None))
                .stats(3, 3)
                .build(),
        );
    }
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.push(
            CardBuilder::new(
                "harbinger",
                "Harbinger of the Tides",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(2, 2)
            .cost(ManaCost {
                neon: 1,
                colorless: 1,
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
            .build(),
        );
        p1.mana_pool.neon = 1;
        p1.mana_pool.colorless = 1;
    }

    // The bounce trigger needs a target -- but since the trigger system auto-fires,
    // let's verify the unit enters the battlefield
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "harbinger".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "harbinger")
    );
}

#[test]
fn etb_counter_top_mystic_snake() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new(
                "mystic_snake",
                "Mystic Snake",
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
                effect: CardEffect::CounterTop,
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
        p1.hand.push(
            CardBuilder::new("dummy_spell", "Dummy", CardKind::Instant)
                .effect(CardEffect::DamagePlayer(1), None)
                .build(),
        );
        p1.mana_pool.neon = 2;
        p1.mana_pool.colorless = 3;
    }

    // Cast the dummy spell first to put something on the stack
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "dummy_spell".into(),
                targets: vec![],
            },
        )
        .unwrap();
    assert_eq!(state.stack.len(), 1);

    // Cast Mystic Snake; its ETB should counter the top of stack
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "mystic_snake".into(),
                targets: vec![],
            },
        )
        .unwrap();
    // First resolve pops the dummy_spell (top of stack). Then resolve the
    // showcase so Mystic Snake enters the battlefield.
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.stack.len(), 0);
    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "mystic_snake")
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// OnDeath Trigger Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn on_death_damage_opponent_zulaport_cutthroat() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "zulaport",
                "Exhaust Cutthroat",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(1, 1)
            .trigger(CardTrigger {
                event: TriggerEvent::Died,
                effect: CardEffect::DamagePlayer(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
    }

    let p2_life_before = state.players.get("p2").unwrap().life;
    state.remove_from_battlefield_and_attachments("p1", "zulaport");
    assert_eq!(state.players.get("p2").unwrap().life, p2_life_before - 1);
}

#[test]
fn on_death_damage_and_gain_life_blood_artist() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "blood_artist",
                "Blood Artist",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(0, 1)
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
            .build(),
        );
    }

    let p1_life_before = state.players.get("p1").unwrap().life;
    let p2_life_before = state.players.get("p2").unwrap().life;

    state.remove_from_battlefield_and_attachments("p1", "blood_artist");

    assert_eq!(state.players.get("p2").unwrap().life, p2_life_before - 1);
    assert_eq!(state.players.get("p1").unwrap().life, p1_life_before + 1);
}

#[test]
fn on_death_create_token_reassembling_skeleton() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "skeleton",
                "Reassembling Skeleton",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(1, 1)
            .trigger(CardTrigger {
                event: TriggerEvent::Died,
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Skeleton",
                    1,
                    1,
                    UnitType::Anomaly,
                )),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
    }

    state.remove_from_battlefield_and_attachments("p1", "skeleton");

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 1);
    assert_eq!(p1.battlefield[0].name, "Skeleton");
    assert!(p1.graveyard.iter().any(|c| c.id == "skeleton"));
}

#[test]
fn on_death_draw_card_gravecrawler() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new(
                "gravecrawler",
                "Gravecrawler",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(2, 1)
            .trigger(CardTrigger {
                event: TriggerEvent::Died,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
    }

    let hand_before = state.players.get("p1").unwrap().hand.len();
    state.remove_from_battlefield_and_attachments("p1", "gravecrawler");
    assert_eq!(state.players.get("p1").unwrap().hand.len(), hand_before + 1);
}

#[test]
fn on_death_create_three_tokens_nexus_crawler() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("wurm", "Nexus Crawler", CardKind::Unit(UnitType::Megafauna))
                .stats(15, 15)
                .keyword(Keyword::Trample)
                .trigger(CardTrigger {
                    event: TriggerEvent::Died,
                    effect: CardEffect::Sequence(vec![
                        CardEffect::CreateToken(TokenSpec::new(
                            "Crawler",
                            5,
                            5,
                            UnitType::Megafauna,
                        )),
                        CardEffect::CreateToken(TokenSpec::new(
                            "Crawler",
                            5,
                            5,
                            UnitType::Megafauna,
                        )),
                        CardEffect::CreateToken(TokenSpec::new(
                            "Crawler",
                            5,
                            5,
                            UnitType::Megafauna,
                        )),
                    ]),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );
    }

    state.remove_from_battlefield_and_attachments("p1", "wurm");

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 3);
    assert!(
        p1.battlefield
            .iter()
            .all(|c| c.name == "Crawler" && c.power == Some(5))
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// OnOtherUnitPlayed Trigger Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn on_other_unit_played_tribal_grove_huntmaster_analog() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // Place Spore Matron on the battlefield first
        // Once the cast resolves, only the unit that was actually played should count.
        let lys = CardBuilder::new(
            "lys_alana",
            "Spore Matron",
            CardKind::Unit(UnitType::BioOrganism),
        )
        .stats(3, 3)
        .trigger(CardTrigger {
            event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
            effect: CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            limit: TriggerLimit::OncePerTurn,
            predicate: None,
            optional_cost: None,
            is_optional: false,
        })
        .build();
        p1.battlefield.push(lys);

        // Add a BioOrganism to hand to cast
        p1.hand.push(
            CardBuilder::new("bio_elf", "Bio Elf", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bio_elf".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Spore Matron + Bio Elf + 1 Spore token = 3
    assert_eq!(p1.battlefield.len(), 3);
    assert!(p1.battlefield.iter().any(|c| c.name == "Spore"));
}

#[test]
fn on_other_unit_played_tribal_ignores_wrong_type() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "lys_alana",
                "Spore Matron",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(3, 3)
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
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
            .build(),
        );

        // Cast a Drone, not a BioOrganism
        p1.hand.push(
            CardBuilder::new("drone_1", "Random Drone", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "drone_1".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Spore Matron + Drone only, no token because wrong type
    assert_eq!(p1.battlefield.len(), 2);
    assert!(!p1.battlefield.iter().any(|c| c.name == "Spore"));
}

#[test]
fn on_other_unit_played_generic_fauna_whisperer() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // Fauna Whisperer triggers on ANY unit
        p1.battlefield.push(
            CardBuilder::new(
                "whisperer",
                "Fauna Whisperer",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 3)
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(None),
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );

        p1.hand.push(
            CardBuilder::new("random_unit", "Random Guy", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    let hand_before = state.players.get("p1").unwrap().hand.len();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "random_unit".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Cast unit from hand (-1), drew a card (+1) = same
    assert_eq!(p1.hand.len(), hand_before);
}

#[test]
fn on_other_unit_played_damage_forge_overlord() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "purphoros",
                "Forge Overlord",
                CardKind::Unit(UnitType::Anomaly),
            )
            .stats(6, 5)
            .keyword(Keyword::Indestructible)
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(None),
                effect: CardEffect::DamagePlayer(2),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );

        p1.hand.push(
            CardBuilder::new("goblin_1", "Gremlin", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    let p2_life = state.players.get("p2").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "goblin_1".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.players.get("p2").unwrap().life, p2_life - 2);
}

#[test]
fn on_other_unit_played_shockwave_grid() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // Shockwave Grid is an Enchantment that triggers on unit entry
        p1.battlefield.push(
            CardBuilder::new("tremors", "Shockwave Grid", CardKind::Aura)
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyEntered(None),
                    effect: CardEffect::DamagePlayer(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );

        p1.hand.push(
            CardBuilder::new("unit_a", "Unit A", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
        p1.hand.push(
            CardBuilder::new("unit_b", "Unit B", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 2;
    }

    let p2_life = state.players.get("p2").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unit_a".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unit_b".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    // Each unit entering should trigger 1 damage = 2 total
    assert_eq!(state.players.get("p2").unwrap().life, p2_life - 2);
}

#[test]
fn trigger_damage_target_requires_target_selection() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // Enchantment with DamageTarget trigger - requires target selection
        p1.battlefield.push(
            CardBuilder::new("tremors", "Shockwave Grid", CardKind::Aura)
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyEntered(None),
                    effect: CardEffect::DamageTarget(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );

        p1.hand.push(
            CardBuilder::new("unit_a", "Unit A", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    // Cast unit - should create pending target selection
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unit_a".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    // Should have pending trigger target
    let p1 = state.players.get("p1").unwrap();
    assert!(p1.has_pending_trigger_target());

    // Select opponent as target
    let p2_life = state.players.get("p2").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::TriggerTargetSelect {
                target: TargetRef::Player("p2".into()),
            },
        )
        .unwrap();
    assert_eq!(state.players.get("p2").unwrap().life, p2_life - 1);

    // Pending should be resolved
    let p1 = state.players.get("p1").unwrap();
    assert!(!p1.has_pending_trigger_target());
}

#[test]
fn trigger_damage_target_can_hit_unit() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("tremors", "Shockwave Grid", CardKind::Aura)
                .trigger(CardTrigger {
                    event: TriggerEvent::AllyEntered(None),
                    effect: CardEffect::DamageTarget(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );

        p1.hand.push(
            CardBuilder::new("unit_a", "Unit A", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 1;

        // Put a 1/1 unit on p2's battlefield to target
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.push(
            CardBuilder::new("enemy", "Enemy", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .build(),
        );
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unit_a".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    // Select the enemy unit as target (1 damage kills 1 toughness)
    let p2_bf_before = state.players.get("p2").unwrap().battlefield.len();
    state
        .update(
            "p1",
            CardGameAction::TriggerTargetSelect {
                target: TargetRef::Card("enemy".into()),
            },
        )
        .unwrap();

    // Unit should be dead
    let p2 = state.players.get("p2").unwrap();
    assert_eq!(p2.battlefield.len(), p2_bf_before - 1);
    assert!(!p2.battlefield.iter().any(|c| c.id == "enemy"));
}

#[test]
fn vex_signal_shaper_creates_pending_card_target_on_enter() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.battlefield.clear();
        p1.hand.push(
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
            .build(),
        );
        p1.mana_pool.neon = 2;
        p1.mana_pool.colorless = 2;
    }

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield
            .push(CardBuilder::new("target_perm", "Target Permanent", CardKind::Aura).build());
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "vex_signal_shaper".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    assert!(p1.has_pending_trigger_target());
    match p1.pending_trigger_target() {
        Some(PendingAction::TriggerTargetSelection { target_kind, .. }) => {
            assert_eq!(*target_kind, TargetKind::Card);
        }
        _ => panic!("expected pending trigger target selection"),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// OnUpkeep Trigger Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn on_upkeep_create_token_thornback_riders() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "wolverine",
                "Thornback Riders",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(4, 4)
            .trigger(CardTrigger {
                event: TriggerEvent::Upkeep,
                effect: CardEffect::Sequence(vec![
                    CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
                    CardEffect::GainLife(1),
                ]),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
    }

    let p1_life = state.players.get("p1").unwrap().life;

    // Pass p1's turn, then p2's turn -> back to p1 triggers OnUpkeep
    pass_turn(&mut state, "p1");
    pass_turn(&mut state, "p2");

    let p1 = state.players.get("p1").unwrap();
    // Should have the thornback riders + at least 1 Spore token from upkeep
    assert!(p1.battlefield.iter().any(|c| c.name == "Spore"));
    assert!(p1.life > p1_life);
}

#[test]
fn on_upkeep_draw_card_junkyard_rat() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("squee", "Junkyard Rat", CardKind::Unit(UnitType::Anomaly))
                .stats(1, 1)
                .trigger(CardTrigger {
                    event: TriggerEvent::Upkeep,
                    effect: CardEffect::DrawCards(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
        );
    }

    let hand_before = state.players.get("p1").unwrap().hand.len();

    // Pass both turns to get back to p1's upkeep
    pass_turn(&mut state, "p1");
    pass_turn(&mut state, "p2");

    // Should have drawn from upkeep trigger + normal draw
    assert!(state.players.get("p1").unwrap().hand.len() > hand_before);
}

// ──────────────────────────────────────────────────────────────────────────────
// OncePerTurn Trigger Limit Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn once_per_turn_trigger_only_fires_once() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "limited_lord",
                "Limited Lord",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 2)
            .trigger(CardTrigger {
                event: TriggerEvent::AllyEntered(Some(UnitType::BioOrganism)),
                effect: CardEffect::CreateToken(TokenSpec::new(
                    "Token",
                    1,
                    1,
                    UnitType::BioOrganism,
                )),
                limit: TriggerLimit::OncePerTurn,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );

        p1.hand.push(
            CardBuilder::new("elf_a", "Elf A", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        p1.hand.push(
            CardBuilder::new("elf_b", "Elf B", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        p1.mana_pool.colorless = 2;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "elf_a".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "elf_b".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    let token_count = p1.battlefield.iter().filter(|c| c.name == "Token").count();
    // Only 1 token despite 2 bio units entering
    assert_eq!(token_count, 1);
}

// ──────────────────────────────────────────────────────────────────────────────
// Keyword Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn keyword_static_buff_stored_on_card() {
    // StaticPowerBuff/StaticToughnessBuff keywords are stored on the card
    // and applied during view construction (not via card_combat_stats)
    let lord = CardBuilder::new(
        "lord",
        "Elvish Champion",
        CardKind::Unit(UnitType::BioOrganism),
    )
    .stats(2, 2)
    .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
    .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::BioOrganism)))
    .build();

    assert!(
        lord.keywords
            .contains(&Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
    );
    assert!(lord.keywords.contains(&Keyword::StaticToughnessBuff(
        1,
        Some(UnitType::BioOrganism)
    )));
}

#[test]
fn keyword_multiple_anthem_keywords_stored() {
    let lord = CardBuilder::new("lord", "Double Lord", CardKind::Unit(UnitType::BioOrganism))
        .stats(2, 2)
        .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
        .keyword(Keyword::StaticPowerBuff(2, None))
        .build();

    let power_buffs: Vec<_> = lord
        .keywords
        .iter()
        .filter(|k| matches!(k, Keyword::StaticPowerBuff(_, _)))
        .collect();
    assert_eq!(power_buffs.len(), 2);
}

#[test]
fn keyword_stealth_bypasses_blockers() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("stealth_unit", "Ghost", CardKind::Unit(UnitType::Drone))
                .stats(2, 1)
                .keyword(Keyword::Haste)
                .keyword(Keyword::Stealth)
                .build(),
        );
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("big_blocker", "Wall", CardKind::Unit(UnitType::None))
                .stats(0, 10)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap(); // Combat
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "stealth_unit".into(),
                target: None,
            },
        )
        .unwrap();

    state
        .update("p1", CardGameAction::ConfirmAttackers)
        .unwrap();

    // Blocker should fail to block a stealth unit
    let block_result = state.update(
        "p2",
        CardGameAction::DeclareBlock {
            attacker_id: "stealth_unit".into(),
            blocker_id: "big_blocker".into(),
        },
    );
    assert!(block_result.is_err());

    // Resolve combat - damage goes through
    state.update("p2", CardGameAction::ConfirmBlockers).unwrap();
    assert_eq!(state.players.get("p2").unwrap().life, 18);
}

#[test]
fn keyword_indestructible_survives_combat() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "indestructible_one",
                "Iron Golem",
                CardKind::Unit(UnitType::None),
            )
            .stats(2, 2)
            .keyword(Keyword::Haste)
            .keyword(Keyword::Indestructible)
            .build(),
        );
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("big_attacker", "Giant", CardKind::Unit(UnitType::None))
                .stats(10, 10)
                .keyword(Keyword::Haste)
                .build(),
        );
    }

    state.update("p1", CardGameAction::NextStage).unwrap();
    state
        .update(
            "p1",
            CardGameAction::DeclareAttack {
                card_id: "indestructible_one".into(),
                target: None,
            },
        )
        .unwrap();
    end_combat_with_blocks(
        &mut state,
        "p1",
        &[("p2", "indestructible_one", "big_attacker")],
    );

    // Indestructible unit should survive
    let p1 = state.players.get("p1").unwrap();
    assert!(p1.battlefield.iter().any(|c| c.id == "indestructible_one"));
}

#[test]
fn keyword_haste_sets_haste_flag() {
    // CardBuilder::keyword(Keyword::Haste) should set card.haste = true
    let card_with_haste = CardBuilder::new("fast", "Fast", CardKind::Unit(UnitType::None))
        .stats(2, 2)
        .keyword(Keyword::Haste)
        .build();
    assert!(card_with_haste.haste);

    let card_without = CardBuilder::new("slow", "Slow", CardKind::Unit(UnitType::None))
        .stats(2, 2)
        .build();
    assert!(!card_without.haste);
}

// ──────────────────────────────────────────────────────────────────────────────
// Spell Effect Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn spell_draw_cards_brainstorm() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("brainstorm", "Brainstorm", CardKind::Instant)
                .cost(ManaCost {
                    neon: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DrawCards(3), None)
                .build(),
        );
        p1.mana_pool.neon = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "brainstorm".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Started with 1 card, played it (-1), drew 3 (+3) = 3
    assert_eq!(state.players.get("p1").unwrap().hand.len(), 3);
}

#[test]
fn spell_damage_opponent_direct() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("bolt", "Lightning Bolt", CardKind::Instant)
                .cost(ManaCost {
                    slag: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DamagePlayer(3), None)
                .build(),
        );
        p1.mana_pool.slag = 1;
    }

    let p2_life = state.players.get("p2").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bolt".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.players.get("p2").unwrap().life, p2_life - 3);
}

#[test]
fn spell_damage_target_unit() {
    let mut state = setup_game();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("target_c", "Target", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .build(),
        );
    }
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("shock", "Shock", CardKind::Instant)
                .cost(ManaCost {
                    slag: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DamageTarget(2),
                    Some(TargetSpec::new(
                        TargetScope::Any,
                        TargetKind::PlayerOrUnit,
                        1,
                    )),
                )
                .build(),
        );
        p1.mana_pool.slag = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "shock".into(),
                targets: vec![TargetRef::Card("target_c".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // 2 damage to a 2-toughness unit should kill it
    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
}

#[test]
fn spell_destroy_target() {
    let mut state = setup_game();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("victim", "Victim", CardKind::Unit(UnitType::None))
                .stats(5, 5)
                .build(),
        );
    }
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("murder", "Murder", CardKind::Instant)
                .cost(ManaCost {
                    glitch: 1,
                    colorless: 2,
                    ..Default::default()
                })
                .effect(
                    CardEffect::DestroyTarget,
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .build(),
        );
        p1.mana_pool.glitch = 1;
        p1.mana_pool.colorless = 2;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "murder".into(),
                targets: vec![TargetRef::Card("victim".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
    assert!(
        state
            .players
            .get("p2")
            .unwrap()
            .graveyard
            .iter()
            .any(|c| c.id == "victim")
    );
}

#[test]
fn spell_board_wipe_destroys_all() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("p1_c1", "My Unit", CardKind::Unit(UnitType::None))
                .stats(2, 2)
                .build(),
        );
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("p2_c1", "Their Unit", CardKind::Unit(UnitType::None))
                .stats(3, 3)
                .build(),
        );
    }
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.push(
            CardBuilder::new("wipe", "Damnation", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 2,
                    colorless: 2,
                    ..Default::default()
                })
                .effect(CardEffect::BoardWipe, None)
                .build(),
        );
        p1.mana_pool.glitch = 2;
        p1.mana_pool.colorless = 2;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "wipe".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // All units should be destroyed
    let p1_units: Vec<_> = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .filter(|c| matches!(c.kind, CardKind::Unit(_)))
        .collect();
    let p2_units: Vec<_> = state
        .players
        .get("p2")
        .unwrap()
        .battlefield
        .iter()
        .filter(|c| matches!(c.kind, CardKind::Unit(_)))
        .collect();
    assert!(p1_units.is_empty());
    assert!(p2_units.is_empty());
}

#[test]
fn spell_counter_top_removes_from_stack() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("victim_spell", "Big Spell", CardKind::Ritual)
                .cost(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DamagePlayer(10), None)
                .build(),
        );
        p1.hand.push(
            CardBuilder::new("counter", "Counterspell", CardKind::Instant)
                .cost(ManaCost {
                    neon: 2,
                    ..Default::default()
                })
                .effect(CardEffect::CounterTop, None)
                .build(),
        );
        p1.mana_pool.colorless = 1;
        p1.mana_pool.neon = 2;
    }

    // Cast the big spell
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "victim_spell".into(),
                targets: vec![],
            },
        )
        .unwrap();
    assert_eq!(state.stack.len(), 1);

    // Counter it
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "counter".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Stack should be empty and opponent should not take damage
    assert_eq!(state.stack.len(), 0);
    assert_eq!(state.players.get("p2").unwrap().life, 20);
}

#[test]
fn spell_discard() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("duress", "Duress", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 1,
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
                .build(),
        );
        p1.mana_pool.glitch = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "duress".into(),
                targets: vec![TargetRef::Player("p2".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p2 = state.players.get("p2").unwrap();
    assert!(p2.has_pending_discard());
    assert_eq!(p2.pending_discard_amount(), Some(1));
}

#[test]
fn spell_add_mana_dark_ritual() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("ritual", "Dark Ritual", CardKind::Instant)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .effect(CardEffect::AddMana(ManaType::Glitch, 3), None)
                .build(),
        );
        p1.mana_pool.glitch = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "ritual".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Started with 1 glitch, spent 1, gained 3 = 3
    assert_eq!(state.players.get("p1").unwrap().mana_pool.glitch, 3);
}

#[test]
fn spell_sequence_wheel_effect() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("wheel", "Data Cascade", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 2,
                    colorless: 1,
                    ..Default::default()
                })
                .effect(CardEffect::Wheel(7), None)
                .build(),
        );
        p1.mana_pool.glitch = 2;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "wheel".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // p2 should have pending discard from the wheel
    // p1 had empty hand after casting, so they drew immediately
    let p1 = state.players.get("p1").unwrap();
    let p2 = state.players.get("p2").unwrap();
    assert!(
        !p1.has_pending_discard(),
        "p1 had empty hand, drew immediately"
    );
    assert!(p2.has_pending_discard(), "p2 should have pending discard");
    assert_eq!(p1.hand.len(), 7, "p1 should have drawn 7 cards");
}

#[test]
fn spell_create_tokens_sorcery() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("dragon_fodder", "Forge Fodder", CardKind::Ritual)
                .cost(ManaCost {
                    slag: 1,
                    colorless: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::Sequence(vec![
                        CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                        CardEffect::CreateToken(TokenSpec::new("Gremlin", 1, 1, UnitType::Anomaly)),
                    ]),
                    None,
                )
                .build(),
        );
        p1.mana_pool.slag = 1;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "dragon_fodder".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(
        p1.battlefield
            .iter()
            .filter(|c| c.name == "Gremlin")
            .count(),
        2
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Tap Effect Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn tap_effect_add_mana() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new(
                "mana_dork",
                "Tendril Sapper",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
            .build(),
        );
        // Remove summoning sickness
        if let Some(card) = p1.battlefield.iter_mut().find(|c| c.id == "mana_dork") {
            card.summoning_sickness = false;
        }
        p1.mana_pool.bio = 0;
    }

    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "mana_dork".into(),
            },
        )
        .unwrap();

    assert_eq!(state.players.get("p1").unwrap().mana_pool.bio, 1);
    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .find(|c| c.id == "mana_dork")
            .unwrap()
            .tapped
    );
}

#[test]
fn tap_effect_create_token() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "token_maker",
                "Imperious Perfect",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 2)
            .keyword(Keyword::StaticPowerBuff(1, Some(UnitType::BioOrganism)))
            .keyword(Keyword::StaticToughnessBuff(1, Some(UnitType::BioOrganism)))
            .tap_effect(CardEffect::CreateToken(TokenSpec::new(
                "Spore",
                1,
                1,
                UnitType::BioOrganism,
            )))
            .build(),
        );
        if let Some(card) = p1.battlefield.iter_mut().find(|c| c.id == "token_maker") {
            card.summoning_sickness = false;
        }
    }

    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "token_maker".into(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 2);
    assert!(p1.battlefield.iter().any(|c| c.name == "Spore"));
}

#[test]
fn tap_effect_create_single_token_from_unit() {
    // The tap handler only supports CreateToken (not Sequence) for token creation
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "maker",
                "Token Maker",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .tap_effect(CardEffect::CreateToken(TokenSpec::new(
                "Spore",
                1,
                1,
                UnitType::BioOrganism,
            )))
            .build(),
        );
        if let Some(card) = p1.battlefield.iter_mut().find(|c| c.id == "maker") {
            card.summoning_sickness = false;
        }
    }

    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "maker".into(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.battlefield.len(), 2); // maker + 1 Spore token
    assert!(p1.battlefield.iter().any(|c| c.name == "Spore"));
}

#[test]
fn tap_effect_sequence_adds_mana_and_draws() {
    // Test that Sequence tap effects work correctly (like Flora, Reclaimed Explorer)
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.clear();
        p1.deck.clear();

        // Add cards to deck for drawing
        p1.deck.push(
            CardBuilder::new(
                "card1",
                "Test Card 1",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .build(),
        );
        p1.deck.push(
            CardBuilder::new(
                "card2",
                "Test Card 2",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .build(),
        );

        // Add Flora-like unit with Sequence tap effect
        p1.battlefield.push(
            CardBuilder::new(
                "selvala",
                "Flora, Reclaimed Explorer",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 4)
            .tap_effect(CardEffect::Sequence(vec![
                CardEffect::AddMana(ManaType::Bio, 3),
                CardEffect::DrawCards(1),
            ]))
            .build(),
        );

        // Remove summoning sickness
        if let Some(card) = p1.battlefield.iter_mut().find(|c| c.id == "selvala") {
            card.summoning_sickness = false;
        }

        // Reset mana pool
        p1.mana_pool = ManaPool::default();
    }

    // Verify initial state
    assert_eq!(state.players.get("p1").unwrap().hand.len(), 0);
    assert_eq!(state.players.get("p1").unwrap().mana_pool.bio, 0);

    // Tap Flora
    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "selvala".into(),
            },
        )
        .unwrap();

    // Verify both effects happened
    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.bio, 3, "Should have added 3 Bio mana");
    assert_eq!(p1.hand.len(), 1, "Should have drawn 1 card");
    assert!(
        p1.battlefield
            .iter()
            .find(|c| c.id == "selvala")
            .unwrap()
            .tapped,
        "Flora should be tapped"
    );
}

#[test]
fn tap_effect_sequence_with_multiple_effects() {
    // Test a more complex Sequence with multiple different effects
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.deck.clear();

        // Add cards for drawing
        for i in 0..3 {
            p1.deck.push(
                CardBuilder::new(
                    &format!("card{}", i),
                    &format!("Card {}", i),
                    CardKind::Ritual,
                )
                .build(),
            );
        }

        // Unit with complex sequence: add mana, draw, gain life
        p1.battlefield.push(
            CardBuilder::new(
                "complex",
                "Complex Ability",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .tap_effect(CardEffect::Sequence(vec![
                CardEffect::AddMana(ManaType::Bio, 2),
                CardEffect::AddMana(ManaType::Neon, 1),
                CardEffect::DrawCards(2),
                CardEffect::GainLife(3),
            ]))
            .build(),
        );

        if let Some(card) = p1.battlefield.iter_mut().find(|c| c.id == "complex") {
            card.summoning_sickness = false;
        }

        p1.mana_pool = ManaPool::default();
        p1.life = 20;
    }

    let initial_hand_size = state.players.get("p1").unwrap().hand.len();

    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "complex".into(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.bio, 2, "Should have added 2 Bio mana");
    assert_eq!(p1.mana_pool.neon, 1, "Should have added 1 Neon mana");
    assert_eq!(
        p1.hand.len(),
        initial_hand_size + 2,
        "Should have drawn 2 cards"
    );
    assert_eq!(p1.life, 23, "Should have gained 3 life");
}

// ──────────────────────────────────────────────────────────────────────────────
// Mana Scaling Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn add_mana_scaling_effect_counts_unit_type() {
    // AddManaScaling counts matching units when applied as a spell/ability effect
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // 3 BioOrganisms on the field
        p1.battlefield.push(
            CardBuilder::new("elf1", "Elf 1", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        p1.battlefield.push(
            CardBuilder::new("elf2", "Elf 2", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        p1.battlefield.push(
            CardBuilder::new("elf3", "Elf 3", CardKind::Unit(UnitType::BioOrganism))
                .stats(1, 1)
                .build(),
        );
        // Not a BioOrganism
        p1.battlefield.push(
            CardBuilder::new("drone1", "Drone", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );

        // Spell that uses AddManaScaling effect
        p1.hand.push(
            CardBuilder::new("ritual", "Green Ritual", CardKind::Instant)
                .effect(
                    CardEffect::AddManaScaling(ManaType::Bio, Some(UnitType::BioOrganism), None),
                    None,
                )
                .build(),
        );
        p1.mana_pool.bio = 0;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "ritual".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // 3 BioOrganisms on the field = 3 bio mana
    assert_eq!(state.players.get("p1").unwrap().mana_pool.bio, 3);
}

// ──────────────────────────────────────────────────────────────────────────────
// Infrastructure / Mana System Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn infrastructure_taps_for_mana() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("bio_cell", "Bio-Cell", CardKind::Infrastructure)
                .tap_effect(CardEffect::AddMana(ManaType::Bio, 1))
                .build(),
        );
        p1.mana_pool = ManaPool::default();
    }

    state
        .update(
            "p1",
            CardGameAction::PlayLand {
                card_id: "bio_cell".into(),
            },
        )
        .unwrap();
    state
        .update(
            "p1",
            CardGameAction::TapCard {
                card_id: "bio_cell".into(),
            },
        )
        .unwrap();

    assert_eq!(state.players.get("p1").unwrap().mana_pool.bio, 1);
}

#[test]
fn mana_cost_prevents_casting_if_insufficient() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("expensive", "Expensive Spell", CardKind::Ritual)
                .cost(ManaCost {
                    bio: 5,
                    colorless: 3,
                    ..Default::default()
                })
                .effect(CardEffect::DrawCards(1), None)
                .build(),
        );
        p1.mana_pool = ManaPool::default();
        p1.mana_pool.bio = 1; // Not enough
    }

    assert!(
        state
            .update(
                "p1",
                CardGameAction::CastSpell {
                    card_id: "expensive".into(),
                    targets: vec![],
                }
            )
            .is_err()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Grant Keyword Effect Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn grant_keyword_trample_via_targeted_spell() {
    // GrantKeyword requires explicit card targets to apply
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "small_unit",
                "Small Unit",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .build(),
        );
        // Spell that grants Trample to a target unit
        p1.hand.push(
            CardBuilder::new("buff_spell", "Bestow Trample", CardKind::Instant)
                .cost(ManaCost {
                    bio: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                    Some(TargetSpec::new(
                        TargetScope::SelfPlayer,
                        TargetKind::Unit,
                        1,
                    )),
                )
                .build(),
        );
        p1.mana_pool.bio = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "buff_spell".into(),
                targets: vec![TargetRef::Card("small_unit".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Small unit should now have trample via effect_manager
    let p1 = state.players.get("p1").unwrap();
    let small = p1
        .battlefield
        .iter()
        .find(|c| c.id == "small_unit")
        .unwrap();
    let (_, _, _, _, _, _, has_trample) = state.card_combat_stats(small);
    assert!(has_trample);
}

#[test]
fn grant_keyword_without_targets_applies_to_owner_units() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.hand.clear();
        p1.battlefield.push(
            CardBuilder::new("alpha", "Alpha", CardKind::Unit(UnitType::Megafauna))
                .stats(2, 2)
                .build(),
        );
        p1.battlefield.push(
            CardBuilder::new("beta", "Beta", CardKind::Unit(UnitType::Megafauna))
                .stats(3, 3)
                .build(),
        );
        p1.hand.push(
            CardBuilder::new("team_trample", "Team Trample", CardKind::Ritual)
                .cost(ManaCost {
                    bio: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::GrantKeyword(Keyword::Trample, ExpireContract::EndOfTurn),
                    None,
                )
                .build(),
        );
        p1.mana_pool.bio = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "team_trample".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p1 = state.players.get("p1").unwrap();
    for unit_id in ["alpha", "beta"] {
        let unit = p1
            .battlefield
            .iter()
            .find(|card| card.id == unit_id)
            .expect("unit should exist");
        let (_, _, _, _, _, _, has_trample) = state.card_combat_stats(unit);
        assert!(has_trample, "Teamwide GrantKeyword should affect {unit_id}");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Predicated ETB Tests (OnEnterBattlefield with predicate)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn etb_with_card_kind_predicate_unit() {
    // ETB predicate CardKind checks the entering card itself against the target
    // A unit with predicate CardKind(Unit(BioOrganism)) only triggers when
    // the entering card is of that kind
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        // Unit that only draws on ETB if it's a BioOrganism (which it is)
        p1.hand.push(
            CardBuilder::new(
                "bio_scout",
                "Bio Scout",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(1, 1)
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::DrawCards(1),
                limit: TriggerLimit::Unlimited,
                predicate: Some(Predicate::CardKind(CardKind::Unit(UnitType::BioOrganism))),
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bio_scout".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();
    // Played from hand (-1), ETB drew 1 (+1) = 1 card in hand
    assert_eq!(p1.hand.len(), 1);
    assert!(p1.battlefield.iter().any(|c| c.id == "bio_scout"));
}

// ──────────────────────────────────────────────────────────────────────────────
// Death Trigger Stacking Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn multiple_death_triggers_stack_damage() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        // 3 units each with their own OnDeath trigger
        for i in 0..3 {
            p1.battlefield.push(
                CardBuilder::new(
                    &format!("payoff_{}", i),
                    &format!("Payoff {}", i),
                    CardKind::Unit(UnitType::Anomaly),
                )
                .stats(1, 1)
                .trigger(CardTrigger {
                    event: TriggerEvent::Died,
                    effect: CardEffect::DamagePlayer(1),
                    limit: TriggerLimit::Unlimited,
                    predicate: None,
                    optional_cost: None,
                    is_optional: false,
                })
                .build(),
            );
        }
    }

    let p2_life = state.players.get("p2").unwrap().life;
    // Kill each payoff - each one's OnDeath fires when IT dies
    state.remove_from_battlefield_and_attachments("p1", "payoff_0");
    state.remove_from_battlefield_and_attachments("p1", "payoff_1");
    state.remove_from_battlefield_and_attachments("p1", "payoff_2");

    // Each dying payoff triggered once, dealing 1 damage each = 3 total
    assert_eq!(state.players.get("p2").unwrap().life, p2_life - 3);
}

// ──────────────────────────────────────────────────────────────────────────────
// Gain Life Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn gain_life_spell() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("heal", "Heal", CardKind::Instant)
                .cost(ManaCost {
                    bio: 1,
                    ..Default::default()
                })
                .effect(CardEffect::GainLife(5), None)
                .build(),
        );
        p1.mana_pool.bio = 1;
    }

    let life_before = state.players.get("p1").unwrap().life;
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "heal".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    assert_eq!(state.players.get("p1").unwrap().life, life_before + 5);
}

// ──────────────────────────────────────────────────────────────────────────────
// Bounce Effect Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bounce_returns_unit_to_hand() {
    let mut state = setup_game();

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("bounce_target", "Big Guy", CardKind::Unit(UnitType::None))
                .stats(5, 5)
                .build(),
        );
    }
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("unsummon", "Unsummon", CardKind::Instant)
                .cost(ManaCost {
                    neon: 1,
                    ..Default::default()
                })
                .effect(
                    CardEffect::Bounce,
                    Some(TargetSpec::new(TargetScope::Any, TargetKind::Unit, 1)),
                )
                .build(),
        );
        p1.mana_pool.neon = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "unsummon".into(),
                targets: vec![TargetRef::Card("bounce_target".into())],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    assert!(state.players.get("p2").unwrap().battlefield.is_empty());
    assert!(
        state
            .players
            .get("p2")
            .unwrap()
            .hand
            .iter()
            .any(|c| c.id == "bounce_target")
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Pending Discard Blocking Tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn pending_discard_blocks_all_players() {
    let mut state = setup_game();

    // Give p2 some cards to discard
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        p2.hand
            .push(CardBuilder::new("card1", "Card 1", CardKind::Instant).build());
        p2.hand
            .push(CardBuilder::new("card2", "Card 2", CardKind::Instant).build());
    }

    // Give p1 a discard spell (like Burglar Rat)
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("discard_spell", "Discard Spell", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .effect(CardEffect::Discard(1), None)
                .build(),
        );
        p1.mana_pool.glitch = 1;

        // Also give p1 another card and some mana for a second action
        p1.hand.push(
            CardBuilder::new("another_spell", "Another Spell", CardKind::Instant)
                .cost(ManaCost {
                    colorless: 1,
                    ..Default::default()
                })
                .effect(CardEffect::DrawCards(1), None)
                .build(),
        );
        p1.mana_pool.colorless = 1;
    }

    // Cast the discard spell
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "discard_spell".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Verify p2 has pending discard
    assert!(state.players.get("p2").unwrap().has_pending_discard());

    // Now try to have p1 cast another spell - this should be BLOCKED
    let result = state.update(
        "p1",
        CardGameAction::CastSpell {
            card_id: "another_spell".into(),
            targets: vec![],
        },
    );

    assert!(
        result.is_err(),
        "p1 should not be able to cast a spell while p2 has pending discard"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("must resolve discard"),
        "Error should mention pending discard"
    );

    // Verify p2 also cannot take actions (like advancing to next stage)
    let result = state.update("p2", CardGameAction::NextStage);
    assert!(
        result.is_err(),
        "p2 should not be able to advance stage while having pending discard"
    );

    // Only DiscardCommit should work
    let result = state.update(
        "p2",
        CardGameAction::DiscardCommit {
            card_ids: vec!["card1".into()],
        },
    );
    assert!(result.is_ok(), "DiscardCommit should be allowed");

    // After resolving discard, p1 should be able to act again
    let result = state.update(
        "p1",
        CardGameAction::CastSpell {
            card_id: "another_spell".into(),
            targets: vec![],
        },
    );
    assert!(
        result.is_ok(),
        "p1 should be able to cast a spell after discard is resolved"
    );
}

#[test]
fn wheel_effect_all_players_discard_and_draw() {
    let mut state = setup_game();

    // Give both players some cards in hand
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        for i in 0..3 {
            p1.hand.push(
                CardBuilder::new(
                    &format!("p1_card{}", i),
                    &format!("P1 Card {}", i),
                    CardKind::Instant,
                )
                .build(),
            );
        }
        p1.hand.push(
            CardBuilder::new("wheel", "Data Cascade", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .effect(CardEffect::Wheel(7), None)
                .build(),
        );
        p1.mana_pool.glitch = 1;
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        for i in 0..5 {
            p2.hand.push(
                CardBuilder::new(
                    &format!("p2_card{}", i),
                    &format!("P2 Card {}", i),
                    CardKind::Instant,
                )
                .build(),
            );
        }
    }

    // Cast Wheel - p1 will have 3 cards after casting (wheel goes to stack/graveyard)
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "wheel".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    // Both players should have pending discard (p1 has 3 cards, p2 has 5)
    let p1 = state.players.get("p1").unwrap();
    let p2 = state.players.get("p2").unwrap();
    assert!(
        p1.has_pending_discard(),
        "p1 should have pending discard for 3 cards"
    );
    assert_eq!(p1.pending_discard_amount(), Some(3));
    assert!(
        p2.has_pending_discard(),
        "p2 should have pending discard for 5 cards"
    );
    assert_eq!(p2.pending_discard_amount(), Some(5));

    // p1 commits their discard
    state
        .update(
            "p1",
            CardGameAction::DiscardCommit {
                card_ids: vec!["p1_card0".into(), "p1_card1".into(), "p1_card2".into()],
            },
        )
        .unwrap();

    // p1 should have drawn 7 cards (from pending_effects)
    let p1 = state.players.get("p1").unwrap();
    assert!(!p1.has_pending_discard(), "p1 discard resolved");
    assert_eq!(p1.hand.len(), 7, "p1 should have drawn 7 cards");

    // p2 still has pending discard
    let p2 = state.players.get("p2").unwrap();
    assert!(p2.has_pending_discard(), "p2 still needs to discard");

    // p2 commits their discard
    state
        .update(
            "p2",
            CardGameAction::DiscardCommit {
                card_ids: vec![
                    "p2_card0".into(),
                    "p2_card1".into(),
                    "p2_card2".into(),
                    "p2_card3".into(),
                    "p2_card4".into(),
                ],
            },
        )
        .unwrap();

    // p2 should have drawn 7 cards
    let p2 = state.players.get("p2").unwrap();
    assert!(!p2.has_pending_discard(), "p2 discard resolved");
    assert_eq!(p2.hand.len(), 7, "p2 should have drawn 7 cards");
}

#[test]
fn faithless_looting_sets_pending_discard() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(
            CardBuilder::new("faithless_looting", "Reckless Salvage", CardKind::Ritual)
                .description("Draw 2, discard 2.")
                .cost(ManaCost {
                    slag: 1,
                    ..Default::default()
                })
                .effect(CardEffect::Loot(2, 2), None)
                .build(),
        );
        p1.mana_pool.slag = 1;
        // Ensure p1 has cards in deck to draw
        p1.deck.clear();
        for i in 0..10 {
            p1.deck.push(
                CardBuilder::new(
                    &format!("card{}", i),
                    &format!("Card {}", i),
                    CardKind::Instant,
                )
                .build(),
            );
        }
    }

    let hand_size_before = state.players.get("p1").unwrap().hand.len();

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "faithless_looting".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p1 = state.players.get("p1").unwrap();

    // Should have drawn 2 cards (hand_size_before - 1 for casting + 2 drawn = hand_size_before + 1)
    // Actually after casting, the spell goes to stack/graveyard, so hand is hand_size_before - 1
    // Then draw 2, so hand is hand_size_before - 1 + 2 = hand_size_before + 1
    assert_eq!(
        p1.hand.len(),
        hand_size_before + 1,
        "Should have drawn 2 cards (net +1 after casting)"
    );

    // Should have pending discard for 2 cards
    assert!(p1.has_pending_discard(), "Should have pending discard");
    assert_eq!(
        p1.pending_discard_amount(),
        Some(2),
        "Should need to discard 2 cards"
    );
    match p1.pending_discard() {
        Some(PendingAction::Discard {
            source_description, ..
        }) => assert_eq!(source_description, "Draw 2, then discard 2."),
        _ => panic!("Expected pending discard action"),
    }
}

#[test]
fn virus_disciple_gains_power_when_opponent_discards() {
    let mut state = setup_game();
    let virus_disciple = card_catalog::glitch::all_cards()
        .into_iter()
        .find_map(|(card, _)| (card.id == "virus_disciple").then_some(card))
        .expect("virus_disciple should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.battlefield.clear();
        p1.mana_pool.glitch = 1;
        p1.battlefield.push(virus_disciple);
        p1.hand.push(
            CardBuilder::new("debug_forget", "Debug Forget", CardKind::Ritual)
                .cost(ManaCost {
                    glitch: 1,
                    ..Default::default()
                })
                .effect(CardEffect::Discard(1), None)
                .build(),
        );
    }

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        p2.hand
            .push(CardBuilder::new("pitch_me", "Pitch Me", CardKind::Ritual).build());
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "debug_forget".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p2 = state.players.get("p2").unwrap();
    assert!(p2.has_pending_discard(), "Opponent should need to discard");

    state
        .update(
            "p2",
            CardGameAction::Discard {
                card_id: "pitch_me".into(),
            },
        )
        .unwrap();

    let virus_disciple = state.players.get("p1").unwrap().battlefield[0].clone();
    assert_eq!(
        state.card_combat_stats(&virus_disciple).0,
        2,
        "Virus Disciple should get +1 power until end of turn when an opponent discards"
    );
}

#[test]
fn search_effect_filters_by_card_kind() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.deck.clear();

        // Add a spell that searches for Infrastructure
        p1.hand.push(
            CardBuilder::new(
                "wood_elves",
                "Wood Elves",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .description("When enters, search for Infrastructure.")
            .stats(1, 1)
            .cost(ManaCost {
                bio: 1,
                colorless: 2,
                ..Default::default()
            })
            .trigger(CardTrigger {
                event: TriggerEvent::Entered,
                effect: CardEffect::Search(Predicate::CardKind(CardKind::Infrastructure)),
                limit: TriggerLimit::Unlimited,
                predicate: None,
                optional_cost: None,
                is_optional: false,
            })
            .build(),
        );

        // Build a deck with various card types - put Infrastructure in the middle
        p1.deck
            .push(CardBuilder::new("sorcery1", "Sorcery 1", CardKind::Ritual).build());
        p1.deck
            .push(CardBuilder::new("instant1", "Instant 1", CardKind::Instant).build());
        p1.deck
            .push(CardBuilder::new("forest", "Forest", CardKind::Infrastructure).build()); // This should be found
        p1.deck
            .push(CardBuilder::new("sorcery2", "Sorcery 2", CardKind::Ritual).build());
        p1.deck
            .push(CardBuilder::new("unit1", "Unit 1", CardKind::Unit(UnitType::Drone)).build());

        // Give enough mana to cast
        p1.mana_pool.bio = 1;
        p1.mana_pool.colorless = 2;
    }

    let hand_size_before = state.players.get("p1").unwrap().hand.len();
    let deck_size_before = state.players.get("p1").unwrap().deck.len();

    // Cast Wood Elves - ETB trigger will start the search
    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "wood_elves".into(),
                targets: vec![],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let p1 = state.players.get("p1").unwrap();

    // Wood Elves should be on battlefield
    assert_eq!(
        p1.battlefield
            .iter()
            .filter(|c| c.id == "wood_elves")
            .count(),
        1,
        "Wood Elves should be on battlefield"
    );

    // Search should be active with matching cards in library_search_results
    assert!(p1.library_search_active, "Library search should be active");
    assert_eq!(
        p1.library_search_results.len(),
        1,
        "Should have 1 matching card (Infrastructure)"
    );
    assert_eq!(
        p1.library_search_results[0].id, "forest",
        "Matching card should be forest"
    );

    // Commit the search - pick the forest
    state
        .update(
            "p1",
            CardGameAction::SearchCommit {
                card_id: "forest".into(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();

    // Search should be resolved
    assert!(
        !p1.library_search_active,
        "Library search should be resolved"
    );
    assert!(
        p1.scry.is_empty(),
        "Scry should be empty after search commit"
    );

    // Deck should be smaller (one card removed)
    assert_eq!(
        p1.deck.len(),
        deck_size_before - 1,
        "Deck should have one less card"
    );

    // Hand should have the Infrastructure card (net 0 change: -1 Wood Elves cast, +1 Infrastructure found)
    assert_eq!(
        p1.hand.len(),
        hand_size_before,
        "Hand size should be same (cast Wood Elves, drew Infrastructure)"
    );

    // Verify the found card is Infrastructure
    let found_card = p1.hand.iter().find(|c| c.id == "forest");
    assert!(
        found_card.is_some(),
        "Should have found the Infrastructure card (forest)"
    );
    assert_eq!(
        found_card.unwrap().kind,
        CardKind::Infrastructure,
        "Found card should be Infrastructure"
    );

    // Verify Sorcery cards were NOT found
    assert!(
        p1.hand.iter().find(|c| c.id == "sorcery1").is_none(),
        "Should not have found sorcery1"
    );
    assert!(
        p1.hand.iter().find(|c| c.id == "sorcery2").is_none(),
        "Should not have found sorcery2"
    );
}

#[test]
fn search_graveyard_effect_returns_selected_match_to_hand() {
    let mut state = setup_game();
    let root_memory = card_catalog::bio::all_cards()
        .into_iter()
        .find_map(|(card, _)| (card.id == "root_memory").then_some(card))
        .expect("root_memory should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.deck.clear();
        p1.graveyard.clear();
        p1.hand.push(root_memory);
        p1.graveyard.push(
            CardBuilder::new(
                "dead_spore",
                "Dead Spore",
                CardKind::Unit(UnitType::BioOrganism),
            )
            .stats(2, 2)
            .build(),
        );
        p1.graveyard
            .push(CardBuilder::new("dead_patch", "Dead Patch", CardKind::Aura).build());
        p1.mana_pool.bio = 1;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "root_memory".into(),
                targets: vec![],
            },
        )
        .unwrap();
    state.update("p1", CardGameAction::ResolveStackTop).unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert!(
        p1.library_search_active,
        "Graveyard search should use the normal search selection flow"
    );
    assert_eq!(
        p1.library_search_results.len(),
        1,
        "Only matching graveyard cards should be shown"
    );
    assert_eq!(
        p1.library_search_results[0].id, "dead_spore",
        "The graveyard unit should be the selectable card"
    );
    assert_eq!(
        p1.pending_effects,
        vec![(
            CardEffect::CreateToken(TokenSpec::new("Spore", 1, 1, UnitType::BioOrganism)),
            vec![],
        )],
        "Root Memory should queue its Spore reward after the graveyard selection"
    );
    assert!(
        p1.graveyard.iter().any(|card| card.id == "dead_spore"),
        "The card should stay in the graveyard until the player commits the search"
    );

    state
        .update(
            "p1",
            CardGameAction::SearchCommit {
                card_id: "dead_spore".into(),
            },
        )
        .unwrap();

    let p1 = state.players.get("p1").unwrap();
    assert!(
        !p1.library_search_active,
        "Graveyard search should resolve after choosing a card"
    );
    assert!(
        p1.scry.is_empty(),
        "Search results should clear after commit"
    );
    assert!(
        p1.hand.iter().any(|card| card.id == "dead_spore"),
        "The selected graveyard card should move to hand"
    );
    assert!(
        !p1.graveyard.iter().any(|card| card.id == "dead_spore"),
        "The selected graveyard card should leave the graveyard"
    );
    assert!(
        p1.battlefield.iter().any(|card| {
            card.name == "Spore" && matches!(card.kind, CardKind::Unit(UnitType::BioOrganism))
        }),
        "Resolving Root Memory should also create a Spore token"
    );
}

#[test]
fn scry_commit_survives_intervening_deck_mutation() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.deck = vec![
            CardBuilder::new("bottom_card", "Bottom Card", CardKind::Infrastructure).build(),
            CardBuilder::new("keep_card", "Keep Card", CardKind::Infrastructure).build(),
            CardBuilder::new("drawn_card", "Drawn Card", CardKind::Infrastructure).build(),
        ];
        p1.scry = vec![
            CardBuilder::new("drawn_card", "Drawn Card", CardKind::Infrastructure).build(),
            CardBuilder::new("keep_card", "Keep Card", CardKind::Infrastructure).build(),
        ];
        p1.hand.push(
            p1.deck
                .pop()
                .expect("the viewed top card should still be drawable"),
        );
        p1.pending_effects = vec![(CardEffect::DrawCards(1), vec![])];
    }

    state
        .update(
            "p1",
            CardGameAction::ScryCommit {
                top: vec!["drawn_card".into()],
                bottom: vec!["keep_card".into()],
            },
        )
        .expect("stale scry should still resolve");

    let p1 = state.players.get("p1").unwrap();
    assert!(p1.scry.is_empty(), "Scry state should clear after commit");
    assert!(
        p1.pending_effects.is_empty(),
        "Queued follow-up effects should still resolve after the commit"
    );
    assert!(
        p1.hand.iter().any(|card| card.id == "drawn_card"),
        "The already-drawn card should stay in hand"
    );
    assert!(
        p1.hand.iter().any(|card| card.id == "bottom_card"),
        "Pending draw effects should still fire after the commit"
    );
    assert_eq!(
        p1.deck
            .iter()
            .map(|card| card.id.as_str())
            .collect::<Vec<_>>(),
        vec!["keep_card"],
        "Remaining scry cards that stay in the deck should be reordered normally"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Can Cast with untapped infrastructure
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn can_cast_with_untapped_slag_forge() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.battlefield.clear();
        p1.mana_pool = ManaPool::default();

        // Add an untapped Slag Forge to battlefield
        p1.battlefield.push(
            CardBuilder::new("slag_forge", "Slag Forge", CardKind::Infrastructure)
                .tap_effect(CardEffect::AddMana(ManaType::Slag, 1))
                .build(),
        );

        // Add a sorcery that costs 1 slag to hand
        p1.hand.push(
            CardBuilder::new("faithless_looting", "Reckless Salvage", CardKind::Ritual)
                .cost(ManaCost {
                    slag: 1,
                    ..Default::default()
                })
                .build(),
        );

        // Add a unit that costs 1 slag + 1 colorless to hand
        p1.hand.push(
            CardBuilder::new(
                "zulaport",
                "Exhaust Cutthroat",
                CardKind::Unit(UnitType::Anomaly),
            )
            .cost(ManaCost {
                slag: 1,
                colorless: 1,
                ..Default::default()
            })
            .stats(1, 1)
            .build(),
        );

        // Fill mana pool from infrastructure (simulates turn start auto-fill)
        CardGameState::fill_mana_from_infrastructure(p1);
    }

    // Check available mana
    let available = state.available_mana_for_player("p1");
    assert_eq!(
        available.slag, 1,
        "Should have 1 slag available from untapped forge"
    );

    // Reckless Salvage costs {slag: 1} - should be castable
    let looting = state
        .players
        .get("p1")
        .unwrap()
        .hand
        .iter()
        .find(|c| c.id == "faithless_looting")
        .unwrap();
    assert!(
        state.can_cast_card("p1", looting),
        "Reckless Salvage (cost: 1 slag) should be castable with 1 untapped Slag Forge"
    );

    // Exhaust Cutthroat costs {slag: 1, colorless: 1} - should NOT be castable (only 1 mana available)
    let zulaport = state
        .players
        .get("p1")
        .unwrap()
        .hand
        .iter()
        .find(|c| c.id == "zulaport")
        .unwrap();
    assert!(
        !state.can_cast_card("p1", zulaport),
        "Exhaust Cutthroat (cost: 1 slag + 1 colorless) should NOT be castable with only 1 Slag Forge"
    );

    // Now check the view
    let view = <CardGameState as GameLogic>::view_for(&state, "p1");
    let viewer = view.viewer.unwrap();
    let looting_view = viewer
        .hand
        .iter()
        .find(|c| c.id == "faithless_looting")
        .unwrap();
    assert!(
        looting_view.can_cast,
        "Reckless Salvage view should have can_cast = true"
    );
    assert!(
        looting_view.playable,
        "Reckless Salvage view should have playable = true"
    );

    let zulaport_view = viewer.hand.iter().find(|c| c.id == "zulaport").unwrap();
    assert!(
        !zulaport_view.can_cast,
        "Exhaust Cutthroat view should have can_cast = false (can't afford)"
    );
}

#[test]
fn stat_type_both_modifier_updates_combat_stats_and_view_breakdown() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("test_drone", "Test Drone", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .build(),
        );
    }

    state
        .effect_manager
        .add_effect(crate::effect::StatModifierEffect {
            id: "both-buff".into(),
            target: EffectTarget::Card("test_drone".into()),
            stat: StatType::Both,
            amount: ModifierAmount::Fixed(2),
            expires: ExpireContract::Never,
            owner_id: "spell-effect".into(),
        });

    let unit = state.players.get("p1").unwrap().battlefield[0].clone();
    let (power, toughness, _, _, _, _, _) = state.card_combat_stats(&unit);
    assert_eq!(power, 4, "StatType::Both should raise combat power");
    assert_eq!(toughness, 4, "StatType::Both should raise combat toughness");

    let view = <CardGameState as GameLogic>::view_for(&state, "p1");
    let viewer = view.viewer.expect("viewer");
    let unit_view = viewer
        .battlefield_units
        .iter()
        .find(|card| card.id == "test_drone")
        .expect("test drone in battlefield view");
    assert_eq!(unit_view.power, Some(4));
    assert_eq!(unit_view.toughness, Some(4));
    assert!(unit_view.stat_modifiers.iter().any(|modifier| {
        modifier.power_mod == 2 && modifier.toughness_mod == 2 && modifier.label == "Spell effect"
    }));
}

#[test]
fn bitrot_trinket_thief_remembers_the_chosen_player() {
    let mut state = setup_game();
    let bitrot = card_catalog::glitch::all_cards()
        .into_iter()
        .find(|(card, _)| card.id == "bitrot_trinket_thief")
        .map(|(card, _)| card)
        .expect("bitrot_trinket_thief should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(bitrot);
        p1.mana_pool.glitch = 2;
        p1.mana_pool.colorless = 1;
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bitrot_trinket_thief".into(),
                targets: vec![TargetRef::Player("p2".into())],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    let bitrot = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|card| card.id == "bitrot_trinket_thief")
        .expect("bitrot should be on the battlefield");

    assert_eq!(bitrot.remembered_player_id.as_deref(), Some("p2"));

    let viewer = <CardGameState as GameLogic>::view_for(&state, "p1")
        .viewer
        .expect("viewer");
    let bitrot_view = viewer
        .battlefield_units
        .iter()
        .find(|card| card.id == "bitrot_trinket_thief")
        .expect("bitrot should be in the battlefield view");
    assert_eq!(
        bitrot_view.remembered_player_name.as_deref(),
        Some("Player p2")
    );
}

#[test]
fn bitrot_trinket_thief_only_hits_the_remembered_player() {
    let mut state = setup_game();
    let bitrot = card_catalog::glitch::all_cards()
        .into_iter()
        .find(|(card, _)| card.id == "bitrot_trinket_thief")
        .map(|(card, _)| card)
        .expect("bitrot_trinket_thief should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(bitrot);
        p1.mana_pool.glitch = 2;
        p1.mana_pool.colorless = 1;
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        p2.hand
            .push(CardBuilder::new("pitch_me", "Pitch Me", CardKind::Ritual).build());
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bitrot_trinket_thief".into(),
                targets: vec![TargetRef::Player("p1".into())],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state.apply_effect(
        "p1",
        &CardEffect::Discard(1),
        &[TargetRef::Player("p2".into())],
    );

    state
        .update(
            "p2",
            CardGameAction::Discard {
                card_id: "pitch_me".into(),
            },
        )
        .unwrap();

    assert_eq!(
        state.players.get("p2").unwrap().life,
        20,
        "Bitrot should ignore discards from players it did not choose"
    );
}

#[test]
fn bitrot_trinket_thief_hits_for_six_when_the_chosen_player_loses_their_last_card() {
    let mut state = setup_game();
    let bitrot = card_catalog::glitch::all_cards()
        .into_iter()
        .find(|(card, _)| card.id == "bitrot_trinket_thief")
        .map(|(card, _)| card)
        .expect("bitrot_trinket_thief should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(bitrot);
        p1.mana_pool.glitch = 2;
        p1.mana_pool.colorless = 1;
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.hand.clear();
        p2.hand
            .push(CardBuilder::new("pitch_me", "Pitch Me", CardKind::Ritual).build());
    }

    state
        .update(
            "p1",
            CardGameAction::CastSpell {
                card_id: "bitrot_trinket_thief".into(),
                targets: vec![TargetRef::Player("p2".into())],
            },
        )
        .unwrap();

    state.update("p1", CardGameAction::ResolveStackTop).unwrap();
    state.apply_effect(
        "p1",
        &CardEffect::Discard(1),
        &[TargetRef::Player("p2".into())],
    );

    state
        .update(
            "p2",
            CardGameAction::Discard {
                card_id: "pitch_me".into(),
            },
        )
        .unwrap();

    assert_eq!(
        state.players.get("p2").unwrap().life,
        16,
        "Bitrot should deal 4 total (1 per discard + 3 last-card bonus) when the chosen player discards their last card"
    );
}

#[test]
fn predicate_text_deduplicates_equivalent_any_card_kinds() {
    let text = CardGameState::predicate_text(&Predicate::Any(vec![
        Predicate::CardKind(CardKind::Relic),
        Predicate::CardKind(CardKind::Relic),
    ]));

    assert_eq!(text, "relic");
}

// ──────────────────────────────────────────────────────────────────────────────
// New effects tests
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn siphon_drains_life_from_target_and_heals_caster() {
    let mut state = setup_game();
    state.players.get_mut("p2").unwrap().life = 20;
    state.players.get_mut("p1").unwrap().life = 15;

    state.apply_effect(
        "p1",
        &CardEffect::Siphon(3),
        &[TargetRef::Player("p2".into())],
    );

    assert_eq!(
        state.players.get("p2").unwrap().life,
        17,
        "Target should lose 3 life"
    );
    assert_eq!(
        state.players.get("p1").unwrap().life,
        18,
        "Caster should gain 3 life"
    );
}

#[test]
fn siphon_caps_drain_at_target_remaining_life() {
    let mut state = setup_game();
    state.players.get_mut("p2").unwrap().life = 2;
    state.players.get_mut("p1").unwrap().life = 10;

    state.apply_effect(
        "p1",
        &CardEffect::Siphon(5),
        &[TargetRef::Player("p2".into())],
    );

    assert_eq!(
        state.players.get("p2").unwrap().life,
        0,
        "Target life should not go below 0"
    );
    assert_eq!(
        state.players.get("p1").unwrap().life,
        12,
        "Caster should only gain what target actually lost"
    );
}

#[test]
fn transmute_mana_converts_one_color_to_another() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.mana_pool = ManaPool::default();
        p1.mana_pool.bio = 3;
    }

    state.apply_effect(
        "p1",
        &CardEffect::TransmuteMana(ManaType::Bio, ManaType::Slag, 2),
        &[],
    );

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.bio, 1, "Should have spent 2 Bio");
    assert_eq!(p1.mana_pool.slag, 2, "Should have gained 2 Slag");
}

#[test]
fn transmute_mana_caps_at_available_source() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.mana_pool = ManaPool::default();
        p1.mana_pool.neon = 1;
    }

    state.apply_effect(
        "p1",
        &CardEffect::TransmuteMana(ManaType::Neon, ManaType::Chrome, 5),
        &[],
    );

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.mana_pool.neon, 0, "All Neon should be consumed");
    assert_eq!(
        p1.mana_pool.chrome, 1,
        "Should only gain what was available"
    );
}

#[test]
fn swap_exchanges_units_between_players() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("unit_a", "Unit A", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .build(),
        );
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("unit_b", "Unit B", CardKind::Unit(UnitType::Virus))
                .stats(3, 3)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::Swap,
        &[
            TargetRef::Card("unit_a".into()),
            TargetRef::Card("unit_b".into()),
        ],
    );

    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "unit_b"),
        "P1 should now control unit_b"
    );
    assert!(
        state
            .players
            .get("p2")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "unit_a"),
        "P2 should now control unit_a"
    );
}

#[test]
fn phase_removes_unit_from_battlefield_to_exile() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "phase_target",
                "Phase Target",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(4, 4)
            .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::Phase,
        &[TargetRef::Card("phase_target".into())],
    );

    let p1 = state.players.get("p1").unwrap();
    assert!(
        !p1.battlefield.iter().any(|c| c.id == "phase_target"),
        "Unit should be removed from battlefield"
    );
    assert!(
        p1.exile.iter().any(|c| c.id == "phase_target"),
        "Unit should be in exile"
    );
    assert!(
        state
            .phased_out
            .iter()
            .any(|(owner, card)| owner == "p1" && card == "phase_target"),
        "Unit should be tracked in phased_out list"
    );
}

#[test]
fn mutate_target_permanently_alters_base_stats() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("mutate_me", "Mutate Me", CardKind::Unit(UnitType::Drone))
                .stats(3, 3)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::MutateTarget(StatModifierSpec {
            stat: StatType::Power,
            amount: ModifierAmount::Fixed(2),
            expires: ExpireContract::Never,
            target: EffectTargetSpec::Targets,
        }),
        &[TargetRef::Card("mutate_me".into())],
    );

    let card = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|c| c.id == "mutate_me")
        .expect("card should still be on battlefield");
    assert_eq!(
        card.power,
        Some(5),
        "Base power should be permanently increased"
    );
    assert_eq!(card.toughness, Some(3), "Toughness should be unchanged");
}

#[test]
fn mutate_target_kills_unit_when_toughness_hits_zero() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("fragile", "Fragile", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::MutateTarget(StatModifierSpec {
            stat: StatType::Toughness,
            amount: ModifierAmount::Fixed(-3),
            expires: ExpireContract::Never,
            target: EffectTargetSpec::Targets,
        }),
        &[TargetRef::Card("fragile".into())],
    );

    assert!(
        !state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "fragile"),
        "Unit with 0 or less toughness should be removed"
    );
}

#[test]
fn polymorph_replaces_unit_with_next_unit_from_library() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("old_unit", "Old Unit", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
        p1.deck.clear();
        // Put a non-unit then a unit on top (deck is a stack, last = top)
        p1.deck
            .push(CardBuilder::new("spell_card", "Spell Card", CardKind::Ritual).build());
        p1.deck.push(
            CardBuilder::new("new_unit", "New Unit", CardKind::Unit(UnitType::Virus))
                .stats(5, 5)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::Polymorph,
        &[TargetRef::Card("old_unit".into())],
    );

    let p1 = state.players.get("p1").unwrap();
    assert!(
        !p1.battlefield.iter().any(|c| c.id == "old_unit"),
        "Original unit should be gone"
    );
    assert!(
        p1.battlefield.iter().any(|c| c.id == "new_unit"),
        "New unit should be on battlefield"
    );
    assert!(
        p1.deck.iter().any(|c| c.id == "spell_card"),
        "Skipped non-unit should be back in library"
    );
}

#[test]
fn surveil_sets_up_scry_with_surveil_flag() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.deck.clear();
        p1.deck
            .push(CardBuilder::new("bottom", "Bottom", CardKind::Ritual).build());
        p1.deck
            .push(CardBuilder::new("top", "Top", CardKind::Ritual).build());
    }

    state.apply_effect("p1", &CardEffect::Surveil(2), &[]);

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.scry.len(), 2, "Should present 2 cards for surveil");
    assert!(p1.surveil_active, "surveil_active flag should be set");
}

#[test]
fn transmit_sets_up_search_from_top_of_library() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.deck.clear();
        p1.deck
            .push(CardBuilder::new("deep", "Deep", CardKind::Ritual).build());
        p1.deck
            .push(CardBuilder::new("mid", "Mid", CardKind::Ritual).build());
        p1.deck
            .push(CardBuilder::new("top", "Top", CardKind::Ritual).build());
    }

    state.apply_effect("p1", &CardEffect::Transmit(3), &[]);

    let p1 = state.players.get("p1").unwrap();
    assert_eq!(p1.scry.len(), 3, "Should present 3 cards to choose from");
    assert!(p1.library_search_active, "Library search should be active");
    assert_eq!(
        p1.deck.len(),
        0,
        "Cards should be removed from deck during selection"
    );
}

#[test]
fn entomb_moves_matching_card_from_library_to_graveyard() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.deck.clear();
        p1.graveyard.clear();
        p1.deck
            .push(CardBuilder::new("spell1", "Spell", CardKind::Ritual).build());
        p1.deck.push(
            CardBuilder::new("creature1", "Creature", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .build(),
        );
        p1.deck
            .push(CardBuilder::new("spell2", "Spell 2", CardKind::Ritual).build());
    }

    state.apply_effect(
        "p1",
        &CardEffect::Entomb(Predicate::CardKind(CardKind::Unit(UnitType::Drone))),
        &[],
    );

    let p1 = state.players.get("p1").unwrap();
    assert!(
        p1.graveyard.iter().any(|c| c.id == "creature1"),
        "Matching card should be in graveyard"
    );
    assert!(
        !p1.deck.iter().any(|c| c.id == "creature1"),
        "Card should no longer be in library"
    );
}

#[test]
fn recycle_shuffles_graveyard_into_library() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.deck.clear();
        p1.graveyard.clear();
        p1.graveyard
            .push(CardBuilder::new("grave1", "Grave 1", CardKind::Ritual).build());
        p1.graveyard
            .push(CardBuilder::new("grave2", "Grave 2", CardKind::Ritual).build());
        p1.graveyard
            .push(CardBuilder::new("grave3", "Grave 3", CardKind::Instant).build());
    }

    state.apply_effect("p1", &CardEffect::Recycle, &[]);

    let p1 = state.players.get("p1").unwrap();
    assert!(p1.graveyard.is_empty(), "Graveyard should be empty");
    assert_eq!(p1.deck.len(), 3, "All 3 cards should be in library");
}

#[test]
fn damage_split_hits_multiple_targets() {
    let mut state = setup_game();
    state.players.get_mut("p2").unwrap().life = 20;
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("weak_unit", "Weak Unit", CardKind::Unit(UnitType::Drone))
                .stats(1, 2)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::DamageSplit(3),
        &[
            TargetRef::Player("p2".into()),
            TargetRef::Card("weak_unit".into()),
        ],
    );
    state.apply_passive_state_updates();

    assert_eq!(
        state.players.get("p2").unwrap().life,
        17,
        "Player should take 3 damage"
    );
    assert!(
        !state
            .players
            .get("p2")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "weak_unit"),
        "Unit with toughness <= damage should be destroyed"
    );
}

#[test]
fn cast_spell_log_includes_targets() {
    let mut state = setup_game();
    let furnace_vent = CardBuilder::new("furnace_vent", "Furnace Vent", CardKind::Instant)
        .description("Deal 3 damage split among any targets.")
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::DamageSplit(3),
            Some(TargetSpec {
                scope: TargetScope::Any,
                kind: TargetKind::PlayerOrUnit,
                max_targets: 3,
                optional: true,
                predicate: None,
            }),
        )
        .build();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(furnace_vent);
        p1.mana_pool = ManaPool {
            slag: 1,
            colorless: 1,
            ..Default::default()
        };
    }

    state
        .handle_cast_spell(
            "p1",
            "furnace_vent".into(),
            vec![TargetRef::Player("p2".into())],
        )
        .expect("cast furnace vent");

    let entry = state.game_log.last().expect("cast log entry");
    assert_eq!(entry.kind, GameLogKind::CastSpell);
    assert!(
        entry.message.contains("targeting Player p2"),
        "expected target list in cast log, got: {}",
        entry.message
    );
}

#[test]
fn resolving_spell_logs_public_outcome_summary() {
    let mut state = setup_game();
    let furnace_vent = CardBuilder::new("furnace_vent", "Furnace Vent", CardKind::Instant)
        .description("Deal 3 damage split among any targets.")
        .cost(ManaCost {
            slag: 1,
            colorless: 1,
            ..Default::default()
        })
        .effect(
            CardEffect::DamageSplit(3),
            Some(TargetSpec {
                scope: TargetScope::Any,
                kind: TargetKind::PlayerOrUnit,
                max_targets: 3,
                optional: true,
                predicate: None,
            }),
        )
        .build();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand.push(furnace_vent);
        p1.mana_pool = ManaPool {
            slag: 1,
            colorless: 1,
            ..Default::default()
        };
    }

    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.life = 20;
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("weak_unit", "Weak Unit", CardKind::Unit(UnitType::Drone))
                .stats(1, 2)
                .build(),
        );
    }

    state
        .handle_cast_spell(
            "p1",
            "furnace_vent".into(),
            vec![
                TargetRef::Player("p2".into()),
                TargetRef::Card("weak_unit".into()),
            ],
        )
        .expect("cast furnace vent");
    state
        .handle_resolve_stack_top("p1")
        .expect("resolve furnace vent");

    let entry = state.game_log.last().expect("resolution log entry");
    assert_eq!(entry.kind, GameLogKind::Resolved);
    assert!(
        entry.message.contains("Player p2 loses 3 life"),
        "expected life loss summary, got: {}",
        entry.message
    );
    assert!(
        entry.message.contains("Weak Unit dies"),
        "expected death summary, got: {}",
        entry.message
    );
}

#[test]
fn discard_log_lists_discarded_card_names() {
    let mut state = setup_game();

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.hand.clear();
        p1.hand
            .push(CardBuilder::new("discard_a", "Furnace Vent", CardKind::Instant).build());
        p1.hand.push(
            CardBuilder::new(
                "discard_b",
                "Slag Feeder",
                CardKind::Unit(UnitType::Anomaly),
            )
            .build(),
        );
        p1.pending_actions.push(PendingAction::Discard {
            amount: 2,
            source_description: "Test discard".to_string(),
            source_card_id: None,
        });
    }

    state
        .handle_discard_commit("p1", vec!["discard_a".into(), "discard_b".into()])
        .expect("discard cards");

    let entry = state
        .game_log
        .iter()
        .find(|entry| entry.kind == GameLogKind::CardDiscarded)
        .expect("discard log entry");

    assert!(
        entry.message.contains("Furnace Vent"),
        "expected first discarded card in log, got: {}",
        entry.message
    );
    assert!(
        entry.message.contains("Slag Feeder"),
        "expected second discarded card in log, got: {}",
        entry.message
    );
}

#[test]
fn damage_each_opponent_unit_kills_weak_units() {
    let mut state = setup_game();
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("weak", "Weak", CardKind::Unit(UnitType::Drone))
                .stats(1, 2)
                .build(),
        );
        p2.battlefield.push(
            CardBuilder::new("tough", "Tough", CardKind::Unit(UnitType::Drone))
                .stats(3, 5)
                .build(),
        );
    }

    state.apply_effect("p1", &CardEffect::DamageEachOpponentUnit(3), &[]);
    state.apply_passive_state_updates();

    let p2 = state.players.get("p2").unwrap();
    assert!(
        !p2.battlefield.iter().any(|c| c.id == "weak"),
        "Unit with toughness <= 3 should be destroyed"
    );
    assert!(
        p2.battlefield.iter().any(|c| c.id == "tough"),
        "Unit with toughness > 3 should survive"
    );
}

#[test]
fn damage_each_opponent_unit_skips_own_units() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("own_weak", "Own Weak", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
    }
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.battlefield.clear();
        p2.battlefield.push(
            CardBuilder::new("enemy_weak", "Enemy Weak", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
    }

    state.apply_effect("p1", &CardEffect::DamageEachOpponentUnit(2), &[]);
    state.apply_passive_state_updates();

    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "own_weak"),
        "Own units should not be damaged"
    );
    assert!(
        !state
            .players
            .get("p2")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "enemy_weak"),
        "Opponent units should be destroyed"
    );
}

#[test]
fn reflect_deals_damage_to_each_opponent() {
    let mut state = setup_game();
    state.players.get_mut("p1").unwrap().life = 20;
    state.players.get_mut("p2").unwrap().life = 20;

    state.apply_effect("p1", &CardEffect::Reflect(4), &[]);

    assert_eq!(
        state.players.get("p1").unwrap().life,
        20,
        "Caster should not take damage"
    );
    assert_eq!(
        state.players.get("p2").unwrap().life,
        16,
        "Opponent should take 4 damage"
    );
}

#[test]
fn conscript_reanimates_unit_from_opponent_graveyard() {
    let mut state = setup_game();
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.graveyard.clear();
        p2.graveyard.push(
            CardBuilder::new("dead_unit", "Dead Unit", CardKind::Unit(UnitType::Virus))
                .stats(4, 4)
                .build(),
        );
    }

    state.apply_effect("p1", &CardEffect::Conscript, &[]);

    assert!(
        state
            .players
            .get("p1")
            .unwrap()
            .battlefield
            .iter()
            .any(|c| c.id == "dead_unit"),
        "Reanimated unit should be on caster's battlefield"
    );
    assert!(
        !state
            .players
            .get("p2")
            .unwrap()
            .graveyard
            .iter()
            .any(|c| c.id == "dead_unit"),
        "Unit should be removed from opponent's graveyard"
    );
    let unit = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|c| c.id == "dead_unit")
        .unwrap();
    assert!(
        unit.summoning_sickness,
        "Reanimated unit should have summoning sickness"
    );
}

#[test]
fn conscript_does_nothing_when_no_units_in_opponent_graveyard() {
    let mut state = setup_game();
    {
        let p2 = state.players.get_mut("p2").unwrap();
        p2.graveyard.clear();
        p2.graveyard
            .push(CardBuilder::new("spell_only", "Spell Only", CardKind::Ritual).build());
    }

    let p1_bf_count = state.players.get("p1").unwrap().battlefield.len();
    state.apply_effect("p1", &CardEffect::Conscript, &[]);

    assert_eq!(
        state.players.get("p1").unwrap().battlefield.len(),
        p1_bf_count,
        "No unit should be added when opponent graveyard has no units"
    );
}

#[test]
fn scavenge_exiles_from_graveyard_and_adds_counters() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.graveyard.clear();
        p1.graveyard.push(
            CardBuilder::new("fodder", "Fodder", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .build(),
        );
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new(
                "beneficiary",
                "Beneficiary",
                CardKind::Unit(UnitType::Drone),
            )
            .stats(1, 1)
            .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::Scavenge(3),
        &[TargetRef::Card("beneficiary".into())],
    );

    let p1 = state.players.get("p1").unwrap();
    assert!(
        p1.exile.iter().any(|c| c.id == "fodder"),
        "Graveyard unit should be exiled"
    );
    assert!(
        !p1.graveyard.iter().any(|c| c.id == "fodder"),
        "Unit should no longer be in graveyard"
    );
    let beneficiary = p1
        .battlefield
        .iter()
        .find(|c| c.id == "beneficiary")
        .unwrap();
    assert_eq!(beneficiary.counters, 3, "Target should gain 3 counters");
}

#[test]
fn scavenge_does_nothing_without_unit_in_graveyard() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.graveyard.clear();
        p1.battlefield.clear();
        p1.battlefield.push(
            CardBuilder::new("lonely", "Lonely", CardKind::Unit(UnitType::Drone))
                .stats(1, 1)
                .build(),
        );
    }

    state.apply_effect(
        "p1",
        &CardEffect::Scavenge(2),
        &[TargetRef::Card("lonely".into())],
    );

    let lonely = state
        .players
        .get("p1")
        .unwrap()
        .battlefield
        .iter()
        .find(|c| c.id == "lonely")
        .unwrap();
    assert_eq!(
        lonely.counters, 0,
        "No counters should be added without graveyard fodder"
    );
}

#[test]
fn reassembling_scrap_returns_from_scrap_heap_on_upkeep() {
    let mut state = setup_game();

    // Locate the card template and put it on p1's battlefield.
    let (scrap_template, _) = card_catalog::slag::all_cards()
        .into_iter()
        .find(|(c, _)| c.id == "reassembling_scrap")
        .expect("reassembling_scrap should exist");

    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.clear();
        let mut scrap = scrap_template.clone();
        scrap.id = "test_scrap".to_string();
        p1.battlefield.push(scrap);
    }

    // Kill it directly: move to graveyard, simulating death.
    // Use the engine's death flow so the Died trigger fires and spawns Scrap Heap.
    state.remove_from_battlefield_and_attachments("p1", "test_scrap");

    // After death the Scrap Heap token should be on the battlefield and
    // the original card should be in the graveyard.
    let p1 = state.players.get("p1").unwrap();
    let heap_count = p1
        .battlefield
        .iter()
        .filter(|c| c.name == "Scrap Heap")
        .count();
    assert_eq!(
        heap_count,
        1,
        "Scrap Heap token should be created when Reassembling Scrap dies — got battlefield={:?}",
        p1.battlefield.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    let in_grave = p1.graveyard.iter().any(|c| c.name == "Reassembling Scrap");
    assert!(
        in_grave,
        "Reassembling Scrap should be in graveyard after death"
    );

    // Now pass turns so p1 reaches their next upkeep. p2 → p1 upkeep.
    pass_turn(&mut state, "p1");
    pass_turn(&mut state, "p2");

    let p1 = state.players.get("p1").unwrap();
    let scrap_back = p1
        .battlefield
        .iter()
        .any(|c| c.name == "Reassembling Scrap");
    let heap_gone = p1.battlefield.iter().all(|c| c.name != "Scrap Heap");
    assert!(
        scrap_back,
        "Reassembling Scrap should be back on battlefield after p1's next upkeep; battlefield={:?}",
        p1.battlefield.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    assert!(
        heap_gone,
        "Scrap Heap should be sacrificed when it reanimates"
    );
}

#[test]
fn finished_table_resets_when_a_new_instance_looks_at_it() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "a");
    join_player(&mut state, "b");
    state.record_instance_id("a", "instance-a");
    // No database here: mark the catalogue caches loaded so View does not fetch.
    state.card_options_loaded = true;
    state.deck_choices_loaded.insert("a".into());
    state.finish_table();
    assert_eq!(state.stage, CardGameStage::Finished);

    // The instance that watched the game end still sees the finished table.
    state
        .update(
            "a",
            CardGameAction::View {
                name: "Player a".into(),
                instance_id: "instance-a".into(),
            },
        )
        .unwrap();
    assert_eq!(state.stage, CardGameStage::Finished);

    // A fresh workflow instance means someone came back for another game.
    state
        .update(
            "a",
            CardGameAction::View {
                name: "Player a".into(),
                instance_id: "instance-a-2".into(),
            },
        )
        .unwrap();
    assert_eq!(state.stage, CardGameStage::Lobby);
    assert!(state.players.is_empty());
    assert!(state.turn_order.is_empty());
    assert!(state.host_id.is_none());
}

#[test]
fn only_the_host_resets_a_live_table() {
    let mut state = CardGameState::new(CardGameInitArgs::default());
    join_player(&mut state, "host");
    join_player(&mut state, "guest");
    assert!(state.update("guest", CardGameAction::Reset).is_err());
    assert_eq!(state.players.len(), 2);

    state.update("host", CardGameAction::Reset).unwrap();
    assert_eq!(state.stage, CardGameStage::Lobby);
    assert!(state.players.is_empty());

    // Anyone may reset once the game is finished.
    join_player(&mut state, "host");
    join_player(&mut state, "guest");
    state.finish_table();
    state.update("guest", CardGameAction::Reset).unwrap();
    assert_eq!(state.stage, CardGameStage::Lobby);
}

fn every_catalog_card() -> Vec<Card> {
    [
        card_catalog::bio::all_cards(),
        card_catalog::chrome::all_cards(),
        card_catalog::glitch::all_cards(),
        card_catalog::multicolor::all_cards(),
        card_catalog::neon::all_cards(),
        card_catalog::slag::all_cards(),
        card_catalog::syndicate::all_cards(),
    ]
    .into_iter()
    .flatten()
    .map(|(card, _)| card)
    .collect()
}

/// Structural contradictions a card's kind rules out: a spell never sits on
/// the battlefield, so stats, keywords, battlefield triggers and paid
/// abilities on it can never matter, and a unit needs a body.
fn card_shape_problems(card: &Card) -> Vec<String> {
    let mut problems = Vec::new();
    let spell = matches!(card.kind, CardKind::Instant | CardKind::Ritual);
    let unit = matches!(card.kind, CardKind::Unit(_));
    if !unit && (card.power.is_some() || card.toughness.is_some()) {
        problems.push(format!("{:?} with stats", card.kind));
    }
    if unit && (card.power.is_none() || card.toughness.is_none()) {
        problems.push("unit without stats".into());
    }
    let works_on_a_spell = |k: &Keyword| matches!(k, Keyword::Gravecast(_) | Keyword::Cascade | Keyword::Flash);
    let unit_body_only = |k: &Keyword| {
        matches!(
            k,
            Keyword::Haste
                | Keyword::Deathtouch
                | Keyword::Piercing
                | Keyword::ReactiveArmor(_)
                | Keyword::Optics(_)
                | Keyword::Stealth
                | Keyword::Aero
                | Keyword::Trample
                | Keyword::Interceptor
                | Keyword::Vigilance
                | Keyword::Menace
                | Keyword::Wither
                | Keyword::Absorb(_)
                | Keyword::Modular(_)
                | Keyword::Evolve
                | Keyword::Rampage(_)
                | Keyword::Compiling
                | Keyword::SelfPowerPerKind(..)
                | Keyword::SelfToughnessPerKind(..)
        )
    };
    for keyword in &card.keywords {
        if spell && !works_on_a_spell(keyword) {
            problems.push(format!("spell with {keyword:?}, which only matters on the battlefield"));
        } else if !spell && !unit && unit_body_only(keyword) {
            problems.push(format!("{:?} with {keyword:?}, which only a unit can use", card.kind));
        }
    }
    if spell {
        for trigger in card.triggers.iter().filter(|t| t.event != TriggerEvent::Entered) {
            problems.push(format!("spell with a {:?} trigger that never fires", trigger.event));
        }
        if card.abilities.iter().any(|a| !a.cost.is_empty()) {
            problems.push("spell with a paid ability it can never use".into());
        }
        let has_effect = card.triggers.iter().any(|t| t.event == TriggerEvent::Entered)
            || card.abilities.iter().any(|a| a.cost.is_empty());
        if !has_effect {
            problems.push("spell that does nothing".into());
        }
    }
    // No card pays mana to get mana straight back.
    let makes_mana = |effect: &CardEffect| {
        let text = format!("{effect:?}");
        text.contains("AddMana") || text.contains("RefundMana")
    };
    if card.triggers.iter().any(|t| t.event == TriggerEvent::Entered && makes_mana(&t.effect)) {
        problems.push("adds mana as it is cast or enters".into());
    }
    if card.abilities.iter().any(|a| a.cost.is_empty() && makes_mana(&a.effect) && !matches!(card.kind, CardKind::Agent)) {
        problems.push("adds mana as it resolves".into());
    }
    if card.abilities.iter().any(|a| format!("{:?}", a.cost).contains("Mana(") && makes_mana(&a.effect)) {
        problems.push("pays mana for an ability that makes mana".into());
    }
    if matches!(card.kind, CardKind::Agent) && card.loyalty.is_none() {
        problems.push("agent without loyalty".into());
    }
    problems
}

#[test]
fn catalog_cards_have_shapes_their_kind_allows() {
    let problems: Vec<String> = every_catalog_card()
        .iter()
        .flat_map(|card| card_shape_problems(card).into_iter().map(move |p| format!("{} ({}): {p}", card.name, card.id)))
        .collect();
    assert!(problems.is_empty(), "{} card problems:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn a_unit_that_grows_per_relic_shows_the_size_it_fights_at() {
    let mut state = setup_game();
    {
        let p1 = state.players.get_mut("p1").unwrap();
        p1.battlefield.push(
            CardBuilder::new("alloy", "Alloy", CardKind::Unit(UnitType::Drone))
                .stats(2, 2)
                .keyword(Keyword::SelfPowerPerKind(1, CardKind::Relic))
                .keyword(Keyword::SelfToughnessPerKind(1, CardKind::Relic))
                .build(),
        );
        p1.battlefield.push(CardBuilder::new("trinket", "Trinket", CardKind::Relic).build());
    }
    let card = state.players["p1"].battlefield.iter().find(|c| c.id == "alloy").unwrap().clone();
    let (power, toughness, ..) = state.card_combat_stats(&card);
    assert_eq!((power, toughness), (3, 3));
    let view = state.card_view_for("p1", &card, Some(CardZone::Battlefield));
    assert_eq!((view.power, view.toughness), (Some(3), Some(3)));
}
