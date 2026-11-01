//! Timed walking and the airborne-coin aiming challenge. The server owns
//! movement, target windows and rewards; clients only send a held nozzle aim.
use super::*;

// A walking segment has enough time for a controller turn and a deliberate
// 0.16-second pull, including on a slower renderer. Clients use this value
// from WalkView for the same movement and countdown as the server.
const HOP_SECONDS: f32 = 0.72;
const DICE_SECONDS: f32 = 2.6;
const EYE_HEIGHT: f32 = 1.3;
const MAX_BONUS: u8 = 3;
const HOLD_SECONDS: f32 = 0.16;

#[derive(Clone, Debug, Serialize)]
pub struct WalkCoinView {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    /// Relative to the animated walker's feet, so aim stays consistent on hills.
    pub height: f32,
    pub active: bool,
    pub progress: f32,
    pub collected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalkView {
    pub id: u32,
    pub seat: u8,
    pub from: u8,
    pub path: Vec<u8>,
    pub elapsed: f32,
    pub waiting:f32,
    pub delay: f32,
    pub hop_seconds: f32,
    pub eye_height: f32,
    pub cone:f32,
    pub range:f32,
    pub rate:f32,
    pub x: f32,
    pub z: f32,
    /// The hop's vertical offset, in world units.
    pub hop: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub vacuum_on: bool,
    pub blowing: bool,
    pub power: f32,
    pub out: f32,
    pub pulling: Vec<(u16,f32)>,
    pub bonus: u8,
    pub earned:i32,
    pub max_bonus: u8,
    pub coins: Vec<WalkCoinView>,
}

pub(super) enum AfterWalk { Land { slide: bool }, Step(TurnStep) }
struct AirCoin { view: WalkCoinView, pull: f32, field_index:usize }
pub(super) struct Walk {
    id: u32, seat: u8, from: u8, path: Vec<u8>, elapsed: f32, delay: f32,
    yaw: f32, pitch: f32, on: bool, input_at: f32, waiting:f32, coins: Vec<AirCoin>, after: AfterWalk,
    motor: Vacuum,
}

#[cfg(test)]
#[path="walking_pickup_tests.rs"]
mod pickup_tests;

fn position(layout: &[Space], walk: &Walk) -> (f32, f32, f32) {
    if walk.elapsed <= walk.delay {
        let from = &layout[walk.from as usize];
        return (from.x, from.z, 0.0);
    }
    let movement = ((walk.elapsed - walk.delay) / HOP_SECONDS).max(0.0);
    let index = (movement.floor() as usize).min(walk.path.len() - 1);
    let t = (movement - index as f32).clamp(0.0, 1.0);
    let from = &layout[if index == 0 {walk.from} else {walk.path[index - 1]} as usize];
    let to = &layout[walk.path[index] as usize];
    if from.link == Some(to.id) { return (to.x, to.z, 0.0); }
    (from.x + (to.x - from.x) * t, from.z + (to.z - from.z) * t, if walk.waiting>0.0 {0.0}else{(t * std::f32::consts::PI).sin() * 0.7})
}

impl Board {
    pub(super) fn start_walk(&mut self, seat: u8, from: u8, path: Vec<u8>, initial: bool, after: AfterWalk) {
        if path.is_empty() { return; }
        if let Some(token) = self.token(seat) { token.facing = None; }
        self.next_walk_id += 1;
        let delay = if initial { DICE_SECONDS } else { 0.0 };
        let mut eligible = Vec::new();
        let mut before = from;
        for (index, to) in path.iter().copied().enumerate() {
            let a = &self.layout[before as usize];
            let b = &self.layout[to as usize];
            if a.link != Some(to) && ((b.x-a.x).powi(2)+(b.z-a.z).powi(2)).sqrt() < 9.0 {
                eligible.push((index, before, to));
            }
            before = to;
        }
        let mut coins=Vec::new();
        if self.walk_bonus<MAX_BONUS && self.ambient_earned<ambient::AMBIENT_TURN_CAP {
            for (field_index,coin) in self.air_coins.iter().enumerate().filter(|(index,_)|self.air_available(*index)) {
                let mut best=f32::INFINITY;
                for (_,a,b) in &eligible {
                    let (a,b)=(&self.layout[*a as usize],&self.layout[*b as usize]);
                    let (dx,dz)=(b.x-a.x,b.z-a.z);
                    let length=dx.hypot(dz).max(0.01);
                    let projection=((coin.view.x-a.x)*dx+(coin.view.z-a.z)*dz)/(length*length);
                    let t=projection.clamp(0.0,1.0);
                    let (cx,cz)=(coin.view.x-a.x-dx*t,coin.view.z-a.z-dz*t);
                    let distance=cx.hypot(cz);
                    best=best.min(distance);
                }
                if best<=VACUUM_RANGE {
                    coins.push(AirCoin{view:WalkCoinView{id:coin.view.id,x:coin.view.x,z:coin.view.z,height:coin.view.height,active:false,progress:0.0,collected:false},pull:0.0,field_index});
                }
            }
            coins.sort_by(|a,b|(a.view.x-self.layout[from as usize].x).hypot(a.view.z-self.layout[from as usize].z).total_cmp(&(b.view.x-self.layout[from as usize].x).hypot(b.view.z-self.layout[from as usize].z)));
        }
        let first = &self.layout[path[0] as usize];
        let at = &self.layout[from as usize];
        self.walk = Some(Walk {id:self.next_walk_id,seat,from,path,elapsed:0.0,delay,yaw:(first.x-at.x).atan2(first.z-at.z),pitch:0.0,on:false,input_at:-10.0,waiting:0.0,coins,after,motor:Vacuum::default()});
        self.step = TurnStep::Walking;
    }

