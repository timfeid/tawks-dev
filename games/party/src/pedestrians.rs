//! Finite city wallets, shared by every vacuum mode. People remain in the
//! world when robbed; only their cash is collected, once until round restock.
use super::*;
const CASH:u8=5;
const SPEED:f32=0.8;

#[cfg(test)]
mod tests {
    use super::*;
    const DT:f32=1.0/60.0;

    fn near_npc()->Board {
        let mut board=Board::for_room(Map::Frozen,&[0,1],10,8);
        let at=board.pedestrians[0].at();
        let space=board.layout.iter().min_by(|a,b|(a.x-at.0).hypot(a.z-at.1).total_cmp(&(b.x-at.0).hypot(b.z-at.1))).unwrap().id;
        board.token(0).unwrap().space=space;board.token(0).unwrap().coins=0;
        let far=board.layout.iter().max_by(|a,b|(a.x-at.0).hypot(a.z-at.1).total_cmp(&(b.x-at.0).hypot(b.z-at.1))).unwrap().id;
        board.token(1).unwrap().space=far;board.token(1).unwrap().coins=0;
        board.crates.clear();board.snowmen.fill(false);
        board.boxes_taken=board.layout.iter().filter(|space|space.kind==SpaceKind::Item).map(|space|space.id).collect();
        board.endless_vacuum=true;
        board
    }

    #[test]
    fn run_over_body_stays_down_without_wallet_duplication_until_round_reset(){
        let mut board=near_npc();let at=board.pedestrians[0].at();let cash=board.tokens[0].coins;
        assert!(board.car_hit_pedestrians(0,(at.0-3.,at.1),(at.0+3.,at.1),0.95,5.).contains(&0));
        let struck=board.pedestrian_views()[0].clone();assert!(struck.knocked_down);assert!(!struck.walking&&!struck.fleeing);assert_eq!(struck.remaining_cash,5);assert_eq!(board.tokens[0].coins,cash);
        for _ in 0..100{board.pedestrians_tick(DT);}let still=&board.pedestrian_views()[0];assert_eq!((still.x,still.z),(struck.x,struck.z));
        assert!(!board.car_hit_pedestrians(0,(at.0-3.,at.1),(at.0+3.,at.1),0.95,5.).contains(&0));
        board.rob_pedestrian(0,0);board.rob_pedestrian(0,0);assert_eq!(board.tokens[0].coins,cash+5);assert_eq!(board.pedestrian_views()[0].remaining_cash,0);assert!(board.pedestrian_views()[0].knocked_down);
        board.restock_pedestrians();assert!(!board.pedestrian_views()[0].knocked_down);assert!(board.pedestrian_views()[0].walking);assert_eq!(board.pedestrian_views()[0].remaining_cash,5);
    }

    #[test]
    fn pedestrians_replace_floating_fields_in_real_rooms_and_wallets_are_finite_per_round() {
        for map in Map::ALL {
            let mut board=Board::for_room(map,&[0,1],10,8);
            assert!(board.air_view().is_empty());
            assert_eq!(board.view().air_remaining,0);
            assert_eq!(board.pedestrians.len(),if map==Map::Frozen {6}else{0});
            board.finish_round(&[vec![0],vec![1]]);
            assert!(board.air_view().is_empty(),"round restock never brings floating coins back");
        }
        let mut board=near_npc();
        let before=board.tokens[0].coins;
        for id in 0..6 {for _ in 0..3 {board.rob_pedestrian(0,id);}}
        assert_eq!(board.tokens[0].coins-before,30);
        assert_eq!(board.pedestrians.len(),6,"the people remain in the world");
        assert!(board.pedestrian_views().iter().all(|npc|npc.robbed&&npc.remaining_cash==0&&npc.fleeing));
        board.turn_to_for_test(1);
        for id in 0..6 {board.rob_pedestrian(1,id);}
        assert_eq!(board.tokens[1].coins,0,"another turn cannot farm emptied shared wallets");
        board.finish_round(&[vec![0],vec![1]]);
        assert!(board.pedestrian_views().iter().all(|npc|!npc.robbed&&npc.remaining_cash==5));
        assert!(Board::on(Map::Frozen,&[0,1],10,8).air_view().len()>0,"old math fixtures retain their explicit coin mode");
    }

