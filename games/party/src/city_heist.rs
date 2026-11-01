//! Separate, simultaneous city game. The board only supplies scenery/traffic;
//! heists, arrests, privacy, bots and scores have their own authoritative state.
use crate::bout::Stick;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

pub const BANK: (f32, f32) = (-4., 28.);
pub const POWER: (f32, f32) = (-12., 28.);
pub const ESCAPE: (f32, f32) = (-28., -28.);
pub const JAIL: (f32, f32) = (28., -28.);
const CREW_SPAWN: (f32, f32) = (-28., 28.);
const SPEED: f32 = 6.0;
const CARRY_SPEED: f32 = 5.4;
const SMOKE_SECONDS: f32 = 4.2;
pub const NIGHT_SECONDS: f32 = 360.;
const LIMIT: f32 = NIGHT_SECONDS;
const STREETS: [f32; 6] = [-64., -40., -16., 16., 40., 64.];
const PATROL: [(f32, f32); 12] = [
    (16., 16.),
    (40., 16.),
    (40., 40.),
    (-16., 40.),
    (-40., 40.),
    (-40., 16.),
    (-40., -16.),
    (-40., -40.),
    (16., -40.),
    (40., -40.),
    (40., -16.),
    (16., -16.),
];
const BRIEFING_SECONDS: f32 = 60.;
const JAIL_SECONDS: f32 = 18.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Police,
    Crew,
}
#[derive(Clone, Debug, Serialize)]
pub struct CityConfig {
    pub police: u8,
    pub jobs: u8,
    pub host_police: bool,
}
impl Default for CityConfig {
    fn default() -> Self {
        Self {
            police: 1,
            jobs: 3,
            host_police: false,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct CityInput {
    pub stick: Stick,
    pub yaw: f32,
    pub interact: bool,
    pub suction: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Smoke,
    Scanner,
}
#[derive(Clone, Debug, Serialize)]
pub struct CityActorView {
    pub seat: u8,
    pub side: Side,
    pub x: Option<f32>,
    pub z: Option<f32>,
    pub yaw: f32,
    pub jailed: bool,
    pub stunned: f32,
    pub carrying: bool,
    pub cuffed: f32,
    pub acting: bool,
    pub suction: bool,
    pub automated: bool,
    pub driving: bool,
    pub car: Option<[f32; 2]>,
    pub jail_seconds: f32,
    pub height: f32,
    pub velocity: [f32; 2],
}
#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub label: &'static str,
    pub x: f32,
    pub z: f32,
    pub progress: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct BriefingView {
    pub remaining: f32,
    pub ready: usize,
    pub total: usize,
    pub you_ready: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct CityView {
    pub round: u8,
    pub jobs: u8,
    pub remaining: f32,
    pub briefing: Option<BriefingView>,
    pub night_score: [u16; 2],
    pub patrol_route: Vec<[f32; 2]>,
    pub job_wait: f32,
    pub dispatch: Option<&'static str>,
    pub draw: bool,
    pub ready: f32,
    pub scores: [u8; 2],
    pub actors: Vec<CityActorView>,
    pub objective: &'static str,
    pub targets: Vec<Target>,
    pub bank: [f32; 2],
    pub jail: [f32; 2],
    pub escape: Option<[f32; 2]>,
    pub alarm: bool,
    pub alert: Option<[f32; 2]>,
    pub loot: Option<[f32; 2]>,
    pub loot_carrier: Option<u8>,
    pub vault_progress: Option<f32>,
    pub power_seconds: Option<f32>,
    pub smokes: Vec<[f32; 3]>,
    pub tools: [u8; 2],
    pub scanner: Option<&'static str>,
    pub scanner_bearing: Option<f32>,
    pub resolved: Option<Side>,
    pub reason: Option<&'static str>,
    pub resolved_for: f32,
}
#[derive(Clone)]
struct Actor {
    seat: u8,
    side: Side,
    at: (f32, f32),
    yaw: f32,
    jailed: bool,
    stun: f32,
    cuff: f32,
    interact: bool,
    suction: bool,
    until: f32,
    automated: bool,
    tools: [u8; 2],
    scan_until: f32,
    immune: f32,
    path: VecDeque<(f32, f32)>,
    goal: (f32, f32),
    repath: f32,
    last_seen: Option<((f32, f32), f32, f32, bool)>,
    in_car: bool,
    car_at: (f32, f32),
    drive_speed: f32,
    car_yaw: f32,
    patrol: usize,
    jail_left: f32,
    height: f32,
    up: f32,
    knock: (f32, f32),
    velocity: (f32, f32),
}
#[derive(Clone, Copy, PartialEq)]
enum Job {
    Power,
    Vault,
    Carry,
}
#[derive(Clone)]
struct Smoke {
    at: (f32, f32),
    left: f32,
}
#[derive(Clone)]
pub struct CityGame {
    pub config: CityConfig,
    pub round: u8,
    pub scores: [u8; 2],
    pub time: f32,
    seed: u64,
    actors: Vec<Actor>,
    job: Job,
    power: f32,
    switch: f32,
    vault: f32,
    carrier: Option<u8>,
    loot: Option<(f32, f32)>,
    escape_progress: f32,
    recover: f32,
    rescue: f32,
    smokes: Vec<Smoke>,
    pub resolved: Option<Side>,
    pub resolved_for: f32,
    demo: bool,
    reason: Option<&'static str>,
    alarm: bool,
    alarm_at: f32,
    briefing_left: f32,
    ready_seats: HashSet<u8>,
    night_score: [u16; 2],
    job_wait: f32,
    dispatch: Option<&'static str>,
    draw: bool,
}
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn buildings() -> &'static [(f32, f32)] {
    static B: std::sync::OnceLock<Vec<(f32, f32)>> = std::sync::OnceLock::new();
    B.get_or_init(|| {
        let map: serde_json::Value = serde_json::from_str(include_str!(
            "../../canvases/src/party/board/maps/frozen.json"
        ))
        .expect("city layout");
        map["props"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| {
                p["model"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("town-building-")
            })
            .map(|p| {
                (
                    p["x"].as_f64().unwrap() as f32,
                    p["z"].as_f64().unwrap() as f32,
                )
            })
            .collect()
    })
}
fn walkable(at: (f32, f32)) -> bool {
    at.0.is_finite()
        && at.1.is_finite()
        && at.0.abs() < 68.
        && at.1.abs() < 68.
        && buildings().iter().all(|&b| distance(at, b) > 4.1)
}
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let d = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * d.0 + (p.1 - a.1) * d.1) / (d.0 * d.0 + d.1 * d.1).max(0.001))
        .clamp(0., 1.);
    distance(p, (a.0 + d.0 * t, a.1 + d.1 * t))
}
fn road_point(at: (f32, f32)) -> (f32, f32) {
    let x = *STREETS
        .iter()
        .min_by(|a, b| (at.0 - **a).abs().total_cmp(&(at.0 - **b).abs()))
        .unwrap();
    let z = *STREETS
        .iter()
        .min_by(|a, b| (at.1 - **a).abs().total_cmp(&(at.1 - **b).abs()))
        .unwrap();
    if (at.0 - x).abs() < (at.1 - z).abs() {
        (x, at.1.clamp(-64., 64.))
    } else {
        (at.0.clamp(-64., 64.), z)
    }
}
fn on_road(at: (f32, f32)) -> bool {
    distance(at, road_point(at)) < 2.2 && walkable(at)
}
fn route(from: (f32, f32), goal: (f32, f32), driving: bool) -> VecDeque<(f32, f32)> {
    const N: usize = 67;
    let cell = |at: (f32, f32)| {
        let x = ((at.0 + 66.) / 2.).round().clamp(0., 66.) as usize;
        let z = ((at.1 + 66.) / 2.).round().clamp(0., 66.) as usize;
        z * N + x
    };
    let point = |id: usize| ((id % N) as f32 * 2. - 66., (id / N) as f32 * 2. - 66.);
    let start = cell(from);
    let end = cell(goal);
    let mut prior = vec![usize::MAX; N * N];
    // Navigation geometry is static. Cache it instead of rechecking every
    // building for every BFS cell, officer, and simulation tick.
    static GRID: std::sync::OnceLock<[Vec<bool>; 2]> = std::sync::OnceLock::new();
    let grid = GRID.get_or_init(|| {
        [
            (0..N * N).map(|id| walkable(point(id))).collect(),
            (0..N * N).map(|id| on_road(point(id))).collect(),
        ]
    });
    let passable = &grid[usize::from(driving)];
    prior[start] = start;
    let mut queue = VecDeque::from([start]);
    while let Some(id) = queue.pop_front() {
        if id == end {
            break;
        }
        let x = id % N;
        let z = id / N;
        for (nx, nz) in [
            (x.wrapping_sub(1), z),
            (x + 1, z),
            (x, z.wrapping_sub(1)),
            (x, z + 1),
        ] {
            if nx >= N || nz >= N {
                continue;
            }
            let next = nz * N + nx;
            if prior[next] != usize::MAX || !passable[next] {
                continue;
            }
            prior[next] = id;
            queue.push_back(next);
        }
    }
    if prior[end] == usize::MAX {
        return VecDeque::new();
    }
    let mut path = VecDeque::from([goal]);
    let mut id = end;
    while id != start {
        path.push_front(point(id));
        id = prior[id];
    }
    path
}
impl CityGame {
    pub fn new(seats: &[(u8, bool)], config: CityConfig, watch: bool) -> Self {
        Self::seeded(seats, config, watch, 0)
    }
    pub fn seeded(seats: &[(u8, bool)], config: CityConfig, watch: bool, seed: u64) -> Self {
        let actors = seats
            .iter()
            .enumerate()
            .map(|(index, &(seat, bot))| {
                let side = if index < config.police as usize {
                    Side::Police
                } else {
                    Side::Crew
                };
                let spawn = if side == Side::Police {
                    // Spread officers among public patrol posts rather than
                    // stacking an entire police squad on the same bank corner.
                    PATROL[(index * 3) % PATROL.len()]
                } else {
                    CREW_SPAWN
                };
                let angle = ((seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(u64::from(seat) * 1442695040888963407)
                    >> 32)
                    % 1000) as f32
                    / 1000.
                    * std::f32::consts::TAU;
                Actor {
                    seat,
                    side,
                    at: if side == Side::Police {
                        spawn
                    } else {
                        (spawn.0 + angle.cos() * 1.4, spawn.1 + angle.sin() * 1.4)
                    },
                    yaw: 0.,
                    jailed: false,
                    stun: 0.,
                    cuff: 0.,
                    interact: false,
                    suction: false,
                    until: 0.,
                    automated: bot || watch,
                    tools: [1, 1],
                    scan_until: 0.,
                    immune: 3.,
                    path: VecDeque::new(),
                    goal: spawn,
                    repath: 0.,
                    last_seen: None,
                    in_car: side == Side::Police,
                    car_at: spawn,
                    drive_speed: 0.,
                    car_yaw: 0.,
                    patrol: (index * 3 + 1) % PATROL.len(),
                    jail_left: 0.,
                    height: 0.,
                    up: 0.,
                    knock: (0., 0.),
                    velocity: (0., 0.),
                }
            })
            .collect();
        Self {
            config,
            seed,
            round: 1,
            scores: [0, 0],
            time: 0.,
            actors,
            job: Job::Power,
            power: 0.,
            switch: 0.,
            vault: 0.,
            carrier: None,
            loot: None,
            escape_progress: 0.,
            recover: 0.,
            rescue: 0.,
            smokes: Vec::new(),
            resolved: None,
            resolved_for: 0.,
            demo: watch,
            reason: None,
            alarm: false,
            alarm_at: 0.,
            briefing_left: if watch || seats.iter().all(|s| s.1) {
                0.
            } else {
                BRIEFING_SECONDS
            },
            ready_seats: HashSet::new(),
            night_score: [0, 0],
            job_wait: 0.,
            dispatch: None,
            draw: false,
        }
    }
    pub fn ready(&mut self, seat: u8) {
        if self.actors.iter().any(|a| a.seat == seat) {
            self.ready_seats.insert(seat);
        }
    }
    pub fn briefing(&self) -> bool {
        self.briefing_left > 0.
    }
    /// Server-only occupied vehicle footprints. Traffic yields to the car
    /// even while its officer is searching on foot.
    pub fn patrol_traffic_obstacles(&self) -> Vec<(f32, f32)> {
        self.actors.iter().filter(|a|a.side==Side::Police).flat_map(|a| {
            let offset=(a.car_yaw.sin()*0.9,a.car_yaw.cos()*0.9);
            [a.car_at,(a.car_at.0+offset.0,a.car_at.1+offset.1),
                (a.car_at.0-offset.0,a.car_at.1-offset.1)]
        }).collect()
    }
    pub fn toggle_car(&mut self, seat: u8) -> bool {
        if self.resolved.is_some() {
            return false;
        }
        let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.seat == seat && a.side == Side::Police && !a.jailed && a.stun <= 0.)
        else {
            return false;
        };
        if a.in_car {
            a.in_car = false;
            a.car_at = a.at;
            if let Some(at) = [
                (a.at.0 + 2.8, a.at.1),
                (a.at.0 - 2.8, a.at.1),
                (a.at.0, a.at.1 + 2.8),
                (a.at.0, a.at.1 - 2.8),
            ]
            .into_iter()
            .find(|p| walkable(*p))
            {
                a.at = at;
            }
        } else if distance(a.at, a.car_at) < 4. {
            a.in_car = true;
            a.at = a.car_at;
            a.car_yaw = a.yaw;
        } else {
            return false;
        }
        a.path.clear();
        a.drive_speed = 0.;
        a.interact = false;
        true
    }
    fn finish_job(&mut self, side: Side, message: &'static str) {
        if self.job_wait > 0. {
            return;
        }
        self.night_score[if side == Side::Police { 0 } else { 1 }] += 1;
        self.job_wait = 8.;
        self.dispatch = Some(message);
        self.carrier = None;
        self.loot = None;
        self.alarm = false;
    }
    fn reset_job(&mut self) {
        self.job = Job::Power;
        self.power = 0.;
        self.switch = 0.;
        self.vault = 0.;
        self.carrier = None;
        self.loot = None;
        self.escape_progress = 0.;
        self.recover = 0.;
        self.rescue = 0.;
        self.alarm = false;
        self.dispatch = None;
        let all_caught = self
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .all(|a| a.jailed);
        for a in &mut self.actors {
            a.interact = false;
            a.suction = false;
            a.cuff = 0.;
            a.tools = [(a.tools[0] + 1).min(2), (a.tools[1] + 1).min(2)];
            if all_caught && a.side == Side::Crew {
                a.jailed = false;
                a.jail_left = 0.;
                a.at = CREW_SPAWN;
                a.immune = 6.;
                a.path.clear();
            }
        }
    }
    pub fn control(&mut self, seat: u8, interact: bool, suction: bool) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat) {
            a.interact = interact;
            a.suction = suction;
            a.until = self.time + 0.3;
        }
    }
    pub fn automate(&mut self, seat: u8, on: bool) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat) {
            a.automated = on;
            a.interact = false;
            a.suction = false;
            a.until = 0.;
        }
    }
    pub fn use_tool(&mut self, seat: u8, tool: Tool) -> bool {
        let Some(a) = self
            .actors
            .iter_mut()
            .find(|a| a.seat == seat && !a.jailed && a.stun <= 0.)
        else {
            return false;
        };
        if self.resolved.is_some() || self.time < 3. || self.briefing_left > 0. {
            return false;
        }
        let slot = if tool == Tool::Smoke { 0 } else { 1 };
        if a.tools[slot] == 0 {
            return false;
        }
        a.tools[slot] -= 1;
        if tool == Tool::Smoke {
            self.smokes.push(Smoke {
                at: a.at,
                left: SMOKE_SECONDS,
            });
        } else {
            a.scan_until = self.time + 5.;
        }
        true
    }
    fn visible(&self, a: (f32, f32), b: (f32, f32), range: f32) -> bool {
        distance(a, b) <= range
            && buildings().iter().all(|&p| segment_distance(p, a, b) > 3.7)
            && self
                .smokes
                .iter()
                .all(|s| segment_distance(s.at, a, b) > 3.3)
    }
    // Eight coarse sectors: useful through smoke without exposing a name,
    // exact position, or the other team's private objective.
    fn scan_signal(&self, actor: &Actor) -> Option<(f32, f32)> {
        if actor.scan_until <= self.time {
            return None;
        }
        let other = self
            .actors
            .iter()
            .filter(|other| other.side != actor.side && !other.jailed)
            .min_by(|a, b| distance(actor.at, a.at).total_cmp(&distance(actor.at, b.at)))?;
        let d = distance(actor.at, other.at);
        if d >= 28. {
            return None;
        }
        let heading = (other.at.0 - actor.at.0).atan2(other.at.1 - actor.at.1);
        let sector = std::f32::consts::FRAC_PI_4;
        Some(((heading / sector).round() * sector, d))
    }
    pub fn view_for(&self, seat: Option<u8>) -> CityView {
        let mine = self.actors.iter().find(|a| Some(a.seat) == seat);
        let crew = mine.is_some_and(|a| a.side == Side::Crew);
        let actors = self
            .actors
            .iter()
            .map(|a| {
                let shown = mine.is_some_and(|me| {
                    (self.demo && me.automated)
                        || me.side == a.side
                        || self.visible(me.at, a.at, 25.)
                });
                CityActorView {
                    seat: a.seat,
                    side: a.side,
                    x: shown.then_some(a.at.0),
                    z: shown.then_some(a.at.1),
                    yaw: if shown { a.yaw } else { 0. },
                    jailed: a.jailed,
                    stunned: a.stun,
                    carrying: shown && self.carrier == Some(a.seat),
                    cuffed: if shown { a.cuff } else { 0. },
                    acting: shown && a.interact,
                    suction: shown && a.suction,
                    automated: a.automated,
                    driving: shown && a.in_car,
                    car: (a.side == Side::Police
                        && mine.is_some_and(|me| {
                            me.side == a.side || self.visible(me.at, a.car_at, 25.)
                        }))
                    .then_some([a.car_at.0, a.car_at.1]),
                    jail_seconds: a.jail_left,
                    height: if shown { a.height } else { 0. },
                    velocity: if shown {
                        [a.velocity.0, a.velocity.1]
                    } else {
                        [0., 0.]
                    },
                }
            })
            .collect();
        let objective = if let Some(side) = self.resolved {
            if side == Side::Crew {
                "Crew escaped with the loot"
            } else {
                "Police secured the city"
            }
        } else if mine.is_some_and(|a| a.jailed) {
            "Teammates can rescue you. Bail releases you in 18 seconds."
        } else if crew {
            match self.job {
                Job::Power => "One teammate holds the power switch; another opens the vault.",
                Job::Vault => "Keep the power off and vacuum the bank vault.",
                Job::Carry => "Get the loot to your secret escape van. Rescue captured teammates.",
            }
        } else if self.alarm {
            "Bank alarm! Catch the carrier or recover the dropped loot."
        } else {
            "Drive the marked streets. F exits your car; hold E near a robber to arrest."
        };
        let mut targets = Vec::new();
        if crew && self.actors.iter().any(|a| a.side == Side::Crew && a.jailed) {
            targets.push(Target {
                label: "JAIL · HOLD E TO RESCUE",
                x: JAIL.0,
                z: JAIL.1,
                progress: self.rescue,
            });
        }
        if crew {
            if self.job != Job::Carry {
                targets.push(Target {
                    label: "POWER SWITCH",
                    x: POWER.0,
                    z: POWER.1,
                    progress: self.switch,
                });
                targets.push(Target {
                    label: "BANK VAULT",
                    x: BANK.0,
                    z: BANK.1,
                    progress: self.vault,
                });
            } else {
                targets.push(Target {
                    label: "ESCAPE VAN",
                    x: ESCAPE.0,
                    z: ESCAPE.1,
                    progress: self.escape_progress,
                });
            }
        }
        if mine.is_some_and(|a| a.side == Side::Police) && self.alarm {
            targets.push(Target {
                label: "BANK ALARM",
                x: BANK.0,
                z: BANK.1,
                progress: 0.,
            });
        }
        if mine.is_some_and(|a| a.side == Side::Police) && !self.alarm {
            let a = mine.unwrap();
            let post = PATROL[a.patrol];
            targets.push(Target {
                label: if a.in_car {
                    "PATROL ROUTE"
                } else {
                    "PATROL CAR"
                },
                x: if a.in_car { post.0 } else { a.car_at.0 },
                z: if a.in_car { post.1 } else { a.car_at.1 },
                progress: 0.,
            });
        }
        let scanner = mine.filter(|a| a.scan_until > self.time).map(|a| {
            let nearest = self
                .actors
                .iter()
                .filter(|other| other.side != a.side && !other.jailed)
                .map(|other| distance(a.at, other.at))
                .fold(f32::INFINITY, f32::min);
            if nearest < 12. {
                "Strong signal · very close"
            } else if nearest < 28. {
                "Radio activity nearby"
            } else {
                "No nearby signal"
            }
        });
        CityView {
            round: self.round,
            jobs: self.config.jobs,
            remaining: (LIMIT - (self.time - 3.).max(0.)).max(0.),
            briefing: self.briefing().then(|| BriefingView {
                remaining: self.briefing_left,
                ready: self
                    .actors
                    .iter()
                    .filter(|a| !a.automated && self.ready_seats.contains(&a.seat))
                    .count(),
                total: self.actors.iter().filter(|a| !a.automated).count(),
                you_ready: mine.is_some_and(|a| a.automated || self.ready_seats.contains(&a.seat)),
            }),
            night_score: self.night_score,
            patrol_route: mine
                .filter(|a| a.in_car)
                .map(|a| a.path.iter().take(32).map(|p| [p.0, p.1]).collect())
                .unwrap_or_default(),
            job_wait: self.job_wait,
            dispatch: self.dispatch,
            draw: self.draw,
            ready: (3. - self.time).max(0.),
            scores: self.scores,
            actors,
            objective,
            targets,
            bank: [BANK.0, BANK.1],
            jail: [JAIL.0, JAIL.1],
            escape: crew.then_some([ESCAPE.0, ESCAPE.1]),
            alarm: self.alarm,
            alert: self.alarm.then_some([BANK.0, BANK.1]),
            loot: self
                .loot
                .filter(|at| crew || mine.is_some_and(|me| self.visible(me.at, *at, 25.)))
                .map(|p| [p.0, p.1]),
            loot_carrier: self.carrier.filter(|seat| {
                crew || self
                    .actors
                    .iter()
                    .find(|a| a.seat == *seat)
                    .is_some_and(|a| mine.is_some_and(|me| self.visible(me.at, a.at, 25.)))
            }),
            vault_progress: crew.then_some(self.vault),
            power_seconds: crew.then_some(self.power),
            smokes: self
                .smokes
                .iter()
                .map(|s| [s.at.0, s.at.1, s.left])
                .collect(),
            tools: mine.map(|a| a.tools).unwrap_or([0, 0]),
            scanner,
            scanner_bearing: mine.and_then(|a| self.scan_signal(a).map(|s| s.0)),
            resolved: self.resolved,
            reason: self.reason,
            resolved_for: self.resolved_for,
        }
    }
    fn resolve(&mut self, side: Side, reason: &'static str) {
        if self.resolved.is_some() {
            return;
        }
        self.resolved = Some(side);
        self.reason = Some(reason);
        self.scores[if side == Side::Police { 0 } else { 1 }] += 1;
    }
    pub fn next_job(&mut self, placements: &[Vec<u8>]) {
        let seats: Vec<_> = self.actors.iter().map(|a| (a.seat, a.automated)).collect();
        let scores = self.scores;
        let next = self.round + 1;
        let mut game = Self::seeded(
            &seats,
            self.config.clone(),
            false,
            self.seed.wrapping_add(next as u64 * 7919),
        );
        game.demo = self.demo;
        game.round = next;
        game.briefing_left = 0.;
        game.scores = scores;
        for a in &mut game.actors {
            if placements
                .first()
                .is_some_and(|winners| winners.contains(&a.seat))
            {
                a.tools = [2, 2];
            }
        }
        *self = game;
    }
    pub fn standings(&self) -> Vec<Vec<u8>> {
        if self.scores[0] == self.scores[1] {
            return vec![self.actors.iter().map(|a| a.seat).collect()];
        }
        let winner = if self.scores[0] > self.scores[1] {
            Side::Police
        } else {
            Side::Crew
        };
        [
            winner,
            if winner == Side::Police {
                Side::Crew
            } else {
                Side::Police
            },
        ]
        .iter()
        .map(|side| {
            self.actors
                .iter()
                .filter(|a| a.side == *side)
                .map(|a| a.seat)
                .collect()
        })
        .collect()
    }
    pub fn automated(&self, seat: u8) -> bool {
        self.actors.iter().any(|a| a.seat == seat && a.automated)
    }
    pub fn active_seats(&self) -> Vec<u8> {
        self.actors.iter().map(|a| a.seat).collect()
    }
    fn ai_goal(&self, index: usize) -> ((f32, f32), bool) {
        let a = &self.actors[index];
        if a.jailed {
            return (JAIL, false);
        }
        if a.side == Side::Police {
            if a.in_car {
                if let Some(r) = self
                    .actors
                    .iter()
                    .filter(|r| {
                        r.side == Side::Crew
                            && !r.jailed
                            && distance(r.at, road_point(r.at)) < 2.4
                            && self.visible(a.at, r.at, 25.)
                    })
                    .min_by(|r, s| distance(a.at, r.at).total_cmp(&distance(a.at, s.at)))
                {
                    return (road_point(r.at), false);
                }
                if self.alarm && self.time - self.alarm_at < 12. {
                    return (road_point(BANK), false);
                }
                return (PATROL[a.patrol], false);
            }
            if let Some((heading, distance)) = self.scan_signal(a) {
                if !self
                    .actors
                    .iter()
                    .any(|r| r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 25.))
                {
                    let reach = distance.min(12.);
                    let coarse = (
                        a.at.0 + heading.sin() * reach,
                        a.at.1 + heading.cos() * reach,
                    );
                    if walkable(coarse) {
                        return (coarse, false);
                    }
                }
            }
            if let Some((at, when, yaw, true)) = a.last_seen {
                if self.time - when < 6.
                    && self.carrier.is_some()
                    && self
                        .actors
                        .iter()
                        .find(|r| Some(r.seat) == self.carrier)
                        .is_none_or(|r| !self.visible(a.at, r.at, 25.))
                {
                    let travel = (self.time - when).min(2.5) * CARRY_SPEED;
                    let predicted = (at.0 + yaw.sin() * travel, at.1 + yaw.cos() * travel);
                    return (if walkable(predicted) { predicted } else { at }, false);
                }
            }
            if let Some(robber) = self
                .actors
                .iter()
                .filter(|r| r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 25.))
                .min_by(|a1, b1| {
                    (self.carrier != Some(a1.seat))
                        .cmp(&(self.carrier != Some(b1.seat)))
                        .then_with(|| distance(a.at, a1.at).total_cmp(&distance(a.at, b1.at)))
                })
            {
                return (robber.at, true);
            }
            if let Some(loot) = self.loot {
                if self.visible(a.at, loot, 25.) {
                    return (loot, true);
                }
            }
            if let Some((at, when, yaw, _)) = a.last_seen {
                let age = self.time - when;
                if age < 8. {
                    // Pursue the direction actually seen, for a short time. Smoke
                    // removes sight, not the officer's memory; a corner can still fool it.
                    let travel = age.min(2.5) * CARRY_SPEED;
                    let predicted = (at.0 + yaw.sin() * travel, at.1 + yaw.cos() * travel);
                    return (if walkable(predicted) { predicted } else { at }, false);
                }
            }
            return (a.car_at, false);
        }
        if self.carrier == Some(a.seat) {
            return (ESCAPE, true);
        }
        if self.actors.iter().any(|r| r.side == Side::Crew && r.jailed) {
            let rescuer = self
                .actors
                .iter()
                .filter(|r| r.side == Side::Crew && !r.jailed && self.carrier != Some(r.seat))
                .min_by(|a, b| distance(a.at, JAIL).total_cmp(&distance(b.at, JAIL)));
            let guarded = self
                .actors
                .iter()
                .any(|p| p.side == Side::Police && distance(p.at, JAIL) < 7.);
            if rescuer.is_some_and(|rescuer| rescuer.seat == a.seat) && (!guarded || a.tools[0] > 0)
            {
                return (JAIL, true);
            }
        }
        if let Some(loot) = self.loot {
            return (loot, true);
        }
        match self.job {
            Job::Power | Job::Vault => {
                let crew: Vec<_> = self
                    .actors
                    .iter()
                    .filter(|r| r.side == Side::Crew && !r.jailed)
                    .map(|r| r.seat)
                    .collect();
                // Once power is safely on, its worker helps finish the vault.
                // Keep enough time to return and refresh power if needed.
                if crew.len() > 1 && crew.first() == Some(&a.seat) && self.power < 6. {
                    (POWER, true)
                } else if self.power > 2. {
                    (BANK, true)
                } else {
                    (POWER, true)
                }
            }
            Job::Carry => {
                let lure = if index % 2 == 0 {
                    (-16., 28.)
                } else {
                    (16., 28.)
                };
                (lure, false)
            }
        }
    }
    pub fn tick(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        mut car_hit: impl FnMut((f32, f32), (f32, f32)) -> bool,
    ) {
        self.tick_with_vehicle_traffic(dt,movement,eligible,|from,to,_|car_hit(from,to));
    }
    /// Vehicles need their full footprint and must also respect stopped cars.
    /// The existing point-actor callback remains available to standalone rules callers.
    pub fn tick_with_vehicle_traffic(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        car_hit: impl FnMut((f32,f32),(f32,f32),Option<f32>)->bool,
    ) {
        self.tick_with_traffic(dt,movement,eligible,car_hit,|_,_|false);
    }
    pub fn tick_with_traffic(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        mut car_hit: impl FnMut((f32,f32),(f32,f32),Option<f32>)->bool,
        mut crossing_wait: impl FnMut((f32,f32),(f32,f32))->bool,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let dt = dt.min(0.05);
        if self.briefing() {
            self.briefing_left = (self.briefing_left - dt).max(0.);
            if self
                .actors
                .iter()
                .filter(|a| !a.automated && eligible.contains(&a.seat))
                .all(|a| self.ready_seats.contains(&a.seat))
            {
                self.briefing_left = 0.;
            }
            for a in &mut self.actors {
                if a.automated {
                    continue;
                }
                if let Some((_, stick, _)) = movement.iter().find(|(seat, _, _)| *seat == a.seat) {
                    let stick = stick.clamped();
                    let speed = if a.in_car { 10. } else { SPEED };
                    let next = (a.at.0 + stick.x * dt * speed, a.at.1 + stick.z * dt * speed);
                    if if a.in_car {
                        on_road(next)
                    } else {
                        walkable(next)
                    } {
                        a.at = next;
                    }
                    if stick.x.hypot(stick.z) > 0.1 {
                        a.yaw = stick.x.atan2(stick.z);
                        a.car_yaw = a.yaw;
                    }
                    if a.in_car {
                        a.car_at = a.at;
                    }
                }
            }
            return;
        }
        self.time += dt;
        if self.resolved.is_some() {
            self.resolved_for += dt;
            return;
        }
        if self.time < 3. {
            return;
        }
        for s in &mut self.smokes {
            s.left -= dt;
        }
        self.smokes.retain(|s| s.left > 0.);
        let sightings: Vec<_> = self
            .actors
            .iter()
            .map(|a| {
                if a.side == Side::Police {
                    self.actors
                        .iter()
                        .filter(|r| {
                            r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 25.)
                        })
                        .min_by(|a1, b1| {
                            (self.carrier != Some(a1.seat))
                                .cmp(&(self.carrier != Some(b1.seat)))
                                .then_with(|| {
                                    distance(a.at, a1.at).total_cmp(&distance(a.at, b1.at))
                                })
                        })
                        .map(|r| (r.at, self.time, r.yaw, self.carrier == Some(r.seat)))
                } else {
                    None
                }
            })
            .collect();
        for (a, sighting) in self.actors.iter_mut().zip(sightings) {
            if let Some(next) = sighting {
                let keep = a.last_seen.is_some_and(|(_, when, _, carrying)| {
                    carrying && !next.3 && self.time - when < 6.
                });
                if !keep {
                    a.last_seen = Some(next);
                }
            }
        }
        let scan_users: Vec<_> = self
            .actors
            .iter()
            .filter(|a| {
                a.automated
                    && a.side == Side::Police
                    && a.tools[1] > 0
                    && a.last_seen.is_some_and(|(_, when, _, _)| {
                        self.time - when > 0.35 && self.time - when < 5.
                    })
            })
            .map(|a| a.seat)
            .collect();
        for seat in scan_users {
            self.use_tool(seat, Tool::Scanner);
        }
        let vehicle_changes: Vec<_> = self
            .actors
            .iter()
            .filter(|a| a.automated && a.side == Side::Police)
            .filter(|a| {
                if a.in_car {
                    self.actors.iter().any(|r| {
                        r.side == Side::Crew
                            && !r.jailed
                            && self.visible(a.at, r.at, 14.)
                            && (distance(r.at, road_point(r.at)) > 2.5
                                || (a.drive_speed < 2. && distance(a.at, r.at) < 5.))
                    }) || (self.alarm
                        && self.time - self.alarm_at < 12.
                        && distance(a.at, road_point(BANK)) < 3.)
                } else {
                    distance(a.at, a.car_at) < 3.8
                        && a.last_seen
                            .is_none_or(|(_, when, _, _)| self.time - when > 8.)
                        && !self.actors.iter().any(|r| {
                            r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 25.)
                        })
                }
            })
            .map(|a| a.seat)
            .collect();
        for seat in vehicle_changes {
            self.toggle_car(seat);
        }
        let goals: Vec<_> = (0..self.actors.len()).map(|i| self.ai_goal(i)).collect();
        let crew_count = self.actors.iter().filter(|a| a.side == Side::Crew).count();
        let police_count = self.actors.len() - crew_count;
        // More crew can rotate carriers and rescues. Give outnumbered officers
        // a modest pursuit edge; stealth and smoke still break their sight.
        let police_speed =
            6.0 + ((crew_count as f32 / police_count.max(1) as f32) - 2.).clamp(0., 2.) * 0.25;
        let carry_speed = CARRY_SPEED;
        let mut vehicle_sweeps = Vec::new();
        for (index, a) in self.actors.iter_mut().enumerate() {
            a.velocity = (0., 0.);
            a.stun = (a.stun - dt).max(0.);
            a.immune = (a.immune - dt).max(0.);
            if a.jailed {
                a.jail_left = (a.jail_left - dt).max(0.);
                if a.jail_left <= 0. {
                    a.jailed = false;
                    a.cuff = 0.;
                    a.at = (JAIL.0 - 4., JAIL.1);
                    a.immune = 6.;
                    a.path.clear();
                }
                continue;
            }
            if a.in_car && distance(a.at, PATROL[a.patrol]) < 3. {
                a.patrol = (a.patrol + 1) % PATROL.len();
            }
            if a.height > 0. || a.up > 0. {
                a.up -= 14. * dt;
                a.height = (a.height + a.up * dt).max(0.);
                let next = (a.at.0 + a.knock.0 * dt, a.at.1 + a.knock.1 * dt);
                if walkable(next) {
                    a.at = next;
                    a.velocity = a.knock;
                } else {
                    a.knock = (0., 0.);
                }
                let drag = (-2. * dt).exp();
                a.knock.0 *= drag;
                a.knock.1 *= drag;
                if a.height <= 0. {
                    a.up = 0.;
                    a.knock = (0., 0.);
                }
                continue;
            }
            if a.stun > 0. {
                continue;
            }
            let mut stick = Stick::default();
            if a.automated || !eligible.contains(&a.seat) {
                let (goal, interact) = goals[index];
                a.interact = !a.in_car && interact && distance(a.at, goal) < 2.;
                a.suction = a.interact && a.side == Side::Crew && distance(a.at, BANK) < 2.5;
                if distance(a.at, goal) > 1.4 {
                    if a.path.is_empty() || distance(a.goal, goal) > 3. && self.time > a.repath {
                        a.path = route(a.at, goal, a.in_car);
                        a.goal = goal;
                        a.repath = self.time + 0.7;
                    }
                    while a
                        .path
                        .front()
                        .is_some_and(|point| distance(a.at, *point) < 0.45)
                    {
                        a.path.pop_front();
                    }
                    if let Some(&target) = a.path.front() {
                        let d = distance(a.at, target).max(0.001);
                        stick = Stick {
                            x: (target.0 - a.at.0) / d,
                            z: (target.1 - a.at.1) / d,
                        };
                    }
                }
            } else {
                if let Some((_, s, yaw)) = movement.iter().find(|(seat, _, _)| *seat == a.seat) {
                    stick = s.clamped();
                    if yaw.is_finite() && !a.in_car {
                        a.yaw = *yaw;
                    }
                }
                if a.in_car && self.time > a.repath {
                    a.path = route(a.at, PATROL[a.patrol], true);
                    a.repath = self.time + 1.2;
                }
                if a.until < self.time {
                    a.interact = false;
                    a.suction = false;
                }
            }
            if a.in_car {
                if stick.x.hypot(stick.z) > 0.1 {
                    let desired = stick.x.atan2(stick.z);
                    let turn = (desired - a.car_yaw)
                        .sin()
                        .atan2((desired - a.car_yaw).cos());
                    a.car_yaw += turn * (1. - (-14. * dt).exp());
                }
                a.yaw = a.car_yaw;
            } else if stick.x.hypot(stick.z) > 0.1 {
                a.yaw = stick.x.atan2(stick.z);
            }
            let speed = if a.in_car {
                let desired = if stick.x.hypot(stick.z) > 0.1 {
                    14. * stick.x.hypot(stick.z).min(1.)
                } else {
                    0.
                };
                a.drive_speed += (desired - a.drive_speed).clamp(-22. * dt, 12. * dt);
                a.drive_speed
            } else if self.carrier == Some(a.seat) {
                carry_speed
            } else if a.side == Side::Police {
                police_speed
            } else {
                SPEED
            };
            let old = a.at;
            let direction = if a.in_car {
                (a.yaw.sin(), a.yaw.cos())
            } else {
                (stick.x, stick.z)
            };
            let next = (
                old.0 + direction.0 * speed * dt,
                old.1 + direction.1 * speed * dt,
            );
            // AI pedestrians wait outside a red crossing instead of repeatedly
            // running a carried bag into traffic. Human players choose the risk.
            if a.automated && !a.in_car && crossing_wait(old,next) {
                continue;
            }
            if a.in_car && car_hit(old,next,Some(a.yaw)) {
                a.drive_speed=0.;
                a.velocity=(0.,0.);
                continue;
            }
            if if a.in_car {
                on_road(next)
            } else {
                walkable(next)
            } {
                a.at = next;
            } else {
                if if a.in_car {
                    on_road((next.0, old.1))
                } else {
                    walkable((next.0, old.1))
                } {
                    a.at.0 = next.0;
                }
                if if a.in_car {
                    on_road((a.at.0, next.1))
                } else {
                    walkable((a.at.0, next.1))
                } {
                    a.at.1 = next.1;
                }
            }
            a.velocity = ((a.at.0 - old.0) / dt, (a.at.1 - old.1) / dt);
            if a.in_car {
                a.car_at = a.at;
                if a.drive_speed > 5. {
                    vehicle_sweeps.push((a.seat, old, a.at, a.yaw));
                }
            }
            if !a.in_car && a.immune <= 0. && car_hit(old, a.at,None) {
                if self.carrier == Some(a.seat) {
                    self.carrier = None;
                    self.loot = Some(old);
                }
                // A traffic bump costs the bag and a short stumble, not an
                // unexplained teleport across the city.
                a.at = old;
                a.stun = 0.8;
                a.immune = 4.;
                a.path.clear();
            }
        }
        // Swept collisions cannot tunnel through a robber between snapshots.
        // A ram never arrests: the officer must stop, get out, and hold cuffs.
        for (cop, from, to, yaw) in vehicle_sweeps {
            let victim = self
                .actors
                .iter()
                .find(|a| {
                    a.side == Side::Crew
                        && !a.jailed
                        && a.immune <= 0.
                        && a.height <= 0.
                        && segment_distance(a.at, from, to) < 1.55
                })
                .map(|a| a.seat);
            if let Some(victim) = victim {
                let a = self.actors.iter_mut().find(|a| a.seat == victim).unwrap();
                if self.carrier == Some(victim) {
                    self.carrier = None;
                    self.loot = Some(a.at);
                }
                a.stun = 1.;
                a.immune = 1.2;
                a.height = 0.05;
                a.up = 5.6;
                a.knock = (yaw.sin() * 14., yaw.cos() * 14.);
                a.velocity = a.knock;
                a.interact = false;
                a.suction = false;
                a.cuff = 0.;
                a.path.clear();
                self.toggle_car(cop);
            }
        }
        if self.time >= LIMIT + 3. {
            if self.night_score[0] == self.night_score[1] {
                self.draw = true;
                self.resolved = Some(Side::Police);
                self.reason = Some("Night tied · minigame next");
            } else {
                self.resolve(
                    if self.night_score[0] > self.night_score[1] {
                        Side::Police
                    } else {
                        Side::Crew
                    },
                    "Six-minute night complete · minigame next",
                );
            }
            return;
        }
        if self.job_wait > 0. {
            self.job_wait = (self.job_wait - dt).max(0.);
            if self.job_wait <= 0. {
                self.reset_job();
            }
            return;
        }
        let smoke_users: Vec<_> = self
            .actors
            .iter()
            .filter(|a| {
                a.automated
                    && a.side == Side::Crew
                    && !a.jailed
                    && a.stun <= 0.
                    && a.tools[0] > 0
                    && (self.carrier == Some(a.seat) || a.cuff > 0.25 || distance(a.at, JAIL) < 7.)
                    && self
                        .actors
                        .iter()
                        .any(|p| p.side == Side::Police && self.visible(a.at, p.at, 9.))
            })
            .map(|a| a.seat)
            .collect();
        for seat in smoke_users {
            self.use_tool(seat, Tool::Smoke);
        }
        let power_workers = self
            .actors
            .iter()
            .filter(|a| {
                a.side == Side::Crew
                    && !a.jailed
                    && a.stun <= 0.
                    && a.interact
                    && distance(a.at, POWER) < 2.3
            })
            .count();
        if power_workers > 0 {
            self.switch = (self.switch + dt / 2.4).min(1.);
            if self.switch >= 1. {
                self.power = 20.;
                self.job = if self.job == Job::Power {
                    Job::Vault
                } else {
                    self.job
                };
            }
        } else {
            self.switch = (self.switch - dt * 0.2).max(0.);
            self.power = (self.power - dt).max(0.);
        }
        if self.job == Job::Vault && self.power > 0. {
            if let Some(worker) = self.actors.iter().find(|a| {
                a.side == Side::Crew
                    && !a.jailed
                    && a.stun <= 0.
                    && (a.interact || a.suction)
                    && distance(a.at, BANK) < if a.suction { 3.5 } else { 2.3 }
            }) {
                self.vault = (self.vault + dt / 12.).min(1.);
                if !self.alarm {
                    self.alarm_at = self.time;
                }
                self.alarm = true;
                if self.vault >= 1. {
                    self.job = Job::Carry;
                    self.carrier = Some(worker.seat);
                }
            }
        }
        // A held cuff action must remain close and visible; packets cannot claim a target.
        let arrests: HashSet<u8> = self
            .actors
            .iter()
            .filter(|p| {
                p.side == Side::Police && !p.in_car && !p.jailed && p.stun <= 0. && p.interact
            })
            .filter_map(|p| {
                self.actors
                    .iter()
                    .filter(|r| {
                        r.side == Side::Crew
                            && !r.jailed
                            && r.immune <= 0.
                            && self.visible(p.at, r.at, 2.25)
                    })
                    .min_by(|a, b| {
                        (self.carrier != Some(a.seat))
                            .cmp(&(self.carrier != Some(b.seat)))
                            .then_with(|| distance(p.at, a.at).total_cmp(&distance(p.at, b.at)))
                            .then(a.seat.cmp(&b.seat))
                    })
                    .map(|r| r.seat)
            })
            .collect();
        for a in &mut self.actors {
            if a.side != Side::Crew || a.jailed {
                continue;
            }
            a.cuff = if arrests.contains(&a.seat) {
                (a.cuff + dt / 1.5).min(1.)
            } else {
                (a.cuff - dt).max(0.)
            };
            if a.cuff >= 1. {
                if self.carrier == Some(a.seat) {
                    self.carrier = None;
                    self.loot = Some(a.at);
                }
                a.jailed = true;
                a.jail_left = JAIL_SECONDS;
                a.at = JAIL;
                a.interact = false;
                a.suction = false;
                a.path.clear();
            }
        }
        let rescue = self.actors.iter().any(|a| {
            a.side == Side::Crew
                && !a.jailed
                && a.stun <= 0.
                && a.interact
                && distance(a.at, JAIL) < 2.5
        });
        self.rescue = if rescue { self.rescue + dt / 3. } else { 0. };
        if self.rescue >= 1. {
            for a in &mut self.actors {
                if a.jailed {
                    a.jailed = false;
                    a.jail_left = 0.;
                    a.cuff = 0.;
                    a.at = (JAIL.0 - 3.5, JAIL.1 + 2.);
                    a.immune = 4.;
                    a.path.clear();
                }
            }
            self.rescue = 0.;
        }
        if let Some(at) = self.loot {
            if let Some(a) = self.actors.iter().find(|a| {
                a.side == Side::Crew
                    && !a.jailed
                    && a.stun <= 0.
                    && (a.interact || a.suction)
                    && distance(a.at, at) < 2.3
            }) {
                self.carrier = Some(a.seat);
                self.loot = None;
                self.recover = 0.;
            } else {
                let police = self.actors.iter().any(|a| {
                    a.side == Side::Police
                        && !a.in_car
                        && a.stun <= 0.
                        && a.interact
                        && distance(a.at, at) < 2.3
                });
                self.recover = if police { self.recover + dt / 3. } else { 0. };
                if self.recover >= 1. {
                    self.finish_job(Side::Police, "Police recovered a bag · next job in 8s");
                }
            }
        }
        let escaping = self
            .carrier
            .and_then(|seat| self.actors.iter().find(|a| a.seat == seat))
            .is_some_and(|a| !a.jailed && a.interact && distance(a.at, ESCAPE) < 3.);
        self.escape_progress = if escaping {
            self.escape_progress + dt / 3.
        } else {
            0.
        };
        if self.escape_progress >= 1. {
            self.finish_job(Side::Crew, "Crew delivered a bag · next job in 8s");
        }
        if self
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .all(|a| a.jailed)
        {
            self.finish_job(Side::Police, "Crew caught · regrouping in 8s");
        }
    }
}

#[cfg(test)]
#[path = "city_heist_tests.rs"]
mod tests;
