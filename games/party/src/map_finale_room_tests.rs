use super::*;
use crate::board::{MINIGAME_COINS,SpaceKind};

fn party()->Room {
    let mut room=Room::new("finale-lifecycle",91);
    room.connect("host","Host","").unwrap();
    for _ in 0..3 {room.add_bot("host").unwrap();}
    room.start_party("host").unwrap();room.finish_opening_for_test();room
}
fn finish_last_turn(room:&mut Room,round:u32) {
    let board=room.board.as_mut().unwrap();board.round=round;
    let view=board.view();let last=*view.order.last().unwrap();
    board.turn_to_for_test(last);
    let coin=view.spaces.iter().find(|space|space.kind==SpaceKind::Coin).unwrap().id;
    board.place_for_test(last,coin,30);board.land_for_test(last);
    let user=room.members.iter().find(|member|member.seat==last).unwrap().user_id.clone();
    room.end_turn(&user).unwrap();
}
fn all_ai(room:&mut Room) {for member in &mut room.members {member.is_bot=true;}}

#[test]
fn map_finale_replaces_only_the_last_random_bout_and_pays_normal_multi_place_rewards_once() {
    let mut earlier=party();finish_last_turn(&mut earlier,9);
    assert!(matches!(earlier.phase,Phase::Vote{..}));
    assert!(earlier.board.as_ref().unwrap().view().finale.is_none());
    let mut room=party();finish_last_turn(&mut room,10);
    assert_eq!(room.phase,Phase::MapFinale);
    assert_eq!(room.view().minigame_name,"City Sprint");
    let before:Vec<_>=room.board.as_ref().unwrap().view().tokens.into_iter().map(|token|(token.seat,token.purse.unwrap())).collect();
    all_ai(&mut room);
    for _ in 0..4800 {room.step();if matches!(room.phase,Phase::Results{..}){break;}}
    let Phase::Results{placements,..}=room.phase.clone() else {panic!("finale reaches Results")};
    for member in &room.members {assert_eq!(member.wins,usize::from(placements[0].contains(&member.seat)) as u32);}
    let finished=room.view().board.unwrap().finale.unwrap();
    assert!(finished.racers.iter().all(|racer|racer.finished));
    assert_eq!(room.view().minigame_name,"City Sprint");
    for _ in 0..RESULTS_TICKS {room.step();}
    assert!(matches!(room.phase,Phase::GameOver{..}));
    let board=room.board.as_ref().unwrap().view();assert!(board.finale.is_none());
    let mut place=0;
    for group in placements {
        let reward=MINIGAME_COINS.get(place).copied().unwrap_or(0);
        for seat in &group {
            let old=before.iter().find(|(old,_)|old==seat).unwrap().1;
            let new=board.tokens.iter().find(|token|token.seat==*seat).unwrap().purse.unwrap();
            assert_eq!(new-old,reward);
        }
        place+=group.len();
    }
}

#[test]
fn map_finale_sandbox_preview_is_host_only_returns_to_sandbox_and_does_not_award_a_match() {
    let mut room=Room::new("finale-preview",92);
    room.connect("host","Host","").unwrap();room.connect("friend","Friend","").unwrap();
    room.dev_tools=true;room.start_sandbox("host").unwrap();room.finish_candy_offer_for_test();
    assert_eq!(room.dev("friend",DevAction::Finale),Err(RoomError::NotHost));
    let before:Vec<_>=room.board.as_ref().unwrap().view().tokens.into_iter().map(|token|(token.seat,token.purse,token.prizes,token.stats.bouts_won)).collect();
    room.dev("host",DevAction::Finale).unwrap();
    assert_eq!(room.dev("host",DevAction::Finale),Err(RoomError::WrongPhase));
    all_ai(&mut room);
    let mut saw_results=false;
    for _ in 0..5400 {
        room.step();saw_results|=matches!(room.phase,Phase::Results{..});
        if saw_results&&room.phase==Phase::Board {break;}
    }
    assert!(saw_results);assert_eq!(room.phase,Phase::Board);assert!(room.sandbox);
    assert!(room.board.as_ref().unwrap().view().finale.is_none());
    assert_eq!(room.parties,0);assert!(room.last_bout.is_none());
    let after:Vec<_>=room.board.as_ref().unwrap().view().tokens.into_iter().map(|token|(token.seat,token.purse,token.prizes,token.stats.bouts_won)).collect();
    assert_eq!(after,before,"preview does not run normal bout payout or finale awards");
}

#[test]
fn map_finale_own_input_expires_and_late_joiners_cannot_gain_a_race_entry() {
    let mut room=Room::new("finale-input",93);
    room.connect("host","Host","").unwrap();room.connect("friend","Friend","").unwrap();
    room.dev_tools=true;room.start_sandbox("host").unwrap();room.finish_candy_offer_for_test();room.dev("host",DevAction::Finale).unwrap();
    let original=room.view().board.unwrap().finale.unwrap();
    room.set_input("host",Stick{x:1.,z:0.},0.,0.,false,false);
    for _ in 0..185 {room.step();}
    let ready=room.view().board.unwrap().finale.unwrap();
    assert_eq!((ready.racers[0].x,ready.racers[0].z),(original.racers[0].x,original.racers[0].z),"a stick held before start expires during countdown");
    room.set_input("host",Stick{x:99.,z:0.},0.,0.,false,false);
    for _ in 0..60 {room.step();}
    let moved=room.view().board.unwrap().finale.unwrap();
    assert!((moved.racers[0].x-ready.racers[0].x).hypot(moved.racers[0].z-ready.racers[0].z)<=2.25);
    let position=(moved.racers[0].x,moved.racers[0].z);
    for _ in 0..30 {room.step();}
    let stopped=room.view().board.unwrap().finale.unwrap();
    assert_eq!((stopped.racers[0].x,stopped.racers[0].z),position);
    room.connect("late","Late","").unwrap();room.set_input("late",Stick{x:1.,z:1.},0.,0.,false,false);
    room.disconnect("friend");room.step();
    let final_view=room.view().board.unwrap().finale.unwrap();
    assert_eq!(final_view.racers.len(),original.racers.len());
    assert!(final_view.racers.iter().find(|racer|racer.seat==1).unwrap().dnf);
}
