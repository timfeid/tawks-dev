//! Last Floor: everyone rides one glass elevator down a tall tower and jumps
//! off onto a passing floor. The lower the floor, the more it pays; ride it
//! all the way to the street and you crash for nothing. Each ride adds a
//! trick: the car speeds up, the floor counter blanks out and then lies, the
//! car stutters to a halt and lurches on. The last ride pays double.
//!
//! The counter is the only exact reading and it is not always honest; the
//! tower itself is always honest but only marks every tenth floor.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Skill, TICK_DT};

pub const KEY: &str = "lastfloor";
pub const TOP: u32 = 40;
pub const FLOOR_HEIGHT: f32 = 3.0;
pub const RIDES: usize = 4;
pub const BOARDING_SECONDS: f32 = 2.5;
pub const REVEAL_SECONDS: f32 = 3.0;
pub const HOP_SECONDS: f32 = 0.45;
pub const DEEPEST_BONUS: u32 = 25;
/// How much faster the car drops once nobody is left aboard.
pub const EMPTY_RUSH: f32 = 4.0;
pub const LURCH_SECONDS: f32 = 0.8;
pub const LURCH_BOOST: f32 = 1.6;
/// The car always reaches the street well inside this, so it only guards
/// against a stuck ride.
const MAX_FALL_SECONDS: f32 = 30.0;
pub const CAR_Z: f32 = 0.0;
pub const LEDGE_Z: f32 = 3.2;
const SLOT_SPACING: f32 = 1.45;

/// What a jump onto `floor` pays, before the final ride doubles it.
pub fn points(floor: u32) -> u32 {
    if floor == 0 || floor > TOP {
        return 0;
    }
    let reach = (TOP + 1 - floor) as f32 / TOP as f32;
    (40.0 * reach + 60.0 * reach * reach).round() as u32
}

/// The floor a rider lands on when they jump at `height` floors up.
pub fn landing(height: f32) -> u32 {
    (height.ceil().max(1.0) as u32).min(TOP)
}

#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub name: &'static str,
    pub speed: f32,
    pub accel: f32,
    pub max_speed: f32,
    pub stutters: usize,
    pub blanks: usize,
    pub lies: usize,
    pub double: bool,
}

pub const PROFILES: [Profile; RIDES] = [
    Profile {
        name: "Express",
        speed: 3.0,
        accel: 0.15,
        max_speed: 5.0,
        stutters: 0,
        blanks: 0,
        lies: 0,
        double: false,
    },
    Profile {
        name: "Rush Hour",
        speed: 2.6,
        accel: 0.6,
        max_speed: 8.0,
        stutters: 0,
        blanks: 4,
        lies: 0,
        double: false,
    },
    Profile {
        name: "Bad Wiring",
        speed: 3.2,
        accel: 0.4,
        max_speed: 8.0,
        stutters: 2,
        blanks: 3,
        lies: 2,
        double: false,
    },
    Profile {
        name: "Free Fall",
        speed: 3.0,
        accel: 0.9,
        max_speed: 10.0,
        stutters: 2,
        blanks: 4,
        lies: 2,
        double: true,
    },
];

#[derive(Clone, Copy, Debug)]
pub struct Stutter {
    pub at: f32,
    pub hold: f32,
}

/// A stretch of the ride when the counter shows nothing (`offset` None) or
/// shows the wrong floor.
#[derive(Clone, Copy, Debug)]
pub struct Glitch {
    pub from: f32,
    pub to: f32,
    pub offset: Option<i32>,
}

/// The elevator car. Pure motion, so a ride can be played out ahead of time
/// to know how long it lasts.
#[derive(Clone, Debug)]
pub struct Car {
    pub height: f32,
    pub speed: f32,
    /// Seconds the car has been moving this ride; holds do not count.
    pub run: f32,
    pub hold: f32,
    pub lurch: f32,
    next_stutter: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarEvent {
    Stutter,
    Lurch,
}

impl Car {
    pub fn at_top() -> Self {
        Car {
            height: TOP as f32 + 0.999,
            speed: 0.0,
            run: 0.0,
            hold: 0.0,
            lurch: 0.0,
            next_stutter: 0,
        }
    }

