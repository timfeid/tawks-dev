//! Tight, bounded cartoon projectiles using the board's existing positioning
//! and Lob action. Server hits resolve once; logs describe presentation flights.
use super::*;

pub const DRIVE_BY_RANGE: f32 = 18.0;
pub const SLAPSHOT_RANGE: f32 = 22.0;
const HIT_WIDTH: f32 = 0.65;

impl Board {
    fn ray_rival(&self, by:u8, origin:(f32,f32), direction:(f32,f32), reach:f32) -> Option<(u8,(f32,f32))> {
        self.ray_rival_eligible(by,origin,direction,reach,None)
    }
    pub(super) fn ray_rival_eligible(&self,by:u8,origin:(f32,f32),direction:(f32,f32),reach:f32,eligible:Option<&[u8]>)->Option<(u8,(f32,f32))> {
        self.tokens.iter().filter(|token|self.rival(by,token.seat)&&eligible.is_none_or(|eligible|eligible.contains(&token.seat)&&token.resting==0)).filter_map(|token|{
            let at=if eligible.is_some(){self.actual_spot_of(token.seat)}else{self.item_spot_of(token.seat)};
            let (dx,dz)=(at.0-origin.0,at.1-origin.1);
            let along=dx*direction.0+dz*direction.1;
            let across=(dx*direction.1-dz*direction.0).abs();
            (along>=0.0 && along<=reach && across<=HIT_WIDTH).then_some((token.seat,at,along))
        }).min_by(|a,b|a.2.total_cmp(&b.2).then(a.0.cmp(&b.0))).map(|(seat,at,_)|(seat,at))
    }