    pub(super) fn walk_view(&self) -> Option<WalkView> {
        let walk = self.walk.as_ref()?;
        let (x,z,hop) = position(self.layout,walk);
        Some(WalkView {id:walk.id,seat:walk.seat,from:walk.from,path:walk.path.clone(),elapsed:walk.elapsed,waiting:walk.waiting,delay:walk.delay,hop_seconds:HOP_SECONDS,eye_height:EYE_HEIGHT,cone:VACUUM_CONE*self.nozzle(walk.seat).0,range:VACUUM_RANGE,rate:self.pull_rate(walk.seat),
            x,z,hop,yaw:walk.yaw,pitch:walk.pitch,vacuum_on:walk.on,blowing:walk.motor.blow,power:walk.motor.power,out:walk.motor.out,
            pulling:{let mut rows:Vec<_>=walk.motor.pulling.iter().map(|(key,pull)|(*key,pull.min(1.0))).collect();rows.sort_by_key(|row|row.0);rows},bonus:self.walk_bonus,earned:self.walk_earned,max_bonus:if self.floating_coins {MAX_BONUS}else{0},
            coins:walk.coins.iter().map(|coin| {
                let mut view=coin.view.clone();
                view.active=!view.collected && self.air_available(coin.field_index) && self.walk_bonus<MAX_BONUS && self.ambient_earned<ambient::AMBIENT_TURN_CAP && walk.elapsed>=walk.delay && (view.x-x).hypot(view.z-z)<=VACUUM_RANGE;
                view.progress=(coin.pull/HOLD_SECONDS).clamp(0.0,1.0);
                view
            }).collect()})
    }

    pub fn aim_walk_vacuum(&mut self, seat: u8, yaw: f32, pitch: f32, on: bool) -> Result<(), BoardError> {
        self.aim_walk_vacuum_with(seat,yaw,pitch,on,false)
    }

