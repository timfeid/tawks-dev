//! World-space finales: ordered checkpoints on the actual board, with one
//! distinct rule per map. Inputs are sticks only; progress and hits are server-owned.
use super::*;
use crate::bout::Stick;

const SPEED:f32=6.0;
const DURATION:f32=75.0;
const READY:f32=3.0;
const RADIUS:f32=1.8;

#[cfg(test)]
#[path="map_finale_tests.rs"]
mod tests;

#[derive(Clone,Debug,Serialize)]
pub struct FinaleCheckpoint {pub x:f32,pub z:f32,pub radius:f32,pub blocked:bool,pub via:Vec<(f32,f32)>}
#[derive(Clone,Debug,Serialize)]
pub struct FinaleObstacle {
    pub kind:&'static str,pub x:f32,pub z:f32,pub yaw:f32,pub half_width:f32,pub half_depth:f32,
}
impl FinaleObstacle {
    fn contains(&self,at:(f32,f32),margin:f32)->bool {
        let dx=at.0-self.x;let dz=at.1-self.z;let (sin,cos)=self.yaw.sin_cos();
        (dx*cos-dz*sin).abs()<=self.half_width+margin && (dx*sin+dz*cos).abs()<=self.half_depth+margin
    }
}
#[derive(Clone,Debug,Serialize)]
pub struct FinaleRacer {
    pub seat:u8,pub x:f32,pub z:f32,pub yaw:f32,pub next_checkpoint:usize,pub waypoint:usize,
    pub finished:bool,pub dnf:bool,pub finish_time:Option<f32>,pub penalties:u8,
    pub stun:f32,pub gauge:f32,pub waiting:bool,pub place:usize,pub slipping:bool,
}
#[derive(Clone,Debug,Serialize)]
pub struct MapFinaleView {
    pub map:Map,pub title:&'static str,pub rules:&'static str,
    pub elapsed:f32,pub remaining:f32,pub duration:f32,pub ready:f32,pub speed:f32,
    pub checkpoints:Vec<FinaleCheckpoint>,pub racers:Vec<FinaleRacer>,pub tide_high:bool,pub obstacles:Vec<FinaleObstacle>,
}
struct Gate {at:(f32,f32),via:Vec<(f32,f32)>,space:u8,wet:bool}
struct Runner {view:FinaleRacer,waypoint:usize,velocity:(f32,f32)}
pub(super) struct MapFinale {map:Map,time:f32,gates:Vec<Gate>,runners:Vec<Runner>,obstacles:Vec<FinaleObstacle>,complete:bool}