    #[test]
    fn pedestrians_require_held_matching_aim_and_have_visible_shared_pickup_progress() {
        let mut board=near_npc();
        board.suck(0).unwrap();
        for _ in 0..30 {board.vacuum_tick(DT);board.pedestrians_tick(DT);}
        assert_eq!(board.tokens[0].coins,0);
        let from=board.rendered_spot_of(0);let at=board.pedestrians[0].at();
        board.aim_vacuum(0,(at.0-from.0).atan2(at.1-from.1)+std::f32::consts::PI,true).unwrap();
        for _ in 0..60 {board.vacuum_tick(DT);board.pedestrians_tick(DT);}
        assert_eq!(board.tokens[0].coins,0);
        let mut progress=false;
        for _ in 0..600 {
            let at=board.pedestrians[0].at();
            board.aim_vacuum(0,(at.0-from.0).atan2(at.1-from.1),true).unwrap();
            board.vacuum_tick(DT);board.pedestrians_tick(DT);
            progress|=board.vacuum.pulling.iter().any(|(key,pull)|*key>>8==PULL_PEDESTRIAN&&*pull>0.0);
            if board.pedestrians[0].cash==0 {break;}
        }
        assert!(progress);
        assert_eq!(board.tokens[0].coins,5);
        assert!(board.pedestrians[0].cash==0);
        assert!(board.log.iter().any(|entry|entry["kind"]=="npc_robbed"&&entry["npc"]==0&&entry["coins"]==5));
    }

    #[test]
    fn pedestrians_can_be_robbed_while_walking_without_airborne_rewards_or_ending_movement() {
        let mut board=near_npc();let from=board.tokens[0].space;
        board.start_walk(0,from,vec![from;8],false,walking::AfterWalk::Step(TurnStep::Done));
        let mut progress=false;
        for _ in 0..300 {
            board.bot_walk_aim(1.0);board.walk_tick(DT);board.pedestrians_tick(DT);
            if let Some(view)=board.walk_view() {progress|=view.pulling.iter().any(|(key,pull)|*key>>8==PULL_PEDESTRIAN&&*pull>0.0);}
            if board.pedestrians[0].cash==0 {break;}
        }
        assert!(progress);
        assert_eq!(board.tokens[0].coins,5);
        let walk=board.walk_view().expect("wallet collection does not discard the walk");
        assert!(walk.coins.is_empty());assert_eq!(walk.max_bonus,0);assert_eq!(walk.bonus,0);
        assert!(matches!(board.step(),TurnStep::Walking));
    }

    #[test]
    fn pedestrians_body_tug_is_bounded_and_ai_chooses_cash_targets() {
        let mut board=near_npc();
        assert_eq!(board.bot_move(),BotMove::Suck);
        board.suck(0).unwrap();
        assert!(matches!(board.bot_move(),BotMove::Vacuum{on:true,blow:false,..}));
        let at=board.pedestrians[0].at();
        board.vacuum.origin=Some((at.0-2.0,at.1));board.vacuum.yaw=std::f32::consts::FRAC_PI_2;
        for _ in 0..600 {board.pedestrian_pressure(0,DT,1.0);}
        let offset=board.pedestrians[0].offset;
        assert!(offset.0.hypot(offset.1)<=0.6001);
        assert!(offset.0.hypot(offset.1)>0.05,"the visible body does respond to suction");
        assert_eq!(board.pedestrians[0].cash,5,"tug alone never claims currency");
        assert_eq!(board.pedestrians.len(),6);
    }

    #[test]
    fn production_lobby_and_sandbox_start_in_snowtown_without_floating_pickups() {
        let mut room=crate::room::Room::new("npc-production",31);
        room.connect("host","Host","").unwrap();room.connect("friend","Friend","").unwrap();
        let preview=room.view().preview_board.unwrap();
        assert_eq!(preview.map,Map::Frozen);assert_eq!(preview.pedestrians.len(),6);assert!(preview.air_coins.is_empty());
        room.dev_tools=true;room.start_sandbox("host").unwrap();
        assert_eq!(room.board.as_ref().unwrap().view().pedestrians.len(),6);
        assert!(room.board.as_ref().unwrap().view().air_coins.is_empty());
    }

