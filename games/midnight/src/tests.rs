use game_framework::GameLogic;

use crate::gamestate::{GameStage, GameState, MidnightAction, MidnightInitArgs, RoleTransaction, RoleTransactionVisibilty};
use crate::roles::RoleCard;

fn table(players: &[&str]) -> GameState {
    let mut game = GameState::new(MidnightInitArgs::default());
    for id in players {
        game.update(
            id,
            MidnightAction::Join { name: id.to_string(), avatar: String::new(), instance_id: format!("instance-{id}") },
        )
        .unwrap();
    }
    game
}

fn enabled(game: &GameState) -> usize {
    game.all_roles.iter().filter(|r| r.enabled).count()
}

/// Turns roles on or off, as the host does, until `count` are enabled.
fn enable_exactly(game: &mut GameState, host: &str, count: usize) {
    let ids: Vec<String> = game.all_roles.iter().map(|r| r.id.clone()).collect();
    for id in ids {
        let on = game.all_roles.iter().find(|r| r.id == id).unwrap().enabled;
        if (enabled(game) < count && !on) || (enabled(game) > count && on) {
            game.update(host, MidnightAction::EnableRole { role_id: id }).unwrap();
        }
    }
    assert_eq!(enabled(game), count);
}

/// Expires stage timers until the night is over.
fn run_night(game: &mut GameState) {
    for _ in 0..50 {
        if game.stage == GameStage::Voting || game.stage == GameStage::Results {
            return;
        }
        game.update("system", MidnightAction::TickStageExpired).unwrap();
    }
    panic!("the night never ended; stuck at {:?}", game.stage);
}

#[test]
fn the_first_player_to_join_hosts() {
    let game = table(&["ada", "bo"]);
    assert!(game.players["ada"].is_host);
    assert!(!game.players["bo"].is_host);
}

#[test]
fn the_host_toggles_a_role_even_while_a_view_holds_its_card() {
    let mut game = table(&["ada", "bo"]);
    let card = game.all_roles[0].clone();
    let before = card.enabled;
    game.update("ada", MidnightAction::EnableRole { role_id: card.id.clone() }).unwrap();
    assert_eq!(game.all_roles[0].enabled, !before);
    assert_eq!(card.enabled, before, "the held copy is left alone");
}

#[test]
fn only_the_host_toggles_roles() {
    let mut game = table(&["ada", "bo"]);
    let id = game.all_roles[0].id.clone();
    assert!(game.update("bo", MidnightAction::EnableRole { role_id: id }).is_err());
}

#[test]
fn starting_needs_a_card_for_every_player_and_three_hidden() {
    let mut game = table(&["ada", "bo"]);
    enable_exactly(&mut game, "ada", 4);
    assert!(game.update("ada", MidnightAction::StartGame).is_err());
    enable_exactly(&mut game, "ada", 5);
    game.update("ada", MidnightAction::StartGame).unwrap();
    assert_eq!(game.stage, GameStage::Day);
    assert_eq!(game.hidden_cards.len(), 3);
    for player in game.players.values() {
        assert_eq!(player.role_card_stack.len(), 1, "{} was dealt one card", player.id);
    }
}

#[test]
fn a_round_runs_through_the_night_to_a_vote_that_executes_the_most_voted() {
    let mut game = table(&["ada", "bo", "cy"]);
    enable_exactly(&mut game, "ada", 6);
    game.update("ada", MidnightAction::StartGame).unwrap();
    run_night(&mut game);
    assert_eq!(game.stage, GameStage::Voting);

    for (voter, target) in [("ada", "bo"), ("bo", "cy"), ("cy", "bo")] {
        game.update(voter, MidnightAction::Vote { target_id: target.into() }).unwrap();
        game.update(voter, MidnightAction::ToggleLock).unwrap();
    }
    assert_eq!(game.stage, GameStage::Results);
    assert!(!game.players["bo"].is_alive);
    assert!(game.players["ada"].is_alive && game.players["cy"].is_alive);
    assert!(game.winner.is_some());
}

#[test]
fn a_vote_has_to_be_chosen_before_it_is_locked() {
    let mut game = table(&["ada", "bo"]);
    enable_exactly(&mut game, "ada", 5);
    game.update("ada", MidnightAction::StartGame).unwrap();
    run_night(&mut game);
    assert!(game.update("ada", MidnightAction::ToggleLock).is_err());
}

#[test]
fn every_player_sees_their_own_role_once_the_game_starts() {
    let mut game = table(&["ada", "bo"]);
    enable_exactly(&mut game, "ada", 5);
    game.update("ada", MidnightAction::StartGame).unwrap();
    for id in ["ada", "bo"] {
        let view = game.view_for(id);
        let dealt = game.players[id].effective_role_card(None);
        assert_eq!(view["role"]["id"], serde_json::json!(dealt.id), "{id}");
    }
}

/// Hands a player a card from the table, as dealing would.
fn deal(game: &mut GameState, player: &str, role_id: &str) {
    let card = game.all_roles.iter().find(|r| r.role_id == role_id).unwrap().clone();
    game.players.get_mut(player).unwrap().role_card_stack =
        vec![RoleTransaction { card, reason: "test".into(), visibility: RoleTransactionVisibilty::Shown }];
}

fn ability_for(game: &GameState, player: &str) -> bool {
    let card: std::sync::Arc<RoleCard> = game.players[player].effective_role_card(None);
    let hook = card.on_night_start.as_ref().expect("the role has a night ability");
    hook(game, &game.players[player]).is_some()
}

#[test]
fn a_lone_hollow_inspects_a_hidden_card_even_with_a_follower_in_play() {
    let mut game = table(&["ada", "bo", "cy"]);
    deal(&mut game, "ada", "hollow");
    deal(&mut game, "bo", "follower");
    deal(&mut game, "cy", "unmarked");
    assert!(ability_for(&game, "ada"));
}

#[test]
fn two_hollows_wake_without_inspecting() {
    let mut game = table(&["ada", "bo", "cy"]);
    deal(&mut game, "ada", "hollow");
    deal(&mut game, "bo", "hollow");
    deal(&mut game, "cy", "unmarked");
    assert!(!ability_for(&game, "ada"));
}