    pub fn step(
        &mut self,
        profile: &Profile,
        stutters: &[Stutter],
        dt: f32,
        empty: bool,
    ) -> Option<CarEvent> {
        if self.hold > 0.0 && !empty {
            self.hold -= dt;
            self.speed = 0.0;
            if self.hold <= 0.0 {
                self.hold = 0.0;
                self.lurch = LURCH_SECONDS;
                return Some(CarEvent::Lurch);
            }
            return None;
        }
        self.hold = 0.0;
        self.run += dt;
        let base = (profile.speed + profile.accel * self.run).min(profile.max_speed);
        let mut boost = if self.lurch > 0.0 { LURCH_BOOST } else { 1.0 };
        if empty {
            boost *= EMPTY_RUSH;
        }
        self.lurch = (self.lurch - dt).max(0.0);
        self.speed = base * boost;
        self.height -= self.speed * dt;
        if !empty {
            if let Some(stutter) = stutters.get(self.next_stutter) {
                if self.height <= stutter.at {
                    self.next_stutter += 1;
                    self.height = stutter.at;
                    self.hold = stutter.hold;
                    self.speed = 0.0;
                    return Some(CarEvent::Stutter);
                }
            }
        }
        None
    }

    /// How long the car takes from the top to the street with nobody
    /// jumping off.
    pub fn fall_seconds(profile: &Profile, stutters: &[Stutter]) -> f32 {
        let mut car = Car::at_top();
        let mut t = 0.0;
        while car.height > 0.0 && t < MAX_FALL_SECONDS {
            car.step(profile, stutters, TICK_DT, false);
            t += TICK_DT;
        }
        t
    }
}

/// Everything a ride does to the riders, settled before it starts.
#[derive(Clone, Debug)]
pub struct Plan {
    pub stutters: Vec<Stutter>,
    pub glitches: Vec<Glitch>,
    pub seconds: f32,
}

impl Plan {
    fn roll(profile: &Profile, rng: &mut StdRng) -> Self {
        let mut stutters: Vec<Stutter> = (0..profile.stutters)
            .map(|_| Stutter {
                at: rng.random_range(9.0..30.0),
                hold: rng.random_range(0.35..0.7),
            })
            .collect();
        stutters.sort_by(|a, b| b.at.total_cmp(&a.at));
        stutters.dedup_by(|a, b| (a.at - b.at).abs() < 3.0);
        let seconds = Car::fall_seconds(profile, &stutters);
        let mut glitches = Vec::new();
        for _ in 0..profile.blanks {
            let from = rng.random_range(0.8..(seconds - 0.5).max(1.0));
            glitches.push(Glitch {
                from,
                to: from + rng.random_range(0.15..0.4),
                offset: None,
            });
        }
        for index in 0..profile.lies {
            // One lie lands in the last stretch, where it hurts.
            let from = if index == 0 {
                (seconds - rng.random_range(1.4..2.0)).max(1.0)
            } else {
                rng.random_range(1.0..(seconds - 2.5).max(1.5))
            };
            let offset = if index == 0 || rng.random_bool(0.7) {
                rng.random_range(3..=6)
            } else {
                -rng.random_range(2..=4)
            };
            glitches.push(Glitch {
                from,
                to: from + rng.random_range(0.8..1.3),
                offset: Some(offset),
            });
        }
        glitches.sort_by(|a, b| a.from.total_cmp(&b.from));
        let mut kept: Vec<Glitch> = Vec::new();
        for glitch in glitches {
            if kept.last().is_none_or(|last| glitch.from > last.to + 0.1) {
                kept.push(glitch);
            }
        }
        Plan {
            stutters,
            glitches: kept,
            seconds,
        }
    }

