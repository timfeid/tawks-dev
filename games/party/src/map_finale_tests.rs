use super::*;
const DT:f32=1.0/60.0;
const SEATS:&[u8]=&[0,1,2,3];

#[test]
fn map_finale_all_four_map_configs_have_distinct_rules_and_bots_finish_in_world_space() {
    let mut titles=HashSet::new();
    for map in Map::ALL {
        let mut board=Board::for_room(map,SEATS,10,51);
        board.start_map_finale(SEATS,SEATS);
        let view=board.finale_view().unwrap();
        assert!(titles.insert(view.title));assert_eq!(view.map,map);
        assert!(view.checkpoints.len()>=4);assert_eq!(view.ready,READY);
        assert_eq!(view.racers.len(),SEATS.len());
        let mut result=None;
        for _ in 0..4800 {
            board.traffic_tick(DT);
            let inputs:Vec<_>=SEATS.iter().map(|seat|(*seat,board.finale_bot_input(*seat))).collect();
            result=board.finale_tick(DT,&inputs,SEATS);
            if result.is_some(){break;}
        }
        assert!(result.is_some(),"{map:?} always reaches a bounded result");
        let view=board.finale_view().unwrap();
        assert!(view.racers.iter().all(|racer|racer.finished),"{map:?} AI should finish the actual route: {view:?}");
        assert!(view.racers.iter().all(|racer|racer.finish_time.unwrap()<=DURATION));
        assert_eq!(result.unwrap().iter().flatten().count(),SEATS.len());
        assert!(board.finale_tick(DT,&[],SEATS).is_none(),"completed results are emitted once");
    }
}

#[test]
fn map_finale_countdown_input_bounds_checkpoint_order_and_disconnects_are_authoritative() {
    let mut board=Board::for_room(Map::Frozen,SEATS,10,52);
    board.start_map_finale(SEATS,SEATS);
    let initial=board.finale_view().unwrap();
    for _ in 0..150 {board.finale_tick(DT,&[(0,Stick{x:999.,z:999.})],SEATS);}
    let ready=board.finale_view().unwrap();
    assert_eq!((ready.racers[0].x,ready.racers[0].z),(initial.racers[0].x,initial.racers[0].z));
    assert_eq!(ready.elapsed,0.);assert_eq!(ready.remaining,DURATION);
    for _ in 0..40 {board.finale_tick(DT,&[(0,Stick{x:f32::NAN,z:f32::INFINITY})],SEATS);}
    assert!(board.finale_view().unwrap().racers.iter().all(|racer|racer.x.is_finite()&&racer.z.is_finite()));
    let old=board.finale_view().unwrap().racers[0].clone();
    board.finale_tick(1000.,&[(0,Stick{x:999.,z:999.})],SEATS);
    let next=board.finale_view().unwrap().racers[0].clone();
    assert!((old.x-next.x).hypot(old.z-next.z)<=SPEED*0.05+0.0001);
    // A future gate cannot be claimed before the current ordered one.
    let last=board.finale.as_ref().unwrap().gates.last().unwrap().at;
    let race=board.finale.as_mut().unwrap();race.runners[0].view.x=last.0;race.runners[0].view.z=last.1;
    board.finale_tick(DT,&[],SEATS);
    assert_eq!(board.finale_view().unwrap().racers[0].next_checkpoint,1);
    board.finale_tick(DT,&[],&[1,2,3]);
    assert!(board.finale_view().unwrap().racers[0].dnf);
    let mut result=None;
    for _ in 0..4800 {result=board.finale_tick(DT,&[],SEATS);if result.is_some(){break;}}
    assert!(result.is_some(),"idle/disconnected and late racers cannot stall the result");
    assert_eq!(board.finale_view().unwrap().racers.len(),SEATS.len());
}

#[test]
fn map_finale_each_map_mechanic_is_real_and_respawns_preserve_checkpoint_progress() {
    for map in [Map::Island,Map::Volcano] {
        let mut board=Board::for_room(map,SEATS,10,53);
        board.start_map_finale(SEATS,SEATS);
        let race=board.finale.as_mut().unwrap();race.time=READY+1.;race.runners[0].view.gauge=0.9999;
        if map==Map::Island {race.runners[0].view.yaw=std::f32::consts::PI;}
        board.finale_tick(DT,&[(0,Stick{x:0.,z:1.})],SEATS);
        let racer=&board.finale_view().unwrap().racers[0];
        assert_eq!(racer.penalties,1);assert_eq!(racer.stun,2.);assert_eq!(racer.gauge,0.);assert_eq!(racer.next_checkpoint,1);
    }
    let mut board=Board::for_room(Map::Shipwreck,SEATS,10,53);
    board.start_map_finale(SEATS,SEATS);
    let race=board.finale.as_mut().unwrap();race.time=READY+0.5;
    let at=race.gates[1].at;race.runners[0].view.x=at.0;race.runners[0].view.z=at.1;
    board.finale_tick(DT,&[],SEATS);
    let view=board.finale_view().unwrap();
    assert!(view.tide_high&&view.checkpoints[1].blocked&&view.racers[0].waiting);
    assert_eq!(view.racers[0].next_checkpoint,1);
    board.finale.as_mut().unwrap().time=READY+4.;
    board.finale_tick(DT,&[],SEATS);
    assert_eq!(board.finale_view().unwrap().racers[0].next_checkpoint,2);
}

