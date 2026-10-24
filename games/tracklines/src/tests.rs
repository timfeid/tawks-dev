use crate::ai;
use crate::game::{Action, Card, Game, Stage, TurnPhase, FINAL_ROUND_TRAINS};
use crate::map::{self, ROUTES, CITIES};

fn table(n: usize) -> Game {
    let mut g = Game::new(7);
    for i in 0..n {
        g.apply(&format!("p{i}"), Action::Join { name: format!("Player {i}"), avatar_url: String::new() }).unwrap();
    }
    g
}

fn started(n: usize) -> Game {
    let mut g = table(n);
    g.apply("p0", Action::Start).unwrap();
    // Everyone keeps their opening tickets.
    let ids: Vec<String> = g.players.iter().map(|p| p.id.clone()).collect();
    for id in ids {
        let keep: Vec<u16> = g.player(&id).unwrap().ticket_offer.iter().take(2).map(|t| t.id).collect();
        g.apply(&id, Action::KeepTickets { keep }).unwrap();
    }
    g
}

#[test]
fn map_is_connected_and_routes_reference_known_cities() {
    for r in ROUTES {
        assert!(map::city(r.from).is_some(), "{} unknown", r.from);
        assert!(map::city(r.to).is_some(), "{} unknown", r.to);
        assert!((1..=6).contains(&r.length));
        if let Some(t) = r.twin {
            let twin = map::route_by_id(t).unwrap();
            assert_eq!(twin.twin, Some(r.id));
            assert!((twin.from == r.from && twin.to == r.to) || (twin.from == r.to && twin.to == r.from));
        }
    }
    for c in CITIES {
        assert!(map::shortest_length("northgate", c.id).is_some(), "{} unreachable", c.id);
    }
    let tickets = map::tickets();
    assert!(tickets.len() >= 30);
    assert!(tickets.iter().all(|t| t.points >= 5));
}

#[test]
fn opening_deal_and_ticket_rules() {
    let mut g = table(3);
    assert!(g.apply("p1", Action::Start).is_err(), "only the host starts");
    g.apply("p0", Action::Start).unwrap();
    assert_eq!(g.stage, Stage::Playing);
    assert_eq!(g.face_up.len(), 5);
    for p in &g.players {
        assert_eq!(p.hand.len(), 4);
        assert_eq!(p.ticket_offer.len(), 3);
    }
    let first = g.players[0].id.clone();
    let one = vec![g.player(&first).unwrap().ticket_offer[0].id];
    assert!(g.apply(&first, Action::KeepTickets { keep: one }).is_err(), "must keep two at the start");
    let two: Vec<u16> = g.player(&first).unwrap().ticket_offer.iter().take(2).map(|t| t.id).collect();
    g.apply(&first, Action::KeepTickets { keep: two }).unwrap();
    assert_eq!(g.player(&first).unwrap().tickets.len(), 2);
    assert_eq!(g.player(&first).unwrap().ticket_offer.len(), 0);
}

#[test]
fn drawing_two_cards_ends_the_turn_and_a_face_up_locomotive_is_a_whole_turn() {
    let mut g = started(2);
    let a = g.players[0].id.clone();
    let b = g.players[1].id.clone();
    assert!(g.apply(&b, Action::DrawCard { face_up: None }).is_err(), "not b's turn");
    g.apply(&a, Action::DrawCard { face_up: None }).unwrap();
    assert_eq!(g.phase, TurnPhase::DrewOne);
    assert!(g.apply(&a, Action::DrawTickets).is_err());
    g.apply(&a, Action::DrawCard { face_up: None }).unwrap();
    assert!(g.is_active(&b));
    assert_eq!(g.player(&a).unwrap().hand.len(), 6);

    g.face_up[0] = Card::Locomotive;
    g.apply(&b, Action::DrawCard { face_up: Some(0) }).unwrap();
    assert!(g.is_active(&a), "a face-up locomotive uses the turn");
    g.face_up[0] = Card::Locomotive;
    g.apply(&a, Action::DrawCard { face_up: None }).unwrap();
    assert!(g.apply(&a, Action::DrawCard { face_up: Some(0) }).is_err(), "no locomotive as a second draw");
}

#[test]
fn claiming_spends_cards_trains_and_scores() {
    let mut g = started(2);
    let a = g.players[0].id.clone();
    let route = map::route_by_id(1).unwrap(); // northgate-frostmere, 4 blue
    {
        let p = g.players.iter_mut().find(|p| p.id == a).unwrap();
        p.hand = vec![Card::Blue, Card::Blue, Card::Blue, Card::Locomotive, Card::Red];
    }
    assert!(g.route_is_claimable(&a, route));
    g.apply(&a, Action::ClaimRoute { route: 1, color: Some(Card::Blue) }).unwrap();
    let p = g.player(&a).unwrap();
    assert_eq!(p.hand, vec![Card::Red]);
    assert_eq!(p.trains, 41);
    assert_eq!(p.score, 7);
    assert_eq!(g.owners.get(&1).map(String::as_str), Some(a.as_str()));
    assert!(!g.route_is_claimable(&a, route));
    assert!(!g.is_active(&a));
}

