use super::*;
const DT:f32=1.0/60.0;

fn clean(map:Map)->Board {
    let mut board=Board::on(map,&[0,1],6,31);
    board.timed_walk=true;
    board.air_coins.clear();board.crates.clear();board.loose.clear();board.flotsam.clear();board.lava_rocks.clear();
    board.coconuts.fill(0);board.snowmen.fill(false);board.storm=None;
    board.boxes_taken=board.layout.iter().filter(|space|space.kind==SpaceKind::Item).map(|space|space.id).collect();
    for token in &mut board.tokens {token.coins=0;}
    board
}

fn walk_near(board:&mut Board,point:(f32,f32)) {
    let from=board.layout.iter().filter(|space|space.link.is_none()&&!space.under).min_by(|a,b|(a.x-point.0).hypot(a.z-point.1).total_cmp(&(b.x-point.0).hypot(b.z-point.1))).unwrap().id;
    let mut at=from;
    let mut path=Vec::new();
    for _ in 0..12 {
        let next=board.layout[at as usize].next[0];
        if board.layout[at as usize].link.is_some() {break;}
        path.push(next);at=next;
    }
    assert!(path.len()>=3);
    // Production commits the logical destination before the walking animation.
    board.token(0).unwrap().space=at;
    let far=board.layout.iter().max_by(|a,b|(a.x-point.0).hypot(a.z-point.1).total_cmp(&(b.x-point.0).hypot(b.z-point.1))).unwrap().id;
    board.token(1).unwrap().space=far;
    board.start_walk(0,from,path,false,AfterWalk::Step(TurnStep::Done));
}

fn track_kind(board:&mut Board,kind:u16,blow:bool)->bool {
    let Some(view)=board.walk_view() else {return false;};
    let out=board.vacuum.out;
    board.vacuum.out=view.out;
    let target=board.pullables(0).into_iter().find(|(key,..)|*key>>8==kind).map(|(_,at,_)|at);
    board.vacuum.out=out;
    if let Some((x,z))=target {
        board.aim_walk_vacuum_with(0,(x-view.x).atan2(z-view.z),0.0,true,blow).unwrap();
    }
    board.walk_tick(DT);
    target.is_some()
}

#[test]
fn walking_pickups_use_all_standing_prop_types_with_real_progress_and_final_destination_unchanged() {
    for (map,kind,what) in [
        (Map::Island,PULL_LOOSE,"coins"),(Map::Island,PULL_CRATE,"crate"),(Map::Island,PULL_BOX,"box"),
        (Map::Island,PULL_PALM,"coconut"),(Map::Volcano,PULL_LAVA,"lava"),
        (Map::Shipwreck,PULL_FLOTSAM,"flotsam"),(Map::Frozen,PULL_SNOWMAN,"snowball"),
    ] {
        let mut board=clean(map);
        let space=board.layout.iter().find(|space|space.kind==SpaceKind::Item&&!space.under&&!space.gated).unwrap().id;
        let at=&board.layout[space as usize];
        let point=match kind {
            PULL_LOOSE=>{board.loose.insert(space,7);(at.x,at.z)},
            PULL_CRATE=>{board.crates.insert(space,4);board.crate_spot(space)},
            PULL_BOX=>{board.boxes_taken.remove(&space);board.box_spot(space)},
            PULL_PALM=>{board.coconuts[0]=1;map.specials().palms[0]},
            PULL_LAVA=>{board.lava_rocks.insert(space);(at.x,at.z)},
            PULL_FLOTSAM=>{
                let (index,point)=map.specials().flotsam.iter().enumerate().min_by(|(_,a),(_,b)| {
                    let near=|point:&&(f32,f32)|board.layout.iter().map(|space|(space.x-point.0).hypot(space.z-point.1)).fold(f32::INFINITY,f32::min);
                    near(a).total_cmp(&near(b))
                }).unwrap();
                board.flotsam.insert(index as u8,5);*point
            },
            PULL_SNOWMAN=>{board.snowmen[0]=true;map.specials().snowmen[0]},
            _=>unreachable!(),
        };
        walk_near(&mut board,point);
        let final_space=board.tokens[0].space;
        let mut progress=false;
        for _ in 0..600 {
            if board.walk.is_none() {break;}
            track_kind(&mut board,kind,false);
            if let Some(view)=board.walk_view() {progress|=view.pulling.iter().any(|(key,pull)|*key>>8==kind && *pull>0.0);}
            if board.log.iter().any(|entry|entry["kind"]=="vacuumed"&&entry["what"]==what) {break;}
        }
        assert!(progress,"{map:?} {what} has visible suction progress");
        assert!(board.log.iter().any(|entry|entry["kind"]=="vacuumed"&&entry["what"]==what),"walking can collect {map:?} {what}");
        assert!(board.walk.is_some(),"a normal pickup must not discard the walking lifecycle");
        assert_eq!(board.tokens[0].space,final_space);
    }
}

