use super::*;

#[test]
fn raids_never_reset_another_players_turn_idle_timer_or_take_their_main_action() {
    let mut room=Room::new("raid-idle",71);
    room.connect("host","Host","").unwrap();
    room.connect("friend","Friend","").unwrap();
    room.start_party("host").unwrap();room.finish_opening_for_test();
    let active=room.seat_of("host").unwrap();
    let actor=room.seat_of("friend").unwrap();
    room.board.as_mut().unwrap().dev_give(actor,Item::SlashRush);
    let main=room.board.as_ref().unwrap().step().clone();
    room.turn_idle=TURN_LIMIT_TICKS-100;
    let idle=room.turn_idle;
    room.use_item("friend",Item::SlashRush,None,None).unwrap();
    room.raid_input("friend",0.0,0.0,0.0).unwrap();
    room.raid_slash("friend").unwrap();
    assert_eq!(room.turn_idle,idle);
    assert_eq!(room.board.as_ref().unwrap().turn_seat(),active);
    assert_eq!(room.board.as_ref().unwrap().step(),&main);
    room.step();
    assert_eq!(room.turn_idle,idle+1,"the active player's clock still runs normally");
    room.disconnect("friend");
    assert!(!room.board.as_ref().unwrap().has_raids());
    assert_eq!(room.board.as_ref().unwrap().turn_seat(),active);
}

#[test]
fn stale_ephemeral_input_and_attack_packets_are_quiet_without_restarting_actions_or_idle_time() {
    let mut room=Room::new("stale-raid",72);
    room.connect("host","Host","").unwrap();room.connect("friend","Friend","").unwrap();
    room.turn_idle=91;
    assert!(room.raid_input("friend",1.0,0.0,0.0).is_ok());
    assert!(room.raid_slash("friend").is_ok());
    assert!(room.position_input("host",1.0,0.0).is_ok());
    assert!(room.brace("friend",false).is_ok());
    assert_eq!(room.turn_idle,91);
    room.dev_tools=true;room.start_sandbox("host").unwrap();room.finish_candy_offer_for_test();
    let actor=room.seat_of("friend").unwrap();
    room.board.as_mut().unwrap().dev_give(actor,Item::SlashRush);
    room.use_item("friend",Item::SlashRush,None,None).unwrap();
    room.turn_idle=123;
    room.raid_slash("friend").unwrap();
    room.raid_slash("friend").unwrap();
    assert_eq!(room.board.as_ref().unwrap().view().log.iter().filter(|entry|entry["kind"]=="raid_slash").count(),1);
    room.board.as_mut().unwrap().end_raid(actor,"test");
    for _ in 0..12 {
        room.raid_input("friend",1.0,0.0,0.0).unwrap();room.raid_slash("friend").unwrap();
        room.position_input("host",1.0,0.0).unwrap();room.brace("friend",false).unwrap();
    }
    assert_eq!(room.turn_idle,123);
    assert!(!room.board.as_ref().unwrap().has_raids());
    assert_eq!(room.board.as_ref().unwrap().view().tokens.iter().find(|token|token.seat==actor).unwrap().stats.items_used,1);
}

#[test]
fn crawler_sandbox_preview_remains_controllable_and_does_not_reset_an_opponents_clock() {
    let mut room=Room::new("crawler-control-preview",73);
    room.connect("host","Host","").unwrap();room.choose_map("host",Map::Island).unwrap();
    room.dev_tools=true;room.start_sandbox("host").unwrap();room.finish_candy_offer_for_test();
    room.dev("host",DevAction::Crawler).unwrap();
    let host=room.seat_of("host").unwrap();let active=room.board.as_ref().unwrap().turn_seat();
    assert_ne!(host,active);assert_eq!(*room.board.as_ref().unwrap().view().order.last().unwrap(),host);
    room.turn_idle=777;room.crawler_input("host",1.,0.).unwrap();assert_eq!(room.turn_idle,777);
    for _ in 0..120 {room.step();}
    assert_eq!(room.board.as_ref().unwrap().turn_seat(),active,"preview bot waits while the host tries zombie control");
    assert_eq!(room.board.as_ref().unwrap().crawler_view().controller,Some(host));
    let before=room.board.as_ref().unwrap().crawler_view();room.step();let after=room.board.as_ref().unwrap().crawler_view();
    assert_eq!((before.x,before.z),(after.x,after.z),"stale input stops, not automatic pursuit");
    assert_eq!(room.parties,0);room.disconnect("host");assert_eq!(room.board.as_ref().unwrap().crawler_view().controller,None);
}

#[test]
fn candy_van_summon_is_a_host_only_sandbox_tool_and_board_time_expires_the_visit() {
    let mut room = Room::new("candy-van-preview", 74);
    room.connect("host", "Host", "").unwrap();
    room.connect("friend", "Friend", "").unwrap();
    assert!(matches!(room.dev("host", DevAction::CandyVan), Err(RoomError::NoDevTools)));
    room.dev_tools = true;
    room.start_sandbox("host").unwrap();room.finish_candy_offer_for_test();
    assert!(matches!(room.dev("friend", DevAction::CandyVan), Err(RoomError::NotHost)));
    room.dev("host", DevAction::Crawler).unwrap(); // Hold the bot turn while the clock runs.
    room.dev("host", DevAction::CandyVan).unwrap();
    assert_eq!(room.board.as_ref().unwrap().candy_van_view().unwrap().remaining, 5.);
    for _ in 0..(6 * TICK_HZ) { room.step(); }
    assert!(room.board.as_ref().unwrap().candy_van_view().is_none());
    assert!(room.board.as_ref().unwrap().view().log.iter().any(|event| event["kind"] == "candy_van_left"));
    room.dev("host", DevAction::CandyVan).unwrap();
    assert_eq!(room.board.as_ref().unwrap().candy_van_view().unwrap().remaining, 5.);
    assert_eq!(room.parties, 0);
}