    pub fn aim_walk_vacuum_with(&mut self, seat: u8, yaw: f32, pitch: f32, on: bool, blow: bool) -> Result<(), BoardError> {
        if seat != self.turn_seat() { return Err(BoardError::NotYourTurn); }
        if !matches!(self.step,TurnStep::Walking) {return Err(BoardError::WrongStep);}
        let walk = self.walk.as_mut().ok_or(BoardError::WrongStep)?;
        if seat != walk.seat {return Err(BoardError::NotYourTurn);}
        if !yaw.is_finite() || !pitch.is_finite() {return Err(BoardError::OutOfReach);}
        walk.yaw=yaw.rem_euclid(std::f32::consts::TAU);
        walk.pitch=pitch.clamp(-1.4,1.4);
        walk.on=on;
        if on && blow!=walk.motor.blow {
            walk.motor.blow=blow;
            walk.motor.power=0.0;
            walk.motor.pulling.clear();
            walk.motor.siphon.clear();
            walk.motor.gust.clear();
            for coin in &mut walk.coins {coin.pull=0.0;}
        }
        walk.input_at=walk.elapsed;
        Ok(())
    }

    pub(super) fn brace_walk(&mut self,seat:u8,on:bool)->Result<(),BoardError> {
        if seat==self.turn_seat() {return Err(BoardError::WrongStep);}
        let walk=self.walk.as_mut().ok_or(BoardError::WrongStep)?;
        if on {walk.motor.braced.insert(seat);} else {walk.motor.braced.remove(&seat);}
        Ok(())
    }

    fn walking_effects(&mut self,walk:&mut Walk,dt:f32)->(bool,bool) {
        let (x,z,_)=position(self.layout,walk);
        let mut motor=std::mem::take(&mut walk.motor);
        motor.origin=Some((x,z));motor.yaw=walk.yaw;
        motor.on=walk.on && walk.elapsed>=walk.delay;
        if walk.elapsed>=walk.delay {motor.out+=dt;}
        let saved=std::mem::replace(&mut self.vacuum,motor);
        let before_space=self.space_of(walk.seat);
        let before_sequence=self.log_sequence;
        let changed=if self.vacuum.on {
            self.vacuum.power=(self.vacuum.power+dt/self.spin_up(walk.seat)).min(1.0);
            let power=self.vacuum.power;
            if self.vacuum.blow {self.blow_tick(walk.seat,dt,power)}
            else {self.suction_tick(walk.seat,dt,power,true).0}
        } else {
            self.vacuum.power=(self.vacuum.power-dt*3.0).max(0.0);
            self.vacuum.pulling.retain(|_,pull|{*pull-=dt*1.5;*pull>0.0});
            false
        };
        let interrupted=self.space_of(walk.seat)!=before_space || self.log.iter().any(|entry|entry["seq"].as_u64().is_some_and(|sequence|sequence>before_sequence) && entry["kind"]=="blown" && entry["seat"]==walk.seat);
        walk.motor=std::mem::replace(&mut self.vacuum,saved);
        (changed,interrupted)
    }