    fn glitch_at(&self, clock: f32) -> Option<&Glitch> {
        self.glitches
            .iter()
            .find(|g| clock >= g.from && clock < g.to)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ride {
    Aboard,
    Jumped { floor: u32, from: f32, at: f32 },
    Crashed,
}

#[derive(Clone, Debug)]
struct Mind {
    skill: Skill,
    /// 0 is a cautious rider who gets off high, 1 a greedy one who waits.
    nerve: f32,
    target: f32,
    reaction: f32,
    compensation: f32,
    trusting: bool,
    bias: f32,
    /// The real delay before this ride's press, against the `reaction` it plans for.
    lag: f32,
    press_at: Option<f32>,
    spooked: bool,
}

#[derive(Clone, Debug)]
pub struct Rider {
    pub seat: u8,
    pub width: f32,
    pub slot: usize,
    pub ride: Ride,
    /// Released the button since boarding, so a held key cannot jump.
    pub armed: bool,
    pub total: u32,
    pub ride_points: u32,
    /// Each finished ride: the floor jumped to, or None for a crash.
    pub log: Vec<Option<u32>>,
    mind: Option<Mind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Boarding,
    Falling,
    Reveal,
    Done,
}

pub struct LastFloor {
    pub riders: Vec<Rider>,
    pub stage: Stage,
    /// Zero-based ride number.
    pub ride: usize,
    pub car: Car,
    pub plans: Vec<Plan>,
    pub time: f32,
    pub clock: f32,
    pub limit: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

fn triggered(input: &Input) -> bool {
    input.fire || input.jump || input.press.is_some()
}

impl LastFloor {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let plans: Vec<Plan> = PROFILES.iter().map(|p| Plan::roll(p, &mut rng)).collect();
        let slowest_mass = contestants.iter().map(|c| BodyProfile::new(c.body_width).mass)
            .fold(1.0_f32, f32::min);
        let limit = (plans
            .iter()
            .map(|p| BOARDING_SECONDS + p.seconds + REVEAL_SECONDS)
            .sum::<f32>()
            / (1.0 + (slowest_mass - 1.0) * 0.5))
            .ceil();
        let bots = contestants.iter().filter(|c| c.is_bot).count().max(1);
        let mut nerves: Vec<f32> = (0..bots)
            .map(|i| (i as f32 + rng.random_range(0.15..0.85)) / bots as f32)
            .collect();
        nerves.shuffle(&mut rng);
        let riders = contestants
            .iter()
            .enumerate()
            .map(|(slot, c)| Rider {
                seat: c.seat,
                width: BodyProfile::new(c.body_width).width,
                slot,
                ride: Ride::Aboard,
                armed: false,
                total: 0,
                ride_points: 0,
                log: Vec::new(),
                mind: c.is_bot.then(|| Mind {
                    skill: Skill(c.skill),
                    nerve: nerves.pop().unwrap_or(0.5),
                    target: 6.0,
                    reaction: 0.5,
                    compensation: 0.8,
                    trusting: false,
                    bias: 0.0,
                    lag: 0.5,
                    press_at: None,
                    spooked: false,
                }),
            })
            .collect();
        let mut game = LastFloor {
            riders,
            stage: Stage::Boarding,
            ride: 0,
            car: Car::at_top(),
            plans,
            time: 0.0,
            clock: 0.0,
            limit,
            rng,
            placed: None,
        };
        game.board();
        game
    }

    pub fn profile(&self) -> &'static Profile {
        &PROFILES[self.ride.min(RIDES - 1)]
    }

    fn plan(&self) -> &Plan {
        &self.plans[self.ride.min(RIDES - 1)]
    }

    fn car_load(&self) -> f32 {
        let aboard = self.riders.iter().filter(|r| r.ride == Ride::Aboard);
        let (total, count) = aboard.fold((0.0, 0u32), |(total, count), rider| {
            (total + BodyProfile::new(rider.width).mass, count + 1)
        });
        if count == 0 { 1.0 } else { total / count as f32 }
    }

    fn load_factor(&self) -> f32 {
        1.0 + (self.car_load() - 1.0) * 0.5
    }

    pub fn rider(&self, seat: u8) -> Option<&Rider> {
        self.riders.iter().find(|r| r.seat == seat)
    }

    fn board(&mut self) {
        self.stage = Stage::Boarding;
        self.clock = 0.0;
        self.car = Car::at_top();
        let double = self.profile().double;
        let late = self.ride as f32 / (RIDES - 1) as f32;
        for rider in &mut self.riders {
            rider.ride = Ride::Aboard;
            rider.armed = false;
            rider.ride_points = 0;
            if let Some(mind) = rider.mind.as_mut() {
                let level = mind.skill.level();
                // Keep personality in the exit plan, but let practiced riders
                // choose a meaningfully lower floor without hugging the street.
                let double_risk = if double { 0.8 } else { 0.0 };
                mind.target = (14.0 - level * 9.0 - mind.nerve - double_risk
                    + self.rng.random_range(-1.0..1.0))
                .max(1.0);
                mind.reaction = mind.skill.reaction() * self.rng.random_range(0.85..1.15);
                mind.lag = (mind.reaction
                    + self.rng.random_range(-1.0..1.0) * (0.06 + (1.0 - level) * 0.12))
                    .max(0.08);
                if self.rng.random_bool((0.05 + (1.0 - level) * 0.08) as f64) {
                    mind.lag += self.rng.random_range(0.15..0.45);
                }
                mind.compensation =
                    (0.5 + level * 0.45 + self.rng.random_range(-0.15..0.15)).clamp(0.2, 1.1);
                mind.trusting = self.rng.random_bool((0.12 + (1.0 - level) * 0.6) as f64);
                mind.bias = self.rng.random_range(-1.0..1.0)
                    * (0.3 + (1.0 - level) * 1.7)
                    * (0.6 + late * 0.4);
                mind.press_at = None;
                mind.spooked = false;
            }
        }
    }

    /// What the floor counter shows right now; None when it is blank.
    pub fn shown(&self) -> Option<u32> {
        match self.stage {
            Stage::Boarding => Some(TOP),
            Stage::Falling => {
                let truth = landing(self.car.height);
                match self.plan().glitch_at(self.clock) {
                    Some(Glitch { offset: None, .. }) => None,
                    Some(Glitch {
                        offset: Some(offset),
                        ..
                    }) => Some((truth as i32 + offset).clamp(1, TOP as i32) as u32),
                    None => Some(truth),
                }
            }
            Stage::Reveal | Stage::Done => Some(0),
        }
    }

    fn jump(&mut self, index: usize, events: &mut Vec<Value>) {
        let floor = landing(self.car.height);
        let from = self.car.height.max(0.0);
        let time = self.time;
        let ride = self.ride;
        let rider = &mut self.riders[index];
        rider.ride = Ride::Jumped {
            floor,
            from,
            at: time,
        };
        events.push(json!({
            "kind": "jump",
            "seat": rider.seat,
            "floor": floor,
            "points": points(floor),
            "ride": ride + 1,
        }));
    }

    fn finish_ride(&mut self, events: &mut Vec<Value>) {
        let double = self.profile().double;
        let factor = if double { 2 } else { 1 };
        let crashed: Vec<u8> = self
            .riders
            .iter()
            .filter(|r| r.ride == Ride::Aboard)
            .map(|r| r.seat)
            .collect();
        for rider in &mut self.riders {
            if rider.ride == Ride::Aboard {
                rider.ride = Ride::Crashed;
            }
        }
        if !crashed.is_empty() {
            events.push(json!({ "kind": "crash", "seats": crashed, "ride": self.ride + 1 }));
        }
        let deepest = self
            .riders
            .iter()
            .filter_map(|r| match r.ride {
                Ride::Jumped { floor, .. } => Some(floor),
                _ => None,
            })
            .min();
        let mut deepest_seats = Vec::new();
        for rider in &mut self.riders {
            let (floor, earned) = match rider.ride {
                Ride::Jumped { floor, .. } => {
                    let bonus = if Some(floor) == deepest {
                        deepest_seats.push(rider.seat);
                        DEEPEST_BONUS
                    } else {
                        0
                    };
                    (Some(floor), (points(floor) + bonus) * factor)
                }
                _ => (None, 0),
            };
            rider.ride_points = earned;
            rider.total += earned;
            rider.log.push(floor);
        }
        if let Some(floor) = deepest {
            events.push(json!({
                "kind": "deepest",
                "seats": deepest_seats,
                "floor": floor,
                "bonus": DEEPEST_BONUS * factor,
            }));
        }
        self.car.height = 0.0;
        self.car.speed = 0.0;
        self.stage = Stage::Reveal;
        self.clock = 0.0;
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, u32)> = self.riders.iter().map(|r| (r.seat, r.total)).collect();
        order.sort_by(|a, b| b.1.cmp(&a.1));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, total) in order {
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if previous == total => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(total);
        }
        self.stage = Stage::Done;
        self.placed = Some(groups);
    }

