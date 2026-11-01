use super::*;
const DT:f32=1.0/60.0;
const ALL:&[u8]=&[0,1,2,3];

fn ready()->Board {
    let mut board=Board::new(ALL,6,9);
    board.timed_items=true;
    board.token(1).unwrap().items=vec![Item::SlashRush,Item::SlashRush];
    board.air_coins.clear();board.crates.clear();
    board
}
fn face_target(board:&mut Board,actor:u8,target:u8) {
    let at=board.raid_target_spot(target);
    let raid=board.raids.get(&actor).unwrap();
    board.raid_input(actor,0.0,0.0,(at.0-raid.x).atan2(at.1-raid.z)).unwrap();
}
fn cool(board:&mut Board) {for _ in 0..37 {board.raids_tick(DT,ALL);}}

#[test]
fn raids_are_owned_off_turn_actors_and_never_replace_main_position_or_step() {
    let mut board=ready();
    board.token(0).unwrap().items=vec![Item::Bomb,Item::SlashRush];
    assert_eq!(board.start_raid(9),Err(BoardError::UnknownTarget));
    assert_eq!(board.start_raid(2),Err(BoardError::NoSuchItem));
    board.use_item(0,Item::Bomb,None).unwrap();
    assert_eq!(board.start_raid(0),Err(BoardError::WrongStep),"own-turn activation is limited to Items/Done");
    let main=board.position_view().unwrap();
    let step=board.step().clone();
    board.use_item(1,Item::SlashRush,None).unwrap();
    assert_eq!(board.turn_seat(),0);
    assert_eq!(board.step(),&step);
    assert_eq!(board.position_view().unwrap().seat,main.seat);
    assert_eq!(board.position_view().unwrap().remaining,main.remaining);
    assert_eq!(board.tokens[1].items,vec![Item::SlashRush]);
    assert_eq!(board.tokens[1].stats.items_used,1);
    assert_eq!(board.start_raid(1),Err(BoardError::WrongStep));
    board.raid_input(1,1.0,0.0,0.0).unwrap();
    board.raids_tick(DT,ALL);
    assert_eq!(board.position_view().unwrap().remaining,main.remaining,"raids do not tick or pause the main action themselves");
    assert_eq!(board.tokens[1].space,0);
    assert_eq!(board.raid_input(2,1.0,0.0,0.0),Err(BoardError::WrongStep));
    assert_eq!(board.raid_slash(2,ALL),Err(BoardError::WrongStep));
}

#[test]
fn raids_normalize_and_bound_each_actor_and_expire_at_ten_seconds_without_tile_rewards() {
    for map in Map::ALL {
        let mut board=Board::on(map,ALL,6,9);
        board.token(1).unwrap().items=vec![Item::SlashRush];
        let before=board.tokens[1].clone();
        board.start_raid(1).unwrap();
        assert_eq!(board.raid_input(1,f32::NAN,0.0,0.0),Err(BoardError::OutOfReach));
        assert_eq!(board.raid_input(1,0.0,0.0,f32::INFINITY),Err(BoardError::OutOfReach));
        for tick in 0..140 {
            let angle=tick as f32*0.1;
            board.raid_input(1,angle.cos()*100.0,angle.sin()*100.0,angle).unwrap();
            let old=board.raid_views()[0].clone();
            board.raids_tick(0.05,ALL);
            let raid=board.raid_views()[0].clone();
            assert!((raid.x-old.x).hypot(raid.z-old.z)<=positioning::POSITION_SPEED*0.05+0.0002);
            assert!((raid.x-raid.home_x).hypot(raid.z-raid.home_z)<=positioning::POSITION_RADIUS+0.0002);
        }
        for _ in 0..10 {board.raids_tick(0.05,ALL);}
        let old=board.raid_views()[0].clone();
        board.raids_tick(0.05,ALL);
        let still=board.raid_views()[0].clone();
        assert_eq!((old.x,old.z),(still.x,still.z),"lost input stops movement");
        for _ in 0..50 {board.raids_tick(0.05,ALL);}
        assert!(!board.has_raids());
        assert_eq!((board.tokens[1].space,board.tokens[1].coins,board.tokens[1].prizes),(before.space,before.coins,before.prizes));
        assert_eq!(board.step(),&TurnStep::Items);
        assert_eq!(board.turn_seat(),0);
    }
}