    pub fn walk_tick(&mut self, dt: f32) -> bool {
        let Some(mut walk)=self.walk.take() else {return false};
        if !dt.is_finite() || dt<=0.0 {self.walk=Some(walk);return false;}
        let before=position(self.layout,&walk);
        walk.elapsed+=dt.min(0.05);
        let next=position(self.layout,&walk);
        walk.waiting=self.traffic_wait((before.0,before.1),(next.0,next.1));
        if walk.waiting>0.0 {walk.elapsed-=dt.min(0.05);walk.input_at-=dt.min(0.05);}
        if walk.elapsed-walk.input_at>0.35 {walk.on=false;}
        let (x,z,_)=position(self.layout,&walk);
        let (widen,keep)=self.nozzle(walk.seat);
        let rate=self.pull_rate(walk.seat);
        let (mut changed,interrupted)=self.walking_effects(&mut walk,dt.min(0.05));
        if interrupted {
            self.step=TurnStep::Done;
            self.say(json!({"kind":"walk_interrupted","seat":walk.seat,"reason":"storm"}));
            return true;
        }
        for coin in &mut walk.coins {
            if coin.view.collected {continue;}
            let (dx,dz)=(coin.view.x-x,coin.view.z-z);
            let distance=(dx*dx+dz*dz).sqrt();
            let active=self.air_available(coin.field_index) && self.ambient_earned<ambient::AMBIENT_TURN_CAP && walk.elapsed>=walk.delay;
            let strength=vacuum_strength((x,z),walk.yaw,(coin.view.x,coin.view.z),widen).unwrap_or(0.0)*keep;
            if active && walk.on && !walk.motor.blow && distance<=VACUUM_RANGE && strength>0.0 {
                coin.pull+=dt.min(0.05)*strength*rate*walk.motor.power;
            } else {coin.pull=(coin.pull-dt*2.0).max(0.0);}
            if coin.pull>=HOLD_SECONDS && self.walk_bonus<MAX_BONUS {
                let earned=self.take_air_coin(walk.seat,coin.field_index);
                if earned==0 {continue;}
                coin.view.collected=true;
                self.walk_bonus+=1;
                self.walk_earned+=earned;
                self.say(json!({"kind":"walk_coin","seat":walk.seat,"coin_id":coin.view.id,"coins":earned,"bonus":self.walk_bonus,"earned":self.walk_earned}));
                changed=true;
            }
        }
        if walk.elapsed>=walk.delay+walk.path.len() as f32*HOP_SECONDS {
            self.step=TurnStep::Roll;
            match walk.after {AfterWalk::Land{slide}=>self.land_on(walk.seat,slide),AfterWalk::Step(step)=>self.step=step}
            true
        } else {self.walk=Some(walk);changed}
    }