    #[test]
    fn pedestrians_are_cash_fallback_and_do_not_preempt_an_affordable_nearby_chest() {
        let mut board=near_npc();
        let space=board.prize_at;
        board.token(0).unwrap().space=space;board.token(0).unwrap().coins=30;
        board.token(1).unwrap().space=space;
        let origin=board.rendered_spot_of(0);
        for npc in &mut board.pedestrians {npc.cash=0;}
        board.pedestrians[0].cash=5;board.pedestrians[0].x=origin.0+4.;board.pedestrians[0].z=origin.1;
        board.suck(0).unwrap();
        let BotMove::Vacuum{yaw,on:true,blow:false}=board.bot_move() else {panic!("aim for the affordable chest")};
        let center=&board.layout[space as usize];
        let expected=(center.x-origin.0).atan2(center.z+2.2-origin.1);
        assert!((yaw-expected).abs()<0.001,"aim at the chest from the visible shared-tile body, not the tile center");
        board.token(0).unwrap().coins=0;
        let BotMove::Vacuum{yaw,on:true,blow:false}=board.bot_move() else {panic!("collect cash when the chest is unaffordable")};
        assert!((yaw-std::f32::consts::FRAC_PI_2).abs()<0.001);
    }
}

#[derive(Clone,Debug,Serialize)]
pub struct PedestrianView {
    pub id:u8,pub x:f32,pub z:f32,pub yaw:f32,
    pub remaining_cash:u8,pub robbed:bool,pub walking:bool,pub fleeing:bool,pub knocked_down:bool,
}
pub(super) struct Pedestrian {
    pub(super) id:u8,pub(super) cash:u8,knocked_down:bool,
    route:Vec<(f32,f32)>,next:usize,x:f32,z:f32,yaw:f32,offset:(f32,f32),panic:f32,
}
impl Pedestrian {pub(super) fn at(&self)->(f32,f32){(self.x+self.offset.0,self.z+self.offset.1)}}
#[derive(Deserialize)]
struct Routes {#[serde(default)] pedestrian_routes:Vec<Route>}
#[derive(Deserialize)]
struct Route {points:Vec<(f32,f32)>}

impl Board {
    pub(super) fn restock_pedestrians(&mut self) {
        if self.map!=Map::Frozen || self.floating_coins {self.pedestrians.clear();return;}
        // Retain identity/position across rounds; only wallets are replenished.
        if !self.pedestrians.is_empty() {for npc in &mut self.pedestrians {npc.cash=CASH;npc.panic=0.0;npc.knocked_down=false;}return;}
        let routes:Routes=serde_json::from_str(self.map.source()).expect("city pedestrian routes");
        self.pedestrians=routes.pedestrian_routes.into_iter().filter(|route|route.points.len()>=2 && route.points.iter().all(|point|point.0.is_finite()&&point.1.is_finite())).take(6).enumerate().map(|(id,route)|{
            let at=route.points[0];let next=route.points[1];
            Pedestrian {id:id as u8,cash:CASH,knocked_down:false,route:route.points,next:1,x:at.0,z:at.1,yaw:(next.0-at.0).atan2(next.1-at.1),offset:(0.0,0.0),panic:0.0}
        }).collect();
    }