#[test]
fn raids_slash_cooldown_one_hit_per_victim_shields_resting_and_disconnect_are_authoritative() {
    let mut board=ready();
    board.tokens[0].shield=true;
    board.tokens[2].resting=1;
    board.start_raid(1).unwrap();
    face_target(&mut board,1,0);
    board.raid_slash(1,&[0,1,2]).unwrap();
    assert!(!board.tokens[0].shield);
    assert_eq!(board.tokens[0].health,MAX_HEALTH);
    assert_eq!(board.tokens[0].coins,START_COINS);
    assert_eq!(board.tokens[2].health,MAX_HEALTH);
    assert_eq!(board.tokens[3].health,MAX_HEALTH);
    assert_eq!(board.raid_slash(1,ALL),Err(BoardError::WrongStep));
    cool(&mut board);
    face_target(&mut board,1,0);
    board.raid_slash(1,&[0,1,2]).unwrap();
    assert_eq!(board.tokens[0].health,MAX_HEALTH,"a shield consumes the single eligible hit, not just a cooldown");
    cool(&mut board);
    board.tokens[2].resting=0;
    face_target(&mut board,1,2);
    board.raid_slash(1,&[0,1,2]).unwrap();
    assert_eq!(board.tokens[2].health,MAX_HEALTH-1);
    assert_eq!(board.tokens[2].coins,START_COINS-3);
    cool(&mut board);
    face_target(&mut board,1,2);
    board.raid_slash(1,&[0,1,2]).unwrap();
    assert_eq!(board.tokens[2].health,MAX_HEALTH-1,"one hit per activation");
    assert!(board.log.iter().any(|entry|entry["kind"]=="raid_hit"&&entry["quality"]=="hit"&&entry["target"]==2));
}

#[test]
fn raids_return_early_when_their_turn_arrives_round_ends_or_actor_disconnects() {
    let mut board=ready();
    board.start_raid(1).unwrap();
    board.step=TurnStep::Done;
    board.end_turn();
    assert!(!board.has_raids());
    assert_eq!(board.turn_seat(),1);
    assert!(board.log.iter().any(|entry|entry["kind"]=="raid_end"&&entry["reason"]=="turn"));
    board.token(0).unwrap().items=vec![Item::SlashRush];
    board.start_raid(0).unwrap();
    board.raids_tick(DT,&[1,2,3]);
    assert!(!board.has_raids());
    board.token(0).unwrap().items=vec![Item::SlashRush];
    board.turn_index=3;
    board.start_raid(0).unwrap();
    board.step=TurnStep::Done;
    assert!(board.end_turn());
    assert!(!board.has_raids());
    assert!(board.log.iter().any(|entry|entry["kind"]=="raid_end"&&entry["reason"]=="round"));
}

#[test]
fn raids_do_not_overwrite_a_real_knockback_or_swap_with_the_old_home_space() {
    let mut board=ready();
    board.start_raid(1).unwrap();
    board.tokens[1].space=4;
    board.raids_tick(DT,ALL);
    assert!(!board.has_raids());
    assert_eq!(board.tokens[1].space,4);
    assert!(board.log.iter().any(|entry|entry["kind"]=="raid_end"&&entry["reason"]=="displaced"));
}

#[test]
fn raids_can_be_vacuumed_and_hit_at_actual_running_position_instead_of_logical_home() {
    let mut board=ready();
    board.place_for_test(1,3,30);
    board.place_for_test(2,40,10);board.place_for_test(3,45,10);
    board.start_raid(1).unwrap();
    let origin=board.spot_of(0);
    board.raids.get_mut(&1).unwrap().x=origin.0+1.5;
    board.raids.get_mut(&1).unwrap().z=origin.1;
    assert_eq!(board.rendered_spot_of(1),(origin.0+1.5,origin.1));
    board.suck(0).unwrap();
    board.aim_vacuum(0,std::f32::consts::FRAC_PI_2,true).unwrap();
    for _ in 0..120 {board.vacuum_tick(DT);}
    assert!(board.tokens[1].coins<30,"standing vacuum targets the running model");
    board.stow_vacuum(0).unwrap();
    board.token(0).unwrap().items=vec![Item::Grapple];
    board.use_item(0,Item::Grapple,None).unwrap();
    board.activate_item(0).unwrap();
    board.calm_for_test();
    board.lob(0,0.0,0.0,0.0).unwrap();
    assert!(board.log.iter().any(|entry|entry["kind"]=="item"&&entry["item"]=="grapple"&&entry["target"]==1));
    assert_eq!(board.tokens[1].space,3);
    assert!(board.has_raids());
}

#[test]
fn raids_ai_uses_real_off_turn_movement_and_slashes_without_progressing_the_main_turn() {
    let mut board=ready();
    board.token(1).unwrap().items=vec![Item::SlashRush];
    board.place_for_test(0,3,30);
    board.place_for_test(2,40,0);board.place_for_test(3,45,0);
    board.bot_raids_tick(&[1],ALL);
    assert!(board.has_raids());
    for _ in 0..610 {board.bot_raids_tick(&[1],ALL);board.raids_tick(DT,ALL);}
    assert!(!board.has_raids());
    assert_eq!(board.turn_seat(),0);
    assert_eq!(board.step(),&TurnStep::Items);
    assert!(board.log.iter().any(|entry|entry["kind"]=="raid_hit"&&entry["target"]==0&&entry["quality"]=="hit"));
    assert_eq!(board.tokens[0].health,MAX_HEALTH-1);
    assert_eq!(board.tokens[1].space,0);
    assert_eq!(board.tokens[1].stats.items_used,1);
}

