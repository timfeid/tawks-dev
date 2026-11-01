//! Timed free-moving actors. Own-turn actions reserve their exact prior step;
//! off-turn actors leave the active turn alone. Hits stay server authoritative.
use super::*;

pub const RAID_SECONDS:f32=10.0;
pub const RAID_RANGE:f32=2.8;
pub const RAID_ARC:f32=0.8;
pub const RAID_COOLDOWN:f32=0.6;
const INPUT_TTL:f32=0.35;

#[cfg(test)]
#[path="raids_tests.rs"]
mod tests;

#[derive(Clone,Debug,Serialize)]
pub struct RaidView {
    pub item:Item,pub ammo:u8,pub shot_cooldown:f32,pub hits:Vec<(u8,u8)>,
    pub seat:u8,
    pub x:f32,
    pub z:f32,
    pub yaw:f32,
    pub home_space:u8,
    pub home_x:f32,
    pub home_z:f32,
    pub remaining:f32,
    pub speed:f32,
    pub radius:f32,
    pub cooldown:f32,
    pub slashing:f32,
}

pub(super) struct Raid {
    item:Item,ammo:u8,shot_cooldown:f32,firing:bool,fire_age:f32,bullet_hits:HashMap<u8,u8>,
    pub(super) x:f32,
    pub(super) z:f32,
    yaw:f32,
    home_space:u8,
    home:(f32,f32),
    remaining:f32,
    input:(f32,f32),
    input_age:f32,
    cooldown:f32,
    slashing:f32,
    hit:HashSet<u8>,
    after:Option<TurnStep>,
    turn:(u32,usize),
}

impl Board {
    pub fn has_raids(&self)->bool {!self.raids.is_empty()}
    pub fn has_raid_for(&self,seat:u8)->bool {self.raids.contains_key(&seat)}
    pub(super) fn raid_views(&self)->Vec<RaidView> {
        let mut views:Vec<_>=self.raids.iter().map(|(seat,raid)|RaidView {item:raid.item,ammo:raid.ammo,shot_cooldown:raid.shot_cooldown,hits:{let mut hits:Vec<_>=raid.bullet_hits.iter().map(|(seat,hits)|(*seat,*hits)).collect();hits.sort();hits},seat:*seat,x:raid.x,z:raid.z,yaw:raid.yaw,home_space:raid.home_space,home_x:raid.home.0,home_z:raid.home.1,remaining:raid.remaining,speed:positioning::POSITION_SPEED,radius:self.movement_extent(),cooldown:raid.cooldown,slashing:raid.slashing}).collect();
        views.sort_by_key(|view|view.seat);
        views
    }

    pub fn start_raid(&mut self,seat:u8)->Result<(),BoardError> {
        self.start_raid_item(seat,Item::SlashRush)
    }
    pub fn start_raid_item(&mut self,seat:u8,item:Item)->Result<(),BoardError> {
        if !matches!(item,Item::SlashRush|Item::DriveBy){return Err(BoardError::NoSuchItem);}
        if self.raids.contains_key(&seat) {return Err(BoardError::WrongStep);}
        let own_turn=seat==self.turn_seat();
        if item==Item::DriveBy&&!own_turn{return Err(BoardError::NotYourTurn);}
        if own_turn&&!matches!(self.step,TurnStep::Items|TurnStep::Done) {return Err(BoardError::WrongStep);}
        let token=self.tokens.iter().find(|token|token.seat==seat).ok_or(BoardError::UnknownTarget)?;
        if token.resting>0 {return Err(BoardError::WrongStep);}
        let slot=token.items.iter().position(|held|*held==item).ok_or(BoardError::NoSuchItem)?;
        let home=self.rendered_spot_of(seat);
        let home_space=token.space;
        let yaw=token.facing.unwrap_or(0.0);
        let token=self.token(seat).unwrap();
        token.items.remove(slot);
        token.stats.items_used+=1;
        self.raids.insert(seat,Raid {item,ammo:if item==Item::DriveBy{10}else{0},shot_cooldown:0.,firing:false,fire_age:1.,bullet_hits:HashMap::new(),x:home.0,z:home.1,yaw,home_space,home,remaining:RAID_SECONDS,input:(0.0,0.0),input_age:INPUT_TTL,cooldown:0.0,slashing:0.0,hit:HashSet::new(),after:own_turn.then(||self.step.clone()),turn:(self.round,self.turn_index)});
        if own_turn {self.step=TurnStep::Raiding;}
        self.say(json!({"kind":"raid_start","seat":seat,"item":item,"x":home.0,"z":home.1,"yaw":yaw,"seconds":RAID_SECONDS,"own_turn":own_turn}));
        Ok(())
    }

