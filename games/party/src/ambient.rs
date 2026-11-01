//! A shared finite field of airborne coins. Aiming collects them; merely
//! landing on or walking through their spaces never does.
use super::*;

pub const AMBIENT_TURN_CAP:u8=6;

#[derive(Clone,Debug,Serialize)]
pub struct AirCoinView {
    pub id:u32,
    pub space:u8,
    pub x:f32,
    pub z:f32,
    pub height:f32,
    pub coins:i32,
    pub available:bool,
    pub collected_by:Option<u8>,
}

pub(super) struct AirCoin {pub(super) view:AirCoinView}

impl Board {
    pub(super) fn refill_air_coins(&mut self) {
        self.air_coins.clear();
        self.ambient_earned=0;
        if !self.floating_coins {return;}
        for space in self.layout.iter().filter(|space|!space.under && matches!(space.kind,SpaceKind::Coin|SpaceKind::Start)) {
            let Some(to)=space.next.first().map(|to|&self.layout[*to as usize]) else {continue};
            if space.link.is_some(){continue;}
            let (dx,dz)=(to.x-space.x,to.z-space.z);
            let length=dx.hypot(dz).max(0.01);
            for pick in 0..3 {
                if self.air_coins.len()>=256 {break;}
                self.next_air_id+=1;
                let along=3.0+pick as f32*0.7;
                let side=if (space.id as usize+pick)%2==0 {1.6} else {-1.6};
                self.air_coins.push(AirCoin{view:AirCoinView{id:self.next_air_id,space:space.id,
                    x:space.x+dx/length*along+dz/length*side,
                    z:space.z+dz/length*along-dx/length*side,
                    height:1.4+pick as f32*0.25,coins:1,available:true,collected_by:None}});
            }
        }
    }

    pub(super) fn air_available(&self,index:usize)->bool {
        self.air_coins.get(index).is_some_and(|coin|coin.view.collected_by.is_none() && (self.gates_open() || !self.layout[coin.view.space as usize].gated))
    }

    fn air_value(&self)->i32 {
        if matches!(self.event,Some(RoundEvent::CoinShower|RoundEvent::LastRound)){2}else{1}
    }

    pub(super) fn air_view(&self)->Vec<AirCoinView> {
        self.air_coins.iter().enumerate().map(|(index,coin)|{
            let mut view=coin.view.clone();view.available=self.air_available(index);view.coins=self.air_value();view
        }).collect()
    }

    /// Both vacuum modes claim the same pool entry exactly once.
    pub(super) fn take_air_coin(&mut self,seat:u8,index:usize)->i32 {
        if seat!=self.turn_seat() || self.ambient_earned>=AMBIENT_TURN_CAP || !self.air_available(index){return 0;}
        let coins=self.air_value().min((AMBIENT_TURN_CAP-self.ambient_earned) as i32);
        self.air_coins[index].view.collected_by=Some(seat);
        self.ambient_earned+=coins as u8;
        self.coins(seat,coins);
        coins
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_on_coin_spaces_never_awards_the_shared_airborne_coins() {
        let mut board=Board::new(&[0,1],6,8);
        let seat=board.turn_seat();
        let space=board.layout.iter().find(|space|space.kind==SpaceKind::Coin).unwrap().id;
        board.place_for_test(seat,space,20);
        board.land_for_test(seat);
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,20);
        assert!(board.air_view().iter().all(|coin|coin.collected_by.is_none()));
    }