fn metadata(map:Map)->(&'static str,&'static str) {
    match map {
        Map::Frozen=>("City Sprint","Weave around roadworks, steer through slippery ice, and dodge city traffic. Follow the green arrows; a car hit sends you to your last checkpoint."),
        Map::Island=>("Coconut Carry","Carry your coconut around the island. Smooth turns keep it balanced; frantic turns drop it and send you back."),
        Map::Volcano=>("Lava Run","Reach each cooling checkpoint before your heat fills. Overheating sends you back to the last checkpoint."),
        Map::Shipwreck=>("Tide Dash","Race the causeways. Marked checkpoints open only at low tide, while the high tide pushes you sideways."),
    }
}
fn nearest(layout:&[Space],at:(f32,f32))->u8 {
    layout.iter().filter(|space|!space.under).min_by(|a,b|(a.x-at.0).hypot(a.z-at.1).total_cmp(&(b.x-at.0).hypot(b.z-at.1))).map(|space|space.id).unwrap_or(0)
}
fn course(map:Map,layout:&[Space])->Vec<Gate> {
    if map==Map::Frozen {
        return [(0.,16.),(16.,16.),(40.,16.),(40.,-16.),(16.,-16.),(-16.,-16.),(-40.,-16.),(-40.,16.),(0.,16.)].into_iter().map(|at|Gate{at,via:vec![at],space:nearest(layout,at),wet:false}).collect();
    }
    let lap:Vec<_>=layout.iter().filter(|space|space.path==0&&!space.under).collect();
    let mut gates=vec![Gate{at:(lap[0].x,lap[0].z),via:vec![],space:lap[0].id,wet:false}];
    let mut previous=0;
    for end in (5..lap.len()).step_by(5).chain(std::iter::once(lap.len())) {
        let at=lap[end%lap.len()];
        let via=(previous+1..=end).map(|index|{let at=lap[index%lap.len()];(at.x,at.z)}).collect();
        gates.push(Gate{at:(at.x,at.z),via,space:at.id,wet:map==Map::Shipwreck && gates.len()%2==1});
        previous=end;
    }
    gates
}
// Race-only roadworks stay on the asphalt. The authoritative route threads
// both sides of alternating barriers, leaving at least a body-wide clear lane.
fn city_obstacles(gates:&mut [Gate])->Vec<FinaleObstacle> {
    let mut obstacles=Vec::new();
    for index in 1..gates.len() {
        let start=gates[index-1].at;let end=gates[index].at;
        let length=(end.0-start.0).hypot(end.1-start.1);let forward=((end.0-start.0)/length,(end.1-start.1)/length);
        let side=(forward.1,-forward.0);let yaw=forward.0.atan2(forward.1);
        let at=|distance:f32,offset:f32|(start.0+forward.0*distance+side.0*offset,start.1+forward.1*distance+side.1*offset);
        let positions:Vec<f32>=if length>=30. {vec![length*0.3,length*0.7]}else{vec![length*0.45]};
        let mut via=Vec::new();
        for (number,distance) in positions.into_iter().enumerate() {
            let offset=if (index+number)%2==0 {0.9}else{-0.9};let center=at(distance,offset);
            obstacles.push(FinaleObstacle{kind:"barrier",x:center.0,z:center.1,yaw,half_width:1.3,half_depth:0.35});
            for d in [distance-2.,distance+2.] {via.push(at(d,-offset.signum()*2.));}
        }
        if length>=24. {
            let center=at(if length>=30. {length*0.5}else{length*0.76},0.);
            obstacles.push(FinaleObstacle{kind:"ice",x:center.0,z:center.1,yaw,half_width:1.5,half_depth:2.0});
        }
        via.push(end);gates[index].via=via;
    }
    obstacles
}
fn race_ground(board:&Board,at:(f32,f32),space:u8,obstacles:&[FinaleObstacle])->bool {
    board.position_ground(at,space) && !obstacles.iter().any(|obstacle|obstacle.kind=="barrier"&&obstacle.contains(at,0.32))
}
fn distance_to_segment(at:(f32,f32),from:(f32,f32),to:(f32,f32))->f32 {
    let d=(to.0-from.0,to.1-from.1);
    let t=(((at.0-from.0)*d.0+(at.1-from.1)*d.1)/(d.0*d.0+d.1*d.1).max(0.00001)).clamp(0.,1.);
    (at.0-from.0-d.0*t).hypot(at.1-from.1-d.1*t)
}
fn tide_high(map:Map,elapsed:f32)->bool {map==Map::Shipwreck && elapsed.rem_euclid(8.)<3.5}
fn rank(race:&MapFinale)->Vec<usize> {
    let mut order:Vec<_>=(0..race.runners.len()).collect();
    let distance=|runner:&Runner|{let target=race.gates[runner.view.next_checkpoint.min(race.gates.len()-1)].at;(runner.view.x-target.0).hypot(runner.view.z-target.1)};
    order.sort_by(|&a,&b|{
        let (a,b)=(&race.runners[a],&race.runners[b]);
        b.view.finished.cmp(&a.view.finished)
            .then_with(||if a.view.finished&&b.view.finished {a.view.finish_time.unwrap().total_cmp(&b.view.finish_time.unwrap())}else{a.view.dnf.cmp(&b.view.dnf)})
            .then(b.view.next_checkpoint.cmp(&a.view.next_checkpoint))
            .then_with(||distance(a).total_cmp(&distance(b)))
            .then(a.view.seat.cmp(&b.view.seat))
    });order
}