    pub(super) fn board_shot(&mut self, by:u8, item:Item, aimed:(f32,f32)) {
        self.step=TurnStep::Items;
        let origin=self.rendered_spot_of(by);
        let delta=(aimed.0-origin.0,aimed.1-origin.1);
        let length=delta.0.hypot(delta.1);
        let maximum=if item==Item::DriveBy {DRIVE_BY_RANGE}else{SLAPSHOT_RANGE};
        let reach=if length.is_finite(){length.clamp(0.0,maximum)}else{0.0};
        let yaw=delta.0.atan2(delta.1);
        let count=if item==Item::DriveBy {3}else{1};
        let mut shields=HashMap::<u8,bool>::new();
        let mut hits=HashMap::<u8,u8>::new();
        let mut shots=Vec::new();
        for index in 0..count {
            let spread=if count==3 {(index as f32-1.0)*0.025}else{0.0};
            let direction=((yaw+spread).sin(),(yaw+spread).cos());
            let end=(origin.0+direction.0*reach,origin.1+direction.1*reach);
            let struck=if reach>0.05 {self.ray_rival(by,origin,direction,reach)}else{None};
            let (target,at,blocked)=match struck {
                Some((seat,at))=>{
                    let blocked=*shields.entry(seat).or_insert_with(||self.blocked(by,seat,item));
                    if !blocked {*hits.entry(seat).or_default()+=1;}
                    (Some(seat),at,blocked)
                },
                None=>(None,end,false),
            };
            let flight=(at.0-origin.0).hypot(at.1-origin.1)/if item==Item::DriveBy {55.0}else{36.0};
            shots.push(json!({"target":target,"x":at.0,"z":at.1,"blocked":blocked,"delay":index as f32*0.1,"flight":flight.max(0.12)}));
        }
        let mut targets:Vec<u8>=hits.keys().copied().collect();targets.sort_unstable();
        let mut result=Vec::new();
        for target in &targets {
            let from=self.space_of(*target);
            let knock=if item==Item::Slapshot {1}else{0};
            let to=self.walk_back(from,knock);
            let loss=if item==Item::DriveBy {i32::from(hits[target]).min(3)}else{4};
            let purse=self.tokens.iter().find(|token|token.seat==*target).map(|token|token.coins).unwrap_or(0);
            let lost=purse.min(loss);
            self.coins(*target,-lost);
            self.scatter(from,lost,2.0,Some(*target));
            if knock>0 {if let Some(token)=self.token(*target){token.previous=None;token.space=to;}}
            result.push(json!({"seat":target,"shots":hits[target],"coins":lost,"damage":1,"from":from,"to":to}));
            self.crossed(*target,by);
        }
        let quality=if !targets.is_empty(){"hit"}else if shields.values().any(|blocked|*blocked){"blocked"}else{"miss"};
        let flight=shots.iter().map(|shot|shot["delay"].as_f64().unwrap_or(0.0)+shot["flight"].as_f64().unwrap_or(0.0)).fold(0.0,f64::max);
        self.say(json!({"kind":"item","item":item,"by":by,"origin_x":origin.0,"origin_z":origin.1,"x":origin.0+yaw.sin()*reach,"z":origin.1+yaw.cos()*reach,"yaw":yaw,"quality":quality,"shots":shots,"hits":result,"flight":flight,"target":targets.first()}));
        // One heart per victim for the entire action, including a three-shot
        // burst. A shield was checked once above and absorbs every tracer.
        for target in targets {self.hurt(target,1);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ready(item:Item)->Board {
        let mut board=Board::new(&[0,1,2],4,71);
        board.dev_first(0);board.timed_items=true;
        board.tokens.iter_mut().for_each(|token|{token.space=0;token.coins=20;});
        board.token(0).unwrap().items=vec![item];board.take_events();board
    }
    fn prepare(board:&mut Board,item:Item)->(f32,f32){
        board.use_item(0,item,None).unwrap();
        if item==Item::DriveBy {assert_eq!(board.step(),&TurnStep::Raiding);return board.spot_of(0);}
        assert_eq!(board.step(),&TurnStep::Position{item});
        assert_eq!(board.tokens[0].items,vec![item]);board.activate_item(0).unwrap();
        let origin=board.spot_of(0);assert!(matches!(board.step(),TurnStep::Lob{..}));origin
    }
    #[test]
    fn both_shots_use_position_lob_consume_once_and_return_without_changing_the_home_tile(){
        for item in [Item::Slapshot] {
            let mut board=ready(item);let home=board.tokens[0].space;prepare(&mut board,item);
            assert!(board.tokens[0].items.is_empty());assert_eq!(board.tokens[0].stats.items_used,1);
            assert_eq!(board.lob(1,0.0,0.5,0.5),Err(BoardError::NotYourTurn));
            board.lob(0,1.0,0.5,0.8).unwrap();assert_eq!(board.step(),&TurnStep::Items);assert!(board.position_view().is_none());assert_eq!(board.tokens[0].space,home);
            assert_eq!(board.lob(0,1.0,0.5,0.8),Err(BoardError::WrongStep));
        }
    }
    #[test]
    fn drive_by_three_spaced_hits_cost_one_heart_and_shield_absorbs_the_group(){
        for shield in [false,true] {
            let mut board=ready(Item::DriveBy);prepare(&mut board,Item::DriveBy);
            board.token(1).unwrap().shield=shield;
            let target=board.item_spot_of(1);let health=board.tokens[1].health;
            let from=board.rendered_spot_of(0);let yaw=(target.0-from.0).atan2(target.1-from.1);
            for _ in 0..100 {board.raid_input(0,0.,0.,yaw).unwrap();board.raid_fire(0,true,yaw).unwrap();board.raids_tick(1./60.,&[0,1,2]);}
            let victim=board.tokens.iter().find(|token|token.seat==1).unwrap();
            assert_eq!(victim.health,health-if shield{0}else{1});assert!(!victim.shield);
            assert!((17..=20).contains(&victim.coins));if shield{assert_eq!(victim.coins,20);}
            let events=board.take_events();let shots:Vec<_>=events.iter().filter(|event|event["item"]=="drive_by"&&event["shots"].is_array()).collect();
            assert_eq!(shots.len(),3);assert!(shots.iter().all(|event|event["shots"].as_array().unwrap().len()==1));
            assert_eq!(events.iter().filter(|event|event["kind"]=="blocked"&&event["seat"]==1).count(),usize::from(shield));
        }
    }
    #[test]
    fn slapshot_is_one_heart_four_coins_and_one_space_without_repeated_damage(){
        let mut board=ready(Item::Slapshot);prepare(&mut board,Item::Slapshot);let target=board.item_spot_of(1);let from=board.space_of(1);let health=board.tokens[1].health;
        board.board_shot(0,Item::Slapshot,target);let victim=board.tokens.iter().find(|token|token.seat==1).unwrap();
        assert_eq!(victim.health,health-1);assert_eq!(victim.coins,16);assert_eq!(victim.space,board.walk_back(from,1));
    }
    #[test]
    fn rays_are_narrow_range_limited_and_hit_the_first_rival_only(){
        let mut board=ready(Item::DriveBy);prepare(&mut board,Item::DriveBy);
        let origin=board.rendered_spot_of(0);let target=board.item_spot_of(1);let delta=(target.0-origin.0,target.1-origin.1);let d=delta.0.hypot(delta.1);let dir=(delta.0/d,delta.1/d);
        assert_eq!(board.ray_rival(0,origin,dir,d+0.1).unwrap().0,1);
        assert!(board.ray_rival(0,origin,(-dir.0,-dir.1),10.0).is_none());
        assert!(board.ray_rival(0,origin,dir,d*0.5).is_none());
        let before:Vec<_>=board.tokens.iter().map(|token|(token.health,token.coins)).collect();board.board_shot(0,Item::DriveBy,origin);assert_eq!(before,board.tokens.iter().map(|token|(token.health,token.coins)).collect::<Vec<_>>());
    }
    #[test]
    fn partners_are_not_hurt_and_nonfinite_aim_is_sanitized(){
        let mut board=ready(Item::Slapshot);board.set_partners(&[(0,1)]);prepare(&mut board,Item::Slapshot);let target=board.item_spot_of(1);
        board.board_shot(0,Item::Slapshot,target);assert_eq!(board.tokens[1].health,MAX_HEALTH);assert_eq!(board.tokens[1].coins,20);
        let mut board=ready(Item::Slapshot);prepare(&mut board,Item::Slapshot);board.lob(0,f32::NAN,f32::INFINITY,f32::NAN).unwrap();
        for event in board.take_events().iter().filter(|event|event["shots"].is_array()){assert!(event["x"].as_f64().unwrap().is_finite());assert!(event["z"].as_f64().unwrap().is_finite());}
    }
}