    fn position(&self, rider: &Rider) -> (f32, f32, f32) {
        let count = self.riders.len().max(1);
        let across = count.min(4);
        let row = rider.slot / 4;
        let col = rider.slot % 4;
        let in_row = if row + 1 < count.div_ceil(4) || count % 4 == 0 {
            across
        } else {
            count % 4
        };
        let x = (col as f32 - (in_row as f32 - 1.0) / 2.0) * SLOT_SPACING;
        let car_z = CAR_Z - 0.55 + row as f32 * 1.1;
        let car_y = self.car.height.max(0.0) * FLOOR_HEIGHT;
        match rider.ride {
            Ride::Aboard | Ride::Crashed => (x, car_y, car_z),
            Ride::Jumped { floor, from, at } => {
                let p = ((self.time - at) / HOP_SECONDS).clamp(0.0, 1.0);
                let y = from * FLOOR_HEIGHT
                    + (floor as f32 - from) * FLOOR_HEIGHT * p
                    + (p * std::f32::consts::PI).sin() * 1.3;
                (x, y, car_z + (LEDGE_Z + row as f32 * 0.6 - car_z) * p)
            }
        }
    }

    fn think(&mut self, index: usize) -> bool {
        let clock = self.clock;
        let truth = self.car.height;
        let speed = self.car.speed;
        let holding = self.car.hold > 0.0;
        let shown = self.shown();
        let roll: f64 = self.rng.random();
        let cruise = self.profile().speed * self.load_factor();
        let Some(mind) = self.riders[index].mind.as_mut() else {
            return false;
        };
        if let Some(at) = mind.press_at {
            if clock >= at {
                mind.press_at = None;
                return true;
            }
            return false;
        }
        if holding && !mind.spooked {
            mind.spooked = true;
            if roll < ((1.0 - mind.nerve) * 0.3) as f64 {
                mind.press_at = Some(clock + mind.lag);
            }
            return false;
        }
        let reading = if mind.trusting {
            match shown {
                Some(floor) => floor as f32 - 0.5,
                None => return false,
            }
        } else {
            truth + mind.bias
        };
        let expected = speed.max(cruise);
        let lead = expected * mind.reaction * mind.compensation;
        if reading <= mind.target + lead {
            mind.press_at = Some(clock + mind.lag);
        }
        false
    }
}