impl Board {
    pub fn start_map_finale(&mut self,seats:&[u8],eligible:&[u8]) {
        self.end_drive("finale");
        self.clear_raids("finale");self.finish_position();self.walk=None;self.vacuum=Vacuum::default();
        let mut gates=course(self.map,self.layout);let obstacles=if self.map==Map::Frozen {city_obstacles(&mut gates)}else{Vec::new()};let start=gates[0].at;
        let direction=gates[1].via[0];let yaw=(direction.0-start.0).atan2(direction.1-start.1);
        let runners=seats.iter().enumerate().map(|(index,seat)|{
            let angle=index as f32*std::f32::consts::TAU/seats.len().max(1) as f32;
            let at=(start.0+angle.cos()*0.45,start.1+angle.sin()*0.45);
            let dnf=!eligible.contains(seat)||self.tokens.iter().any(|token|token.seat==*seat&&token.resting>0);
            Runner{view:FinaleRacer{seat:*seat,x:at.0,z:at.1,yaw,next_checkpoint:1,waypoint:0,finished:false,dnf,finish_time:None,penalties:0,stun:0.,gauge:0.,waiting:false,place:0,slipping:false},waypoint:0,velocity:(0.,0.)}
        }).collect();
        self.finale=Some(MapFinale{map:self.map,time:0.,gates,runners,obstacles,complete:false});
        self.step=TurnStep::Done;
        self.say(json!({"kind":"map_finale_start","map":self.map,"title":metadata(self.map).0,"seconds":DURATION}));
    }

    pub(super) fn finale_view(&self)->Option<MapFinaleView> {
        let race=self.finale.as_ref()?;let elapsed=(race.time-READY).max(0.);let high=tide_high(race.map,elapsed);
        let order=rank(race);let mut racers:Vec<_>=race.runners.iter().map(|runner|{let mut view=runner.view.clone();view.waypoint=runner.waypoint;view}).collect();
        for (place,index) in order.into_iter().enumerate(){racers[index].place=place+1;}
        Some(MapFinaleView{map:race.map,title:metadata(race.map).0,rules:metadata(race.map).1,elapsed,remaining:(DURATION-elapsed).max(0.),duration:DURATION,ready:(READY-race.time).max(0.),speed:SPEED,
            checkpoints:race.gates.iter().map(|gate|FinaleCheckpoint{x:gate.at.0,z:gate.at.1,radius:RADIUS,blocked:high&&gate.wet,via:gate.via.clone()}).collect(),racers,tide_high:high,obstacles:race.obstacles.clone()})
    }

    pub fn finale_title(&self)->Option<&'static str> {self.finale.as_ref().map(|race|metadata(race.map).0)}

    pub(super) fn finale_spot(&self,seat:u8)->Option<(f32,f32)> {self.finale.as_ref()?.runners.iter().find(|runner|runner.view.seat==seat).map(|runner|(runner.view.x,runner.view.z))}
    pub fn end_finale_preview(&mut self) {self.finale=None;self.step=TurnStep::Items;}

    pub fn finale_bot_input(&self,seat:u8)->Stick {
        let Some(race)=self.finale.as_ref() else{return Stick::default();};
        let Some(runner)=race.runners.iter().find(|runner|runner.view.seat==seat&&!runner.view.dnf&&!runner.view.finished) else{return Stick::default();};
        if race.time<READY || runner.view.stun>0. {return Stick::default();}
        let gate=&race.gates[runner.view.next_checkpoint];
        let at=gate.via[runner.waypoint.min(gate.via.len()-1)];
        let from=(runner.view.x,runner.view.z);let d=(at.0-from.0,at.1-from.1);let distance=d.0.hypot(d.1).max(0.001);
        let next=(from.0+d.0/distance*0.4,from.1+d.1/distance*0.4);
        if self.traffic_wait(from,next)>0. {return Stick::default();}
        if gate.wet && tide_high(race.map,(race.time-READY).max(0.)) && (from.0-gate.at.0).hypot(from.1-gate.at.1)<RADIUS+0.5 {return Stick::default();}
        Stick{x:d.0/distance,z:d.1/distance}
    }

