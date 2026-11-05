use super::*;
use crate::board::{MINIGAME_COINS,PRIZE_COST,RoundEvent};

fn lobby()->Room {
    let mut room=Room::new("chosen-rounds",33);
    room.connect("host","Host","").unwrap();
    room.connect("friend","Friend","").unwrap();
    room
}

#[test]
fn round_settings_default_to_ten_and_host_choice_flows_through_preview_and_entire_board() {
    for rounds in [10,15,20,25,30] {
        let mut room=lobby();
        assert_eq!(room.view().rounds,10);
        assert_eq!(room.view().preview_board.unwrap().rounds,10);
        room.choose_rounds("host",rounds).unwrap();
        assert_eq!(room.view().rounds,rounds);
        assert_eq!(room.view().preview_board.unwrap().rounds,u32::from(rounds));
        room.start_party("host").unwrap();
        let board=room.board.as_mut().unwrap();
        assert_eq!(board.rounds,u32::from(rounds));
        for round in 1..=u32::from(rounds) {
            assert_eq!(board.round,round);
            assert_eq!(board.view().event,None);
            assert_eq!(board.prize_cost(0),PRIZE_COST);
            assert_eq!(board.open_prizes().len(),1);
            let continues=board.finish_round(&[vec![0],vec![1]]);
            assert_eq!(continues,round<u32::from(rounds));
            let view=board.view();
            let reward=view.log.iter().rev().find(|entry|entry["kind"]=="minigame"&&entry["seat"]==0).unwrap();
            assert_eq!(reward["coins"],MINIGAME_COINS[0],"final bout pays the same as the others");
        }
    }
}

#[test]
fn round_settings_reject_non_host_invalid_sizes_and_changes_after_start() {
    let mut room=lobby();
    assert_eq!(room.choose_rounds("friend",15),Err(RoomError::NotHost));
    for rounds in [0,1,9,31,255] {
        assert_eq!(room.choose_rounds("host",rounds),Err(RoomError::InvalidRounds));
        assert_eq!(room.rounds,10);
    }
    room.choose_rounds("host",25).unwrap();
    room.start_party("host").unwrap();
    assert_eq!(room.choose_rounds("host",30),Err(RoomError::WrongPhase));
    assert_eq!(room.rounds,25);
    assert_eq!(room.board.as_ref().unwrap().rounds,25);
}

#[test]
fn round_settings_sandbox_keeps_its_independent_endless_length() {
    let mut room=lobby();
    room.choose_rounds("host",20).unwrap();
    room.dev_tools=true;
    room.start_sandbox("host").unwrap();
    assert_eq!(room.rounds,20);
    assert_eq!(room.board.as_ref().unwrap().rounds,SANDBOX_ROUNDS);
}
#[test]
fn native_lobby_host_can_open_sandbox_without_global_dev_tools() {
    let mut room = lobby();
    assert_eq!(room.check_start_sandbox("host"), Err(RoomError::NoDevTools));
    room.enable_native_mode();
    assert_eq!(room.check_start_sandbox("friend"), Err(RoomError::NotHost));
    assert!(room.check_start_sandbox("host").is_ok());
    assert!(room.view_for(Some(0)).dev_tools);
    assert!(!room.view_for(Some(1)).dev_tools);
    room.start_sandbox("host").unwrap();
    assert!(room.sandbox);
    assert!(!room.dev_tools);
}

#[test]
fn round_restocking_clears_old_modifiers_but_keeps_map_hazards_and_new_pickups() {
    for map in Map::ALL {
        let mut board=Board::on(map,&[0,1],10,83);
        board.set_event_for_test_full(RoundEvent::Underdog);
        board.set_event_for_test_full(RoundEvent::Bounty);
        board.set_event_for_test_full(RoundEvent::Plague);
        let before=board.view();
        assert!(board.finish_round(&[vec![0],vec![1]]));
        let after=board.view();
        assert_eq!(after.event,None);assert_eq!(after.underdog,None);assert_eq!(after.bounty,None);assert_eq!(after.plague,None);
        assert!(!after.crates.is_empty());assert!(!after.air_coins.is_empty());
        assert!(after.air_coins.iter().all(|coin|coin.collected_by.is_none() && coin.coins==1));
        assert!(after.air_coins.iter().any(|coin|coin.available));
        if before.tide.is_some() {assert_ne!(before.tide,after.tide);}
        assert!(after.storm.is_some(),"the storm still advances on {map:?}");
    }
}
