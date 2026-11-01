use super::*;
use serde_json::{Value, json};
fn lobby() -> Room {
    let mut r = Room::new("candy-pause", 37);
    r.connect("host", "Host", "").unwrap();
    r.connect("friend", "Friend", "").unwrap();
    r
}
fn sandbox() -> Room {
    let mut r = lobby();
    r.dev_tools = true;
    r.start_sandbox("host").unwrap();
    for _ in 0..TICK_HZ * 6 {
        r.step();
    }
    assert!(!r.board_paused());
    r
}
fn stable_board(r: &Room) -> Value {
    let view = serde_json::to_value(r.board.as_ref().unwrap().view()).unwrap();
    let mut keep = serde_json::Map::new();
    for key in [
        "round",
        "turn",
        "turn_seat",
        "order",
        "tokens",
        "walk",
        "vacuum",
        "positioning",
        "raids",
        "driving",
        "traffic",
        "crawler",
        "pedestrians",
    ] {
        keep.insert(key.into(), view[key].clone());
    }
    Value::Object(keep)
}
#[test]
fn offer_and_visit_freeze_all_actors_fuses_turn_timers_and_bots_then_restore_exactly() {
    let mut r = sandbox();
    r.dev("host", DevAction::Give { item: Item::Shield })
        .unwrap();
    r.use_item("host", Item::Shield, None, None).unwrap();
    r.fuse_ticks = 19;
    r.turn_idle = 23;
    r.bot_think = 17;
    r.turn_done = 11;
    let before = stable_board(&r);
    r.dev("host", DevAction::CandyVan).unwrap();
    r.join_candy_visit("friend").unwrap();
    assert!(r.board_paused());
    assert_eq!(r.check("host", 0.5), Err(RoomError::WrongPhase));
    r.use_item("friend", Item::DriveBy, None, None).unwrap();
    r.face("host", 2.).unwrap();
    r.crawler_input("friend", 1., 0.).unwrap();
    r.set_input("friend", Stick { x: 1., z: 0. }, 1., 0., false, false);
    for _ in 0..TICK_HZ * 6 {
        r.step();
    }
    assert_eq!(
        r.board.as_ref().unwrap().candy_van_view().unwrap().phase,
        crate::board::CandyVanPhase::Visit
    );
    assert_eq!(stable_board(&r), before);
    let run = r.board.as_ref().unwrap().candy_runs_view()[0].clone();
    r.candy_visit_input("friend", 1., 0.).unwrap();
    for _ in 0..30 {
        r.step();
    }
    let moved = r.board.as_ref().unwrap().candy_runs_view()[0].clone();
    assert!((moved.x - run.x).hypot(moved.z - run.z) <= 8. * 0.35 + 0.2);
    let mut ticks = 0;
    while r.board_paused() && ticks < 1500 {
        r.step();
        ticks += 1;
    }
    assert!(!r.board_paused());
    assert_eq!(stable_board(&r), before);
    assert_eq!(
        (r.fuse_ticks, r.turn_idle, r.bot_think, r.turn_done),
        (19, 23, 17, 11)
    );
    r.step();
    assert_eq!(r.fuse_ticks, 20);
}
#[test]
fn no_one_joining_costs_only_five_seconds_and_the_board_resumes() {
    let mut r = sandbox();
    let before = stable_board(&r);
    r.dev("host", DevAction::CandyVan).unwrap();
    let mut ticks = 0;
    while r.board_paused() && ticks < 400 {
        r.step();
        ticks += 1;
    }
    assert!((TICK_HZ * 5..=TICK_HZ * 5 + 2).contains(&ticks));
    assert_eq!(stable_board(&r), before);
    assert!(r.board.as_ref().unwrap().candy_runs_view().is_empty());
}
#[test]
fn off_turn_visitor_can_buy_and_personalized_views_do_not_show_others_buy_controls() {
    let mut r = sandbox();
    let seat = r.seat_of("friend").unwrap();
    let space = r
        .board
        .as_ref()
        .unwrap()
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == seat)
        .unwrap()
        .space;
    r.board.as_mut().unwrap().place_for_test(seat, space, 60);
    r.dev("host", DevAction::CandyVan).unwrap();
    assert_eq!(r.join_candy_visit("unknown"), Err(RoomError::WrongPhase));
    r.join_candy_visit("friend").unwrap();
    for _ in 0..TICK_HZ * 5 + 2 {
        r.step();
    }
    let mut bought = false;
    for _ in 0..TICK_HZ * 15 {
        let view = r.view_for(Some(seat)).board.unwrap();
        let Some(van) = view.candy_van else {
            break;
        };
        let Some(run) = view.candy_runs.iter().find(|run| run.seat == seat) else {
            break;
        };
        if van.can_buy {
            r.buy_candy("friend").unwrap();
            r.buy_candy("friend").unwrap();
            bought = true;
            break;
        }
        let delta = (van.x - run.x, van.z - run.z);
        let length = delta.0.hypot(delta.1).max(1.);
        r.candy_visit_input("friend", delta.0 / length, delta.1 / length)
            .unwrap();
        r.step();
    }
    assert!(
        bought,
        "sprint can reach van from the randomized city spawn"
    );
    let board = r.board.as_ref().unwrap();
    let token = board
        .view()
        .tokens
        .into_iter()
        .find(|t| t.seat == seat)
        .unwrap();
    assert_eq!(token.prizes, 1);
    assert_eq!(token.purse, Some(60 - crate::board::PRIZE_COST));
    assert!(
        !r.view_for(Some(r.seat_of("host").unwrap()))
            .board
            .unwrap()
            .candy_van
            .unwrap()
            .can_buy
    );
}
#[test]
fn sandbox_exit_stays_available_while_all_other_dev_actions_are_paused() {
    let mut r = sandbox();
    r.dev("host", DevAction::CandyVan).unwrap();
    assert_eq!(
        r.dev("host", DevAction::Coins { coins: 20 }),
        Err(RoomError::WrongPhase)
    );
    r.dev("host", DevAction::Exit).unwrap();
    assert_eq!(r.phase, Phase::Lobby);
    assert!(!r.sandbox);
}
#[test]
fn candy_commands_have_explicit_protocol_and_paused_action_allowlist() {
    use crate::protocol::ClientMsg;
    assert!(matches!(
        serde_json::from_value::<ClientMsg>(json!({"type":"join_candy_visit"})).unwrap(),
        ClientMsg::JoinCandyVisit
    ));
    assert!(matches!(
        serde_json::from_value::<ClientMsg>(json!({"type":"candy_visit_input","x":1,"z":0}))
            .unwrap(),
        ClientMsg::CandyVisitInput { .. }
    ));
    assert!(ClientMsg::JoinCandyVisit.allowed_while_candy_paused());
    assert!(ClientMsg::BuyCandy.allowed_while_candy_paused());
    assert!(
        ClientMsg::Dev {
            tool: DevAction::Exit
        }
        .allowed_while_candy_paused()
    );
    assert!(!ClientMsg::Roll.allowed_while_candy_paused());
    assert!(!ClientMsg::RaidFire { on: true, yaw: 0. }.allowed_while_candy_paused());
}
#[test]
fn practice_lasts_thirty_seconds_unless_every_human_presses_go() {
    let mut r = lobby();
    for member in &mut r.members {
        member.in_round = true;
    }
    r.start_practice();
    assert!(matches!(r.phase, Phase::Practice { seconds: 30. }));
    for _ in 0..TICK_HZ * 29 {
        r.step();
    }
    assert!(matches!(r.phase, Phase::Practice { .. }));
    for _ in 0..TICK_HZ {
        r.step();
    }
    assert!(matches!(r.phase, Phase::Countdown { .. }));
    let mut r = lobby();
    for member in &mut r.members {
        member.in_round = true;
    }
    r.start_practice();
    r.ready("host").unwrap();
    r.step();
    assert!(matches!(r.phase, Phase::Practice { .. }));
    r.ready("friend").unwrap();
    r.step();
    assert!(matches!(r.phase, Phase::Countdown { .. }));
}

#[test]
fn disconnect_during_visit_defers_actor_cleanup_until_original_board_resumes() {
    let mut r = sandbox();
    r.board.as_mut().unwrap().dev_give(1, Item::SlashRush);
    r.use_item("friend", Item::SlashRush, None, None).unwrap();
    assert!(r.board.as_ref().unwrap().has_raid_for(1));
    r.dev("host", DevAction::CandyVan).unwrap();
    r.join_candy_visit("host").unwrap();
    let before = stable_board(&r);
    r.disconnect("friend");
    for _ in 0..TICK_HZ * 6 {
        r.step();
    }
    assert_eq!(stable_board(&r), before);
    while r.board_paused() {
        r.step();
    }
    assert!(r.board.as_ref().unwrap().has_raid_for(1));
    r.step();
    assert!(!r.board.as_ref().unwrap().has_raid_for(1));
}