    /// Practice opponents aim too; lower skill leaves more of the target
    /// outside the nozzle, rather than granting them automatic bonus coins.
    pub fn bot_walk_aim(&mut self, skill: f32) {
        let Some(walk)=self.walk.as_ref() else {return};
        let (x,z,_)=position(self.layout,walk);
        if let Some(at)=self.bot_pedestrian_aim((x,z)) {
            let wobble=(1.0-skill.clamp(0.0,1.0))*0.12*(walk.elapsed*3.0).sin();
            let (seat,yaw)=(walk.seat,(at.0-x).atan2(at.1-z)+wobble);
            let _=self.aim_walk_vacuum(seat,yaw,0.0,true);
            return;
        }
        let Some(coin)=walk.coins.iter().filter(|coin|!coin.view.collected && self.air_available(coin.field_index) && (coin.view.x-x).hypot(coin.view.z-z)<=VACUUM_RANGE).min_by(|a,b|(a.view.x-x).hypot(a.view.z-z).total_cmp(&(b.view.x-x).hypot(b.view.z-z))) else {return};
        let (dx,dz)=(coin.view.x-x,coin.view.z-z);
        let wobble=(1.0-skill.clamp(0.0,1.0))*0.8*(walk.elapsed*3.0).sin();
        let (seat,yaw,pitch)=(walk.seat,dx.atan2(dz)+wobble,(coin.view.height-EYE_HEIGHT).atan2((dx*dx+dz*dz).sqrt()));
        let _=self.aim_walk_vacuum(seat,yaw,pitch,true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT:f32=1.0/60.0;

    fn challenge() -> Board {
        let mut board=Board::on(Map::Island,&[0,1],6,6);
        let seat=board.turn_seat();
        board.timed_walk=true;
        board.start_walk(seat,0,vec![1,2,3,4,5],false,AfterWalk::Step(TurnStep::Done));
        board
    }

    fn track(board:&mut Board) {
        if let Some(view)=board.walk_view() {
            if let Some(coin)=view.coins.iter().find(|coin| coin.active && !coin.collected) {
                let (dx,dz)=(coin.x-view.x,coin.z-view.z);
                board.aim_walk_vacuum(view.seat,dx.atan2(dz),(coin.height-view.eye_height).atan2((dx*dx+dz*dz).sqrt()),true).unwrap();
            }
        }
        board.walk_tick(DT);
    }

    fn finish(board:&mut Board) {
        for _ in 0..1800 {
            if board.walk.is_none() {return;}
            // Advance the same signal clock as the room; a walker can wait for green.
            board.traffic_tick(DT);
            board.walk_tick(DT);
        }
        panic!("walk did not finish");
    }

    #[test]
    fn walking_nozzle_rejects_other_seats_nonfinite_aim_and_actions_after_landing() {
        let mut board=challenge();
        let seat=board.turn_seat();
        assert_eq!(board.aim_walk_vacuum(1-seat,0.0,0.0,true),Err(BoardError::NotYourTurn));
        assert_eq!(board.aim_walk_vacuum(seat,f32::NAN,0.0,true),Err(BoardError::OutOfReach));
        assert_eq!(board.aim_walk_vacuum(seat,0.0,f32::INFINITY,true),Err(BoardError::OutOfReach));
        assert_eq!(board.suck(seat),Err(BoardError::WrongStep),"standing and walking vacuums cannot overlap");
        assert!(!board.end_turn(),"a direct end-turn cannot skip movement");
        assert_eq!(board.turn_seat(),seat);
        finish(&mut board);
        assert_eq!(board.aim_walk_vacuum(seat,0.0,0.0,true),Err(BoardError::WrongStep));
    }

    #[test]
    fn airborne_coins_require_both_held_suction_and_accurate_aim() {
        let mut unattended=challenge();
        let before=unattended.tokens[0].coins;
        finish(&mut unattended);
        assert_eq!(unattended.tokens[0].coins,before,"walking past coins never auto-collects");
        let mut missed=challenge();
        for _ in 0..200 {
            if missed.walk.is_none() {break;}
            missed.aim_walk_vacuum(missed.turn_seat(),0.0,1.4,true).unwrap();
            missed.walk_tick(DT);
        }
        assert_eq!(missed.tokens[0].coins,before,"holding the nozzle away from targets earns nothing");
        assert_eq!(missed.walk_bonus,0);
    }

    #[test]
    fn tracked_coins_award_once_and_the_bonus_is_capped_across_walk_legs() {
        let mut board=challenge();
        let seat=board.turn_seat();
        let before=board.tokens.iter().find(|t|t.seat==seat).unwrap().coins;
        for _ in 0..600 {if board.walk.is_none(){break;}track(&mut board);}
        assert_eq!(board.walk_bonus,3,"precise tracking can collect all three targets");
        assert_eq!(board.tokens.iter().find(|t|t.seat==seat).unwrap().coins,before+3);
        let ids:Vec<u64>=board.view().log.iter().filter(|entry|entry["kind"]=="walk_coin").map(|entry|entry["coin_id"].as_u64().unwrap()).collect();
        assert_eq!(ids.len(),ids.iter().copied().collect::<HashSet<_>>().len(),"each reward has a unique collected target");
        board.start_walk(seat,5,vec![6,7,8],false,AfterWalk::Step(TurnStep::Done));
        assert!(board.walk_view().unwrap().coins.is_empty(),"a junction does not reset the roll's reward cap");
        for _ in 0..600 {if board.walk.is_none(){break;}track(&mut board);}
        assert_eq!(board.tokens.iter().find(|t|t.seat==seat).unwrap().coins,before+3);
    }

    #[test]
    fn accurately_tracking_at_ten_frames_per_second_can_collect_airborne_coins() {
        let mut board=challenge();
        // Aim is only refreshed once every six server ticks, as it would be
        // with a 10 FPS renderer. Server movement and rewards still tick at 60 Hz.
        for tick in 0..600 {
            if board.walk.is_none() {break;}
            if tick%6==0 {
                let view=board.walk_view().unwrap();
                if let Some(coin)=view.coins.iter().find(|coin|coin.active && !coin.collected) {
                    let (dx,dz)=(coin.x-view.x,coin.z-view.z);
                    board.aim_walk_vacuum(view.seat,dx.atan2(dz),(coin.height-view.eye_height).atan2((dx*dx+dz*dz).sqrt()),true).unwrap();
                }
            }
            board.walk_tick(DT);
        }
        assert_eq!(board.walk_bonus,MAX_BONUS,"a low frame rate must still allow deliberate accurate tracking");
        assert!(board.walk.is_none());
    }

    #[test]
    fn walking_and_standing_vacuums_share_the_field_and_turn_payout_budget() {
        let mut board=challenge();
        board.endless_vacuum=true;
        board.clear_crates_for_test();
        board.coconuts.fill(0);
        let seat=board.turn_seat();
        board.place_for_test(1-seat,40,0);
        for _ in 0..600 {if board.walk.is_none(){break;}track(&mut board);}
        assert_eq!(board.walk_bonus,3);
        assert_eq!(board.walk_earned,3);
        let caught:Vec<u32>=board.air_view().iter().filter(|coin|coin.collected_by==Some(seat)).map(|coin|coin.id).collect();
        assert_eq!(caught.len(),3);
        board.suck(seat).unwrap();
        for _ in 0..600 {
            let (x,z)=board.spot_of(seat);
            if let Some(coin)=board.air_view().into_iter().find(|coin|coin.available && (coin.x-x).hypot(coin.z-z)<=VACUUM_RANGE) {
                board.aim_vacuum(seat,(coin.x-x).atan2(coin.z-z),true).unwrap();
            }
            board.vacuum_tick(DT);
        }
        assert_eq!(board.ambient_earned,6,"standing can collect only the turn budget left after walking");
        let after:Vec<u32>=board.air_view().iter().filter(|coin|coin.collected_by==Some(seat)).map(|coin|coin.id).collect();
        assert_eq!(after.len(),6);
        assert!(caught.iter().all(|id|after.contains(id)),"walk claims remain consumed during standing suction");
        assert_eq!(board.walk_earned,3,"standing pickups do not rewrite the walking HUD's earned total");
    }

    #[test]
    fn walking_honors_the_full_horizontal_cone_without_hidden_pitch_or_time_windows() {
        for offset in [12.0f32,18.0] {
            let mut board=Board::on(Map::Island,&[0,1],6,1);
            board.air_coins.truncate(1);
            let from=&board.layout[0];
            board.air_coins[0].view.x=from.x+2.0;
            board.air_coins[0].view.z=from.z+12.0;
            let seat=board.turn_seat();
            board.start_walk(seat,0,vec![1,2,3,4,5],false,AfterWalk::Step(TurnStep::Done));
            let view=board.walk_view().unwrap();
            assert_eq!(view.range,VACUUM_RANGE);
            assert_eq!(view.cone,VACUUM_CONE);
            for _ in 0..50 {board.walk_tick(DT);}
            for _ in 0..160 {
                let Some(view)=board.walk_view() else {break};
                let coin=&view.coins[0];
                let yaw=(coin.x-view.x).atan2(coin.z-view.z)-offset.to_radians();
                board.aim_walk_vacuum(seat,yaw,-1.4,true).unwrap();
                board.walk_tick(DT);
            }
            assert_eq!(board.walk_bonus,if offset<VACUUM_CONE.to_degrees(){1}else{0},"the drawn horizontal cone alone decides acceptance, even after a second of travel and with downward cosmetic pitch");
        }
    }

    #[test]
    fn walking_targets_reference_the_same_shared_world_pickups() {
        let board=challenge();
        let walk=board.walk.as_ref().unwrap();
        for coin in &walk.coins {
            let field=&board.air_coins[coin.field_index].view;
            assert_eq!(coin.view.id,field.id);
            assert_eq!((coin.view.x,coin.view.z,coin.view.height),(field.x,field.z,field.height));
            assert!(board.air_available(coin.field_index));
        }
        assert!(!walk.coins.is_empty());
    }

    #[test]
    fn one_old_suction_packet_cannot_keep_collecting_after_input_stops() {
        let mut board=challenge();
        let seat=board.turn_seat();
        board.aim_walk_vacuum(seat,0.0,0.0,true).unwrap();
        for _ in 0..30 {board.walk_tick(DT);}
        assert!(!board.walk_view().unwrap().vacuum_on,"stale held inputs time out");
    }

    #[test]
    fn branch_choices_wait_for_arrival_then_continue_without_another_dice_pause() {
        let mut board=Board::on(Map::Island,&[0,1],6,1);
        board.timed_walk=true;
        let layout=board.layout;
        let (before,at)=layout.iter().filter(|s|s.path>0 && !s.under && !s.gated && s.link.is_none() && s.next.len()==1)
            .find_map(|s|{let at=s.next[0];(layout[at as usize].path==0 && layout[at as usize].kind==SpaceKind::Coin).then_some((s.id,at))}).unwrap();
        let seat=board.turn_seat();
        board.place_for_test(seat,before,20);
        board.fixed_roll=Some(3);
        board.roll(seat).unwrap();
        assert_eq!(board.step(),&TurnStep::Walking);
        assert_eq!(board.walk_view().unwrap().path,vec![at]);
        assert_eq!(board.walk_view().unwrap().delay,DICE_SECONDS);
        assert_eq!(board.choose_branch(seat,layout[at as usize].next[0]),Err(BoardError::WrongStep));
        finish(&mut board);
        let TurnStep::Branch{options,left}=board.step().clone() else {panic!("route choice appears on arrival")};
        assert_eq!(left,2);
        assert!(!options.iter().any(|option|option.node==before));
        board.choose_branch(seat,options[0].node).unwrap();
        assert_eq!(board.step(),&TurnStep::Walking);
        assert_eq!(board.walk_view().unwrap().delay,0.0);
        assert!(board.walk_view().unwrap().path.iter().all(|id|!layout[*id as usize].gated));
        finish(&mut board);
    }

    #[test]
    fn timed_walks_respect_closed_gates_and_open_the_same_route_in_late_rounds() {
        let mut board=Board::on(Map::Frozen,&[0,1],6,1);
        board.timed_walk=true;
        let layout=board.layout;
        let (from,gate)=layout.iter().filter(|space|!space.gated).find_map(|space|
            space.next.iter().copied().find(|to|layout[*to as usize].gated).map(|gate|(space.id,gate))
        ).unwrap();
        let seat=board.turn_seat();
        board.place_for_test(seat,from,20);
        assert!(!board.walk_ways(seat,from).contains(&gate));
        board.fixed_roll=Some(1);
        board.roll(seat).unwrap();
        if let TurnStep::Branch{options,..}=board.step().clone() {
            assert!(!options.iter().any(|option|option.node==gate));
            board.choose_branch(seat,options[0].node).unwrap();
        }
        let view=board.walk_view().expect("the open route starts a timed walk");
        assert!(view.path.iter().all(|node|!layout[*node as usize].gated));
        finish(&mut board);
        board.place_for_test(seat,from,20);
        board.step=TurnStep::Roll;
        board.round=board.gates_open_from().unwrap();
        board.fixed_roll=Some(1);
        board.roll(seat).unwrap();
        let TurnStep::Branch{options,..}=board.step().clone() else {panic!("the newly opened route is offered")};
        assert!(options.iter().any(|option|option.node==gate));
        board.choose_branch(seat,gate).unwrap();
        assert_eq!(board.walk_view().unwrap().path,vec![gate]);
        finish(&mut board);
    }

    #[test]
    fn portal_hops_wait_for_the_die_and_do_not_spawn_coins_across_the_gap() {
        let mut board=Board::on(Map::Island,&[0,1],6,1);
        let from=board.layout.iter().find(|space|space.link.is_some()).unwrap();
        let to=from.link.unwrap();
        let (from_id,x,z)=(from.id,from.x,from.z);
        let seat=board.turn_seat();
        board.start_walk(seat,from_id,vec![to],true,AfterWalk::Step(TurnStep::Done));
        let before=board.walk_view().unwrap();
        assert_eq!((before.x,before.z),(x,z),"the die pause stays at the portal entrance");
        assert!(before.coins.is_empty(),"portal gaps have no physical target corridor");
        for _ in 0..165 {board.walk_tick(DT);}
        let hopped=board.walk_view().unwrap();
        assert_eq!((hopped.x,hopped.z),(board.layout[to as usize].x,board.layout[to as usize].z));
        assert_eq!(hopped.hop,0.0,"teleporting never arcs through the intervening scenery");
        finish(&mut board);
    }

    #[test]
    fn real_room_rolls_open_and_close_a_server_timed_walking_window() {
        use crate::room::{Room,RoomError,DevAction};
        let mut room=Room::new("walking-window",7);
        room.connect("alice","Alice","").unwrap();
        room.connect("bob","Bob","").unwrap();
        room.choose_map("alice",Map::Island).unwrap();
        room.dev_tools=true;
        room.start_sandbox("alice").unwrap();
        room.board.as_mut().unwrap().place_for_test(0,0,START_COINS);
        room.dev("alice",DevAction::Give{item:Item::GoldenDie}).unwrap();
        room.use_item("alice",Item::GoldenDie,None,Some(2)).unwrap();
        room.check("alice",0.5).unwrap();
        room.roll("alice").unwrap();
        let view=room.view().board.unwrap();
        assert_eq!(view.turn,TurnStep::Walking);
        assert_eq!(view.walk.as_ref().unwrap().delay,DICE_SECONDS);
        assert!(matches!(room.aim_walk_vacuum("bob",0.0,0.0,true),Err(RoomError::Board(BoardError::NotYourTurn))));
        assert!(matches!(room.end_turn("alice"),Err(RoomError::Board(BoardError::WrongStep))));
        for _ in 0..360 {
            room.step();
            if room.view().board.as_ref().is_some_and(|board|board.walk.is_none()){break;}
        }
        let view=room.view().board.unwrap();
        assert!(view.walk.is_none());
        assert_ne!(view.turn,TurnStep::Walking);
        assert!(!view.sucked,"walk suction does not consume the standing vacuum");
    }

    #[test]
    fn a_real_room_ignores_in_flight_walk_aim_after_arrival() {
        use crate::room::{Room,RoomError,DevAction};
        let mut room=Room::new("late-walk-packets",7);
        room.connect("alice","Alice","").unwrap();
        room.connect("bob","Bob","").unwrap();
        room.choose_map("alice",Map::Island).unwrap();
        room.dev_tools=true;
        room.start_sandbox("alice").unwrap();
        room.board.as_mut().unwrap().place_for_test(0,0,START_COINS);
        room.dev("alice",DevAction::Give{item:Item::GoldenDie}).unwrap();
        room.use_item("alice",Item::GoldenDie,None,Some(2)).unwrap();
        room.check("alice",0.5).unwrap();
        room.roll("alice").unwrap();
        assert!(matches!(room.aim_walk_vacuum("bob",0.0,0.0,true),Err(RoomError::Board(BoardError::NotYourTurn))));
        for _ in 0..360 {
            room.step();
            if room.view().board.as_ref().is_some_and(|board|board.walk.is_none()){break;}
        }
        let before=serde_json::to_value(room.view().board.unwrap()).unwrap();
        for _ in 0..3 {
            room.aim_walk_vacuum("alice",1.2,0.3,true).unwrap();
            room.aim_walk_vacuum("bob",0.0,0.0,false).unwrap();
        }
        assert_eq!(serde_json::to_value(room.view().board.unwrap()).unwrap(),before,"late inputs cannot restart walking, alter a prompt, or award coins");
        assert!(matches!(room.aim_walk_vacuum("nonmember",0.0,0.0,true),Err(RoomError::WrongPhase)));
        room.suck("alice").unwrap();
        assert!(matches!(room.aim_vacuum("bob",0.0,true,false),Err(RoomError::Board(BoardError::NotYourTurn))));
        room.stow_vacuum("alice").unwrap();
        let before=serde_json::to_value(room.view().board.unwrap()).unwrap();
        room.aim_vacuum("alice",1.0,true,false).unwrap();
        room.aim_vacuum("bob",1.0,false,false).unwrap();
        assert_eq!(serde_json::to_value(room.view().board.unwrap()).unwrap(),before,"late stationary aim after a forced stow is harmless too");
    }
}