#[test]
fn twin_routes_stay_closed_at_small_tables() {
    let mut g = started(2);
    let a = g.players[0].id.clone();
    let b = g.players[1].id.clone();
    for id in [&a, &b] {
        g.players.iter_mut().find(|p| &p.id == id).unwrap().hand = vec![Card::Locomotive; 4];
    }
    g.apply(&a, Action::ClaimRoute { route: 3, color: None }).unwrap();
    assert!(!g.route_is_claimable(&b, map::route_by_id(4).unwrap()), "twin closed with two players");
    assert!(!g.route_is_claimable(&a, map::route_by_id(4).unwrap()));
}

#[test]
fn game_ends_one_round_after_someone_runs_low_on_trains() {
    let mut g = started(3);
    let ids: Vec<String> = g.players.iter().map(|p| p.id.clone()).collect();
    g.players[0].trains = FINAL_ROUND_TRAINS + 1;
    g.players[0].hand = vec![Card::Locomotive; 2];
    // Active player claims a 1-length? None exist; use a 2-length grey route (18) with 2 locomotives.
    g.players[0].trains = 4;
    g.apply(&ids[0], Action::ClaimRoute { route: 18, color: None }).unwrap();
    assert_eq!(g.player(&ids[0]).unwrap().trains, 2);
    assert!(g.final_round_by.is_some());
    assert_eq!(g.stage, Stage::Playing);
    g.apply(&ids[1], Action::DrawCard { face_up: None }).unwrap();
    g.apply(&ids[1], Action::DrawCard { face_up: None }).unwrap();
    assert_eq!(g.stage, Stage::Playing);
    g.apply(&ids[2], Action::DrawCard { face_up: None }).unwrap();
    g.apply(&ids[2], Action::DrawCard { face_up: None }).unwrap();
    assert_eq!(g.stage, Stage::Finished, "everyone else took one more turn");
    assert!(g.winner_id.is_some());
}

#[test]
fn tickets_score_when_connected_and_cost_when_not() {
    let mut g = started(2);
    let a = g.players[0].id.clone();
    let b = g.players[1].id.clone();
    let ticket = map::tickets().into_iter().find(|t| t.from == "northgate" && t.to == "saltmarsh").unwrap();
    g.players.iter_mut().find(|p| p.id == a).unwrap().tickets = vec![ticket.clone()];
    g.players.iter_mut().find(|p| p.id == b).unwrap().tickets = vec![ticket.clone()];
    // northgate -> frostmere (1) -> westwick (7) -> saltmarsh (18)
    for r in [1u16, 7, 18] {
        g.owners.insert(r, a.clone());
        g.players.iter_mut().find(|p| p.id == a).unwrap().claimed.push(r);
    }
    g.players.iter_mut().find(|p| p.id == a).unwrap().trains = 0;
    g.players[0].hand = vec![Card::Locomotive; 2];
    let before_a = g.player(&a).unwrap().score;
    let before_b = g.player(&b).unwrap().score;
    g.players[0].trains = 1; // already below the threshold: trigger on the next end of turn
    g.apply(&a, Action::DrawCard { face_up: None }).unwrap();
    g.apply(&a, Action::DrawCard { face_up: None }).unwrap();
    g.apply(&b, Action::DrawCard { face_up: None }).unwrap();
    g.apply(&b, Action::DrawCard { face_up: None }).unwrap();
    assert_eq!(g.stage, Stage::Finished);
    let after_a = g.player(&a).unwrap().score;
    let after_b = g.player(&b).unwrap().score;
    assert!(after_a - before_a >= ticket.points as i32, "connected ticket pays");
    assert_eq!(after_b - before_b, -(ticket.points as i32), "unfinished ticket costs");
    assert_eq!(g.longest_path.as_ref().map(|(id, _)| id.as_str()), Some(a.as_str()));
}

#[test]
fn ai_plays_a_full_game_against_itself() {
    let mut g = table(1);
    g.apply("p0", Action::AddAi).unwrap();
    g.apply("p0", Action::AddAi).unwrap();
    // Let p0 be an AI too, so the table runs itself.
    g.players.iter_mut().find(|p| p.id == "p0").unwrap().is_ai = true;
    g.apply("p0", Action::Start).unwrap();
    let mut steps = 0;
    while g.stage == Stage::Playing && steps < 5000 {
        steps += 1;
        let ids: Vec<String> = g.players.iter().map(|p| p.id.clone()).collect();
        let mut moved = false;
        for id in ids {
            if let Some(action) = ai::next_action(&g, &id) {
                g.apply(&id, action).unwrap();
                moved = true;
                break;
            }
        }
        assert!(moved, "AI has no move at step {steps} phase {:?} active {:?}", g.phase, g.active_player().map(|p| &p.id));
    }
    assert_eq!(g.stage, Stage::Finished, "game finished within {steps} steps");
    assert!(g.players.iter().any(|p| !p.claimed.is_empty()));
}