#[test]
fn own_turn_slash_rush_hits_and_restores_exact_prior_step_without_an_extra_roll() {
    for after in [TurnStep::Items,TurnStep::Done] {
        let mut board=ready();board.token(0).unwrap().items=vec![Item::SlashRush];board.step=after.clone();
        board.use_item(0,Item::SlashRush,None).unwrap();
        assert_eq!(board.step(),&TurnStep::Raiding);assert_eq!(board.roll(0),Err(BoardError::WrongStep));
        face_target(&mut board,0,1);board.raid_slash(0,ALL).unwrap();
        assert_eq!(board.tokens[1].health,MAX_HEALTH-1);
        for _ in 0..601 {board.raids_tick(DT,ALL);}
        assert!(!board.has_raids());assert_eq!(board.step(),&after);assert_eq!(board.turn_seat(),0);
        if after==TurnStep::Done {assert_eq!(board.roll(0),Err(BoardError::WrongStep));}
        assert_eq!(board.tokens[0].stats.items_used,1);
    }
}

#[test]
fn free_raid_movement_can_pass_eight_units_but_never_advances_the_logical_tile() {
    let mut board=ready();board.start_raid(1).unwrap();let original=board.raid_views()[0].clone();
    for _ in 0..180 {board.raid_input(1,1.,0.,0.).unwrap();board.raids_tick(DT,ALL);}
    let moved=board.raid_views()[0].clone();
    assert!((moved.x-original.x).hypot(moved.z-original.z)>9.0);
    assert!(board.position_ground((moved.x,moved.z),moved.home_space));
    assert_eq!(board.tokens[1].space,0);
}

#[test]
fn drive_by_stays_on_foot_fires_ten_spaced_single_shots_and_counts_three_hits_per_heart() {
    let mut board=ready();board.token(0).unwrap().items=vec![Item::DriveBy];
    board.tokens[1].coins=50;board.tokens[1].health=MAX_HEALTH;
    board.place_for_test(2,40,0);board.place_for_test(3,45,0);
    board.use_item(0,Item::DriveBy,None).unwrap();assert_eq!(board.step(),&TurnStep::Raiding);
    let at=board.raid_target_spot(1);let from=board.rendered_spot_of(0);let yaw=(at.0-from.0).atan2(at.1-from.1);
    assert_eq!(board.raid_fire(0,true,f32::NAN),Err(BoardError::OutOfReach));
    for tick in 0..100 {
        board.raid_input(0,0.,0.,yaw).unwrap();board.raid_fire(0,true,yaw).unwrap();board.raids_tick(DT,ALL);
        if tick<48 {assert_eq!(board.tokens[1].health,MAX_HEALTH,"one or two bullets are not a heart");}
    }
    assert_eq!(board.tokens[1].health,MAX_HEALTH-1);assert_eq!(board.raid_views()[0].ammo,7);
    assert_eq!(board.tokens[1].coins,47);
    board.raid_fire(0,false,yaw).unwrap();let ammo=board.raid_views()[0].ammo;
    for _ in 0..60 {board.raid_input(0,0.,0.,yaw).unwrap();board.raids_tick(DT,ALL);}
    assert_eq!(board.raid_views()[0].ammo,ammo,"trigger release stops fire");
    for _ in 0..500 {
        if !board.has_raids(){break;}
        board.raid_input(0,0.,0.,yaw+std::f32::consts::PI).unwrap();board.raid_fire(0,true,yaw+std::f32::consts::PI).unwrap();board.raids_tick(DT,ALL);
    }
    assert!(!board.has_raids());assert_eq!(board.step(),&TurnStep::Items);
    let shots:Vec<_>=board.take_events().into_iter().filter(|entry|entry["kind"]=="item"&&entry["item"]=="drive_by").collect();
    assert_eq!(shots.len(),10);assert!(shots.iter().all(|entry|entry["shots"].as_array().unwrap().len()==1));
    assert_eq!(board.tokens[0].stats.items_used,1);
}

#[test]
fn raid_slashes_and_live_projectiles_target_the_rental_car_instead_of_its_home_tile() {
    let mut board=Board::on(Map::Frozen,ALL,10,75);board.timed_items=true;
    board.token(0).unwrap().items=vec![Item::DriveCar];
    board.token(1).unwrap().items=vec![Item::SlashRush];
    board.use_item(0,Item::DriveCar,None).unwrap();board.start_raid(1).unwrap();
    let home=board.actual_spot_of(0);
    let point=(home.0+20.,home.1);
    let driver=board.driver.as_mut().unwrap();driver.view.x=point.0;driver.view.z=point.1;
    let raid=board.raids.get_mut(&1).unwrap();raid.x=point.0-2.;raid.z=point.1;
    assert_eq!(board.raid_target_spot(0),point);
    let ray=board.ray_rival_eligible(1,(point.0-2.,point.1),(1.,0.),5.,Some(ALL));
    assert_eq!(ray,Some((0,point)));
    face_target(&mut board,1,0);board.raid_slash(1,ALL).unwrap();
    assert_eq!(board.tokens[0].health,MAX_HEALTH-1);
    assert!(matches!(board.step(),TurnStep::Driving),"an off-turn slash does not take over the driver's turn");
}