impl Bout for LastFloor {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rule": "Jump off onto a passing floor. The lower you get off, the more you score. Ride to the bottom and you crash.",
            "radius": 14.0,
            "top": TOP,
            "floor_height": FLOOR_HEIGHT,
            "rides": RIDES,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.stage == Stage::Done {
            return;
        }
        self.time += dt;
        self.clock += dt;
        let falling = self.stage == Stage::Falling;
        for (seat, input) in inputs {
            let Some(index) = self.riders.iter().position(|r| r.seat == *seat) else {
                continue;
            };
            let pressed = triggered(input);
            let rider = &self.riders[index];
            if pressed
                && falling
                && rider.armed
                && rider.ride == Ride::Aboard
                && self.car.height > 0.0
            {
                self.jump(index, events);
            }
            self.riders[index].armed = !pressed;
        }
        match self.stage {
            Stage::Boarding => {
                if self.clock >= BOARDING_SECONDS {
                    self.stage = Stage::Falling;
                    self.clock = 0.0;
                    events.push(json!({ "kind": "doors", "ride": self.ride + 1 }));
                }
            }
            Stage::Falling => {
                let empty = !self.riders.iter().any(|r| r.ride == Ride::Aboard);
                let factor = self.load_factor();
                let mut profile = *self.profile();
                profile.speed *= factor;
                profile.accel *= factor;
                profile.max_speed *= factor;
                let stutters = self.plans[self.ride].stutters.clone();
                match self.car.step(&profile, &stutters, dt, empty) {
                    Some(CarEvent::Stutter) => {
                        events.push(json!({ "kind": "stutter", "floor": landing(self.car.height) }))
                    }
                    Some(CarEvent::Lurch) => events.push(json!({ "kind": "lurch" })),
                    None => {}
                }
                if self.car.height <= 0.0 || self.clock >= MAX_FALL_SECONDS {
                    self.finish_ride(events);
                }
            }
            Stage::Reveal => {
                if self.clock >= REVEAL_SECONDS {
                    if self.ride + 1 >= RIDES {
                        self.settle();
                        events.push(json!({ "kind": "final" }));
                    } else {
                        self.ride += 1;
                        self.board();
                        events.push(json!({
                            "kind": "ride",
                            "ride": self.ride + 1,
                            "name": self.profile().name,
                            "double": self.profile().double,
                        }));
                    }
                }
            }
            Stage::Done => {}
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.stage != Stage::Falling {
            return Input::default();
        }
        let Some(index) = self.riders.iter().position(|r| r.seat == seat) else {
            return Input::default();
        };
        if self.riders[index].ride != Ride::Aboard {
            return Input::default();
        }
        Input {
            fire: self.think(index),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        let profile = self.profile();
        let stage = match self.stage {
            Stage::Boarding => "boarding",
            Stage::Falling => "falling",
            Stage::Reveal => "reveal",
            Stage::Done => "done",
        };
        let stage_seconds = match self.stage {
            Stage::Boarding => BOARDING_SECONDS,
            Stage::Reveal => REVEAL_SECONDS,
            _ => 0.0,
        };
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": self.limit,
            "ride": self.ride + 1,
            "rides": RIDES,
            "ride_name": profile.name,
            "double": profile.double,
            "stage": stage,
            "stage_left": round((stage_seconds - self.clock).max(0.0)),
            "top": TOP,
            "floor_height": FLOOR_HEIGHT,
            "height": round(self.car.height.max(0.0)),
            "speed": round(self.car.speed),
            "load": round(self.car_load()),
            "shown": self.shown(),
            "stutter": self.car.hold > 0.0,
            "lurch": self.car.lurch > 0.0,
            "players": self.riders.iter().map(|r| {
                let (x, y, z) = self.position(r);
                let (state, floor) = match r.ride {
                    Ride::Aboard => ("aboard", None),
                    Ride::Jumped { floor, .. } => ("jumped", Some(floor)),
                    Ride::Crashed => ("crashed", None),
                };
                json!({
                    "seat": r.seat,
                    "x": round(x),
                    "y": round(y),
                    "z": round(z),
                    "yaw": std::f32::consts::PI,
                    "vx": 0.0,
                    "vz": 0.0,
                    "state": state,
                    "floor": floor,
                    "points": r.total,
                    "ride_points": r.ride_points,
                    "log": r.log,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn press() -> Input {
        Input {
            fire: true,
            ..Default::default()
        }
    }

    fn tick(game: &mut LastFloor, inputs: &[(u8, Input)]) -> Vec<Value> {
        let mut events = Vec::new();
        game.step(inputs, TICK_DT, &mut events);
        events
    }

    fn idle(count: u8) -> Vec<(u8, Input)> {
        (0..count).map(|s| (s, Input::default())).collect()
    }

    fn until(game: &mut LastFloor, count: u8, done: impl Fn(&LastFloor) -> bool) {
        for _ in 0..(60 * 120) {
            if done(game) {
                return;
            }
            tick(game, &idle(count));
        }
        panic!("condition never met");
    }

    fn run_bots(game: &mut LastFloor, seats: &[u8]) -> Vec<Value> {
        let mut events = Vec::new();
        for _ in 0..(60 * 120) {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|s| (*s, game.bot_input(*s, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        events
    }

    #[test]
    fn lower_floors_pay_more_and_the_street_pays_nothing() {
        assert_eq!(points(1), 100);
        assert_eq!(points(0), 0);
        assert_eq!(points(TOP + 1), 0);
        for floor in 1..TOP {
            assert!(points(floor) > points(floor + 1), "floor {floor}");
        }
        assert!(points(TOP) > 0);
        assert_eq!(landing(0.2), 1);
        assert_eq!(landing(7.0), 7);
        assert_eq!(landing(7.01), 8);
    }

    #[test]
    fn heavier_riders_speed_the_shared_car_until_they_jump() {
        let field = |width| (0..4).map(|seat| {
            let mut rider = Contestant::person(seat);
            rider.body_width = width;
            rider
        }).collect::<Vec<_>>();
        let mut light = LastFloor::new(&field(0.7), 9);
        let mut heavy = LastFloor::new(&field(1.4), 9);
        until(&mut light, 4, |g| g.stage == Stage::Falling);
        until(&mut heavy, 4, |g| g.stage == Stage::Falling);
        tick(&mut light, &[]);
        tick(&mut heavy, &[]);
        assert!(heavy.car.speed > light.car.speed);
        assert!(heavy.car.height < light.car.height);
        assert!(light.limit >= heavy.limit);

        let mut mixed = field(0.7);
        mixed[0].body_width = 1.4;
        let mut game = LastFloor::new(&mixed, 9);
        until(&mut game, 4, |g| g.stage == Stage::Falling);
        let before = game.snapshot()["load"].as_f64().unwrap();
        game.riders[0].armed = true;
        tick(&mut game, &[(0, press())]);
        assert!(game.snapshot()["load"].as_f64().unwrap() < before);
    }

    #[test]
    fn a_press_lands_on_the_floor_being_passed() {
        let mut game = LastFloor::new(&people(2), 3);
        until(&mut game, 2, |g| {
            g.stage == Stage::Falling && g.car.height < 20.0
        });
        let expected = landing(game.car.height);
        let events = tick(&mut game, &[(0, press()), (1, Input::default())]);
        let jump = events.iter().find(|e| e["kind"] == "jump").expect("a jump");
        assert_eq!(jump["seat"], 0);
        assert_eq!(jump["floor"], expected);
        assert!(
            matches!(game.rider(0).unwrap().ride, Ride::Jumped { floor, .. } if floor == expected)
        );
        until(&mut game, 2, |g| g.stage == Stage::Reveal);
        let me = game.rider(0).unwrap();
        assert_eq!(me.log, vec![Some(expected)]);
        assert_eq!(
            me.total,
            points(expected) + DEEPEST_BONUS,
            "the only jumper is also the deepest"
        );
        assert_eq!(game.rider(1).unwrap().log, vec![None]);
        assert_eq!(
            game.rider(1).unwrap().total,
            0,
            "riding to the street pays nothing"
        );
    }

    #[test]
    fn a_held_button_or_a_press_during_boarding_never_jumps() {
        let mut game = LastFloor::new(&people(1), 5);
        for _ in 0..(60 * 6) {
            tick(&mut game, &[(0, press())]);
        }
        assert_eq!(game.stage, Stage::Falling);
        assert_eq!(
            game.rider(0).unwrap().ride,
            Ride::Aboard,
            "holding from the doors does nothing"
        );
        tick(&mut game, &idle(1));
        tick(&mut game, &[(0, press())]);
        assert!(
            matches!(game.rider(0).unwrap().ride, Ride::Jumped { .. }),
            "release and press again jumps"
        );
    }

    #[test]
    fn one_jump_per_ride() {
        let mut game = LastFloor::new(&people(1), 9);
        until(&mut game, 1, |g| {
            g.stage == Stage::Falling && g.car.height < 30.0
        });
        tick(&mut game, &[(0, press())]);
        let first = game.rider(0).unwrap().ride;
        for _ in 0..30 {
            tick(&mut game, &idle(1));
            let events = tick(&mut game, &[(0, press())]);
            assert!(events.iter().all(|e| e["kind"] != "jump"));
        }
        assert_eq!(game.rider(0).unwrap().ride, first);
    }

    #[test]
    fn once_everyone_is_off_the_car_rushes_down() {
        let mut game = LastFloor::new(&people(2), 2);
        until(&mut game, 2, |g| {
            g.stage == Stage::Falling && g.car.height < 36.0
        });
        tick(&mut game, &[(0, press()), (1, press())]);
        let start = game.time;
        until(&mut game, 2, |g| g.stage == Stage::Reveal);
        assert!(
            game.time - start < 3.0,
            "an empty car drops fast, took {}",
            game.time - start
        );
    }

    #[test]
    fn the_counter_is_honest_on_the_first_ride_and_lies_later() {
        let mut game = LastFloor::new(&people(1), 11);
        let mut lied = [false; RIDES];
        let mut blanked = [false; RIDES];
        for _ in 0..(60 * 120) {
            if game.stage == Stage::Falling {
                match game.shown() {
                    None => blanked[game.ride] = true,
                    Some(shown) if shown != landing(game.car.height) => lied[game.ride] = true,
                    _ => {}
                }
            }
            tick(&mut game, &idle(1));
            if game.placements().is_some() {
                break;
            }
        }
        assert!(!lied[0] && !blanked[0], "ride one is honest");
        assert!(blanked[1] && !lied[1], "ride two only blanks");
        assert!(lied[2] && lied[3], "the last two rides lie");
    }

    #[test]
    fn the_last_ride_doubles_and_later_rides_are_faster() {
        assert!(PROFILES[RIDES - 1].double);
        assert!(PROFILES[..RIDES - 1].iter().all(|p| !p.double));
        let game = LastFloor::new(&people(2), 4);
        assert!(game.plans[RIDES - 1].seconds < game.plans[0].seconds);
        let mut game = LastFloor::new(&people(1), 4);
        until(&mut game, 1, |g| {
            g.ride == RIDES - 1 && g.stage == Stage::Falling && g.car.height < 25.0
        });
        let floor = landing(game.car.height);
        tick(&mut game, &[(0, press())]);
        until(&mut game, 1, |g| g.stage != Stage::Falling);
        assert_eq!(
            game.rider(0).unwrap().ride_points,
            (points(floor) + DEEPEST_BONUS) * 2
        );
    }

    #[test]
    fn stutters_halt_the_car_then_lurch() {
        let mut game = LastFloor::new(&people(1), 21);
        let mut seen = Vec::new();
        for _ in 0..(60 * 120) {
            seen.extend(
                tick(&mut game, &idle(1))
                    .into_iter()
                    .filter_map(|e| e["kind"].as_str().map(str::to_owned)),
            );
            if game.placements().is_some() {
                break;
            }
        }
        assert!(seen.iter().filter(|k| *k == "stutter").count() >= 2);
        assert_eq!(
            seen.iter().filter(|k| *k == "stutter").count(),
            seen.iter().filter(|k| *k == "lurch").count()
        );
        assert_eq!(seen.iter().filter(|k| *k == "crash").count(), RIDES);
    }

    #[test]
    fn nobody_jumping_is_a_four_way_tie_at_zero() {
        let mut game = LastFloor::new(&people(4), 8);
        until(&mut game, 4, |g| g.placements().is_some());
        assert_eq!(game.placements().unwrap(), vec![vec![0, 1, 2, 3]]);
        assert!(
            game.riders
                .iter()
                .all(|r| r.total == 0 && r.log == vec![None; RIDES])
        );
    }

    #[test]
    fn equal_totals_share_a_place() {
        let mut game = LastFloor::new(&people(3), 13);
        for ride in 0..RIDES {
            until(&mut game, 3, |g| {
                g.ride == ride && g.stage == Stage::Falling && g.car.height < 15.0
            });
            tick(
                &mut game,
                &[(0, press()), (1, press()), (2, Input::default())],
            );
            until(&mut game, 3, |g| g.stage != Stage::Falling);
        }
        until(&mut game, 3, |g| g.placements().is_some());
        assert_eq!(game.placements().unwrap(), vec![vec![0, 1], vec![2]]);
    }

    #[test]
    fn a_leaver_keeps_riding_and_places_last() {
        let mut field = people(1);
        field.extend((1..4).map(|s| Contestant::bot(s, 0.7)));
        let mut game = LastFloor::new(&field, 17);
        let mut events = Vec::new();
        for _ in 0..(60 * 120) {
            let mut inputs = vec![(0u8, Input::default())];
            inputs.extend((1..4u8).map(|s| (s, game.bot_input(s, TICK_DT))));
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        let placed = game.placements().expect("finishes without the leaver");
        assert!(placed.last().unwrap().contains(&0));
        assert_eq!(game.rider(0).unwrap().total, 0);
    }

    #[test]
    fn a_bout_runs_between_forty_five_and_ninety_seconds() {
        for seed in 0..40 {
            let game = LastFloor::new(&people(4), seed);
            assert!(
                (45.0..=90.0).contains(&game.limit),
                "seed {seed}: {}",
                game.limit
            );
        }
        let mut game = LastFloor::new(&people(4), 1);
        until(&mut game, 4, |g| g.placements().is_some());
        assert!(
            game.time <= game.limit + 0.1 && game.time >= 45.0,
            "{}",
            game.time
        );
    }

    #[test]
    fn eight_bots_finish_and_everyone_gets_a_slot_in_the_car() {
        let field: Vec<Contestant> = (0..8).map(|s| Contestant::bot(s, 0.5)).collect();
        let mut game = LastFloor::new(&field, 23);
        let snap = game.snapshot();
        let xs: Vec<(f64, f64)> = snap["players"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["x"].as_f64().unwrap(), p["z"].as_f64().unwrap()))
            .collect();
        for (i, a) in xs.iter().enumerate() {
            for b in &xs[i + 1..] {
                assert!(
                    (a.0 - b.0).hypot(a.1 - b.1) > 1.0,
                    "riders overlap: {a:?} {b:?}"
                );
            }
        }
        let seats: Vec<u8> = (0..8).collect();
        let events = run_bots(&mut game, &seats);
        assert!(game.placements().is_some());
        assert!(events.iter().any(|e| e["kind"] == "jump"));
    }

    #[test]
    fn bots_hold_their_nerve_differently() {
        let field: Vec<Contestant> = (0..6).map(|s| Contestant::bot(s, 0.6)).collect();
        let game = LastFloor::new(&field, 31);
        let mut nerves: Vec<f32> = game
            .riders
            .iter()
            .map(|r| r.mind.as_ref().unwrap().nerve)
            .collect();
        nerves.sort_by(f32::total_cmp);
        assert!(
            nerves[0] < 0.25 && nerves[5] > 0.75,
            "a spread from cautious to greedy: {nerves:?}"
        );
    }

    /// Many seeded bouts of bots at every skill: they finish, mostly jump,
    /// and stronger bots reach deeper floors and score more.
    #[test]
    fn seeded_bot_balance() {
        let mut lines = Vec::new();
        let mut average = Vec::new();
        let mut mean_floors = Vec::new();
        for &(name, skill) in crate::bout::SKILLS {
            let (mut rides, mut crashes, mut floors, mut jumps, mut total) =
                (0u32, 0u32, 0u32, 0u32, 0u64);
            for seed in 0..150u64 {
                let field: Vec<Contestant> = (0..4).map(|s| Contestant::bot(s, skill)).collect();
                let mut game = LastFloor::new(&field, seed * 7 + 1);
                run_bots(&mut game, &[0, 1, 2, 3]);
                assert!(game.placements().is_some(), "{name} seed {seed} finished");
                for rider in &game.riders {
                    for entry in &rider.log {
                        rides += 1;
                        match entry {
                            Some(floor) => {
                                jumps += 1;
                                floors += floor;
                            }
                            None => crashes += 1,
                        }
                    }
                    total += rider.total as u64;
                }
            }
            let crash_rate = crashes as f32 / rides as f32;
            let mean = total as f32 / (150.0 * 4.0);
            let mean_floor = floors as f32 / jumps.max(1) as f32;
            lines.push(format!(
                "{name}: crash {:.0}%, mean floor {:.1}, mean score {:.0}",
                crash_rate * 100.0,
                mean_floor,
                mean,
            ));
            assert!(crash_rate < 0.1, "{name} crash rate {crash_rate}");
            mean_floors.push(mean_floor);
            average.push(mean);
        }
        println!("{}", lines.join("\n"));
        assert!(mean_floors.windows(2).all(|pair| pair[0] > pair[1]));
        assert!(average.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            average[3] > average[0] * 1.2,
            "ruthless beats relaxed: {average:?}"
        );
    }
}