    /// Car body swept against actual displayed pedestrian positions. Wallets
    /// remain finite and untouched; repeated passes cannot mint cash or hits.
    pub(super) fn car_hit_pedestrians(&mut self,seat:u8,from:(f32,f32),to:(f32,f32),radius:f32,speed:f32)->Vec<u8>{
        if ![from.0,from.1,to.0,to.1,radius,speed].into_iter().all(f32::is_finite)||radius<=0.0{return vec![];}
        let delta=(to.0-from.0,to.1-from.1);let length=delta.0*delta.0+delta.1*delta.1;let mut hits=Vec::new();
        for npc in &mut self.pedestrians {if npc.knocked_down{continue;}let at=npc.at();let t=(((at.0-from.0)*delta.0+(at.1-from.1)*delta.1)/length.max(0.00001)).clamp(0.,1.);if(at.0-from.0-delta.0*t).hypot(at.1-from.1-delta.1*t)<=radius{npc.knocked_down=true;npc.panic=0.;hits.push((npc.id,at));}}
        for (npc,at) in &hits{self.say(json!({"kind":"npc_run_over","seat":seat,"car":100+seat,"npc":npc,"x":at.0,"z":at.1,"speed":speed.abs()}));}hits.into_iter().map(|(id,_)|id).collect()
    }

    pub(super) fn pedestrian_views(&self)->Vec<PedestrianView> {
        self.pedestrians.iter().map(|npc|{let (x,z)=npc.at();PedestrianView{id:npc.id,x,z,yaw:npc.yaw,remaining_cash:npc.cash,robbed:npc.cash==0,walking:!npc.knocked_down,fleeing:!npc.knocked_down&&npc.panic>0.0,knocked_down:npc.knocked_down}}).collect()
    }

    pub fn pedestrians_tick(&mut self,dt:f32)->bool {
        if !dt.is_finite()||dt<=0.0 {return false;}
        let dt=dt.min(0.05);
        for npc in &mut self.pedestrians {
            if npc.knocked_down {continue;}
            npc.panic=(npc.panic-dt).max(0.0);
            let to=npc.route[npc.next];let (dx,dz)=(to.0-npc.x,to.1-npc.z);let length=dx.hypot(dz);
            let step=dt*if npc.panic>0.0 {1.5}else{SPEED};
            if length<=step {npc.x=to.0;npc.z=to.1;npc.next=(npc.next+1)%npc.route.len();}
            else {npc.x+=dx/length*step;npc.z+=dz/length*step;npc.yaw=dx.atan2(dz);}
            npc.offset.0*=1.0-dt*1.8;npc.offset.1*=1.0-dt*1.8;
        }
        !self.pedestrians.is_empty()
    }

    pub(super) fn pedestrian_pressure(&mut self,seat:u8,dt:f32,power:f32) {
        let origin=self.vacuum.origin.unwrap_or_else(||self.actual_spot_of(seat));
        let reach=self.reach_of(seat);
        for npc in &mut self.pedestrians {
            if npc.cash==0 || npc.knocked_down {continue;}
            let at=npc.at();let strength=reach(at,1.0).unwrap_or(0.0);
            let (dx,dz)=(origin.0-at.0,origin.1-at.1);let distance=dx.hypot(dz).max(0.001);
            let step=dt.min(0.05)*power*strength*0.25;
            npc.offset.0+=dx/distance*step;npc.offset.1+=dz/distance*step;
            let offset=npc.offset.0.hypot(npc.offset.1);
            if offset>0.6 {npc.offset.0*=0.6/offset;npc.offset.1*=0.6/offset;}
        }
    }

    pub(super) fn rob_pedestrian(&mut self,seat:u8,id:u8) {
        if seat!=self.turn_seat() {return;}
        let Some(npc)=self.pedestrians.iter_mut().find(|npc|npc.id==id&&npc.cash>0) else {return;};
        let cash=std::mem::take(&mut npc.cash);let (x,z)=npc.at();npc.panic=1.5;
        self.coins(seat,i32::from(cash));
        self.say(json!({"kind":"npc_robbed","seat":seat,"npc":id,"coins":cash,"x":x,"z":z}));
    }

    pub(super) fn bot_pedestrian_aim(&self,origin:(f32,f32))->Option<(f32,f32)> {
        self.pedestrians.iter().filter(|npc|npc.cash>0).map(|npc|npc.at()).filter(|at|(at.0-origin.0).hypot(at.1-origin.1)<=VACUUM_RANGE).min_by(|a,b|(a.0-origin.0).hypot(a.1-origin.1).total_cmp(&(b.0-origin.0).hypot(b.1-origin.1)))
    }
}