    pub fn raid_input(&mut self,seat:u8,x:f32,z:f32,yaw:f32)->Result<(),BoardError> {
        if !x.is_finite() || !z.is_finite() || !yaw.is_finite() {return Err(BoardError::OutOfReach);}
        let raid=self.raids.get_mut(&seat).ok_or(BoardError::WrongStep)?;
        let length=x.hypot(z).max(1.0);
        raid.input=(x/length,z/length);
        raid.yaw=yaw.rem_euclid(std::f32::consts::TAU);
        raid.input_age=0.0;
        Ok(())
    }

    pub fn raid_fire(&mut self,seat:u8,on:bool,yaw:f32)->Result<(),BoardError> {
        if !yaw.is_finite(){return Err(BoardError::OutOfReach);}
        let raid=self.raids.get_mut(&seat).filter(|raid|raid.item==Item::DriveBy).ok_or(BoardError::WrongStep)?;
        raid.yaw=yaw.rem_euclid(std::f32::consts::TAU);raid.firing=on;raid.fire_age=0.;Ok(())
    }

    fn fire_raid_shot(&mut self,seat:u8,eligible:&[u8]) {
        let Some(raid)=self.raids.get_mut(&seat).filter(|raid|raid.item==Item::DriveBy&&raid.ammo>0&&raid.shot_cooldown<=0.0001)else{return;};
        let (origin,yaw)=((raid.x,raid.z),raid.yaw);raid.ammo-=1;raid.shot_cooldown=0.8;
        let ammo=raid.ammo;let direction=(yaw.sin(),yaw.cos());
        let hit=self.ray_rival_eligible(seat,origin,direction,shots::DRIVE_BY_RANGE,Some(eligible));
        let mut at=(origin.0+direction.0*shots::DRIVE_BY_RANGE,origin.1+direction.1*shots::DRIVE_BY_RANGE);
        let mut damage=0;let mut count=0;let mut blocked=false;let mut result=Vec::new();
        if let Some((target,point))=hit {
            at=point;
            let hits=self.raids.get_mut(&seat).unwrap().bullet_hits.entry(target).or_default();*hits+=1;count=*hits;
            if *hits==3 {
                *hits=0;blocked=self.blocked(seat,target,Item::DriveBy);
                if !blocked {
                    damage=1;
                    let loss=self.tokens.iter().find(|token|token.seat==target).map(|token|token.coins.min(3)).unwrap_or(0);
                    self.coins(target,-loss);self.spill(target,loss);self.hurt(target,1);self.crossed(target,seat);
                    result.push(json!({"seat":target,"shots":3,"coins":loss,"damage":1}));
                }
            }
        }
        let target=hit.map(|hit|hit.0);let flight=(at.0-origin.0).hypot(at.1-origin.1)/55.;
        self.say(json!({"kind":"item","item":"drive_by","by":seat,"origin_x":origin.0,"origin_z":origin.1,"x":at.0,"z":at.1,"yaw":yaw,"quality":if blocked{"blocked"}else if target.is_some(){"hit"}else{"miss"},"shots":[{"target":target,"x":at.0,"z":at.1,"blocked":blocked,"delay":0,"flight":flight.max(0.12)}],"hits":result,"flight":flight.max(0.12),"target":target}));
        self.say(json!({"kind":"raid_shot","seat":seat,"target":target,"hit_count":count,"damage":damage,"blocked":blocked,"ammo":ammo,"x":at.0,"z":at.1,"origin_x":origin.0,"origin_z":origin.1,"yaw":yaw}));
        if ammo==0 {self.end_raid(seat,"ammo");}
    }