#[test]
fn map_finale_city_car_impact_respawns_and_emits_shared_impact_event_at_collision_point() {
    let mut board=Board::for_room(Map::Frozen,SEATS,10,54);
    board.start_map_finale(SEATS,SEATS);
    let car=board.traffic_view().unwrap().cars[0].clone();
    let race=board.finale.as_mut().unwrap();race.time=READY+1.;race.runners[0].view.x=car.x;race.runners[0].view.z=car.z;
    board.finale_tick(DT,&[],SEATS);
    let racer=&board.finale_view().unwrap().racers[0];
    assert_eq!(racer.penalties,1);assert_eq!(racer.stun,2.);
    let hit=board.log.iter().find(|entry|entry["kind"]=="traffic_hit"&&entry["finale"]==true).unwrap();
    assert_eq!(hit["car"],car.id);assert_eq!(hit["x"],json!(car.x));assert_eq!(hit["z"],json!(car.z));
}

#[test]
fn city_roadworks_have_clear_slalom_routes_and_do_not_change_other_finales() {
    for map in Map::ALL {
        let mut board=Board::for_room(map,SEATS,10,55);board.start_map_finale(SEATS,SEATS);
        let race=board.finale.as_ref().unwrap();
        if map!=Map::Frozen {assert!(race.obstacles.is_empty());continue;}
        assert_eq!(race.obstacles.iter().filter(|obstacle|obstacle.kind=="barrier").count(),12);
        assert_eq!(race.obstacles.iter().filter(|obstacle|obstacle.kind=="ice").count(),7);
        for gate in &race.gates {
            assert!(race_ground(&board,gate.at,gate.space,&race.obstacles),"checkpoint stays clear");
            for &point in &gate.via {assert!(race_ground(&board,point,gate.space,&race.obstacles),"arrows must thread a real gap");}
        }
    }
}

#[test]
fn city_barriers_block_forward_motion_but_allow_going_around() {
    let mut board=Board::for_room(Map::Frozen,SEATS,10,56);board.start_map_finale(SEATS,SEATS);
    let cars=board.traffic_view().unwrap().cars;
    let obstacle=board.finale.as_ref().unwrap().obstacles.iter().find(|obstacle|obstacle.kind=="barrier"&&cars.iter().all(|car|(car.x-obstacle.x).hypot(car.z-obstacle.z)>6.)).unwrap().clone();
    let (sin,cos)=obstacle.yaw.sin_cos();
    let race=board.finale.as_mut().unwrap();race.time=READY+1.;
    race.runners[0].view.x=obstacle.x-sin*1.4;race.runners[0].view.z=obstacle.z-cos*1.4;
    for _ in 0..20 {board.finale_tick(DT,&[(0,Stick{x:sin,z:cos})],SEATS);}
    let at=&board.finale_view().unwrap().racers[0];
    let along=(at.x-obstacle.x)*sin+(at.z-obstacle.z)*cos;
    assert!(along<=-obstacle.half_depth-0.31,"barrier is solid, not decoration: {along}");
    for _ in 0..22 {board.finale_tick(DT,&[(0,Stick{x:cos,z:-sin})],SEATS);}
    for _ in 0..25 {board.finale_tick(DT,&[(0,Stick{x:sin,z:cos})],SEATS);}
    let at=&board.finale_view().unwrap().racers[0];
    assert!((at.x-obstacle.x)*sin+(at.z-obstacle.z)*cos>obstacle.half_depth+0.32,"there is a usable route around it");
    assert_eq!(at.penalties,0);
}

#[test]
fn ice_carries_momentum_and_responds_gradually_to_steering() {
    let mut board=Board::for_room(Map::Frozen,SEATS,10,57);board.start_map_finale(SEATS,SEATS);
    let cars=board.traffic_view().unwrap().cars;
    let ice=board.finale.as_ref().unwrap().obstacles.iter().find(|obstacle|obstacle.kind=="ice"&&cars.iter().all(|car|(car.x-obstacle.x).hypot(car.z-obstacle.z)>5.)).unwrap().clone();
    let race=board.finale.as_mut().unwrap();race.time=READY+1.;let runner=&mut race.runners[0];runner.view.x=ice.x;runner.view.z=ice.z;runner.velocity=(SPEED,0.);
    board.finale_tick(DT,&[(0,Stick{x:0.,z:1.})],SEATS);
    let view=board.finale_view().unwrap();let racer=&view.racers[0];
    assert!(racer.slipping);assert!(racer.x>ice.x+0.08,"ice keeps the previous momentum");
    assert!(racer.z>ice.z&&racer.z<ice.z+0.02,"steering responds without snapping instantly");
    assert_eq!(racer.penalties,0);
}