    pub fn finale_tick(&mut self,dt:f32,inputs:&[(u8,Stick)],eligible:&[u8])->Option<Vec<Vec<u8>>> {
        if !dt.is_finite()||dt<=0. {return None;}
        let mut race=self.finale.take()?;
        if race.complete {self.finale=Some(race);return None;}
        let dt=dt.min(0.05);race.time+=dt;
        let elapsed=(race.time-READY).max(0.);let high=tide_high(race.map,elapsed);
        let mut events=Vec::new();
        for runner in &mut race.runners {
            let view=&mut runner.view;
            if !eligible.contains(&view.seat){view.dnf=true;}
            if view.dnf||view.finished||race.time<READY {continue;}
            view.stun=(view.stun-dt).max(0.);view.waiting=false;
            if view.stun>0. {continue;}
            let input=inputs.iter().find(|(seat,_)|*seat==view.seat).map(|(_,input)|input.clamped()).unwrap_or_default();
            let old=(view.x,view.z);let speed=input.x.hypot(input.z);
            if speed>0.01 {
                let yaw=input.x.atan2(input.z);
                let turn=(yaw-view.yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
                if race.map==Map::Island {view.gauge+=(turn.abs()/std::f32::consts::PI)*0.65;}
                view.yaw=yaw;
            }
            if race.map==Map::Island {view.gauge=(view.gauge-dt*0.25).max(0.);}
            if race.map==Map::Volcano {view.gauge+=dt/6.;}
            view.slipping=race.obstacles.iter().any(|obstacle|obstacle.kind=="ice"&&obstacle.contains(old,0.));
            let desired=(input.x*SPEED,input.z*SPEED);
            if view.slipping {
                let response=1.-(-dt*1.8).exp();
                runner.velocity.0+=(desired.0*0.9-runner.velocity.0)*response;
                runner.velocity.1+=(desired.1*0.9-runner.velocity.1)*response;
            }else{runner.velocity=desired;}
            let mut next=(view.x+runner.velocity.0*dt,view.z+runner.velocity.1*dt);
            let gate=&race.gates[view.next_checkpoint];
            if high {next.0+=((elapsed*0.7).sin()*0.7+0.5)*dt;}
            if race_ground(self,next,gate.space,&race.obstacles) {view.x=next.0;view.z=next.1;}
            else {
                if race_ground(self,(next.0,view.z),gate.space,&race.obstacles) {view.x=next.0;}else{runner.velocity.0=0.;}
                if race_ground(self,(view.x,next.1),gate.space,&race.obstacles) {view.z=next.1;}else{runner.velocity.1=0.;}
            }
            let collision=self.traffic_collision(old,(view.x,view.z));
            if collision.is_some()||view.gauge>=1. {
                if let Some(car)=collision {
                    let speed=self.traffic_view().and_then(|traffic|traffic.cars.into_iter().find(|entry|entry.id==car)).map(|car|car.speed).unwrap_or(4.2);
                    events.push(json!({"kind":"traffic_hit","car":car,"seat":view.seat,"x":view.x,"z":view.z,"speed":speed,"finale":true}));
                }
                let home=race.gates[view.next_checkpoint-1].at;
                view.x=home.0;view.z=home.1;view.stun=2.;view.gauge=0.;view.penalties=view.penalties.saturating_add(1);view.slipping=false;runner.velocity=(0.,0.);runner.waypoint=0;
                events.push(json!({"kind":"map_finale_respawn","seat":view.seat,"x":home.0,"z":home.1,"car":collision,"reason":if collision.is_some(){"traffic"}else if race.map==Map::Island{"coconut"}else{"heat"}}));
                continue;
            }
            if runner.waypoint+1<gate.via.len() {
                let at=gate.via[runner.waypoint];
                if (view.x-at.0).hypot(view.z-at.1)<=1.0 {runner.waypoint+=1;}
            }
            if distance_to_segment(gate.at,old,(view.x,view.z))<=RADIUS {
                if high&&gate.wet {view.waiting=true;}
                else {
                    view.next_checkpoint+=1;view.gauge=0.;runner.waypoint=0;
                    events.push(json!({"kind":"map_finale_checkpoint","seat":view.seat,"checkpoint":view.next_checkpoint-1,"x":view.x,"z":view.z}));
                    if view.next_checkpoint==race.gates.len() {view.finished=true;view.finish_time=Some(elapsed);events.push(json!({"kind":"map_finale_finish","seat":view.seat,"time":elapsed}));}
                }
            }
        }
        if elapsed>=DURATION {for runner in &mut race.runners {if !runner.view.finished {runner.view.dnf=true;}}}
        let done=race.time>=READY&&race.runners.iter().all(|runner|runner.view.finished||runner.view.dnf);
        let result=if done {
            race.complete=true;
            let mut groups:Vec<Vec<u8>>=Vec::new();let mut previous:Option<&FinaleRacer>=None;
            for index in rank(&race) {
                let racer=&race.runners[index].view;
                let tie=previous.is_some_and(|previous|racer.finished&&previous.finished&&racer.finish_time==previous.finish_time);
                if tie {groups.last_mut().unwrap().push(racer.seat);}else{groups.push(vec![racer.seat]);}
                previous=Some(racer);
            }
            Some(groups)
        } else {None};
        self.finale=Some(race);for event in events {self.say(event);}result
    }
}