    /// Eligibility is supplied by the authenticated room, never by socket data.
    pub fn raid_slash(&mut self,seat:u8,eligible:&[u8])->Result<(),BoardError> {
        if !eligible.contains(&seat) {return Err(BoardError::WrongStep);}
        if self.tokens.iter().all(|token|token.seat!=seat || token.resting>0) {return Err(BoardError::WrongStep);}
        let raid=self.raids.get(&seat).ok_or(BoardError::WrongStep)?;
        if raid.item!=Item::SlashRush{return Err(BoardError::WrongStep);}
        if raid.cooldown>0.0001 || raid.remaining<=0.0 {return Err(BoardError::WrongStep);}
        let (origin,yaw)=((raid.x,raid.z),raid.yaw);
        let mut targets:Vec<_>=self.tokens.iter().filter(|token|eligible.contains(&token.seat) && token.resting==0 && self.rival(seat,token.seat) && !raid.hit.contains(&token.seat))
            .filter_map(|token| {
                let at=self.raid_target_spot(token.seat);
                let (dx,dz)=(at.0-origin.0,at.1-origin.1);
                let distance=dx.hypot(dz);
                let off=(dx.atan2(dz)-yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
                (distance<=RAID_RANGE && (distance<0.6 || off.abs()<=RAID_ARC)).then_some((token.seat,at))
            }).collect();
        targets.sort_by_key(|(target,_)|*target);
        let raid=self.raids.get_mut(&seat).unwrap();
        raid.cooldown=RAID_COOLDOWN;raid.slashing=0.18;
        for (target,_) in &targets {raid.hit.insert(*target);}
        self.say(json!({"kind":"raid_slash","seat":seat,"x":origin.0,"z":origin.1,"origin_x":origin.0,"origin_z":origin.1,"yaw":yaw,"quality":if targets.is_empty(){"miss"}else{"swing"}}));
        for (target,at) in targets {
            if self.blocked(seat,target,Item::SlashRush) {
                self.say(json!({"kind":"raid_hit","seat":seat,"target":target,"x":origin.0,"z":origin.1,"target_x":at.0,"target_z":at.1,"yaw":yaw,"quality":"blocked","damage":0,"coins":0}));
                continue;
            }
            let lost=self.tokens.iter().find(|token|token.seat==target).map(|token|token.coins.min(3)).unwrap_or(0);
            self.coins(target,-lost);
            self.spill(target,lost);
            self.hurt(target,1);
            self.crossed(target,seat);
            self.say(json!({"kind":"raid_hit","seat":seat,"target":target,"x":origin.0,"z":origin.1,"origin_x":origin.0,"origin_z":origin.1,"target_x":at.0,"target_z":at.1,"yaw":yaw,"quality":"hit","damage":1,"coins":lost,"mature":self.mature}));
        }
        Ok(())
    }

    pub fn end_raid(&mut self,seat:u8,reason:&str) {
        if let Some(raid)=self.raids.remove(&seat) {
            if let Some(after)=raid.after {
                if raid.turn==(self.round,self.turn_index)&&seat==self.turn_seat()&&matches!(self.step,TurnStep::Raiding) {
                    self.step=if reason=="knockout" {TurnStep::Done}else{after};
                }
            }
            self.say(json!({"kind":"raid_end","seat":seat,"x":raid.x,"z":raid.z,"yaw":raid.yaw,"home_space":raid.home_space,"reason":reason}));
        }
    }

    pub(super) fn clear_raids(&mut self,reason:&str) {
        let seats:Vec<_>=self.raids.keys().copied().collect();
        for seat in seats {self.end_raid(seat,reason);}
    }

    pub fn raids_tick(&mut self,dt:f32,eligible:&[u8])->bool {
        if !dt.is_finite() || dt<=0.0 {return false;}
        let dt=dt.min(0.05);
        let mut changed=false;
        let mut seats:Vec<_>=self.raids.keys().copied().collect();seats.sort();
        for seat in seats {
            let Some(mut raid)=self.raids.remove(&seat) else {continue;};
            let awake=self.tokens.iter().any(|token|token.seat==seat && token.resting==0);
            raid.remaining=(raid.remaining-dt).max(0.0);
            let reason=if !eligible.contains(&seat){Some("disconnect")}else if !awake{Some("knockout")}else if seat==self.turn_seat()&&raid.after.is_none(){Some("turn")}else if self.space_of(seat)!=raid.home_space{Some("displaced")}else if raid.remaining<=0.0001{Some("timeout")}else{None};
            if let Some(reason)=reason {self.raids.insert(seat,raid);self.end_raid(seat,reason);changed=true;continue;}
            raid.input_age+=dt;raid.fire_age+=dt;raid.cooldown=(raid.cooldown-dt).max(0.0);raid.shot_cooldown=(raid.shot_cooldown-dt).max(0.);raid.slashing=(raid.slashing-dt).max(0.0);
            let before=(raid.x,raid.z);
            if raid.input_age<=INPUT_TTL {(raid.x,raid.z)=self.bounded_position_step(before,raid.home,raid.home_space,raid.input,dt);}
            changed|=before!=(raid.x,raid.z);
            let shoot=raid.item==Item::DriveBy&&raid.firing&&raid.ammo>0&&raid.shot_cooldown<=0.0001&&(raid.input_age<=INPUT_TTL||raid.fire_age<=INPUT_TTL);
            self.raids.insert(seat,raid);
            if shoot {self.fire_raid_shot(seat,eligible);changed=true;}
        }
        changed
    }

    /// Positioning, moving turns and other raiders are all real target origins.
    fn raid_target_spot(&self,seat:u8)->(f32,f32) {
        self.actual_spot_of(seat)
    }

    pub fn bot_raids_tick(&mut self,bots:&[u8],eligible:&[u8]) {
        for &seat in bots {
            if (seat==self.turn_seat()&&!self.raids.contains_key(&seat)) || !eligible.contains(&seat) {continue;}
            let token=self.tokens.iter().find(|token|token.seat==seat);
            if token.is_none_or(|token|token.resting>0) {continue;}
            let origin=self.rendered_spot_of(seat);
            let already=self.raids.get(&seat).map(|raid|&raid.hit);
            let target=self.tokens.iter().filter(|token|eligible.contains(&token.seat) && token.resting==0 && self.rival(seat,token.seat) && already.is_none_or(|hit|!hit.contains(&token.seat)))
                .filter(|token|token.coins>0)
                .map(|token|(token,self.raid_target_spot(token.seat)))
                .filter(|(_,at)|(at.0-origin.0).hypot(at.1-origin.1)<=positioning::POSITION_RADIUS+RAID_RANGE)
                .max_by_key(|(token,_)|(token.prizes,token.coins)).map(|(_,at)|at);
            if !self.raids.contains_key(&seat) {
                if target.is_none() || !token.is_some_and(|token|token.items.contains(&Item::SlashRush)) {continue;}
                if self.start_raid(seat).is_err() {continue;}
            }
            let Some(at)=target else {let _=self.raid_input(seat,0.0,0.0,0.0);continue;};
            let raid=self.raids.get(&seat).unwrap();
            let (dx,dz)=(at.0-raid.x,at.1-raid.z);
            let distance=dx.hypot(dz).max(0.001);
            let gun=raid.item==Item::DriveBy;
            let moving=distance>if gun{10.0}else{RAID_RANGE*0.7};
            let _=self.raid_input(seat,if moving{dx/distance}else{0.0},if moving{dz/distance}else{0.0},dx.atan2(dz));
            if gun {let _=self.raid_fire(seat,true,dx.atan2(dz));} else {let _=self.raid_slash(seat,eligible);}
        }
    }
}