    #[test]
    fn ambient_claims_require_the_turn_owner_are_once_only_and_share_a_six_coin_cap() {
        let mut board=Board::new(&[0,1],6,8);
        let seat=board.turn_seat();
        let before=board.tokens.iter().find(|token|token.seat==seat).unwrap().coins;
        assert_eq!(board.take_air_coin(1-seat,0),0);
        assert_eq!(board.take_air_coin(seat,0),1);
        assert_eq!(board.take_air_coin(seat,0),0);
        for index in 1..20 {board.take_air_coin(seat,index);}
        assert_eq!(board.ambient_earned,AMBIENT_TURN_CAP);
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,before+6);
        assert_eq!(board.air_view().iter().filter(|coin|coin.collected_by.is_some()).count(),6);
    }

    #[test]
    fn a_round_refill_uses_fresh_ids_and_double_value_events_still_respect_the_cap() {
        let mut board=Board::new(&[0,1],6,8);
        let old:HashSet<u32>=board.air_view().iter().map(|coin|coin.id).collect();
        board.refill_air_coins();
        assert!(board.air_view().iter().all(|coin|!old.contains(&coin.id)));
        board.event=Some(RoundEvent::CoinShower);
        let seat=board.turn_seat();
        for index in 0..8 {board.take_air_coin(seat,index);}
        assert_eq!(board.ambient_earned,6);
        assert_eq!(board.air_view().iter().filter(|coin|coin.collected_by.is_some()).count(),3);
        assert!(board.air_view().iter().all(|coin|coin.coins==2));
    }

    #[test]
    fn standing_vacuum_needs_accurate_held_aim_and_never_recollects_a_field_coin() {
        let mut board=Board::new(&[0,1],6,8);
        let seat=board.turn_seat();
        board.endless_vacuum=true;
        board.clear_crates_for_test();
        board.place_for_test(1-seat,30,0);
        board.air_coins.truncate(1);
        let coin=board.air_coins[0].view.clone();
        let (x,z)=board.spot_of(seat);
        let yaw=(coin.x-x).atan2(coin.z-z);
        let before=board.tokens.iter().find(|token|token.seat==seat).unwrap().coins;
        board.suck(seat).unwrap();
        board.aim_vacuum(seat,yaw+std::f32::consts::PI,true).unwrap();
        for _ in 0..180 {board.vacuum_tick(1.0/60.0);}
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,before);
        assert!(board.air_available(0));
        board.aim_vacuum(seat,yaw,true).unwrap();
        for _ in 0..180 {board.vacuum_tick(1.0/60.0);}
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,before+1);
        assert_eq!(board.air_view()[0].collected_by,Some(seat));
        for _ in 0..600 {board.vacuum_tick(1.0/60.0);}
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,before+1,"holding forever cannot farm a collected coin");
    }

    #[test]
    fn a_shared_tile_uses_the_rendered_standing_origin_for_coin_aim() {
        let mut board=Board::new(&[0,1,2,3],6,8);
        let seat=board.turn_seat();
        let coin=&board.air_coins[0].view;
        let (x,z)=board.spot_of(seat);
        let offset=(0.5+4.0*0.12)*std::f32::consts::FRAC_1_SQRT_2;
        let yaw=(coin.x-x-offset).atan2(coin.z-z-offset);
        board.vacuum.yaw=yaw;
        assert!(board.reach_of(seat)((coin.x,coin.z),1.0).is_some(),"aiming through the rendered coin must be accepted from the fanned-out character");
    }

    #[test]
    fn pulling_a_storm_onto_yourself_stows_the_vacuum_before_the_blowback_view() {
        let mut board=Board::new(&[0,1],6,8);
        let seat=board.turn_seat();
        board.air_coins.clear();board.clear_crates_for_test();board.coconuts.fill(0);
        board.place_for_test(seat,1,20);board.place_for_test(1-seat,40,0);
        board.storm=Some(0);
        let (x,z)=board.spot_of(seat);
        let storm=&board.layout[0];
        let yaw=(storm.x-x).atan2(storm.z-z);
        board.suck(seat).unwrap();board.aim_vacuum(seat,yaw,true).unwrap();
        for _ in 0..240 {if !matches!(board.step(),TurnStep::Vacuum{..}){break;}board.vacuum_tick(1.0/60.0);}
        assert_ne!(board.space_of(seat),1,"the storm catches and throws the holder");
        assert_eq!(board.step(),&TurnStep::Items,"the vacuum is put away so the camera can show the throw safely");
        assert!(board.view().vacuum.is_none());
        assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,20-STORM_COINS);
        let events=board.take_events();
        let catch=events.iter().position(|event|event["kind"]=="blown").unwrap();
        let stow=events.iter().position(|event|event["kind"]=="vacuum_done").unwrap();
        assert!(catch<stow);
    }

    fn empty_vacuum_board()->(Board,u8) {
        let mut board=Board::new(&[0,1],6,8);
        let seat=board.turn_seat();
        board.air_coins.clear();board.clear_crates_for_test();board.coconuts.fill(0);
        board.boxes_taken=board.layout.iter().filter(|space|space.kind==SpaceKind::Item).map(|space|space.id).collect();
        board.place_for_test(1-seat,40,0);
        (board,seat)
    }

    #[test]
    fn coins_at_twelve_degrees_and_far_reach_collect_before_the_battery_ends() {
        for angle in [12.0f32,13.5] {
            let (mut board,seat)=empty_vacuum_board();
            let (x,z)=board.rendered_spot_of(seat);
            let angle=angle.to_radians();
            board.air_coins.push(AirCoin{view:AirCoinView{id:999,space:0,x:x+angle.sin()*13.5,z:z+angle.cos()*13.5,height:1.5,coins:1,available:true,collected_by:None}});
            let before=board.tokens.iter().find(|token|token.seat==seat).unwrap().coins;
            board.suck(seat).unwrap();board.aim_vacuum(seat,0.0,true).unwrap();
            for _ in 0..((VACUUM_SECONDS+1.)*60.) as usize {board.vacuum_tick(1.0/60.0);}
            let inside=angle<VACUUM_CONE;
            assert_eq!(board.air_coins[0].view.collected_by.is_some(),inside);
            assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().coins,before+if inside{1}else{0});
            assert!(board.view().vacuum.is_none(),"the ordinary twenty-second battery still runs out");
        }
        assert!(vacuum_strength((0.0,0.0),0.0,(0.0,14.1),1.0).is_none());
    }

    #[test]
    fn the_full_visible_cone_catches_or_blows_the_chest_at_its_outer_edge() {
        for blow in [false,true] {
            let (mut board,seat)=empty_vacuum_board();
            let original=board.prize_at;
            board.place_for_test(seat,original,40);
            board.suck(seat).unwrap();
            for _ in 0..300 {
                if !matches!(board.step(),TurnStep::Vacuum{..}) || board.vacuum.chest_blown {break;}
                let chest=&board.layout[original as usize];
                let (dx,dz)=Board::chest_drift(board.vacuum.out+1.0/60.0);
                let from=board.rendered_spot_of(seat);
                let yaw=(chest.x+dx-from.0).atan2(chest.z+dz-from.1)-12.0f32.to_radians();
                board.aim_vacuum_with(seat,yaw,true,blow).unwrap();board.vacuum_tick(1.0/60.0);
            }
            if blow {
                assert!(board.vacuum.chest_blown,"a chest at the visible edge can be blown without a hidden narrower cone");
                assert_ne!(board.prize_at,original);
            } else {
                assert_eq!(board.tokens.iter().find(|token|token.seat==seat).unwrap().prizes,1,"a chest at the visible edge can be collected before the battery runs out");
            }
        }
    }

    #[test]
    fn crate_and_item_box_targets_match_their_visible_off_tile_positions() {
        let (mut board,seat)=empty_vacuum_board();
        board.crates.insert(1,0);
        let from=board.rendered_spot_of(seat);let target=board.crate_spot(1);
        board.suck(seat).unwrap();board.aim_vacuum(seat,(target.0-from.0).atan2(target.1-from.1),true).unwrap();
        for _ in 0..240 {board.vacuum_tick(1.0/60.0);}
        assert!(board.tokens.iter().find(|token|token.seat==seat).unwrap().carrying,"aiming at the visible crate picks it up");
        let (mut board,seat)=empty_vacuum_board();
        let item=board.layout.iter().find(|space|space.kind==SpaceKind::Item && space.id>0).unwrap().id;
        board.place_for_test(seat,item-1,10);board.boxes_taken.remove(&item);
        let from=board.rendered_spot_of(seat);let target=board.box_spot(item);
        board.suck(seat).unwrap();board.aim_vacuum(seat,(target.0-from.0).atan2(target.1-from.1),true).unwrap();
        for _ in 0..300 {board.vacuum_tick(1.0/60.0);}
        assert!(board.boxes_taken.contains(&item),"aiming at the visible purple-space box opens it");
    }

    #[test]
    fn every_owned_upgrade_collects_faster_in_both_vacuum_modes() {
        fn collect_ticks(upgrade:Option<Upgrade>,walking:bool)->usize {
            let (mut board,seat)=empty_vacuum_board();
            if let Some(upgrade)=upgrade{board.upgrade_for_test(seat,upgrade);}
            let (x,z)=board.spot_of(seat);
            board.air_coins.push(AirCoin{view:AirCoinView{id:999,space:0,x:x+6.0,z,height:1.5,coins:1,available:true,collected_by:None}});
            if walking {
                board.start_walk(seat,0,vec![1,2],false,walking::AfterWalk::Step(TurnStep::Done));
                assert!((board.walk_view().unwrap().rate-if upgrade.is_some(){1.15}else{1.0}).abs()<0.001);
            } else {board.suck(seat).unwrap();}
            for tick in 1..300 {
                if walking {
                    let view=board.walk_view().unwrap();let coin=&view.coins[0];
                    board.aim_walk_vacuum(seat,(coin.x-view.x).atan2(coin.z-view.z),0.0,true).unwrap();board.walk_tick(1.0/60.0);
                } else {
                    board.aim_vacuum(seat,std::f32::consts::FRAC_PI_2,true).unwrap();board.vacuum_tick(1.0/60.0);
                }
                if board.air_coins[0].view.collected_by.is_some(){return tick;}
            }
            panic!("held target was not collected");
        }
        for walking in [false,true] {
            let baseline=collect_ticks(None,walking);
            for upgrade in Upgrade::ALL {assert!(collect_ticks(Some(upgrade),walking)<baseline,"{upgrade:?} must improve throughput in walking={walking}");}
        }
        let (mut board,seat)=empty_vacuum_board();
        for upgrade in Upgrade::ALL{board.upgrade_for_test(seat,upgrade);}
        assert!((board.pull_rate(seat)-1.6).abs()<0.001);
        assert_eq!(board.nozzle(seat).1,1.0,"a wider nozzle never weakens suction");
    }
}