#[test]
fn walking_chests_require_held_aim_and_money_and_success_does_not_stop_the_walk() {
    for map in Map::ALL {
        let mut board=clean(map);
        let prize=board.prize_at;
        let point=(board.layout[prize as usize].x,board.layout[prize as usize].z);
        board.token(0).unwrap().coins=30;
        walk_near(&mut board,point);
        let before=board.tokens[0].coins;
        // Neither walking through a chest nor idle motor ticks buy it.
        for _ in 0..20 {board.walk_tick(DT);}
        assert_eq!(board.tokens[0].prizes,0);
        assert_eq!(board.prize_at,prize);
        for _ in 0..600 {
            if board.walk.is_none() {break;}
            track_kind(&mut board,PULL_CHEST,false);
            if board.tokens[0].prizes>0 {break;}
        }
        assert_eq!(board.tokens[0].prizes,1,"{map:?} chest is a real walking target");
        assert_eq!(board.tokens[0].coins,before-board.prize_cost(0));
        assert!(board.walk.is_some(),"a prize moves the chest without resetting the walk to Items");
        assert!(matches!(board.step(),TurnStep::Walking));
        assert!(board.log.iter().any(|entry|entry["kind"]=="prize"&&entry["seat"]==0));

        let mut poor=clean(map);
        let prize=poor.prize_at;
        poor.token(0).unwrap().coins=14;
        assert!(!poor.collect(0,(PULL_CHEST<<8)|prize as u16),"a stale completed pull cannot spend more than the current purse");
        assert_eq!(poor.tokens[0].prizes,0);
        assert_eq!(poor.tokens[0].coins,14);
    }
}

#[test]
fn walking_storm_catches_the_actual_walker_and_ends_motion_without_replaying_the_old_path() {
    let mut board=clean(Map::Island);
    board.token(0).unwrap().coins=10;
    board.token(0).unwrap().space=5;
    board.token(1).unwrap().space=30;
    board.start_walk(0,0,vec![0,0,0,0,0],false,AfterWalk::Step(TurnStep::Done));
    board.storm=Some(1);
    for _ in 0..600 {
        if board.walk.is_none() {break;}
        track_kind(&mut board,PULL_STORM,false);
    }
    assert!(board.walk.is_none());
    assert_eq!(board.step(),&TurnStep::Done);
    assert_eq!(board.tokens[0].coins,10-STORM_COINS);
    assert_ne!(board.tokens[0].space,5,"the old committed destination is not restored");
    assert!(board.log.iter().any(|entry|entry["kind"]=="walk_interrupted"&&entry["reason"]=="storm"));
    let space=board.tokens[0].space;
    for _ in 0..300 {board.walk_tick(DT);}
    assert_eq!(board.tokens[0].space,space);
}

#[test]
fn walking_blow_uses_the_same_stream_and_does_not_collect_what_it_scatters() {
    let mut board=clean(Map::Island);
    board.loose.insert(0,7);
    board.token(0).unwrap().space=5;
    board.token(1).unwrap().space=30;
    board.start_walk(0,0,vec![0,0,0,0,0],false,AfterWalk::Step(TurnStep::Done));
    for _ in 0..180 {
        if board.walk.is_none() {break;}
        track_kind(&mut board,PULL_LOOSE,true);
        if !board.loose.contains_key(&0) {break;}
    }
    assert!(!board.loose.contains_key(&0));
    assert_eq!(board.tokens[0].coins,0);
    assert!(board.log.iter().any(|entry|entry["kind"]=="scatter"));
    assert!(board.walk_view().unwrap().blowing);
    assert!(board.walk_view().unwrap().power>0.0);
}

#[test]
fn walking_motor_state_is_isolated_from_standing_suction_and_resets_on_release_or_mode_change() {
    let mut board=clean(Map::Island);
    board.loose.insert(0,7);
    board.start_walk(0,0,vec![0,0,0,0],false,AfterWalk::Step(TurnStep::Done));
    board.vacuum.pulling.insert(123,0.75);
    for _ in 0..10 {track_kind(&mut board,PULL_LOOSE,false);}
    let view=board.walk_view().unwrap();
    assert!(!view.pulling.is_empty());
    assert_eq!(board.vacuum.pulling.get(&123),Some(&0.75));
    board.aim_walk_vacuum_with(0,0.0,0.0,true,true).unwrap();
    assert!(board.walk_view().unwrap().pulling.is_empty());
    assert_eq!(board.walk_view().unwrap().power,0.0);
    assert_eq!(board.vacuum.pulling.get(&123),Some(&0.75));
}
