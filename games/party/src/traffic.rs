//! Deterministic town traffic. Idle route spaces are outside the road;
//! automatic walkers wait for green, free-moving actors choose their risk.
use super::*;
const PERIOD: f32 = 16.0;
const GREEN_SECONDS: f32 = 6.0;
const CAR_COUNT: u8 = 36;
const STREETS: [f32;6] = [-64.,-40.,-16.,16.,40.,64.];
// Nearby crosswalks share a junction's phase; perpendicular streets alternate.
// Two seconds of clearance separate directions, and adjacent junctions differ.
fn signal_phase(time:f32,x:f32,z:f32)->f32 {
    let index=|at:f32|STREETS.iter().enumerate().min_by(|(_,a),(_,b)|(*a-at).abs().total_cmp(&(*b-at).abs())).map(|(i,_)|i).unwrap_or(0);
    (time+((index(x)*3+index(z)*5)%16) as f32).rem_euclid(PERIOD)
}
fn vehicle_green(time:f32,x:f32,z:f32,axis:&str)->bool {
    let phase=signal_phase(time,x,z);
    if axis=="x" {phase<GREEN_SECONDS} else {(8.0..14.0).contains(&phase)}
}
fn pedestrian_signal(time:f32,x:f32,z:f32,axis:&str)->(bool,f32) {
    let phase=signal_phase(time,x,z);
    if axis=="x" {
        if phase<8. {(true,8.-phase)} else if phase<14. {(false,14.-phase)} else {(true,24.-phase)}
    } else if phase<6. {(false,6.-phase)} else {(true,16.-phase)}
}
fn stop_lines()->&'static [(f32,f32,&'static str)] {
    static LINES:std::sync::OnceLock<Vec<(f32,f32,&'static str)>>=std::sync::OnceLock::new();
    LINES.get_or_init(||{
        let mut lines=Vec::new();
        for x in STREETS {for z in STREETS {lines.push((x,z,"x"));lines.push((x,z,"z"));}}
        for &(x,z,walk_axis) in crossings(){lines.push((x,z,if walk_axis=="x"{"z"}else{"x"}));}
        lines
    })
}
const CAR_SPEED: f32 = 4.2;
const CAR_HALF_LENGTH: f32 = 0.8;
// External vehicle footprints supply their center and axial samples, without
// lateral expansion. Two 1.1-unit capsules plus a small bumper stand-off.
const VEHICLE_CLEARANCE: f32 = 2.2 + 0.15;
/// How close a patrol car with its lights on makes traffic pull over and stop.
const YIELD_RANGE: f32 = 14.;
/// How far a yielding car edges toward the curb.
const PULL_OVER: f32 = 1.2;
/// How long a car stuck against a patrol car waits before it gives way.
const PINNED_SECONDS: f32 = 2.;
/// How fast a patrol car pressing into a stuck car pushes it aside, in m/s.
const NUDGE_SPEED: f32 = 2.5;

fn in_junction(at: (f32, f32)) -> bool {
    STREETS.iter().any(|x| (at.0 - x).abs() < 3.5) && STREETS.iter().any(|z| (at.1 - z).abs() < 3.5)
}
fn touching(car: &Car, obstacles: &[(f32, f32)]) -> bool {
    let direction = (car.yaw.sin() * CAR_HALF_LENGTH, car.yaw.cos() * CAR_HALF_LENGTH);
    let back = (car.at.0 - direction.0, car.at.1 - direction.1);
    let front = (car.at.0 + direction.0, car.at.1 + direction.1);
    obstacles.iter().any(|&point| segment_distance(point, back, front) < VEHICLE_CLEARANCE - 0.05)
}

fn avoids_vehicle_obstacles(car: &Car, travel: f32, obstacles: &[(f32, f32)]) -> bool {
    let direction = (car.yaw.sin(), car.yaw.cos());
    let current_back = (car.at.0 - direction.0 * CAR_HALF_LENGTH, car.at.1 - direction.1 * CAR_HALF_LENGTH);
    let current_front = (car.at.0 + direction.0 * CAR_HALF_LENGTH, car.at.1 + direction.1 * CAR_HALF_LENGTH);
    let along = car.along + if car.reverse { -travel } else { travel };
    let (next, yaw) = car_at_on(along, car.reverse, car.loop_half);
    let next_direction = (yaw.sin(), yaw.cos());
    let next_back = (next.0 - next_direction.0 * CAR_HALF_LENGTH, next.1 - next_direction.1 * CAR_HALF_LENGTH);
    let next_front = (next.0 + next_direction.0 * CAR_HALF_LENGTH, next.1 + next_direction.1 * CAR_HALF_LENGTH);
    for &point in obstacles {
        if !point.0.is_finite() || !point.1.is_finite() { continue; }
        // Faraway cars cannot intersect this bounded step, including a turn.
        if (point.0-car.at.0).hypot(point.1-car.at.1) > VEHICLE_CLEARANCE + CAR_HALF_LENGTH + travel { continue; }
        let current = segment_distance(point, current_back, current_front);
        if current < VEHICLE_CLEARANCE {
            // An obstacle can be moved into an existing overlap by its owner.
            // Never teleport traffic; permit only movement out of that overlap.
            if segment_distance(point, next_back, next_front) + 0.00001 < current { return false; }
        } else if segment_distance(point, current_back, next_front) < VEHICLE_CLEARANCE
            || segment_distance(point, next_back, next_front) < VEHICLE_CLEARANCE {
            return false;
        }
    }
    true
}

fn vehicle_obstacle_limit(car: &Car, mut travel: f32, dt: f32, obstacles: &[(f32, f32)]) -> f32 {
    let forward = (car.yaw.sin(), car.yaw.cos());
    for &point in obstacles {
        if !point.0.is_finite() || !point.1.is_finite() { continue; }
        let offset = (point.0-car.at.0, point.1-car.at.1);
        let along = offset.0*forward.0 + offset.1*forward.1;
        let across = (offset.0*forward.1-offset.1*forward.0).abs();
        if along < 0. || across >= VEHICLE_CLEARANCE { continue; }
        let nose = CAR_HALF_LENGTH + (VEHICLE_CLEARANCE.powi(2)-across.powi(2)).sqrt();
        let remaining = (along-nose).max(0.);
        let speed = if remaining <= 0.025 { 0. } else { (remaining*3.5).min(CAR_SPEED) };
        travel = travel.min(remaining).min(speed*dt);
    }
    if avoids_vehicle_obstacles(car, travel, obstacles) { return travel; }
    // The projected lane check above anticipates straight approaches. This
    // swept check also protects a car's nose as it turns around a junction.
    let (mut safe, mut blocked) = (0., travel);
    for _ in 0..12 {
        let candidate = (safe+blocked)*0.5;
        if avoids_vehicle_obstacles(car, candidate, obstacles) { safe=candidate; }
        else { blocked=candidate; }
    }
    safe
}
#[derive(Clone, Debug, Serialize)]
pub struct TrafficRoadView {
    pub points: Vec<(f32, f32)>,
    pub width: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct TrafficCarView {
    pub id: u8,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub speed: f32,
    pub braking: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct CrossingView {
    pub id: u8,
    pub x: f32,
    pub z: f32,
    pub axis: &'static str,
    pub pedestrian_green: bool,
    pub remaining: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct TrafficView {
    pub time: f32,
    pub roads: Vec<TrafficRoadView>,
    pub cars: Vec<TrafficCarView>,
    pub crossings: Vec<CrossingView>,
}
#[derive(Clone, Debug)]
pub(super) struct Car {
    id: u8,
    along: f32,
    reverse: bool,
    loop_half: f32,
    old: (f32, f32),
    at: (f32, f32),
    yaw: f32,
    speed: f32,
    braking: bool,
    shove: (f32, f32),
    spin: f32,
    stall: f32,
    /// How long the car has sat still against a patrol car.
    pinned: f32,
    /// Taken off the streets by the sandbox's traffic setting.
    parked: bool,
}
pub(super) struct Traffic {
    time: f32,
    cars: Vec<Car>,
    actors: HashMap<u8, (f32, f32)>,
    near: HashMap<(u8, u8), f32>,
}
fn roads() -> &'static [design::RoadDesign] {
    static ROADS: std::sync::OnceLock<Vec<design::RoadDesign>>=std::sync::OnceLock::new();
    ROADS.get_or_init(||serde_json::from_str::<design::Design>(Map::Frozen.source()).expect("city streets").roads)
}
fn crossings() -> &'static [(f32,f32,&'static str)] {
    static CROSSINGS:std::sync::OnceLock<Vec<(f32,f32,&'static str)>>=std::sync::OnceLock::new();
    CROSSINGS.get_or_init(||{
        let json:serde_json::Value=serde_json::from_str(Map::Frozen.source()).expect("city map");
        let mut result:Vec<(f32,f32,&'static str)>=Vec::new();
        for path in json["paths"].as_array().unwrap(){
            if path["under"]==true||path["gated"]==true {continue;}
            let mut points:Vec<(f32,f32)>=path["points"].as_array().unwrap().iter().map(|p|(p[0].as_f64().unwrap() as f32,p[1].as_f64().unwrap() as f32)).collect();
            if path["closed"]==true {points.push(points[0]);}
            for pair in points.windows(2){
                let (a,b)=(pair[0],pair[1]);let r=(b.0-a.0,b.1-a.1);
                for road in roads(){for line in road.points.windows(2){
                    let (c,d)=(line[0],line[1]);let s=(d.0-c.0,d.1-c.1);let cross=r.0*s.1-r.1*s.0;
                    if cross.abs()<0.0001 {continue;}
                    let q=(c.0-a.0,c.1-a.1);let t=(q.0*s.1-q.1*s.0)/cross;let u=(q.0*r.1-q.1*r.0)/cross;
                    if !(0.0..=1.0).contains(&t)||!(0.0..=1.0).contains(&u){continue;}
                    let at=(a.0+r.0*t,a.1+r.1*t);
                    if result.iter().any(|p|(p.0-at.0).hypot(p.1-at.1)<1.5){continue;}
                    result.push((at.0,at.1,if s.0.abs()>s.1.abs(){"z"}else{"x"}));
                }}
            }
        }result
    })
}
fn car_at_on(along:f32,reverse:bool,loop_half:f32)->((f32,f32),f32){
    let half = loop_half + if reverse { 1.05 } else { -1.05 };
    let edge = half * 2.;
    let distance = along.rem_euclid(edge * 4.);
    let side = (distance / edge).floor() as usize;
    let t = distance % edge;
    let (point, yaw) = match side {
        0 => ((-half + t, -half), std::f32::consts::FRAC_PI_2),
        1 => ((half, -half + t), 0.),
        2 => ((half - t, half), -std::f32::consts::FRAC_PI_2),
        _ => ((-half, half - t), std::f32::consts::PI),
    };
    (
        point,
        if reverse {
            yaw + std::f32::consts::PI
        } else {
            yaw
        },
    )
}
impl Traffic {
    pub(super) fn new() -> Self {
        let cars = (0..CAR_COUNT)
            .map(|id| {
                let reverse = id % 2 == 1;
                let loop_half=if id<12 {16.} else if id<24 {40.} else {64.};
                let perimeter=(loop_half+if reverse{1.05}else{-1.05})*8.;
                let along=(f32::from((id%12)/2)+if reverse{0.5}else{0.})*perimeter/6.;
                let (at, yaw) = car_at_on(along, reverse,loop_half);
                Car {
                    id,
                    along,
                    reverse,
                    loop_half,
                    old: at,
                    at,
                    yaw,
                    speed: CAR_SPEED,
                    braking: false,
                    shove: (0., 0.),
                    spin: 0.,
                    stall: 0.,
                    pinned: 0.,
                    parked: false,
                }
            })
            .collect();
        Self {
            time: 0.,
            cars,
            actors: HashMap::new(),
            near: HashMap::new(),
        }
    }
}
fn segment_distance(point: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let t = (((point.0 - a.0) * dx + (point.1 - a.1) * dz) / (dx * dx + dz * dz).max(0.000001))
        .clamp(0., 1.);
    (point.0 - a.0 - dx * t).hypot(point.1 - a.1 - dz * t)
}
fn segment_pair_distance(a:(f32,f32),b:(f32,f32),c:(f32,f32),d:(f32,f32))->f32 {
    let cross=|u:(f32,f32),v:(f32,f32)|u.0*v.1-u.1*v.0;
    let ab=(b.0-a.0,b.1-a.1);let cd=(d.0-c.0,d.1-c.1);let ac=(c.0-a.0,c.1-a.1);
    let denominator=cross(ab,cd);
    if denominator.abs()>0.000001 {
        let t=cross(ac,cd)/denominator;let u=cross(ac,ab)/denominator;
        if (0.0..=1.0).contains(&t)&&(0.0..=1.0).contains(&u){return 0.;}
    }
    segment_distance(a,c,d).min(segment_distance(b,c,d)).min(segment_distance(c,a,b)).min(segment_distance(d,a,b))
}
// Relative center motion against the Minkowski sum of both capsule axes.
// This handles the entire swept nose, including perpendicular crossings,
// without gaps between a few sampled points along the police car.
fn swept_capsule_distance(from:(f32,f32),to:(f32,f32),a:(f32,f32),b:(f32,f32))->f32 {
    let hull=[(a.0+b.0,a.1+b.1),(-a.0+b.0,-a.1+b.1),(-a.0-b.0,-a.1-b.1),(a.0-b.0,a.1-b.1)];
    let area=(a.0*b.1-a.1*b.0).abs();
    let inside=|point:(f32,f32)|{
        if area<0.000001{return false;}
        let mut positive=false;let mut negative=false;
        for i in 0..4 {let p=hull[i];let q=hull[(i+1)%4];let cross=(q.0-p.0)*(point.1-p.1)-(q.1-p.1)*(point.0-p.0);positive|=cross>0.000001;negative|=cross< -0.000001;}
        !(positive&&negative)
    };
    if inside(from)||inside(to){return 0.;}
    (0..4).map(|i|segment_pair_distance(from,to,hull[i],hull[(i+1)%4])).fold(f32::INFINITY,f32::min)
}
impl Board {
    pub fn traffic_view(&self) -> Option<TrafficView> {
        if self.map != Map::Frozen {
            return None;
        }
        let t = &self.traffic;
        Some(TrafficView {
            time: t.time,
            roads: roads().iter().map(|road|TrafficRoadView{points:road.points.clone(),width:road.width}).collect(),
            cars: t
                .cars
                .iter()
                .filter(|c| !c.parked)
                .map(|c| TrafficCarView {
                    id: c.id,
                    x: c.at.0,
                    z: c.at.1,
                    yaw: c.yaw,
                    speed: c.speed,
                    braking: c.braking,
                })
                .collect(),
            crossings: crossings().iter().copied()
            .enumerate()
            .map(|(id, (x, z, axis))| {let (pedestrian_green,remaining)=pedestrian_signal(t.time,x,z,axis);CrossingView {
                id: id as u8,x,z,axis,pedestrian_green,remaining,
            }})
            .collect(),
        })
    }
    pub(super) fn on_traffic_road(&self, point: (f32, f32), margin: f32) -> bool {
        self.map == Map::Frozen
            && roads().iter().any(|road|road.points.windows(2).any(|p|segment_distance(point,p[0],p[1])<road.width*0.5+margin))
    }
    /// Shared by automatic walks and the city finale; no additional player prompt.
    pub fn traffic_wait(&self, from: (f32, f32), next: (f32, f32)) -> f32 {
        if self.map != Map::Frozen || self.on_traffic_road(from, 0.) {return 0.;}
        let direction=(next.0-from.0,next.1-from.1);
        let walk_axis=if direction.0.abs()>direction.1.abs(){"x"}else{"z"};
        if self.on_traffic_road(next,0.55) {
            let (green,remaining)=pedestrian_signal(self.traffic.time,next.0,next.1,walk_axis);
            if green {0.}else{remaining}
        }else{0.}
    }
    /// Swept moving-car capsule, reusable by the finale's checkpoint racers.
    pub fn traffic_collision(&self, from: (f32, f32), to: (f32, f32)) -> Option<u8> {
        if self.map != Map::Frozen {
            return None;
        }
        self.traffic
            .cars
            .iter()
            .find(|car| car.speed>0.01 && self.traffic_car_distance(car, from, to) < 1.1)
            .map(|c| c.id)
    }
    pub(super) fn traffic_vehicle_collision(&self, from: (f32, f32), to: (f32, f32)) -> Option<u8> {
        if self.map != Map::Frozen {
            return None;
        }
        self.traffic
            .cars
            .iter()
            .find(|car| self.traffic_car_distance(car, from, to) < 1.1)
            .map(|car| car.id)
    }
    /// A patrol car's own attempted movement against both stopped and moving
    /// traffic. `yaw` is the patrol car's current heading, not its input stick.
    /// Return true to keep the old position; an existing overlap may only open.
    pub fn traffic_blocks_patrol_vehicle(&self, from:(f32,f32),to:(f32,f32),yaw:f32)->bool {
        if self.map!=Map::Frozen{return false;}
        if ![from.0,from.1,to.0,to.1,yaw].iter().all(|v|v.is_finite()){return true;}
        let patrol_axis=(yaw.sin()*0.9,yaw.cos()*0.9);
        self.traffic.cars.iter().any(|car|{
            let relative_from=(from.0-car.old.0,from.1-car.old.1);
            let relative_to=(to.0-car.at.0,to.1-car.at.1);
            if segment_distance((0.,0.),relative_from,relative_to)>CAR_HALF_LENGTH+0.9+2.2{return false;}
            let traffic_axis=(car.yaw.sin()*CAR_HALF_LENGTH,car.yaw.cos()*CAR_HALF_LENGTH);
            let initial=swept_capsule_distance(relative_from,relative_from,patrol_axis,traffic_axis);
            if initial<2.2 {
                let final_distance=swept_capsule_distance(relative_to,relative_to,patrol_axis,traffic_axis);
                let motion=(relative_to.0-relative_from.0,relative_to.1-relative_from.1);
                let center_squared=relative_from.0.powi(2)+relative_from.1.powi(2);
                let opening=if center_squared<0.000001 {motion.0.powi(2)+motion.1.powi(2)>0.000001}
                    else {relative_from.0*motion.0+relative_from.1*motion.1>0.000001};
                // A player must be able to back out of an overlap introduced
                // by spawning/parking. Approaching through it is still blocked.
                return !opening||final_distance+0.00001<initial;
            }
            swept_capsule_distance(relative_from,relative_to,patrol_axis,traffic_axis)<2.2
        })
    }
    /// A patrol car at speed pushes the traffic in its way aside instead of
    /// stopping dead. The shoved car stalls, then drifts back into its lane.
    pub fn shove_traffic(&mut self, from: (f32, f32), to: (f32, f32), yaw: f32, speed: f32) -> u8 {
        if self.map != Map::Frozen || ![from.0, from.1, to.0, to.1, yaw, speed].iter().all(|v| v.is_finite()) {
            return 0;
        }
        let patrol_axis = (yaw.sin() * 0.9, yaw.cos() * 0.9);
        let right = (yaw.cos(), -yaw.sin());
        let mut shoved = 0;
        for car in &mut self.traffic.cars {
            let relative_from = (from.0 - car.old.0, from.1 - car.old.1);
            let relative_to = (to.0 - car.at.0, to.1 - car.at.1);
            if segment_distance((0., 0.), relative_from, relative_to) > CAR_HALF_LENGTH + 0.9 + 2.6 {
                continue;
            }
            let traffic_axis = (car.yaw.sin() * CAR_HALF_LENGTH, car.yaw.cos() * CAR_HALF_LENGTH);
            if swept_capsule_distance(relative_from, relative_to, patrol_axis, traffic_axis) >= 2.5 {
                continue;
            }
            let lateral = (car.at.0 - to.0) * right.0 + (car.at.1 - to.1) * right.1;
            let side = if lateral < 0. { -1. } else { 1. };
            let room = (3.2 - car.shove.0.hypot(car.shove.1)).max(0.);
            let push = (2.9 - lateral.abs()).clamp(0.6, 3.).min(room);
            if push <= 0. {
                continue;
            }
            let delta = (right.0 * side * push, right.1 * side * push);
            car.shove = (car.shove.0 + delta.0, car.shove.1 + delta.1);
            car.at = (car.at.0 + delta.0, car.at.1 + delta.1);
            car.old = car.at;
            car.stall = 1.6;
            car.spin = side * (0.25 + speed * 0.03).min(0.7);
            car.yaw += car.spin;
            car.speed = 0.;
            car.braking = true;
            shoved += 1;
        }
        shoved
    }
    /// A car pinned against a patrol car for a while leaves for the farthest
    /// clear stretch of its own loop, so a wedged patrol car always comes free.
    fn give_way(&mut self, obstacles: &[(f32, f32)]) {
        let pinned: Vec<usize> = (0..self.traffic.cars.len())
            .filter(|&i| self.traffic.cars[i].pinned > PINNED_SECONDS)
            .collect();
        for index in pinned {
            let car = self.traffic.cars[index].clone();
            let perimeter = (car.loop_half + if car.reverse { 1.05 } else { -1.05 }) * 8.;
            let clear = |along: f32| {
                let (at, _) = car_at_on(along, car.reverse, car.loop_half);
                let far_from_traffic = self.traffic.cars.iter().all(|other| {
                    other.id == car.id || (other.at.0 - at.0).hypot(other.at.1 - at.1) > 9.
                });
                let far_from_patrol = obstacles
                    .iter()
                    .all(|point| (point.0 - at.0).hypot(point.1 - at.1) > 12.);
                far_from_traffic && far_from_patrol
            };
            let Some(along) = (1..16)
                .map(|step| car.along + perimeter * step as f32 / 16. * if step % 2 == 0 { 1. } else { -1. })
                .map(|along| along.rem_euclid(perimeter))
                .find(|along| clear(*along))
            else {
                continue;
            };
            let car = &mut self.traffic.cars[index];
            car.along = along;
            (car.at, car.yaw) = car_at_on(along, car.reverse, car.loop_half);
            car.old = car.at;
            car.shove = (0., 0.);
            car.spin = 0.;
            car.stall = 0.;
            car.pinned = 0.;
        }
    }
    /// The sandbox's traffic setting: 0 clears the streets, 1 leaves every
    /// third car, 2 or more runs them all.
    pub fn set_traffic_level(&mut self, level: u8) {
        for car in &mut self.traffic.cars {
            let keep = level >= 2 || (level == 1 && car.id % 3 == 0);
            if keep && car.parked {
                car.parked = false;
                (car.at, car.yaw) = car_at_on(car.along, car.reverse, car.loop_half);
                car.old = car.at;
                car.shove = (0., 0.);
                car.spin = 0.;
                car.stall = 0.;
            } else if !keep {
                car.parked = true;
                car.at = (9999., 9999.);
                car.old = car.at;
                car.speed = 0.;
            }
        }
    }
    /// Parks traffic car `id` at `at` facing `yaw` and holds it there for
    /// `hold` seconds, for scripted reviews of a wedged patrol car.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_park_traffic(&mut self, id: u8, at: (f32, f32), yaw: f32, hold: f32) {
        if let Some(car) = self.traffic.cars.iter_mut().find(|car| car.id == id) {
            let (lane, lane_yaw) = car_at_on(car.along, car.reverse, car.loop_half);
            car.shove = (at.0 - lane.0, at.1 - lane.1);
            car.spin = yaw - lane_yaw;
            car.at = at;
            car.old = at;
            car.yaw = yaw;
            car.stall = hold;
            car.speed = 0.;
        }
    }
    /// The traffic car nearest `at`, its position and whether it is stopped.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_nearest_traffic(&self, at: (f32, f32)) -> Option<(u8, (f32, f32), f32)> {
        self.traffic
            .cars
            .iter()
            .map(|car| (car.id, car.at, (car.at.0 - at.0).hypot(car.at.1 - at.1)))
            .min_by(|a, b| a.2.total_cmp(&b.2))
    }
    /// A patrol car pressing slowly into traffic it is stuck against pushes
    /// that car aside at walking pace, where a fast one would shove it.
    pub fn nudge_traffic(&mut self, from: (f32, f32), to: (f32, f32), yaw: f32, dt: f32) -> u8 {
        if self.map != Map::Frozen || ![from.0, from.1, to.0, to.1, yaw, dt].iter().all(|v| v.is_finite()) {
            return 0;
        }
        let patrol_axis = (yaw.sin() * 0.9, yaw.cos() * 0.9);
        let mut nudged = 0;
        for car in &mut self.traffic.cars {
            let relative_from = (from.0 - car.old.0, from.1 - car.old.1);
            let relative_to = (to.0 - car.at.0, to.1 - car.at.1);
            let traffic_axis = (car.yaw.sin() * CAR_HALF_LENGTH, car.yaw.cos() * CAR_HALF_LENGTH);
            if swept_capsule_distance(relative_from, relative_to, patrol_axis, traffic_axis) >= 2.3 {
                continue;
            }
            let away = (car.at.0 - from.0, car.at.1 - from.1);
            let length = away.0.hypot(away.1).max(0.001);
            let step = NUDGE_SPEED * dt;
            let delta = (away.0 / length * step, away.1 / length * step);
            car.shove = (car.shove.0 + delta.0, car.shove.1 + delta.1);
            car.at = (car.at.0 + delta.0, car.at.1 + delta.1);
            car.old = car.at;
            car.stall = car.stall.max(0.8);
            car.speed = 0.;
            car.braking = true;
            // Pushing a car that cannot go anywhere still hurries it off.
            car.pinned += dt;
            nudged += 1;
        }
        nudged
    }
    fn traffic_car_distance(&self, car: &Car, from: (f32, f32), to: (f32, f32)) -> f32 {
        let d = (car.yaw.sin() * CAR_HALF_LENGTH, car.yaw.cos() * CAR_HALF_LENGTH);
        let start = (car.old.0 - from.0 - d.0, car.old.1 - from.1 - d.1);
        let end = (car.at.0 - to.0 + d.0, car.at.1 - to.1 + d.1);
        segment_distance((0., 0.), start, end)
    }
    pub fn traffic_tick(&mut self, dt: f32) -> bool {
        self.traffic_tick_with_obstacles(dt, &[])
    }
    /// Advance ordinary traffic around occupied or parked vehicle capsules.
    /// Footprint points are unexpanded center/axial samples in world units.
    /// Existing callers without external vehicles keep using `traffic_tick`.
    pub fn traffic_tick_with_obstacles(&mut self, dt: f32, obstacles: &[(f32, f32)]) -> bool {
        self.traffic_tick_city(dt, obstacles, &[])
    }
    /// As `traffic_tick_with_obstacles`, with patrol cars running their lights
    /// at `sirens`: traffic near one pulls toward the curb and stops, except
    /// inside a junction, which it clears first. A car left pinned against a
    /// patrol car gives way by leaving for the far side of its loop.
    pub fn traffic_tick_city(&mut self, dt: f32, obstacles: &[(f32, f32)], sirens: &[(f32, f32)]) -> bool {
        if self.map != Map::Frozen || !dt.is_finite() || dt <= 0. {
            return false;
        }
        let dt = dt.min(0.05);
        self.traffic.time += dt;
        let walkers:Vec<_>=self.walk_view().filter(|walk|self.on_traffic_road((walk.x,walk.z),0.65)).map(|walk|vec![(walk.x,walk.z)]).unwrap_or_default();
        let before:Vec<_>=self.traffic.cars.iter().map(|car|(car.id,car.along,car.reverse,car.loop_half)).collect();
        let mut events=Vec::new();let mut changed=false;
        for car in &mut self.traffic.cars {
            if car.parked {
                car.at = (9999., 9999.);
                car.old = car.at;
                car.speed = 0.;
                continue;
            }
            car.old=car.at;
            let forward=(car.yaw.sin(),car.yaw.cos());
            let axis=if forward.0.abs()>0.5 {"x"}else{"z"};
            let ahead=|point:(f32,f32)| {let d=(point.0-car.at.0,point.1-car.at.1);(d.0*forward.0+d.1*forward.1,(d.0*forward.1-d.1*forward.0).abs())};
            let mut travel=CAR_SPEED*dt;
            for &(x,z,line_axis) in stop_lines(){
                if axis!=line_axis || vehicle_green(self.traffic.time,x,z,axis){continue;}
                let (along,across)=ahead((x,z));
                // A vehicle already inside the crossing clears it before stopping.
                if across<2.1 && along>=3.75 && along<4.+travel {travel=travel.min((along-4.).max(0.));}
            }
            // Keep an automatic walker protected if its crossing phase changes.
            for &walker in &walkers {let (along,across)=ahead(walker);if across<2.2&&along>0.&&along<4.5+travel {travel=0.;}}
            let perimeter=(car.loop_half+if car.reverse{1.05}else{-1.05})*8.;
            for &(id,along,reverse,half) in &before {
                if id==car.id||reverse!=car.reverse||half!=car.loop_half {continue;}
                let gap=if reverse{car.along-along}else{along-car.along}.rem_euclid(perimeter);
                travel=travel.min((gap-4.2).max(0.));
            }
            if !obstacles.is_empty() { travel=vehicle_obstacle_limit(car,travel,dt,obstacles); }
            let yielding = !in_junction(car.at)
                && sirens.iter().any(|&(x, z)| (x - car.at.0).hypot(z - car.at.1) < YIELD_RANGE);
            if yielding {
                travel = 0.;
                let right = (car.yaw.cos(), -car.yaw.sin());
                let edged = car.shove.0 * right.0 + car.shove.1 * right.1;
                let step = (PULL_OVER - edged).clamp(0., 1.5 * dt);
                car.shove = (car.shove.0 + right.0 * step, car.shove.1 + right.1 * step);
                car.stall = car.stall.max(0.3);
            }
            if travel < 0.001 && touching(car, obstacles) {
                car.pinned += dt;
            } else {
                car.pinned = 0.;
            }
            if car.stall > 0. {
                car.stall = (car.stall - dt).max(0.);
                travel = 0.;
            } else {
                let settle = (-0.9 * dt).exp();
                car.shove = (car.shove.0 * settle, car.shove.1 * settle);
                if car.shove.0.hypot(car.shove.1) < 0.02 { car.shove = (0., 0.); }
            }
            car.spin *= (-2.5 * dt).exp();
            let was=car.braking;car.braking=travel<0.001;car.speed=travel/dt;
            car.along+=if car.reverse{-travel}else{travel};
            (car.at,car.yaw)=car_at_on(car.along,car.reverse,car.loop_half);
            car.at = (car.at.0 + car.shove.0, car.at.1 + car.shove.1);
            car.yaw += car.spin;
            if !was&&car.braking {events.push(json!({"kind":"traffic_brake","car":car.id,"x":car.at.0,"z":car.at.1}));}
            changed|=was!=car.braking;
        }
        self.give_way(obstacles);
        let mut moving: Vec<u8> = self.raids.keys().copied().collect();
        if let Some(p) = &self.positioning {
            if !moving.contains(&p.seat) {
                moving.push(p.seat);
            }
        }
        self.traffic.actors.retain(|seat, _| moving.contains(seat));
        let mut hits = Vec::new();
        for seat in moving {
            if self.tokens.iter().any(|t| t.seat == seat && (t.resting > 0 || self.layout[t.space as usize].under)) {
                continue;
            }
            let at = self.actual_spot_of(seat);
            let old = self.traffic.actors.insert(seat, at).unwrap_or(at);
            if let Some(car) = self.traffic_collision(old, at) {
                hits.push((seat, car, at));
            } else {
                for car in &self.traffic.cars {
                    let key = (car.id, seat);
                    if car.speed>0.01 && self.traffic_car_distance(car, old, at) < 2.25
                        && self
                            .traffic
                            .near
                            .get(&key)
                            .is_none_or(|last| self.traffic.time - *last > 5.)
                    {
                        self.traffic.near.insert(key, self.traffic.time);
                        events.push(json!({"kind":"traffic_near_miss","car":car.id,"seat":seat,"x":at.0,"z":at.1,"speed":CAR_SPEED}));
                    }
                }
            }
        }
        for (seat, car, at) in &hits {
            self.hurt(*seat, MAX_HEALTH);
            self.raids.remove(seat);
            if self.positioning.as_ref().is_some_and(|p| p.seat == *seat) {
                self.finish_position();
            }
            if *seat == self.turn_seat() {
                self.step = TurnStep::Done;
            }
            events.push(json!({"kind":"traffic_hit","car":car,"seat":seat,"x":at.0,"z":at.1,"speed":CAR_SPEED}));
        }
        let changed = changed || !hits.is_empty();
        for event in events {
            self.say(event);
        }
        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_landing_tile_is_clear_of_swept_traffic() {
        let b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        for space in b.layout.iter().filter(|s| !s.under) {
            assert!(
                !b.on_traffic_road((space.x, space.z), 0.95),
                "unsafe landing {}",
                space.id
            );
        }
    }
    #[test]
    fn idle_waiting_players_never_take_passive_traffic_damage() {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        for _ in 0..1800 {
            b.traffic_tick(0.05);
        }
        assert!(
            b.tokens
                .iter()
                .all(|t| t.health == MAX_HEALTH && t.resting == 0)
        );
    }
    #[test]
    fn intersections_are_staggered_and_perpendicular_directions_never_share_green() {
        let mut b=Board::on(Map::Frozen,&[0,1],10,7);
        assert_eq!(b.traffic.cars.len(),36);
        let mut saw_mixed_cars=false;
        for tick in 0..640 {
            b.traffic_tick(0.05);
            let view=b.traffic_view().unwrap();
            assert!(view.crossings.iter().any(|light|light.pedestrian_green),"some crossings are green at tick {tick}");
            assert!(view.crossings.iter().any(|light|!light.pedestrian_green),"not every crossing switches together at tick {tick}");
            assert!(view.crossings.iter().all(|light|light.remaining>0.&&light.remaining<=10.01));
            saw_mixed_cars|=view.cars.iter().any(|car|car.braking)&&view.cars.iter().any(|car|!car.braking);
            for x in STREETS {for z in STREETS {assert!(!(vehicle_green(view.time,x,z,"x")&&vehicle_green(view.time,x,z,"z")));}}
        }
        assert!(saw_mixed_cars,"one junction stopping never freezes the whole city");
    }
    #[test]
    fn red_signal_stops_at_its_line_and_queue_leaves_safe_spacing() {
        let mut b=Board::on(Map::Frozen,&[0,1],10,7);
        for _ in 0..3600 {
            b.traffic_tick(0.05);
            for car in &b.traffic.cars {
                for other in &b.traffic.cars {if car.id!=other.id&&car.reverse==other.reverse&&car.loop_half==other.loop_half {
                    let perimeter=(car.loop_half+if car.reverse{1.05}else{-1.05})*8.;
                    let gap=(other.along-car.along).rem_euclid(perimeter);assert!(gap>=4.19,"cars must queue, not overlap: {gap}");
                }}
            }
        }
    }
    #[test]
    fn a_local_red_stops_its_approaching_car_then_releases_it_on_green() {
        let mut b=Board::on(Map::Frozen,&[0,1],10,7);
        b.traffic.time=4.; // This junction is in its all-red clearance.
        let car=&mut b.traffic.cars[0];car.along=26.95;
        (car.at,car.yaw)=car_at_on(car.along,car.reverse,car.loop_half);car.old=car.at;
        let before=car.at;
        for _ in 0..5 {b.traffic_tick(0.05);}
        assert!(b.traffic.cars[0].braking);
        assert!((b.traffic.cars[0].at.0-before.0).abs()<0.01);
        assert!(b.traffic.cars.iter().any(|car|car.speed>0.));
        b.traffic.time=13.;b.traffic_tick(0.05);
        assert!(!b.traffic.cars[0].braking);assert!(b.traffic.cars[0].at.0>before.0);
    }
    #[test]
    fn collision_is_swept_and_non_town_maps_have_no_traffic() {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        let c = &mut b.traffic.cars[0];
        c.old = (-3., 0.);
        c.at = (3., 0.);
        c.yaw = std::f32::consts::FRAC_PI_2;
        assert_eq!(b.traffic_collision((0., 0.), (0., 0.)), Some(0));
        assert!(b.traffic_collision((0., 4.), (0., 4.)).is_none());
        let other = Board::new(&[0, 1], 10, 7);
        assert!(other.traffic_view().is_none());
        assert!(other.traffic_collision((0., 0.), (0., 0.)).is_none());
    }
    #[test]
    fn free_actor_car_contact_is_instant_knockout_even_with_shield() {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        b.begin_position(0, Item::Grapple);
        let car = b.traffic.cars[0].at;
        let p = b.positioning.as_mut().unwrap();
        p.x = car.0;
        p.z = car.1;
        b.tokens[0].shield = true;
        b.traffic_tick(0.01);
        assert_eq!(b.tokens[0].resting, 1);
        assert_eq!(b.tokens[0].health, MAX_HEALTH);
        assert!(b.positioning.is_none());
        assert_eq!(b.step(), &TurnStep::Done);
        assert_eq!(b.roll(0), Err(BoardError::WrongStep));
        assert_eq!(b.tokens[1].resting, 0);
        assert!(
            b.log
                .iter()
                .any(|e| e["kind"] == "traffic_hit" && e["seat"] == 0)
        );
    }
    #[test]
    fn automatic_crossing_wait_keeps_vacuum_live_and_finishes_safely() {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        b.timed_walk = true;
        let edge = b
            .layout
            .iter()
            .filter(|s| !s.under)
            .find_map(|a| {
                a.next
                    .iter()
                    .filter_map(|n| {
                        let z = &b.layout[*n as usize];
                        (!z.under
                            && !b.on_traffic_road((a.x, a.z), 0.6)
                            && !b.on_traffic_road((z.x, z.z), 0.6)
                            && b.on_traffic_road(((a.x + z.x) * 0.5, (a.z + z.z) * 0.5), 0.))
                        .then_some((a.id, z.id))
                    })
                    .next()
            })
            .unwrap();
        b.start_walk(
            0,
            edge.0,
            vec![edge.1],
            false,
            walking::AfterWalk::Step(TurnStep::Done),
        );
        let mut paused = false;
        let mut held_for = 0.;
        for _ in 0..900 {
            b.traffic_tick(0.0166667);
            if b.walk_view().is_none() {
                break;
            }
            b.aim_walk_vacuum(0, 0., 0., true).unwrap();
            let old = b.walk_view().unwrap().elapsed;
            b.walk_tick(0.0166667);
            if let Some(w) = b.walk_view() {
                if w.waiting > 0. {
                    paused = true;
                    held_for += 0.0166667;
                    assert!((w.elapsed - old).abs() < 0.0001);
                    assert!(w.vacuum_on);
                    assert_eq!(w.hop,0.0,"red-light wait has grounded feet");
                    assert!(!b.on_traffic_road((w.x, w.z), 0.));
                }
            }
        }
        assert!(paused);
        assert!(held_for > 1. && held_for <= 6.1);
        assert!(b.walk_view().is_none());
        assert!(
            b.tokens
                .iter()
                .all(|t| t.health == MAX_HEALTH && t.resting == 0)
        );
    }
    #[test]
    fn off_turn_raid_car_death_never_ends_the_other_players_turn() {
        let mut b = Board::on(Map::Frozen, &[0, 1], 10, 7);
        b.tokens[1].items = vec![Item::SlashRush];
        b.start_raid(1).unwrap();
        let car = b.traffic.cars[0].at;
        let r = b.raids.get_mut(&1).unwrap();
        r.x = car.0;
        r.z = car.1;
        b.traffic_tick(0.01);
        assert_eq!(b.tokens[1].resting, 1);
        assert!(!b.has_raid_for(1));
        assert_eq!(b.step(), &TurnStep::Items);
        assert_eq!(b.turn_seat(), 0);
        assert!(b.roll(0).is_ok());
    }

    fn isolated_lane(reverse: bool) -> Board {
        let mut board = Board::on(Map::Frozen, &[0, 1], 10, 7);
        board.traffic.cars.retain(|car| car.id == u8::from(reverse));
        let car = &mut board.traffic.cars[0];
        let half = car.loop_half + if reverse { 1.05 } else { -1.05 };
        car.along = half;
        (car.at, car.yaw) = car_at_on(car.along, reverse, car.loop_half);
        car.old = car.at;
        board
    }

    fn capsule_distance(car: &Car, obstacle: (f32, f32)) -> f32 {
        let axis = (car.yaw.sin()*CAR_HALF_LENGTH, car.yaw.cos()*CAR_HALF_LENGTH);
        segment_distance(obstacle, (car.at.0-axis.0, car.at.1-axis.1), (car.at.0+axis.0, car.at.1+axis.1))
    }

    #[test]
    fn parked_vehicle_stops_traffic_and_removing_it_releases_both_directions() {
        for reverse in [false, true] {
            let mut board = isolated_lane(reverse);
            let car = &board.traffic.cars[0];
            let forward = (car.yaw.sin(), car.yaw.cos());
            let center = (car.at.0+forward.0*8., car.at.1+forward.1*8.);
            let obstacle = [center,
                (center.0+forward.0*0.9,center.1+forward.1*0.9),
                (center.0-forward.0*0.9,center.1-forward.1*0.9)];
            let start = car.along;
            let sign = if reverse { -1. } else { 1. };
            let mut slowed = false;
            for _ in 0..1600 {
                let old = board.traffic.cars[0].along;
                board.traffic_tick_with_obstacles(0.02, &obstacle);
                let car = &board.traffic.cars[0];
                let distance = (car.along-old)*sign;
                assert!(distance >= -0.0001 && distance <= CAR_SPEED*0.02+0.0001, "traffic must not teleport");
                assert!(obstacle.iter().all(|point|capsule_distance(car,*point)>=VEHICLE_CLEARANCE-0.001), "vehicle overlap in reverse={reverse}");
                slowed |= car.speed > 0.01 && car.speed < CAR_SPEED-0.1;
            }
            let stopped = &board.traffic.cars[0];
            assert!((stopped.along-start)*sign>0.5, "the fixture must approach the patrol car");
            assert!(stopped.braking && stopped.speed<0.01);
            assert!(slowed, "traffic should slow on approach, not teleport or tunnel");
            let parked_at = stopped.along;
            for _ in 0..1600 {board.traffic_tick_with_obstacles(0.02,&[]);}
            assert!((board.traffic.cars[0].along-parked_at)*sign>1., "traffic should resume after the parked car leaves");
        }
    }

    #[test]
    fn vehicle_obstacles_preserve_same_lane_queue_spacing_in_both_directions() {
        for reverse in [false,true] {
            let mut board=isolated_lane(reverse);
            let mut follower=board.traffic.cars[0].clone();
            let sign=if reverse {-1.}else{1.};
            follower.id+=2;follower.along-=sign*5.;
            (follower.at,follower.yaw)=car_at_on(follower.along,reverse,follower.loop_half);follower.old=follower.at;
            board.traffic.cars.push(follower);
            let leader=&board.traffic.cars[0];
            let obstacle=(leader.at.0+leader.yaw.sin()*8.,leader.at.1+leader.yaw.cos()*8.);
            for _ in 0..1600 {
                board.traffic_tick_with_obstacles(0.02,&[obstacle]);
                let leader=&board.traffic.cars[0];let follower=&board.traffic.cars[1];
                let gap=(leader.along-follower.along)*sign;
                assert!(gap>=4.19,"queued traffic overlapped behind a parked car: {gap}");
            }
            assert!(board.traffic.cars.iter().all(|car|car.braking));
            let before:Vec<_>=board.traffic.cars.iter().map(|car|car.along).collect();
            for _ in 0..1600 {board.traffic_tick_with_obstacles(0.02,&[]);}
            assert!(board.traffic.cars.iter().zip(before).all(|(car,old)|(car.along-old)*sign>1.));
        }
    }

    #[test]
    fn the_vehicle_capsule_cannot_swing_through_a_parked_car_at_a_corner() {
        for reverse in [false,true] {
            let mut board=isolated_lane(reverse);
            let car=&mut board.traffic.cars[0];
            let half=car.loop_half+if reverse{1.05}else{-1.05};
            let corner=half*2.;let sign=if reverse{-1.}else{1.};
            car.along=corner-sign*2.;
            (car.at,car.yaw)=car_at_on(car.along,reverse,car.loop_half);car.old=car.at;
            let obstacle=car_at_on(corner+sign*2.6,reverse,car.loop_half).0;
            for _ in 0..1600 {
                board.traffic_tick_with_obstacles(0.02,&[obstacle]);
                assert!(capsule_distance(&board.traffic.cars[0],obstacle)>=VEHICLE_CLEARANCE-0.001,
                    "turning nose crossed a parked vehicle, reverse={reverse}");
            }
            let before=board.traffic.cars[0].along;
            for _ in 0..1600 {board.traffic_tick_with_obstacles(0.02,&[]);}
            assert!((board.traffic.cars[0].along-before)*sign>1.);
        }
    }

    #[test]
    fn patrol_cars_stop_before_stationary_traffic_and_can_back_out_of_overlap() {
        for reverse in [false,true] {
            let mut board=isolated_lane(reverse);
            let car=&mut board.traffic.cars[0];car.speed=0.;car.old=car.at;
            let center=car.at;let yaw=car.yaw;let forward=(yaw.sin(),yaw.cos());
            let behind=|distance:f32|(center.0-forward.0*distance,center.1-forward.1*distance);
            assert!(board.traffic_blocks_patrol_vehicle(behind(4.1),behind(3.7),yaw),"stopped traffic still occupies its lane");
            assert!(!board.traffic_blocks_patrol_vehicle(behind(4.1),behind(4.3),yaw));
            assert!(board.traffic_blocks_patrol_vehicle(behind(3.),behind(2.8),yaw),"cannot approach through an existing overlap");
            assert!(!board.traffic_blocks_patrol_vehicle(behind(3.),behind(3.2),yaw),"retreat must release a preexisting overlap");
            assert!(!board.traffic_blocks_patrol_vehicle(center,behind(0.2),yaw),"a coincident spawn must not trap the driver forever");
        }
    }

    #[test]
    fn patrol_nose_detects_moving_cross_traffic_between_clear_endpoints() {
        let mut board=isolated_lane(false);
        let car=&mut board.traffic.cars[0];
        car.old=(-4.,0.);car.at=(4.,0.);car.yaw=std::f32::consts::FRAC_PI_2;car.speed=CAR_SPEED;
        // Both endpoint vehicle positions are clear, but the moving car
        // crosses the patrol nose between snapshots.
        assert!(board.traffic_blocks_patrol_vehicle((0.,-3.),(0.,-2.8),0.));
        assert!(!board.traffic_blocks_patrol_vehicle((0.,-6.),(0.,-5.8),0.));
        assert!(board.traffic_blocks_patrol_vehicle((f32::NAN,0.),(0.,0.),0.));
        let elsewhere=Board::on(Map::Island,&[0,1],5,7);
        assert!(!elsewhere.traffic_blocks_patrol_vehicle((0.,-3.),(0.,-2.8),0.));
    }

    fn wedged_patrol(seed: u64) -> (crate::city_heist::CityGame, Board, (f32, f32)) {
        use crate::city_heist::{CityConfig, CityGame};
        let seats = [(0, false), (1, true), (2, true)];
        let config = CityConfig { police: 1, jobs: 1, host_police: true, ..Default::default() };
        let mut city = CityGame::seeded(&seats, config, false, seed);
        let mut board = Board::for_room(Map::Frozen, &[0, 1, 2], 10, seed);
        board.use_manual_events();
        city.ready(0);
        // Past the night's opening countdown, which holds everyone still.
        for _ in 0..200 {
            city.tick_streets(0.02, &[], &[0, 1, 2], &mut board);
        }
        // The playtest jam: a crossing taxi off the front-left corner and a
        // car pressed against the right side, both unable to move on.
        let at = (16., 16.);
        city.qa_place_car(0, at, 0.);
        board.qa_park_traffic(0, (14.6, 18.4), std::f32::consts::FRAC_PI_2, 30.);
        board.qa_park_traffic(1, (18., 16.2), 0., 30.);
        (city, board, at)
    }
    fn drive(city: &mut crate::city_heist::CityGame, board: &mut Board, seconds: f32, stick: crate::bout::Stick) {
        for _ in 0..(seconds / 0.02) as usize {
            board.traffic_tick_city(0.02, &city.patrol_traffic_obstacles(), &city.patrol_sirens());
            city.tick_streets(0.02, &[(0, stick, 0.)], &[0, 1, 2], board);
        }
    }
    fn patrol_at(city: &crate::city_heist::CityGame) -> (f32, f32) {
        let me = city.view_for(Some(0));
        let a = me.actors.iter().find(|a| a.seat == 0).unwrap();
        (a.x.unwrap(), a.z.unwrap())
    }
    #[test]
    fn a_patrol_car_wedged_at_a_junction_drives_out_as_its_driver_presses_on() {
        let (mut city, mut board, start) = wedged_patrol(3);
        assert!(board.traffic_blocks_patrol_vehicle(start, (start.0, start.1 + 0.3), 0.), "the jam blocks the car");
        drive(&mut city, &mut board, 1., crate::bout::Stick { x: 0., z: 1. });
        assert!(board.qa_nearest_traffic(start).is_some_and(|(_, _, d)| d > 2.3), "a slow push moves the car in the way");
        drive(&mut city, &mut board, 5., crate::bout::Stick { x: 0., z: 1. });
        let now = patrol_at(&city);
        assert!((now.0 - start.0).hypot(now.1 - start.1) > 6., "the patrol car drove away: {now:?}");
    }
    #[test]
    fn a_car_pinned_against_a_parked_patrol_car_gives_way() {
        let (mut city, mut board, start) = wedged_patrol(5);
        drive(&mut city, &mut board, 3., crate::bout::Stick::default());
        let near = board.qa_nearest_traffic(start).unwrap();
        assert!(near.2 > 4., "pinned traffic left after a couple of seconds: {near:?}");
    }
    #[test]
    fn traffic_pulls_over_and_stops_for_a_patrol_car_with_its_lights_on() {
        let mut board = Board::for_room(Map::Frozen, &[0, 1, 2], 10, 7);
        let car = board.traffic.cars.iter().find(|car| !in_junction(car.at)).cloned().unwrap();
        let siren = (car.at.0 + car.yaw.sin() * 8., car.at.1 + car.yaw.cos() * 8.);
        for _ in 0..50 {
            board.traffic_tick_city(0.02, &[], &[siren]);
        }
        let after = board.traffic.cars.iter().find(|c| c.id == car.id).unwrap();
        assert!(after.speed < 0.01, "it stops");
        let right = (car.yaw.cos(), -car.yaw.sin());
        assert!(after.shove.0 * right.0 + after.shove.1 * right.1 > 0.5, "and edges to the curb");
        for _ in 0..200 {
            board.traffic_tick_city(0.02, &[], &[]);
        }
        let later = board.traffic.cars.iter().find(|c| c.id == car.id).unwrap();
        assert!(later.speed > 1., "it drives on once the patrol car has gone");
    }

    #[test]
    fn a_patrol_car_nosed_into_a_building_resets_onto_the_street() {
        use crate::bout::Stick;
        use crate::city_heist::{CityConfig, CityGame};
        let seats = [(0, false), (1, true), (2, true)];
        let config = CityConfig { police: 1, jobs: 1, host_police: true, ..Default::default() };
        let mut city = CityGame::seeded(&seats, config, false, 3);
        let mut board = Board::for_room(Map::Frozen, &[0, 1, 2], 10, 3);
        board.use_manual_events();
        city.ready(0);
        for _ in 0..200 {
            city.tick_streets(0.02, &[], &[0, 1, 2], &mut board);
        }
        city.qa_place_car(0, (17.5, 22.), std::f32::consts::FRAC_PI_2);
        let press = [(0, Stick { x: 1., z: 0. }, 0.)];
        for _ in 0..250 {
            city.tick_streets(0.02, &press, &[0, 1, 2], &mut board);
        }
        assert!(city.view_for(Some(0)).car_stuck);
        for _ in 0..60 {
            city.tick_streets(0.02, &[(0, Stick::default(), 0.)], &[0, 1, 2], &mut board);
        }
        assert!(city.view_for(Some(0)).car_stuck, "letting go keeps the reset on offer");
        assert!(city.reset_car(0, &mut board), "reset");
        let view = city.view_for(Some(0));
        let me = view.actors.iter().find(|a| a.seat == 0).unwrap();
        let at = (me.x.unwrap(), me.z.unwrap());
        for _ in 0..30 {
            city.tick_streets(0.02, &[(0, Stick::default(), 0.)], &[0, 1, 2], &mut board);
        }
        let view = city.view_for(Some(0));
        let me = view.actors.iter().find(|a| a.seat == 0).unwrap();
        assert_eq!((me.x.unwrap(), me.z.unwrap()), at, "it stays where it was put");
        let yaw = me.yaw;
        let ahead = [(0, Stick { x: yaw.sin(), z: yaw.cos() }, 0.)];
        for _ in 0..150 {
            city.tick_streets(0.02, &ahead, &[0, 1, 2], &mut board);
        }
        let view = city.view_for(Some(0));
        let me = view.actors.iter().find(|a| a.seat == 0).unwrap();
        assert!(((me.x.unwrap() - at.0).hypot(me.z.unwrap() - at.1)) > 6., "drives off from {at:?} to {:?}", (me.x, me.z));
    }
}
