//! Separate, simultaneous city game. The board only supplies scenery/traffic;
//! heists, arrests, privacy, bots and scores have their own authoritative state.
use crate::bout::Stick;
use crate::city_arrest::{self as arrest, Arrest, ArrestView, Stage, Struggle};
use crate::city_buildings::{self, reach};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// The vault, inside the bank's back room.
pub const BANK: (f32, f32) = (8., 30.);
pub const POWER: (f32, f32) = (-9., 32.);
/// The escape van, parked inside its garage.
pub const ESCAPE: (f32, f32) = (-28.5, -30.);
/// The jail's front room, before the cell bars.
pub const JAIL: (f32, f32) = (28., -29.);
/// Inside the cell.
pub const JAIL_CELL: (f32, f32) = (28., -33.5);
/// The security desk in the bank's front room: big crews' second post.
pub const CAMERAS: (f32, f32) = (-1.8, 33.2);
/// On the street outside the jail door.
pub const JAIL_EXIT: (f32, f32) = (28., -22.);
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
const BRIEFING_SECONDS: f32 = 20.;
const TOP_SPEED: f32 = 15.;
const SHOVE_SPEED: f32 = 6.;
/// How far ahead a stuck car feels for what it is pressing against, in m/s.
const NUDGE_PROBE: f32 = 2.5;
/// How long a car pressed to drive can barely move before it may be reset.
pub const STUCK_SECONDS: f32 = 3.;
/// Below this a pressed car counts as not moving, in m/s.
const STUCK_PACE: f32 = 0.6;
/// A stuck car that gets this far from where it stuck is free again.
const STUCK_CLEAR: f32 = 1.5;
/// A patrol car runs its lights, and traffic yields, above this speed.
const LIGHTS_SPEED: f32 = 1.5;
/// Off the road a car slides on snow: its heading follows the wheel this
/// fast (10 on the road), it gathers and sheds speed more slowly, and it
/// tops out lower.
pub const SNOW_GRIP: f32 = 1.6;
const ROAD_GRIP: f32 = 10.;
const SNOW_ACCEL: f32 = 5.;
const SNOW_BRAKE: f32 = 7.;
const SNOW_TOP_SPEED: f32 = 11.;
const RAM_STUN: f32 = 1.8;
const IMPACT_SECONDS: f32 = 1.5;
/// How a sandbox dummy behaves: a full bot, or standing still, running
/// from the other side, or taking it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    Bot,
    Still,
    Flee,
    Fight,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Landmark {
    Bank,
    Jail,
    Power,
    Escape,
    Car,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StationKind {
    Power,
    Vault,
    Van,
    Jail,
}
/// The city sandbox's tools; the room runs these only inside a sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum CityTool {
    Role { police: bool },
    Spawn { police: bool, behavior: Behavior },
    Despawn,
    Behave { behavior: Behavior },
    Alarm { on: bool },
    Traffic { level: u8 },
    Teleport { place: Landmark },
    ResetCar,
    Arrest,
    Station { station: StationKind },
    Give { item: ItemKind },
    Flat,
    Skill { level: BotSkill },
}
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
    /// Bags the vault spills; None scales with the crew.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bags: Option<usize>,
    /// Every bot's skill from 0 to 1; None plays at 0.5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<f32>,
}
impl Default for CityConfig {
    fn default() -> Self {
        Self {
            police: 1,
            jobs: 3,
            host_police: false,
            bags: None,
            skill: None,
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
    pub held: bool,
    /// Driving on snow, and how far the car's nose is turned from where it
    /// is actually going, in radians.
    pub snow: bool,
    pub drift: f32,
    /// The heist building room they stand in.
    pub room: Option<&'static str>,
    pub flat: bool,
    pub rolling: bool,
    pub lunging: bool,
    pub diving: bool,
    pub dive_phase: f32,
    pub spinning: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Station {
    Power,
    Cameras,
    Vault,
    Bag,
    Jail,
    Van,
    Loot,
    Repair,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Impact {
    pub id: u32,
    pub kind: &'static str,
    pub x: f32,
    pub z: f32,
    pub age: f32,
    /// Who took the hit, and how hard (m/s), so the client can pick a
    /// small or a big effect. Effects never change the hit itself.
    pub victim: Option<u8>,
    pub force: f32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Target {
    pub id: u32,
    pub kind: TargetKind,
    pub primary: bool,
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
    /// Audible moving patrol cars, including ones outside visual range.
    pub audible_cars: Vec<[f32; 4]>,
    pub objective: &'static str,
    /// What I should be doing now, and what my buttons do here.
    pub step: Option<Step>,
    pub context: Option<ContextAction>,
    /// The crew's job, for the crew only.
    pub job: Option<&'static str>,
    pub stamps: Vec<Stamp>,
    pub targets: Vec<Target>,
    pub bank: [f32; 2],
    pub jail: [f32; 2],
    pub escape: Option<[f32; 2]>,
    pub alarm: bool,
    pub alert: Option<[f32; 2]>,
    pub loot: Option<[f32; 2]>,
    pub bags: Vec<BagView>,
    pub pads: Vec<PadView>,
    pub hazards: Vec<HazardView>,
    pub item: Option<ItemKind>,
    /// The motor-pool repair bay, for police.
    pub bay: Option<[f32; 2]>,
    /// Seconds until my dodge or takedown is ready again.
    pub dodge_ready: f32,
    pub takedown_ready: f32,
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
    pub arrests: Vec<ArrestView>,
    pub slow: f32,
    pub slow_left: f32,
    /// The visible officer and suspect in the active hands-up callout.
    pub callout: Option<[u8; 2]>,
    pub clarity_ready: f32,
    pub clarity_owner: Option<u8>,
    pub cuff_ready: f32,
    /// My car has been stuck long enough to offer a reset onto a clear lane.
    pub car_stuck: bool,
    /// Why my last arrest ended, while it is fresh.
    pub notice: Option<&'static str>,
    pub grab: f32,
    pub impacts: Vec<Impact>,
    pub bags_left: Option<usize>,
    pub check: Option<CheckView>,
    pub second_post: bool,
    pub cameras_seconds: Option<f32>,
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
    heading: f32,
    grab: Option<(u8, f32)>,
    cuff_cooldown: f32,
    notice: Option<(&'static str, f32)>,
    ram_guard: f32,
    /// How long the car has barely moved while its driver presses to go.
    stuck_for: f32,
    /// Where it last pressed without moving.
    stuck_at: (f32, f32),
    /// A sandbox dummy's behaviour; None for players and ordinary bots.
    dummy: Option<Behavior>,
    /// Added from the sandbox tools, as a dummy or a bot.
    spawned: bool,
    /// The skill check this actor is working, and a pause after a failed one.
    check: Option<checks::Check>,
    check_wait: f32,
    /// A bot's skill from 0 to 1; players are 0.5 and never use it.
    pub(super) skill: f32,
    was_interact: bool,
    mind: ai::Mind,
    item: Option<ItemKind>,
    /// How many hazards a bot steered around when it last planned a route.
    seen_hazards: usize,
    /// Seconds left posing as the carrier with a decoy bag.
    fake_bag: f32,
    /// Just used an item; no picking up another underfoot at once.
    item_guard: f32,
    roll: f32,
    roll_yaw: f32,
    roll_speed: f32,
    roll_cooldown: f32,
    lunge: f32,
    lunge_yaw: f32,
    lunge_hit: bool,
    lunge_target: Option<u8>,
    lunge_baton: bool,
    lunge_cooldown: f32,
    dive_view: f32,
    ground_escape_used: bool,
    flop_pending: bool,
    /// Seconds a flat tyre has left, and repair progress by hand.
    tire: f32,
    repair: f32,
    /// A car spinning out on a banana.
    spin: f32,
    /// When the last stun ended, for halving stacked stuns.
    stun_ended: f32,
}
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Officer(Stage),
    Robber,
}
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Copy)]
struct Callout {
    officer: u8,
    suspect: u8,
}
#[derive(Clone)]
pub struct CityGame {
    pub config: CityConfig,
    /// A free-roam sandbox: no night clock and no winner.
    pub sandbox: bool,
    pub round: u8,
    pub scores: [u8; 2],
    pub time: f32,
    seed: u64,
    actors: Vec<Actor>,
    job: Job,
    power: f32,
    switch: f32,
    vault: f32,
    bags: Vec<bags::Bag>,
    /// Seconds the security desk (big crews' second post) stays down.
    cameras: f32,
    pub tuning: formats::Tuning,
    check_seq: u32,
    /// Coarse spots police heard a fumble, until when.
    noises: Vec<((f32, f32), f32)>,
    /// Passed and failed attempts at each check, for balance reports.
    pub check_passes: [u32; 8],
    pub check_fails: [u32; 8],
    /// Cuff-slip attempts and escapes, for the balance harness.
    pub slips: [u32; 2],
    pub jobs: Vec<stats::JobRecord>,
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
    arrests: Vec<Arrest>,
    slow_left: f32,
    slow_cooldown: f32,
    slow_owner: Option<u8>,
    slow_held: bool,
    callout: Option<Callout>,
    impacts: Vec<Impact>,
    impact_seq: u32,
    stamps: Vec<Stamp>,
    stamp_seq: u32,
    items: items::Items,
}
/// What the city needs from the streets around it: traffic collisions,
/// crossing signals and, for patrol cars, shoving stopped cars aside.
pub trait Streets {
    fn blocked(&mut self, from: (f32, f32), to: (f32, f32), yaw: Option<f32>) -> bool;
    fn crossing_wait(&mut self, from: (f32, f32), to: (f32, f32)) -> bool;
    fn shove(&mut self, _from: (f32, f32), _to: (f32, f32), _yaw: f32, _speed: f32) -> u8 {
        0
    }
    /// Push stuck-against traffic aside at walking pace.
    fn nudge(&mut self, _from: (f32, f32), _to: (f32, f32), _yaw: f32, _dt: f32) -> u8 {
        0
    }
    /// Whether a patrol car could sit at `at` facing `yaw` without touching traffic.
    fn clear_for_car(&mut self, _at: (f32, f32), _yaw: f32) -> bool {
        true
    }
}
struct Callbacks<A, B>(A, B);
impl<A, B> Streets for Callbacks<A, B>
where
    A: FnMut((f32, f32), (f32, f32), Option<f32>) -> bool,
    B: FnMut((f32, f32), (f32, f32)) -> bool,
{
    fn blocked(&mut self, from: (f32, f32), to: (f32, f32), yaw: Option<f32>) -> bool {
        (self.0)(from, to, yaw)
    }
    fn crossing_wait(&mut self, from: (f32, f32), to: (f32, f32)) -> bool {
        (self.1)(from, to)
    }
}
impl Streets for crate::board::Board {
    fn blocked(&mut self, from: (f32, f32), to: (f32, f32), yaw: Option<f32>) -> bool {
        match yaw {
            Some(yaw) => self.traffic_blocks_patrol_vehicle(from, to, yaw),
            None => self.traffic_collision(from, to).is_some(),
        }
    }
    fn crossing_wait(&mut self, from: (f32, f32), to: (f32, f32)) -> bool {
        self.traffic_wait(from, to) > 0.
    }
    fn shove(&mut self, from: (f32, f32), to: (f32, f32), yaw: f32, speed: f32) -> u8 {
        self.shove_traffic(from, to, yaw, speed)
    }
    fn nudge(&mut self, from: (f32, f32), to: (f32, f32), yaw: f32, dt: f32) -> u8 {
        self.nudge_traffic(from, to, yaw, dt)
    }
    fn clear_for_car(&mut self, at: (f32, f32), yaw: f32) -> bool {
        !self.traffic_blocks_patrol_vehicle(at, at, yaw)
    }
}
fn wrap(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}
/// Steering within a third of a radian of a street snaps to it, so a turned
/// camera does not leave a held key driving diagonally into the curb.
fn lane_assist(desired: f32) -> f32 {
    let quarter = std::f32::consts::FRAC_PI_2;
    let axis = (desired / quarter).round() * quarter;
    if wrap(desired - axis).abs() < 0.35 {
        axis
    } else {
        desired
    }
}
/// Why an arrest ended without a jailing; each side is told in a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    CuffTimeout,
    EscortTimeout,
    BrokeFree,
    OfficerDown,
    OfficerGone,
    JobOver,
}
impl Release {
    /// What the officer and the robber are told.
    pub fn messages(self) -> (&'static str, &'static str) {
        match self {
            Release::CuffTimeout => (
                "Too slow on the cuffs. They slipped away.",
                "You slipped the cuffs. Run!",
            ),
            Release::EscortTimeout => (
                "You held them too long. They got away.",
                "They could not hold you. Run!",
            ),
            Release::BrokeFree => ("They broke free!", "You broke free! Run!"),
            Release::OfficerDown => (
                "You were knocked down. They got away.",
                "Your officer went down. Run!",
            ),
            Release::OfficerGone => ("Arrest dropped.", "The officer let go. Run!"),
            Release::JobOver => (
                "The job is over. Arrest called off.",
                "The job is over. You are free.",
            ),
        }
    }
}
/// How long an arrest message stays on screen.
pub const NOTICE_SECONDS: f32 = 3.5;
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn in_bounds(at: (f32, f32)) -> bool {
    at.0.is_finite() && at.1.is_finite() && at.0.abs() < 68. && at.1.abs() < 68.
}
fn walkable(at: (f32, f32)) -> bool {
    in_bounds(at) && city_buildings::open_for_body(at)
}
fn drivable(at: (f32, f32)) -> bool {
    in_bounds(at) && city_buildings::open_for_car(at)
}
fn passable(at: (f32, f32), in_car: bool) -> bool {
    if in_car { drivable(at) } else { walkable(at) }
}
/// One step of a held robber toward the spot behind their officer. Through a
/// door they follow the officer's own line rather than cutting the wall.
fn led_step(old: (f32, f32), cop: (f32, f32), behind: (f32, f32), dt: f32) -> (f32, f32) {
    let follow = |target: (f32, f32), keep: f32| {
        let gap = distance(old, target);
        let pull = (gap * (1. - (-12. * dt).exp())).min(arrest::ESCORT_SPEED * 2.5 * dt);
        let pull = pull.min((gap - keep).max(0.));
        let step = (
            old.0 + (target.0 - old.0) / gap.max(0.001) * pull,
            old.1 + (target.1 - old.1) / gap.max(0.001) * pull,
        );
        for next in [step, (step.0, old.1), (old.0, step.1)] {
            if walkable(next) && !city_buildings::body_blocked(old, next) {
                return next;
            }
        }
        old
    };
    if walkable(behind) && city_buildings::clear_walk(old, behind) {
        follow(behind, 0.)
    } else {
        follow(cop, 0.7)
    }
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
/// Lane spots away from junctions, nearest first, each facing along its
/// street in the direction closest to `facing`. Lanes sit either side of
/// the centre lines the traffic loops run on.
fn clear_lanes(from: (f32, f32), facing: f32) -> Vec<((f32, f32), f32)> {
    const STREETS: [f32; 6] = [-64., -40., -16., 16., 40., 64.];
    let off_junction = |t: f32| STREETS.iter().all(|s| (t - s).abs() > 5.);
    let along = |yaw: f32, back: f32| {
        if wrap(facing - yaw).abs() <= wrap(facing - back).abs() { yaw } else { back }
    };
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut spots = Vec::new();
    for street in STREETS {
        for lane in [-1.05, 1.05] {
            for step in -33..=33 {
                let t = step as f32 * 2.;
                if !off_junction(t) {
                    continue;
                }
                spots.push(((street + lane, t), along(0., std::f32::consts::PI)));
                spots.push(((t, street + lane), along(quarter, -quarter)));
            }
        }
    }
    spots.retain(|(at, yaw)| {
        let ahead = (at.0 + yaw.sin() * 2., at.1 + yaw.cos() * 2.);
        let behind = (at.0 - yaw.sin() * 2., at.1 - yaw.cos() * 2.);
        distance(*at, from) > 3. && on_road(*at) && on_road(ahead) && on_road(behind)
    });
    spots.sort_by(|a, b| distance(a.0, from).total_cmp(&distance(b.0, from)));
    spots
}
/// Open ground off the streets, where a car slides.
fn on_snow(at: (f32, f32)) -> bool {
    drivable(at) && !on_road(at)
}
fn on_road(at: (f32, f32)) -> bool {
    distance(at, road_point(at)) < 2.2 && drivable(at)
}
fn route(from: (f32, f32), goal: (f32, f32), driving: bool) -> VecDeque<(f32, f32)> {
    route_avoiding(from, goal, driving, &[])
}
/// A route that also keeps clear of the given spots, each with its radius:
/// the hazards this actor can see.
fn route_avoiding(
    from: (f32, f32),
    goal: (f32, f32),
    driving: bool,
    avoid: &[((f32, f32), f32)],
) -> VecDeque<(f32, f32)> {
    let hazard = |p: (f32, f32)| avoid.iter().any(|(at, r)| distance(p, *at) < *r);
    // One-metre cells: doors leave a single cell open through each wall.
    const N: usize = 135;
    const HALF: f32 = 67.;
    let cell = |at: (f32, f32)| {
        let x = (at.0 + HALF).round().clamp(0., HALF * 2.) as usize;
        let z = (at.1 + HALF).round().clamp(0., HALF * 2.) as usize;
        z * N + x
    };
    let point = |id: usize| ((id % N) as f32 - HALF, (id / N) as f32 - HALF);
    let start = cell(from);
    let mut end = cell(goal);
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
    if !passable[end] {
        let (gx, gz) = ((end % N) as i32, (end / N) as i32);
        if let Some(near) = (-2..=2)
            .flat_map(|dz| (-2..=2).map(move |dx| (gx + dx, gz + dz)))
            .filter(|&(x, z)| (0..N as i32).contains(&x) && (0..N as i32).contains(&z))
            .map(|(x, z)| z as usize * N + x as usize)
            .filter(|&id| passable[id])
            .min_by(|&a, &b| distance(point(a), goal).total_cmp(&distance(point(b), goal)))
        {
            end = near;
        }
    }
    let mut prior = vec![usize::MAX; N * N];
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
            if prior[next] != usize::MAX || !passable[next] || next != end && hazard(point(next)) {
                continue;
            }
            prior[next] = id;
            queue.push_back(next);
        }
    }
    if prior[end] == usize::MAX {
        return VecDeque::new();
    }
    let mut cells = Vec::new();
    let mut id = end;
    while id != start {
        cells.push(point(id));
        id = prior[id];
    }
    cells.reverse();
    if passable[cell(goal)] {
        cells.push(goal);
    }
    if driving {
        return cells.into();
    }
    // Pull the staircase of cells into straight runs wherever nothing stands between.
    let mut path = VecDeque::new();
    let mut here = from;
    let mut index = 0;
    while index < cells.len() {
        let mut far = index;
        while far + 1 < cells.len()
            && city_buildings::clear_walk(here, cells[far + 1])
            && avoid
                .iter()
                .all(|(at, r)| segment_distance(*at, here, cells[far + 1]) >= *r)
        {
            far += 1;
        }
        path.push_back(cells[far]);
        here = cells[far];
        index = far + 1;
    }
    path
}
impl CityGame {
    pub fn new(seats: &[(u8, bool)], config: CityConfig, watch: bool) -> Self {
        Self::seeded(seats, config, watch, 0)
    }
    pub fn seeded(seats: &[(u8, bool)], config: CityConfig, watch: bool, seed: u64) -> Self {
        let selected_skill = config.skill.unwrap_or(0.5);
        // Four robbers get far more check and cuff-slip chances than two officers.
        // Keep each preset stronger than the previous one without amplifying that edge.
        let crew_skill = if config.police == 2 && seats.len() == 6 && config.skill.is_some() {
            if selected_skill < 0.4 {
                0.4 + (selected_skill - 0.4) * 0.25
            } else if selected_skill <= 0.7 {
                0.4 + (selected_skill - 0.4) * 0.5
            } else {
                0.55 + (selected_skill - 0.7) * 0.1
            }
        } else {
            selected_skill
        };
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
                    heading: 0.,
                    grab: None,
                    cuff_cooldown: 0.,
                    notice: None,
                    ram_guard: 0.,
                    stuck_for: 0.,
                    stuck_at: (0., 0.),
                    dummy: None,
                    spawned: false,
                    check: None,
                    check_wait: 0.,
                    skill: if side == Side::Crew {
                        crew_skill
                    } else {
                        selected_skill
                    },
                    was_interact: false,
                    mind: ai::Mind::default(),
                    item: None,
                    seen_hazards: 0,
                    fake_bag: 0.,
                    item_guard: 0.,
                    roll: 0.,
                    roll_yaw: 0.,
                    roll_speed: 0.,
                    roll_cooldown: 0.,
                    lunge: 0.,
                    lunge_yaw: 0.,
                    lunge_hit: false,
                    lunge_target: None,
                    lunge_baton: false,
                    lunge_cooldown: 0.,
                    dive_view: 0.,
                    ground_escape_used: false,
                    flop_pending: false,
                    tire: 0.,
                    repair: 0.,
                    spin: 0.,
                    stun_ended: 0.,
                }
            })
            .collect();
        let mut tuning = formats::Tuning::for_game(seats.len(), config.police);
        if config.police == 2 && seats.len() == 6 {
            if let Some(skill) = config.skill {
                // At high bot skill, four robbers rescue each other fast enough
                // that arrests seldom become a full police wipe.
                tuning.jail_seconds += 10. * ((skill - 0.4) / 0.6).clamp(0., 1.);
            }
        }
        let mut game = Self {
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
            bags: Vec::new(),
            cameras: 0.,
            tuning,
            check_seq: 0,
            noises: Vec::new(),
            check_passes: [0; 8],
            check_fails: [0; 8],
            slips: [0, 0],
            jobs: vec![stats::JobRecord::starting(0.)],
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
            arrests: Vec::new(),
            slow_left: 0.,
            slow_cooldown: 0.,
            slow_owner: None,
            slow_held: false,
            callout: None,
            impacts: Vec::new(),
            impact_seq: 0,
            stamps: Vec::new(),
            stamp_seq: 0,
            items: items::Items::default(),
            sandbox: false,
        };
        game.seed_items();
        for index in 0..game.actors.len() {
            game.actors[index].mind = game.mind_for(game.actors[index].seat);
        }
        game
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
        self.actors
            .iter()
            .filter(|a| a.side == Side::Police)
            .flat_map(|a| {
                let offset = (a.car_yaw.sin() * 0.9, a.car_yaw.cos() * 0.9);
                [
                    a.car_at,
                    (a.car_at.0 + offset.0, a.car_at.1 + offset.1),
                    (a.car_at.0 - offset.0, a.car_at.1 - offset.1),
                ]
            })
            .collect()
    }
    /// Seats a patrol car's driver at `at`, facing `yaw`, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_place_car(&mut self, seat: u8, at: (f32, f32), yaw: f32) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat && a.side == Side::Police) {
            a.in_car = true;
            a.at = at;
            a.car_at = at;
            a.car_yaw = yaw;
            a.heading = yaw;
            a.yaw = yaw;
            a.drive_speed = 0.;
        }
    }
    /// Stands a seat on foot at `at`, its patrol car (if any) parked at
    /// `car`, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_place_on_foot(&mut self, seat: u8, at: (f32, f32), yaw: f32, car: Option<(f32, f32)>) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat) {
            a.in_car = false;
            a.at = at;
            a.yaw = yaw;
            a.jailed = false;
            a.stun = 0.;
            a.immune = 0.;
            a.path.clear();
            a.drive_speed = 0.;
            a.last_seen = None;
            if let Some(car) = car {
                a.car_at = car;
            }
        }
    }
    /// Finishes the cuffing on this robber so the escort and its slip check
    /// start on the next tick, optionally with a wider grip.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_cuff_now(&mut self, robber: u8, grip: Option<f32>) {
        for x in self.arrests.iter_mut().filter(|x| x.robber == robber) {
            x.t = std::f32::consts::TAU;
            x.cursor = arrest::figure_eight(x.t);
            if let (Some(grip), Some(cuffs)) = (grip, x.cuffs.as_mut()) {
                cuffs.grip = grip;
            }
        }
    }
    /// A walking route around walls and through doors, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_walk_route(from: (f32, f32), to: (f32, f32)) -> Vec<(f32, f32)> {
        route(from, to, false).into()
    }
    /// Moves a parked patrol car, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_park_car(&mut self, seat: u8, at: (f32, f32)) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat && !a.in_car) {
            a.car_at = at;
        }
    }
    /// Records an impact as if it happened, for scripted reviews of effects.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_impact(&mut self, kind: &'static str, at: (f32, f32), victim: Option<u8>, force: f32) {
        self.impact_on(kind, at, victim, force);
    }
    /// Opens the vault job with the power already cut, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_vault_job(&mut self) {
        self.job_wait = 0.;
        self.job = Job::Vault;
        self.power = 600.;
        self.vault = 0.;
        self.bags.clear();
    }
    /// Starts the power job with no progress, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_power_job(&mut self) {
        self.job_wait = 0.;
        self.job = Job::Power;
        self.power = 0.;
        self.switch = 0.;
        self.bags.clear();
    }
    /// Sounds the bank alarm, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_raise_alarm(&mut self) {
        if !self.alarm {
            self.alarm_at = self.time;
        }
        self.alarm = true;
    }
    /// Starts the night as a free-roam sandbox: no clock, no winner, no briefing.
    pub fn begin_sandbox(&mut self) {
        self.sandbox = true;
        self.briefing_left = 0.;
        self.time = self.time.max(3.);
        // Players start on their own feet beside the car, so the stick moves
        // a person they can see from the first frame.
        let drivers: Vec<u8> = self
            .actors
            .iter()
            .filter(|a| !a.automated && a.in_car)
            .map(|a| a.seat)
            .collect();
        for seat in drivers {
            self.toggle_car(seat);
        }
    }
    fn index_of(&self, seat: u8) -> Option<usize> {
        self.actors.iter().position(|a| a.seat == seat)
    }
    fn drop_arrests_of(&mut self, seat: u8) {
        while let Some(index) = self.arrests.iter().position(|x| x.officer == seat || x.robber == seat) {
            self.release(index, 0., Release::OfficerGone);
        }
    }
    /// Switches a seat between officer and crew, on foot, with a patrol car
    /// parked on the nearest lane for an officer.
    pub fn sandbox_role(&mut self, seat: u8, police: bool) -> bool {
        self.drop_arrests_of(seat);
        let Some(index) = self.index_of(seat) else { return false };
        let a = &mut self.actors[index];
        a.side = if police { Side::Police } else { Side::Crew };
        a.in_car = false;
        a.jailed = false;
        a.stun = 0.;
        a.immune = 1.;
        a.check = None;
        a.path.clear();
        if police {
            if let Some((at, yaw)) = clear_lanes(a.at, a.yaw).into_iter().next() {
                a.car_at = at;
                a.car_yaw = yaw;
            }
        }
        let at = self.actors[index].at;
        self.drop_bag(seat, at);
        true
    }
    /// Adds a dummy or bot for `seat` a few metres in front of `near`.
    pub fn sandbox_spawn(&mut self, seat: u8, near: u8, police: bool, behavior: Behavior) -> bool {
        if self.index_of(seat).is_some() {
            return false;
        }
        let Some(template) = self.index_of(near) else { return false };
        let origin = self.actors[template].at;
        let yaw = self.actors[template].yaw;
        let at = [5., 3.5, 2.]
            .into_iter()
            .flat_map(|d| {
                [0., 1.2, -1.2, 2.4, -2.4]
                    .into_iter()
                    .map(move |turn: f32| (origin.0 + (yaw + turn).sin() * d, origin.1 + (yaw + turn).cos() * d))
            })
            .find(|p| walkable(*p))
            .unwrap_or(origin);
        let mut a = self.actors[template].clone();
        a.seat = seat;
        a.side = if police { Side::Police } else { Side::Crew };
        a.at = at;
        a.car_at = at;
        a.in_car = false;
        a.jailed = false;
        a.stun = 0.;
        a.cuff = 0.;
        a.interact = false;
        a.suction = false;
        a.immune = 0.;
        a.path.clear();
        a.grab = None;
        a.cuff_cooldown = 0.;
        a.notice = None;
        a.stuck_for = 0.;
        a.velocity = (0., 0.);
        a.automated = behavior == Behavior::Bot;
        a.dummy = (behavior != Behavior::Bot).then_some(behavior);
        a.spawned = true;
        a.mind = self.mind_for(seat);
        a.yaw = (origin.0 - at.0).atan2(origin.1 - at.1);
        self.actors.push(a);
        true
    }
    pub fn sandbox_despawn(&mut self, seat: u8) -> bool {
        self.drop_arrests_of(seat);
        if let Some(at) = self.actors.iter().find(|a| a.seat == seat).map(|a| a.at) {
            self.drop_bag(seat, at);
        }
        let before = self.actors.len();
        self.actors.retain(|a| a.seat != seat);
        before != self.actors.len()
    }
    /// The spawned dummies and bots, newest last.
    pub fn sandbox_spawned(&self) -> Vec<u8> {
        self.actors.iter().filter(|a| a.spawned).map(|a| a.seat).collect()
    }
    pub fn sandbox_behave(&mut self, behavior: Behavior) {
        for a in &mut self.actors {
            if a.spawned {
                a.automated = behavior == Behavior::Bot;
                a.dummy = (behavior != Behavior::Bot).then_some(behavior);
            }
        }
    }
    pub fn set_alarm(&mut self, on: bool) {
        if on && !self.alarm {
            self.alarm_at = self.time;
        }
        self.alarm = on;
    }
    /// Moves a seat to a landmark: an officer at the wheel by road, anyone
    /// else on foot beside it.
    pub fn sandbox_teleport(&mut self, seat: u8, place: Landmark, streets: &mut impl Streets) -> bool {
        let Some(index) = self.index_of(seat) else { return false };
        let target = match place {
            Landmark::Bank => BANK,
            Landmark::Jail => JAIL,
            Landmark::Power => POWER,
            Landmark::Escape => ESCAPE,
            Landmark::Car => self.actors[index].car_at,
        };
        self.drop_arrests_of(seat);
        let a = &self.actors[index];
        if a.in_car {
            let Some((at, yaw)) = clear_lanes(target, a.car_yaw)
                .into_iter()
                .find(|(at, yaw)| streets.clear_for_car(*at, *yaw))
            else {
                return false;
            };
            let a = &mut self.actors[index];
            a.at = at;
            a.car_at = at;
            a.car_yaw = yaw;
            a.heading = yaw;
            a.yaw = yaw;
            a.drive_speed = 0.;
        } else {
            let at = [(0., 2.5), (2.5, 0.), (0., -2.5), (-2.5, 0.), (0., 0.)]
                .into_iter()
                .map(|(dx, dz)| (target.0 + dx, target.1 + dz))
                .find(|p| walkable(*p) && city_buildings::room_at(*p) == city_buildings::room_at(target))
                .unwrap_or(target);
            let a = &mut self.actors[index];
            a.at = at;
            a.jailed = false;
        }
        let a = &mut self.actors[index];
        a.path.clear();
        a.velocity = (0., 0.);
        a.stuck_for = 0.;
        true
    }
    /// Resets a car whether or not it is stuck, as the sandbox's own button.
    pub fn sandbox_reset_car(&mut self, seat: u8, streets: &mut impl Streets) -> bool {
        if let Some(index) = self.index_of(seat) {
            if self.actors[index].in_car {
                self.actors[index].stuck_for = STUCK_SECONDS;
            }
        }
        self.reset_car(seat, streets)
    }
    /// Starts an arrest at once: an officer cuffs the nearest robber, and a
    /// robber is cuffed by the nearest officer.
    pub fn sandbox_arrest(&mut self, seat: u8) -> bool {
        let Some(index) = self.index_of(seat) else { return false };
        if self.role(seat).is_some() {
            return false;
        }
        let me = self.actors[index].clone();
        let other = self
            .actors
            .iter()
            .filter(|o| o.side != me.side && !o.jailed && self.role(o.seat).is_none())
            .min_by(|x, y| distance(x.at, me.at).total_cmp(&distance(y.at, me.at)))
            .map(|o| o.seat);
        let Some(other) = other else { return false };
        let (officer, robber) = if me.side == Side::Police { (seat, other) } else { (other, seat) };
        for (who, at) in [(officer, me.at), (robber, (me.at.0 + me.yaw.sin() * 1.2, me.at.1 + me.yaw.cos() * 1.2))] {
            if let Some(i) = self.index_of(who) {
                let a = &mut self.actors[i];
                a.in_car = false;
                a.stun = 0.;
                a.immune = 0.;
                a.height = 0.;
                a.up = 0.;
                if who != seat {
                    a.at = if walkable(at) { at } else { me.at };
                }
            }
        }
        self.start_arrest(officer, robber);
        true
    }
    /// Puts a seat at a crew work station with that station's job open, so
    /// its beat can be practised on demand.
    pub fn sandbox_station(&mut self, seat: u8, station: StationKind, streets: &mut impl Streets) -> bool {
        self.drop_arrests_of(seat);
        let Some(index) = self.index_of(seat) else { return false };
        if self.actors[index].side == Side::Police {
            self.sandbox_role(seat, false);
        }
        self.job_wait = 0.;
        match station {
            StationKind::Power => {
                self.job = Job::Power;
                self.switch = 0.;
                self.bags.clear();
                self.sandbox_teleport(seat, Landmark::Power, streets);
                self.actors[index].at = POWER;
            }
            StationKind::Vault => {
                self.job = Job::Vault;
                self.power = 600.;
                self.vault = 0.;
                self.bags.clear();
                self.sandbox_teleport(seat, Landmark::Bank, streets);
                self.actors[index].at = BANK;
            }
            StationKind::Van => {
                self.job = Job::Carry;
                self.bags = vec![bags::Bag { at: ESCAPE, carrier: Some(seat) }];
                self.escape_progress = 0.;
                self.sandbox_teleport(seat, Landmark::Escape, streets);
                self.actors[index].at = ESCAPE;
            }
            StationKind::Jail => {
                let prisoner = self
                    .actors
                    .iter()
                    .position(|a| a.side == Side::Crew && a.seat != seat && a.spawned);
                let Some(prisoner) = prisoner else { return false };
                let p = &mut self.actors[prisoner];
                p.jailed = true;
                p.jail_left = 600.;
                p.at = JAIL_CELL;
                self.rescue = 0.;
                self.sandbox_teleport(seat, Landmark::Jail, streets);
                self.actors[index].at = (JAIL.0 - 1.5, JAIL.1);
            }
        }
        true
    }
    /// Where patrol cars are driving with their lights on, for traffic to yield.
    pub fn patrol_sirens(&self) -> Vec<(f32, f32)> {
        self.actors
            .iter()
            .filter(|a| a.side == Side::Police && a.in_car && a.drive_speed > LIGHTS_SPEED)
            .map(|a| a.car_at)
            .collect()
    }
    /// Lifts a stuck patrol car onto the nearest clear stretch of road, facing
    /// along the street. Only a car that has been stuck can be reset.
    pub fn reset_car(&mut self, seat: u8, streets: &mut impl Streets) -> bool {
        let Some(index) = self
            .actors
            .iter()
            .position(|a| a.seat == seat && a.in_car && a.stuck_for >= STUCK_SECONDS)
        else {
            return false;
        };
        let from = self.actors[index].at;
        let facing = self.actors[index].car_yaw;
        let others: Vec<(f32, f32)> = self
            .actors
            .iter()
            .filter(|a| a.seat != seat)
            .flat_map(|a| [a.at, a.car_at])
            .collect();
        let Some((at, yaw)) = clear_lanes(from, facing)
            .into_iter()
            .filter(|(at, _)| others.iter().all(|o| distance(*o, *at) > 4.))
            .find(|(at, yaw)| streets.clear_for_car(*at, *yaw))
        else {
            return false;
        };
        let a = &mut self.actors[index];
        a.at = at;
        a.car_at = at;
        a.car_yaw = yaw;
        a.heading = yaw;
        a.yaw = yaw;
        a.drive_speed = 0.;
        a.velocity = (0., 0.);
        a.stuck_for = 0.;
        a.path.clear();
        a.notice = Some(("Your car is back on a clear lane.", NOTICE_SECONDS));
        true
    }
    pub fn toggle_car(&mut self, seat: u8) -> bool {
        if self.resolved.is_some() {
            return false;
        }
        if self.role(seat).is_some() {
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
        a.heading = a.car_yaw;
        a.interact = false;
        true
    }
    fn end_job(&mut self, message: &'static str) {
        if self.job_wait > 0. {
            return;
        }
        self.job_wait = 8.;
        self.dispatch = Some(message);
        if let Some(job) = self.jobs.last_mut() {
            job.ended = Some(self.time);
        }
        self.bags.clear();
        self.alarm = false;
    }
    fn reset_job(&mut self) {
        self.jobs.push(stats::JobRecord::starting(self.time));
        while !self.arrests.is_empty() {
            self.release(0, 0., Release::JobOver);
        }
        self.job = Job::Power;
        self.power = 0.;
        self.switch = 0.;
        self.vault = 0.;
        self.bags.clear();
        self.cameras = 0.;
        for a in &mut self.actors {
            a.check = None;
        }
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
    fn role(&self, seat: u8) -> Option<(usize, Role)> {
        self.arrests.iter().enumerate().find_map(|(index, x)| {
            if x.officer == seat {
                Some((index, Role::Officer(x.stage)))
            } else if x.robber == seat {
                Some((index, Role::Robber))
            } else {
                None
            }
        })
    }
    /// The officer's cursor on the figure eight, in pad units.
    pub fn trace(&mut self, seat: u8, x: f32, y: f32) {
        if let Some(a) = self.arrests.iter_mut().find(|a| a.officer == seat) {
            a.set_cursor(x, y);
        }
    }
    /// A held robber's timed struggle. Dazed robbers cannot struggle yet.
    /// The robber's input to their cuff-slip check.
    pub fn slip_input(&mut self, seat: u8, input: crate::city_slip::SlipInput) -> bool {
        if self.actors.iter().any(|a| a.seat == seat && a.automated) {
            return false;
        }
        let Some(cuffs) = self
            .arrests
            .iter_mut()
            .find(|x| x.robber == seat && x.stage == Stage::Escort)
            .and_then(|x| x.cuffs.as_mut())
        else {
            return false;
        };
        cuffs.input(input);
        true
    }
    pub fn struggle(&mut self, seat: u8) -> bool {
        let Some(index) = self.arrests.iter().position(|a| a.robber == seat) else {
            return false;
        };
        let on_ground = self.arrests[index].stage == Stage::Ground;
        if self.actors.iter().any(|a| a.seat == seat && (a.automated || a.stun > 0. && !on_ground)) {
            return false;
        }
        let result = self.arrests[index].struggle();
        if let Some(at) = self.actors.iter().find(|a| a.seat == seat).map(|a| a.at) {
            match result {
                Struggle::Escaped => {}
                Struggle::Hit => self.impact("struggle", at),
                Struggle::Miss => self.impact("fumble", at),
                Struggle::Ignored => {}
            }
        }
        if result == Struggle::Escaped {
            if let Some(robber) = self.actors.iter_mut().find(|a| a.seat == seat) {
                robber.stun = 0.;
            }
            self.release(index, 0.5, Release::BrokeFree);
            self.stamp("broke_free", Side::Crew);
        }
        matches!(result, Struggle::Hit | Struggle::Escaped)
    }
    /// A callout selects a visible suspect in front of an officer. The server
    /// owns the selection; clients can only suggest their look direction.
    pub fn callout(&mut self, seat: u8, yaw: f32) -> bool {
        let Some(officer) = self.actors.iter().find(|a| a.seat == seat) else { return false };
        if officer.side != Side::Police || officer.in_car || officer.jailed || officer.stun > 0.
            || self.slow_cooldown > 0. || self.slow_owner.is_some() || !yaw.is_finite() {
            return false;
        }
        let target = self.actors.iter()
            .filter(|a| a.side == Side::Crew && !a.jailed && self.visible(officer.at, a.at, 14.))
            .filter(|a| wrap((a.at.0 - officer.at.0).atan2(a.at.1 - officer.at.1) - yaw).abs() < 0.42)
            .min_by(|a, b| distance(officer.at, a.at).total_cmp(&distance(officer.at, b.at)))
            .map(|a| a.seat);
        if let Some(suspect) = target {
            self.callout = Some(Callout { officer: seat, suspect });
            self.impact("hands_up", officer.at);
            true
        } else { false }
    }

    /// Hold-to-use focus. Letting go stops slow motion immediately; a dive
    /// already committed is allowed to finish before the shared recharge.
    pub fn clarity(&mut self, seat: u8, held: bool) -> bool {
        if self.slow_owner == Some(seat) {
            if !held {
                self.slow_held = false;
                self.slow_left = 0.;
                if !self.actors.iter().any(|a| a.seat == seat && a.lunge > 0.) {
                    self.slow_owner = None;
                    self.slow_cooldown = 5.;
                }
            }
            return true;
        }
        if !held || self.slow_owner.is_some() || self.slow_cooldown > 0. { return false; }
        let Some(callout) = self.callout else { return false; };
        if callout.officer != seat || !self.callout_visible(callout) { return false; }
        self.slow_owner = Some(seat);
        self.slow_held = true;
        self.slow_left = 2.;
        true
    }

    fn callout_visible(&self, callout: Callout) -> bool {
        let Some(cop) = self.actors.iter().find(|a| a.seat == callout.officer) else { return false };
        let Some(target) = self.actors.iter().find(|a| a.seat == callout.suspect) else { return false };
        cop.side == Side::Police && !cop.in_car && !cop.jailed && target.side == Side::Crew
            && !target.jailed && self.visible(cop.at, target.at, 16.)
            && wrap((target.at.0 - cop.at.0).atan2(target.at.1 - cop.at.1) - cop.yaw).abs() < 1.0
    }

    /// World time runs slower only while an officer is holding Clarity.
    pub fn time_scale(&self) -> f32 {
        if self.slow_left <= 0. {
            return 1.;
        }
        let elapsed = 2. - self.slow_left.min(2.);
        let depth = (elapsed / 0.15).min(self.slow_left / 0.4).min(1.);
        1. - (1. - arrest::SLOW_SCALE) * depth
    }
    fn impact(&mut self, kind: &'static str, at: (f32, f32)) {
        self.impact_on(kind, at, None, 0.);
    }
    pub(super) fn impact_on(&mut self, kind: &'static str, at: (f32, f32), victim: Option<u8>, force: f32) {
        self.impact_seq = self.impact_seq.wrapping_add(1);
        self.impacts.push(Impact {
            id: self.impact_seq,
            kind,
            x: at.0,
            z: at.1,
            age: 0.,
            victim,
            force,
        });
    }
    fn start_arrest(&mut self, officer: u8, robber: u8) {
        let Some(r) = self.actors.iter().position(|a| a.seat == robber) else {
            return;
        };
        let Some(o) = self.actors.iter().position(|a| a.seat == officer) else {
            return;
        };
        let face = self.actors[r].at;
        let from = self.actors[o].at;
        let yaw = (face.0 - from.0).atan2(face.1 - from.1);
        let ground = !self.actors[o].automated
            && self.actors[o].lunge_hit
            && self.actors[o].lunge_target == Some(robber)
            && !self.actors[r].automated
            && !self.actors[r].ground_escape_used;
        if ground {
            self.actors[r].ground_escape_used = true;
        }
        let cop = &mut self.actors[o];
        cop.yaw = yaw;
        cop.grab = None;
        cop.interact = false;
        cop.path.clear();
        let thief = &mut self.actors[r];
        thief.yaw = yaw + std::f32::consts::PI;
        thief.interact = false;
        thief.suction = false;
        thief.path.clear();
        self.arrests.push(if ground { Arrest::ground(officer, robber) } else { Arrest::new(officer, robber) });
        self.impact("grab", face);
    }
    /// Ends an arrest without a jailing: the robber gets a head start and the
    /// officer a cooldown, so neither can chain slow motion back to back.
    fn release(&mut self, index: usize, officer_stun: f32, why: Release) {
        let x = self.arrests.remove(index);
        let (to_officer, to_robber) = why.messages();
        let mut at = None;
        for a in &mut self.actors {
            if a.seat == x.robber {
                a.cuff = 0.;
                a.immune = a.immune.max(2.5);
                a.path.clear();
                a.notice = Some((to_robber, NOTICE_SECONDS));
                at = Some(a.at);
            } else if a.seat == x.officer {
                a.cuff_cooldown = arrest::OFFICER_COOLDOWN;
                a.grab = None;
                a.stun = a.stun.max(officer_stun);
                a.path.clear();
                a.notice = Some((to_officer, NOTICE_SECONDS));
            }
        }
        if let Some(at) = at {
            self.impact("escape", at);
        }
    }
    fn jail(&mut self, index: usize) {
        let x = self.arrests.remove(index);
        if let Some(at) = self.actors.iter().find(|a| a.seat == x.robber).map(|a| a.at) {
            self.drop_bag(x.robber, at);
        }
        let mut at = None;
        let jail_seconds = self.tuning.jail_seconds;
        for a in &mut self.actors {
            if a.seat == x.robber {
                at = Some(a.at);
                if let Some(job) = self.jobs.last_mut() {
                    job.arrests += 1;
                }
                a.jailed = true;
                a.jail_left = jail_seconds;
                a.at = JAIL_CELL;
                a.cuff = 0.;
                a.interact = false;
                a.suction = false;
                a.path.clear();
            } else if a.seat == x.officer {
                a.cuff_cooldown = 1.5;
                a.grab = None;
            }
        }
        if let Some(at) = at {
            self.impact("jailed", at);
            self.stamp("busted", Side::Police);
        }
    }
    fn update_arrests(&mut self, dt: f32, eligible: &[u8]) {
        let mut index = 0;
        while index < self.arrests.len() {
            let (officer, robber) = (self.arrests[index].officer, self.arrests[index].robber);
            let (Some(o), Some(r)) = (
                self.actors.iter().position(|a| a.seat == officer),
                self.actors.iter().position(|a| a.seat == robber),
            ) else {
                self.arrests.remove(index);
                continue;
            };
            let cop = &self.actors[o];
            let gone = !cop.automated && !eligible.contains(&cop.seat);
            if gone || cop.jailed || cop.stun > 0. || cop.in_car || self.actors[r].jailed {
                let why = if cop.stun > 0. {
                    Release::OfficerDown
                } else {
                    Release::OfficerGone
                };
                self.release(index, 0., why);
                continue;
            }
            let robot_officer = cop.automated || cop.dummy == Some(Behavior::Fight);
            let dummy = self.actors[r].dummy;
            let robot_robber = (self.actors[r].automated
                || !eligible.contains(&robber)
                || matches!(dummy, Some(Behavior::Flee | Behavior::Fight)))
                && dummy != Some(Behavior::Still);
            let roll = self.roll(officer, robber, self.arrests[index].clock);
            let x = &mut self.arrests[index];
            if robot_officer && x.stage == Stage::Cuffing {
                x.automated_trace(dt, roll < 0.004);
            }
            let beat = x.phase_at(x.clock);
            let crossing = beat < 0.5 && x.phase_at(x.clock + dt) >= 0.5;
            let broke = x.escape >= 1.;
            let slipped = x.advance(dt);
            if robot_robber && crossing && self.actors[r].stun <= 0. {
                // Bots land about half their beats, so a careful officer still wins.
                if roll > 0.5 || dummy == Some(Behavior::Fight) {
                    self.arrests[index].struggle();
                } else {
                    let x = &mut self.arrests[index];
                    x.lockout = arrest::STRUGGLE_LOCKOUT;
                }
            }
            if slipped {
                let at = self.actors[r].at;
                self.impact("slip", at);
            }
            let x = &self.arrests[index];
            match x.stage {
                Stage::Ground => {
                    if x.stage_clock >= arrest::GROUND_ESCAPE_SECONDS {
                        self.arrests[index].stage = Stage::Cuffing;
                        self.arrests[index].stage_clock = 0.;
                    }
                }
                Stage::Cuffing => {
                    let progress = x.progress();
                    self.actors[r].cuff = progress;
                    if progress >= 1. {
                        let bot_skill = robot_robber.then_some(self.actors[r].skill);
                        let seed = self.seed ^ (u64::from(robber) << 48) ^ u64::from((self.time * 60.) as u32);
                        self.slips[0] += 1;
                        let x = &mut self.arrests[index];
                        x.stage = Stage::Escort;
                        x.stage_clock = 0.;
                        x.escape = 0.;
                        x.lockout = 0.;
                        x.cuffs = Some(crate::city_slip::SlipCheck::new(seed, bot_skill));
                        let at = self.actors[r].at;
                        // Cuffing the carrier seizes the bag at once; the escort
                        // only decides whether the robber goes to jail.
                        if self.carrying(robber) {
                            self.bag_scored(Some(robber), at, Side::Police, "Police seized the loot · next job in 8s");
                            self.stamp("bag_seized", Side::Police);
                        } else if self.actors[r].fake_bag > 0. {
                            self.actors[r].fake_bag = 0.;
                            self.impact("dud", at);
                        }
                        self.actors[r].cuff = 1.;
                        self.impact("cuffed", at);
                    } else if x.stage_clock > arrest::CUFF_LIMIT {
                        self.release(index, 0., Release::CuffTimeout);
                        continue;
                    }
                }
                Stage::Escort => {
                    let cop_at = self.actors[o].at;
                    let assist = self.actors.iter().any(|a| {
                        a.side == Side::Crew
                            && a.seat != robber
                            && !a.jailed
                            && a.stun <= 0.
                            && a.interact
                            && distance(a.at, cop_at) < 2.6
                    });
                    // The officer tightens the cuffs by holding still with the
                    // use button down; a bot does it once a slip looks likely.
                    let cop = &self.actors[o];
                    let tighten = if robot_officer {
                        x.cuffs.as_ref().is_some_and(|c| c.streak > 2.)
                    } else {
                        cop.interact
                    };
                    let x = &mut self.arrests[index];
                    if assist {
                        x.escape = (x.escape + dt / 2.2).min(1.);
                    }
                    let mut slipped_free = false;
                    if let Some(cuffs) = x.cuffs.as_mut() {
                        cuffs.tightening = tighten;
                        if cuffs.outcome.is_none()
                            && cuffs.advance(dt) == Some(crate::city_slip::Outcome::Escaped)
                        {
                            slipped_free = true;
                        }
                    }
                    if slipped_free {
                        x.escape = 1.;
                        self.slips[1] += 1;
                    }
                    let x = &mut self.arrests[index];
                    let (escape, clock) = (if broke { 1. } else { x.escape }, x.stage_clock);
                    let car = self.actors[o].car_at;
                    if distance(cop_at, car) < arrest::DELIVER_RANGE || reach(cop_at, JAIL, 4.)
                    {
                        self.jail(index);
                        continue;
                    }
                    if escape >= 1. {
                        self.release(index, 1.2, Release::BrokeFree);
                        self.stamp("broke_free", Side::Crew);
                        continue;
                    }
                    if clock > arrest::ESCORT_LIMIT {
                        self.release(index, 0., Release::EscortTimeout);
                        continue;
                    }
                    let yaw = self.actors[o].yaw;
                    let behind = (cop_at.0 - yaw.sin() * 1.1, cop_at.1 - yaw.cos() * 1.1);
                    let thief = &mut self.actors[r];
                    let old = thief.at;
                    thief.at = led_step(old, cop_at, behind, dt);
                    thief.velocity = ((thief.at.0 - old.0) / dt, (thief.at.1 - old.1) / dt);
                    thief.yaw = yaw;
                }
            }
            index += 1;
        }
    }
    fn roll(&self, a: u8, b: u8, clock: f32) -> f32 {
        let mut h = self.seed
            ^ (u64::from(a) << 40)
            ^ (u64::from(b) << 32)
            ^ u64::from((clock * 60.) as u32);
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 32;
        (h % 10_000) as f32 / 10_000.
    }
    #[cfg(test)]
    pub(crate) fn place(&mut self, seat: u8, at: (f32, f32), in_car: bool, yaw: f32) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat) {
            a.at = at;
            a.in_car = in_car;
            a.car_at = if in_car { at } else { a.car_at };
            a.yaw = yaw;
            a.car_yaw = yaw;
            a.heading = yaw;
            a.immune = 0.;
            a.path.clear();
        }
    }
    fn station_for(&self, a: &Actor) -> Option<(Station, (f32, f32))> {
        if a.jailed || a.stun > 0. || a.in_car || self.role(a.seat).is_some() {
            return None;
        }
        if a.side == Side::Police {
            if let Some(at) = self
                .loose_bags()
                .into_iter()
                .chain(self.items.decoys.iter().map(|d| d.at))
                .find(|at| reach(a.at, *at, 2.3))
            {
                return Some((Station::Loot, at));
            }
            if a.tire > 0. && distance(a.at, a.car_at) < items::REPAIR_RANGE {
                return Some((Station::Repair, a.car_at));
            }
            return None;
        }
        if self.carrying(a.seat) {
            return reach(a.at, ESCAPE, 3.).then_some((Station::Van, ESCAPE));
        }
        if let Some(at) = self.loose_bags().into_iter().find(|at| reach(a.at, *at, 2.3)) {
            return Some((Station::Bag, at));
        }
        if reach(a.at, JAIL, 2.5) && self.actors.iter().any(|r| r.side == Side::Crew && r.jailed) {
            return Some((Station::Jail, JAIL));
        }
        if self.job != Job::Carry && reach(a.at, POWER, 2.3) {
            return Some((Station::Power, POWER));
        }
        if self.tuning.second_post && self.job != Job::Carry && reach(a.at, CAMERAS, 2.3) {
            return Some((Station::Cameras, CAMERAS));
        }
        if self.vault_open_for_work() && reach(a.at, BANK, 3.5) {
            return Some((Station::Vault, BANK));
        }
        None
    }
    /// Starts a check when someone presses A at a station: on the press for
    /// players, and whenever a bot stands at one ready to work.
    fn start_checks(&mut self) {
        for index in 0..self.actors.len() {
            let a = &self.actors[index];
            let pressed = a.interact && (!a.was_interact || a.automated);
            let ready = pressed && a.check.is_none() && a.check_wait <= 0.;
            let station = if ready { self.station_for(a) } else { None };
            self.actors[index].was_interact = self.actors[index].interact;
            if let Some((station, at)) = station {
                self.start_check(index, station, at);
            }
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
        if tool == Tool::Smoke {
            if let Some(pin) = self.arrests.iter().position(|x| x.robber == seat && x.stage == Stage::Ground) {
                let Some(actor) = self.actors.iter().position(|a| a.seat == seat && a.side == Side::Crew && a.tools[0] > 0) else {
                    return false;
                };
                self.actors[actor].tools[0] -= 1;
                self.smokes.push(Smoke { at: self.actors[actor].at, left: SMOKE_SECONDS });
                self.actors[actor].stun = 0.;
                self.release(pin, 0.5, Release::BrokeFree);
                return true;
            }
        }
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
        if self
            .arrests
            .iter()
            .any(|x| x.robber == seat || x.officer == seat)
        {
            return false;
        }
        // Smoke is the crew's gadget and the radio ping the police's.
        if (tool == Tool::Smoke) != (a.side == Side::Crew) {
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
    fn recharge_scanners(&mut self) {
        let cooldown = self.tuning.ping_cooldown;
        for a in &mut self.actors {
            // scan_until is five seconds after the last ping.
            if a.side == Side::Police
                && a.tools[1] == 0
                && a.scan_until > 0.
                && self.time >= a.scan_until - 5. + cooldown
            {
                a.tools[1] = 1;
            }
        }
    }
    fn visible(&self, a: (f32, f32), b: (f32, f32), range: f32) -> bool {
        distance(a, b) <= range
            && !city_buildings::sight_blocked(a, b)
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
                    // A decoy carrier looks exactly like the real one to police.
                    carrying: shown
                        && (self.carrying(a.seat)
                            || a.fake_bag > 0. && mine.is_some_and(|me| me.side == Side::Police)),
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
                    held: shown && self.role(a.seat).is_some(),
                    snow: shown && a.in_car && on_snow(a.at),
                    drift: if shown && a.in_car { wrap(a.car_yaw - a.heading) } else { 0. },
                    room: if shown { city_buildings::room_at(a.at) } else { None },
                    flat: shown && a.tire > 0.,
                    rolling: shown && a.roll > 0.,
                    lunging: shown && a.lunge > 0.,
                    diving: shown && a.dive_view > 0.,
                    dive_phase: if shown { (1. - a.dive_view / items::DIVE_VIEW_SECONDS).clamp(0., 1.) } else { 0. },
                    spinning: shown && a.spin > 0.,
                }
            })
            .collect::<Vec<_>>();
        let objective = if let Some(side) = self.resolved {
            if side == Side::Crew {
                "Crew escaped with the loot"
            } else {
                "Police secured the city"
            }
        } else if mine.is_some_and(|a| a.jailed) {
            "Teammates can rescue you. Bail releases you when the timer ends."
        } else if let Some((_, role)) = mine.and_then(|a| self.role(a.seat)) {
            match role {
                Role::Officer(Stage::Ground) => {
                    "Keep them pinned. They have one chance to break free."
                }
                Role::Officer(Stage::Cuffing) => {
                    "Trace the figure eight to cuff them. Stay on the line."
                }
                Role::Officer(Stage::Escort) => {
                    "Drag them to your patrol car or the jail before they break free."
                }
                Role::Robber => if mine.is_some_and(|a| self.arrests.iter().any(|x| x.robber == a.seat && x.stage == Stage::Ground)) {
                    "Hit the narrow escape window once, or use smoke to break free."
                } else {
                    "Struggle on the beat while they cuff you, then slip the cuffs: four clean seconds."
                },
            }
        } else if crew {
            match self.job {
                Job::Power => "One teammate holds the power switch; another opens the vault.",
                Job::Vault => "Keep the power off and vacuum the bank vault.",
                Job::Carry => "Get the loot to your secret escape van. Rescue captured teammates.",
            }
        } else if self.alarm {
            "Bank alarm! Catch the carrier or recover the dropped loot."
        } else {
            "Ram a robber to daze them, then get out and grab them close by to start cuffing."
        };
        let targets = self.targets_for(mine);
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
            remaining: if self.sandbox {
                LIMIT
            } else {
                (LIMIT - (self.time - 3.).max(0.)).max(0.)
            },
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
            audible_cars: self.actors.iter().filter(|a| a.side == Side::Police && a.in_car)
                .filter(|a| mine.is_some_and(|me| distance(me.at, a.car_at) < 55.))
                .map(|a| [a.seat as f32, a.car_at.0, a.car_at.1, a.drive_speed.abs()]).collect(),
            objective,
            step: mine.map(|a| self.step_for(a)),
            context: mine.and_then(|a| self.context_for(a)),
            job: crew.then_some(match self.job {
                Job::Power => "power",
                Job::Vault => "vault",
                Job::Carry => "carry",
            }),
            stamps: self
                .stamps
                .iter()
                .filter(|s| s.audience.is_none_or(|side| mine.is_some_and(|me| me.side == side)))
                .copied()
                .collect(),
            targets,
            bank: [BANK.0, BANK.1],
            jail: [JAIL.0, JAIL.1],
            escape: crew.then_some([ESCAPE.0, ESCAPE.1]),
            alarm: self.alarm,
            alert: self.alarm.then_some([BANK.0, BANK.1]),
            // Police see dropped loot only among `bags`, where decoys look the same.
            loot: self.loose_bags().first().filter(|_| crew).map(|p| [p.0, p.1]),
            bags: self.bag_views(mine),
            pads: mine.map_or_else(Vec::new, |a| self.pad_views(a.side)),
            hazards: self.hazard_views(mine),
            item: mine.and_then(|a| a.item),
            bay: mine
                .filter(|a| a.side == Side::Police)
                .map(|_| [items::BAY.0, items::BAY.1]),
            dodge_ready: mine.map_or(0., |a| a.roll_cooldown),
            takedown_ready: mine.map_or(0., |a| a.lunge_cooldown),
            loot_carrier: self.carrier_seats().into_iter().find(|seat| {
                crew || self
                    .actors
                    .iter()
                    .find(|a| a.seat == *seat)
                    .is_some_and(|a| mine.is_some_and(|me| self.visible(me.at, a.at, 25.)))
            }),
            bags_left: crew.then_some(self.bags.len()),
            check: mine.and_then(|a| self.check_view(a)),
            second_post: crew && self.tuning.second_post,
            cameras_seconds: (crew && self.tuning.second_post).then_some(self.cameras),
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
            arrests: self
                .arrests
                .iter()
                .filter(|x| {
                    actors
                        .iter()
                        .any(|a| (a.seat == x.officer || a.seat == x.robber) && a.x.is_some())
                })
                .map(Arrest::view)
                .collect(),
            slow: self.time_scale(),
            slow_left: self.slow_left,
            callout: self.callout.filter(|c| self.callout_visible(*c)
                && actors.iter().any(|a| a.seat == c.officer && a.x.is_some())
                && actors.iter().any(|a| a.seat == c.suspect && a.x.is_some()))
                .map(|c| [c.officer, c.suspect]),
            clarity_ready: self.slow_cooldown,
            clarity_owner: self.slow_owner,
            cuff_ready: mine.map(|a| a.cuff_cooldown).unwrap_or(0.),
            car_stuck: mine.is_some_and(|a| a.in_car && a.stuck_for >= STUCK_SECONDS),
            notice: mine.and_then(|a| a.notice).map(|(text, _)| text),
            grab: mine
                .and_then(|a| a.grab)
                .map(|(_, held)| (held / arrest::GRAB_SECONDS).min(1.))
                .unwrap_or(0.),
            impacts: self
                .impacts
                .iter()
                .filter(|i| {
                    mine.is_some_and(|me| {
                        (self.demo && me.automated) || self.visible(me.at, (i.x, i.z), 25.)
                    })
                })
                .copied()
                .collect(),
            actors,
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
            return (JAIL_CELL, false);
        }
        if let Some((_, Role::Officer(Stage::Escort))) = self.role(a.seat) {
            let goal = if distance(a.at, a.car_at) <= distance(a.at, JAIL) {
                a.car_at
            } else {
                JAIL
            };
            return (goal, false);
        }
        // In 1v4, the vault specialist should finish a started dial before
        // covering a post; leaving cancels the check and strands the job.
        if self.config.police == 1
            && self.actors.len() == 5
            && a.side == Side::Crew
            && a.check.as_ref().is_some_and(|check| check.kind == CheckKind::Dial)
            && self.vault_open_for_work()
        {
            return (BANK, true);
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
                if let Some(at) = self.heard(a) {
                    return (road_point(at), false);
                }
                if let Some(bag) = self.bag_views(Some(a)).into_iter().min_by(|x, y| {
                    distance(a.at, (x.x, x.z)).total_cmp(&distance(a.at, (y.x, y.z)))
                }) {
                    return (road_point((bag.x, bag.z)), false);
                }
                if let Some(spot) = self.inspecting(a) {
                    return (road_point(spot), false);
                }
                return (PATROL[a.patrol], false);
            }
            if a.tire > 0. && self.role(a.seat).is_none() {
                return (a.car_at, true);
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
                    && self.any_carrier()
                    && self
                        .actors
                        .iter()
                        .filter(|r| self.carrying(r.seat))
                        .all(|r| !self.visible(a.at, r.at, 25.))
                {
                    let travel = (self.time - when).min(2.5) * CARRY_SPEED;
                    let predicted = (at.0 + yaw.sin() * travel, at.1 + yaw.cos() * travel);
                    return (if walkable(predicted) { predicted } else { at }, false);
                }
            }
            // Secure a bag already within reach before chasing another robber.
            if let Some(bag) = self
                .bag_views(Some(a))
                .into_iter()
                .filter(|bag| distance(a.at, (bag.x, bag.z)) < 6.)
                .min_by(|x, y| distance(a.at, (x.x, x.z)).total_cmp(&distance(a.at, (y.x, y.z))))
            {
                return ((bag.x, bag.z), true);
            }
            if let Some(robber) = self
                .actors
                .iter()
                .filter(|r| r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 25.))
                .min_by(|a1, b1| {
                    (!(self.carrying(a1.seat) || a1.fake_bag > 0.))
                        .cmp(&!(self.carrying(b1.seat) || b1.fake_bag > 0.))
                        .then_with(|| distance(a.at, a1.at).total_cmp(&distance(a.at, b1.at)))
                })
            {
                if self.cut_off(a, robber.seat) {
                    return (ESCAPE, false);
                }
                return (robber.at, true);
            }
            // Bots fall for decoys exactly as players can.
            if let Some(bag) = self
                .bag_views(Some(a))
                .into_iter()
                .min_by(|x, y| distance(a.at, (x.x, x.z)).total_cmp(&distance(a.at, (y.x, y.z))))
            {
                return ((bag.x, bag.z), true);
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
            if let Some(at) = self.heard(a) {
                return (at, false);
            }
            // Answer a fresh alarm; an old one is worth no more than a patrol.
            if self.alarm && self.time - self.alarm_at < 25. {
                return (self.answer_alarm(a), false);
            }
            if let Some(spot) = self.inspecting(a) {
                return (spot, false);
            }
            return (a.car_at, false);
        }
        if self.carrying(a.seat) {
            return (ESCAPE, true);
        }
        if self.actors.iter().any(|r| r.side == Side::Crew && r.jailed) {
            let rescuer = self
                .actors
                .iter()
                .filter(|r| r.side == Side::Crew && !r.jailed && !self.carrying(r.seat))
                .min_by(|a, b| distance(a.at, JAIL).total_cmp(&distance(b.at, JAIL)));
            let guarded = self.actors.iter().any(|p| {
                p.side == Side::Police && distance(p.at, JAIL) < 7. && self.visible(a.at, p.at, 25.)
            });
            if rescuer.is_some_and(|rescuer| rescuer.seat == a.seat) && (!guarded || a.tools[0] > 0)
            {
                return (JAIL, true);
            }
        }
        // Each loose bag draws the free robber nearest to it.
        for bag in self.loose_bags() {
            let nearest = self
                .actors
                .iter()
                .filter(|r| r.side == Side::Crew && !r.jailed && !self.carrying(r.seat))
                .min_by(|x, y| distance(x.at, bag).total_cmp(&distance(y.at, bag)))
                .map(|r| r.seat);
            if nearest == Some(a.seat) {
                return (bag, true);
            }
        }
        let alone = self.actors.iter().filter(|r| r.side == Side::Crew && !r.jailed).count() <= 1;
        let refresh = |left: f32| left < 10.;
        match (self.job, self.crew_post(a.seat)) {
            (Job::Carry, _) | (_, hud::Post::Lure) => {
                // Shadow the nearest carrier, or draw police off down a street.
                if let Some(c) = self
                    .actors
                    .iter()
                    .filter(|r| self.carrying(r.seat))
                    .min_by(|x, y| distance(x.at, a.at).total_cmp(&distance(y.at, a.at)))
                {
                    let behind = (c.at.0 - c.yaw.sin() * 3., c.at.1 - c.yaw.cos() * 3.);
                    return (if walkable(behind) { behind } else { c.at }, false);
                }
                let lure = if index % 2 == 0 { (-16., 28.) } else { (16., 28.) };
                (lure, false)
            }
            (_, hud::Post::Power) => (POWER, refresh(self.power)),
            (_, hud::Post::Cameras) => (CAMERAS, refresh(self.cameras)),
            (_, hud::Post::Vault) => {
                let power_alone = alone || self.post_uncovered(hud::Post::Power, POWER);
                let desk_alone = alone || self.post_uncovered(hud::Post::Cameras, CAMERAS);
                if power_alone && self.power <= 2. {
                    (POWER, true)
                } else if desk_alone && self.tuning.second_post && self.cameras <= 2. {
                    (CAMERAS, true)
                } else {
                    (BANK, self.vault_open_for_work())
                }
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
        self.tick_with_vehicle_traffic(dt, movement, eligible, |from, to, _| car_hit(from, to));
    }
    /// Vehicles need their full footprint and must also respect stopped cars.
    /// The existing point-actor callback remains available to standalone rules callers.
    pub fn tick_with_vehicle_traffic(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        car_hit: impl FnMut((f32, f32), (f32, f32), Option<f32>) -> bool,
    ) {
        self.tick_with_traffic(dt, movement, eligible, car_hit, |_, _| false);
    }
    pub fn tick_with_traffic(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        car_hit: impl FnMut((f32, f32), (f32, f32), Option<f32>) -> bool,
        crossing_wait: impl FnMut((f32, f32), (f32, f32)) -> bool,
    ) {
        self.tick_streets(
            dt,
            movement,
            eligible,
            &mut Callbacks(car_hit, crossing_wait),
        );
    }
    pub fn tick_streets(
        &mut self,
        dt: f32,
        movement: &[(u8, Stick, f32)],
        eligible: &[u8],
        streets: &mut impl Streets,
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
                    if a.in_car {
                        // Briefing uses car-relative controls too. World-relative
                        // input made D slide the patrol car left before the chase.
                        let throttle = stick.z.clamp(-1., 1.);
                        let desired_speed = if throttle >= 0. { 10. * throttle } else { 8. * throttle };
                        let turn_rate = 2.4 * (a.drive_speed.abs() / 4.).clamp(0., 1.);
                        let reverse = if a.drive_speed < -0.2 { -1. } else { 1. };
                        a.car_yaw = wrap(a.car_yaw - stick.x * reverse * turn_rate * dt);
                        a.drive_speed += (desired_speed - a.drive_speed).clamp(-26. * dt, 13. * dt);
                        let grip = 1. - (-ROAD_GRIP * dt).exp();
                        a.heading = wrap(a.heading + wrap(a.car_yaw - a.heading) * grip);
                        a.yaw = a.car_yaw;
                        let next = (
                            a.at.0 + a.heading.sin() * a.drive_speed * dt,
                            a.at.1 + a.heading.cos() * a.drive_speed * dt,
                        );
                        if on_road(next) { a.at = next; } else { a.drive_speed = 0.; }
                        a.car_at = a.at;
                    } else {
                        let next = (a.at.0 + stick.x * dt * SPEED, a.at.1 + stick.z * dt * SPEED);
                        if walkable(next) { a.at = next; }
                        if stick.x.hypot(stick.z) > 0.1 { a.yaw = stick.x.atan2(stick.z); }
                    }
                }
            }
            return;
        }
        let real = dt;
        let slow_scale = self.time_scale();
        // Human strafing stays responsive during Clarity for either team.
        // Compensate for slowed world time, then add a small burst over normal.
        let strafe_boost = if self.slow_left > 0. { 1.15 / slow_scale } else { 1. };
        let dt = dt * slow_scale;
        self.slow_left = (self.slow_left - real).max(0.);
        self.slow_cooldown = (self.slow_cooldown - real).max(0.);
        // Keep the beam on its target while they remain in sight and range.
        self.callout = self.callout.filter(|callout| self.callout_visible(*callout));
        if let Some(seat) = self.slow_owner {
            let diving = self.actors.iter().any(|a| a.seat == seat && a.lunge > 0.);
            if diving && self.slow_held && self.slow_left < 0.5 {
                self.slow_left = 0.5;
            } else if !diving && (self.slow_left <= 0. || self.callout.is_none()) {
                self.slow_left = 0.;
                self.slow_owner = None;
                self.slow_held = false;
                self.slow_cooldown = 5.;
            }
        }
        for impact in &mut self.impacts {
            impact.age += real;
        }
        self.impacts.retain(|i| i.age < IMPACT_SECONDS);
        self.age_stamps(real);
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
                            (!(self.carrying(a1.seat) || a1.fake_bag > 0.))
                                .cmp(&!(self.carrying(b1.seat) || b1.fake_bag > 0.))
                                .then_with(|| {
                                    distance(a.at, a1.at).total_cmp(&distance(a.at, b1.at))
                                })
                        })
                        .map(|r| (r.at, self.time, r.yaw, self.carrying(r.seat) || r.fake_bag > 0.))
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
                    if a.tire > 0.
                        && !self.actors.iter().any(|r| {
                            r.side == Side::Crew && !r.jailed && self.visible(a.at, r.at, 14.)
                        })
                    {
                        return true;
                    }
                    if self.bag_views(Some(a)).into_iter().any(|bag| {
                        distance(a.at, road_point((bag.x, bag.z))) < 4.
                    }) {
                        return true;
                    }
                    self.actors.iter().any(|r| {
                        r.side == Side::Crew
                            && !r.jailed
                            && self.visible(a.at, r.at, 14.)
                            && (distance(r.at, road_point(r.at)) > 2.5
                                || (a.drive_speed < 2. && distance(a.at, r.at) < 5.))
                    }) || (self.alarm
                        && self.time - self.alarm_at < 12.
                        && distance(a.at, road_point(BANK)) < 3.)
                        || self
                            .inspecting(a)
                            .is_some_and(|spot| distance(a.at, road_point(spot)) < 4.)
                } else {
                    distance(a.at, a.car_at) < 3.8
                        && a.tire <= 0.
                        && self.bag_views(Some(a)).is_empty()
                        && self.inspecting(a).is_none()
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
        let goals = self.bot_goals();
        // A sandbox dummy that flees or fights reacts to the nearest player
        // of the other side, or failing that the nearest bot.
        let foes: Vec<Option<(f32, f32)>> = self
            .actors
            .iter()
            .map(|a| {
                a.dummy?;
                let foes = || self.actors.iter().filter(|o| o.side != a.side && !o.jailed);
                let nearest = |list: Vec<(f32, f32)>| {
                    list.into_iter()
                        .min_by(|x, y| distance(*x, a.at).total_cmp(&distance(*y, a.at)))
                };
                nearest(foes().filter(|o| !o.automated && o.dummy.is_none()).map(|o| o.at).collect())
                    .or_else(|| nearest(foes().map(|o| o.at).collect()))
            })
            .collect();
        let police_speed = self.tuning.police_speed;
        let carry_speed = self.tuning.carry_speed;
        let mut vehicle_sweeps = Vec::new();
        let mut shoves = Vec::new();
        let roles: Vec<_> = self
            .actors
            .iter()
            .map(|a| self.role(a.seat).map(|r| r.1))
            .collect();
        let carriers = self.carrier_seats();
        let mut bumped_bags = Vec::new();
        // An officer tightening the cuffs has to stand still to do it.
        let holding: Vec<u8> = self
            .arrests
            .iter()
            .filter(|x| x.cuffs.as_ref().is_some_and(|c| c.tightening && c.outcome.is_none()))
            .map(|x| x.officer)
            .collect();
        let avoid: Vec<Vec<((f32, f32), f32)>> =
            self.actors.iter().map(|a| self.hazards_to_avoid(a)).collect();
        let now = self.time;
        for (index, a) in self.actors.iter_mut().enumerate() {
            a.velocity = (0., 0.);
            a.stun = (a.stun - dt).max(0.);
            a.immune = (a.immune - dt).max(0.);
            a.cuff_cooldown = (a.cuff_cooldown - dt).max(0.);
            a.notice = a
                .notice
                .map(|(text, left)| (text, left - dt))
                .filter(|(_, left)| *left > 0.);
            a.ram_guard = (a.ram_guard - dt).max(0.);
            let role = roles[index];
            if matches!(role, Some(Role::Robber | Role::Officer(Stage::Ground | Stage::Cuffing))) {
                continue;
            }
            if holding.contains(&a.seat) {
                continue;
            }
            if a.jailed {
                a.jail_left = (a.jail_left - dt).max(0.);
                if a.jail_left <= 0. {
                    a.jailed = false;
                    a.cuff = 0.;
                    a.at = JAIL_EXIT;
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
                if walkable(next) && !city_buildings::body_blocked(a.at, next) {
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
            if Self::forced_move(a, dt) {
                continue;
            }
            let mut stick = Stick::default();
            if a.automated || !eligible.contains(&a.seat) {
                let (goal, interact) = goals[index];
                a.interact = !a.in_car && interact && distance(a.at, goal) < 2.;
                a.suction = a.interact && a.side == Side::Crew && reach(a.at, BANK, 2.5);
                if a.seen_hazards != avoid[index].len() {
                    a.seen_hazards = avoid[index].len();
                    a.path.clear();
                }
                if distance(a.at, goal) > 1.4 {
                    if a.path.is_empty() || distance(a.goal, goal) > 3. && self.time > a.repath {
                        a.path = route_avoiding(a.at, goal, a.in_car, &avoid[index]);
                        // When a hazard closes the only way (a doorway, say),
                        // going over it beats standing still.
                        if a.path.is_empty() && !avoid[index].is_empty() {
                            a.path = route(a.at, goal, a.in_car);
                        }
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
                        stick = ai::wobble(
                            now,
                            a,
                            Stick {
                                x: (target.0 - a.at.0) / d,
                                z: (target.1 - a.at.1) / d,
                            },
                        );
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
            if let Some(behavior) = a.dummy {
                stick = Stick::default();
                a.interact = false;
                if let Some(foe) = foes[index] {
                    let gap = distance(a.at, foe).max(0.001);
                    let toward = Stick {
                        x: (foe.0 - a.at.0) / gap,
                        z: (foe.1 - a.at.1) / gap,
                    };
                    match behavior {
                        Behavior::Flee if gap < 18. => {
                            stick = Stick {
                                x: -toward.x,
                                z: -toward.z,
                            }
                        }
                        Behavior::Fight if a.side == Side::Police => {
                            stick = if gap > 1.4 { toward } else { Stick::default() };
                            a.interact = gap < 2.;
                            a.until = self.time + 0.3;
                        }
                        _ => {}
                    }
                }
            }
            if a.in_car {
                let push = stick.x.hypot(stick.z).min(1.);
                let snow = on_snow(a.at) || a.spin > 0.;
                let flat = a.tire > 0.;
                let mut top = if snow { SNOW_TOP_SPEED } else { TOP_SPEED };
                if flat {
                    top = top.min(items::FLAT_TOP_SPEED);
                }
                let mut desired_speed = if push > 0.1 { top * push } else { 0. };
                if push > 0.1 && a.automated {
                    // Bots steer grid paths point to point; they keep the
                    // original snappy steering their route following expects.
                    desired_speed = desired_speed.min(14.);
                    let turn = wrap(stick.x.atan2(stick.z) - a.car_yaw);
                    a.car_yaw = wrap(a.car_yaw + turn * (1. - (-14. * dt).exp()));
                } else if !a.automated {
                    // Human driving is car-relative: forward/back is signed
                    // throttle, and left/right changes heading only while moving.
                    let throttle = stick.z.clamp(-1., 1.);
                    desired_speed = if throttle >= 0. {
                        top * throttle
                    } else {
                        -top.min(8.) * -throttle
                    };
                    let turn_rate = 2.4 * (a.drive_speed.abs() / 4.).clamp(0., 1.);
                    let reverse = if a.drive_speed < -0.2 { -1. } else { 1. };
                    a.car_yaw = wrap(a.car_yaw - stick.x * reverse * turn_rate * dt);
                }
                let (brake, accel) = if snow { (SNOW_BRAKE, SNOW_ACCEL) } else { (26., 13.) };
                a.drive_speed += (desired_speed - a.drive_speed).clamp(-brake * dt, accel * dt);
                let grip = if a.automated && !snow {
                    1.
                } else {
                    1. - (-(if snow { SNOW_GRIP } else { ROAD_GRIP }) * dt).exp()
                };
                a.heading = wrap(a.heading + wrap(a.car_yaw - a.heading) * grip);
                a.yaw = a.car_yaw;
            } else if !a.automated && (a.side == Side::Police || strafe_boost > 1.) {
                // Keep the player's camera aim while strafing in Clarity.
            } else if stick.x.hypot(stick.z) > 0.1 {
                a.yaw = stick.x.atan2(stick.z);
            }
            let speed = if a.in_car {
                a.drive_speed
            } else if role.is_some() {
                arrest::ESCORT_SPEED
            } else if carriers.contains(&a.seat) || a.fake_bag > 0. {
                carry_speed
            } else if a.side == Side::Police {
                police_speed
            } else {
                SPEED
            };
            let old = a.at;
            let direction = if a.in_car {
                (a.heading.sin(), a.heading.cos())
            } else if !a.automated && strafe_boost > 1. {
                let ahead = (a.yaw.sin(), a.yaw.cos());
                let right = (-ahead.1, ahead.0);
                let lateral = stick.x * right.0 + stick.z * right.1;
                let forward = stick.x * ahead.0 + stick.z * ahead.1;
                (
                    ahead.0 * forward + right.0 * lateral * strafe_boost,
                    ahead.1 * forward + right.1 * lateral * strafe_boost,
                )
            } else {
                (stick.x, stick.z)
            };
            let next = (
                old.0 + direction.0 * speed * dt,
                old.1 + direction.1 * speed * dt,
            );
            // AI pedestrians wait outside a red crossing instead of repeatedly
            // running a carried bag into traffic. Human players choose the risk.
            if a.automated && !a.in_car && streets.crossing_wait(old, next) {
                continue;
            }
            if a.in_car
                && a.drive_speed > SHOVE_SPEED
                && streets.shove(old, next, a.yaw, a.drive_speed) > 0
            {
                a.drive_speed *= 0.6;
                shoves.push(next);
            }
            let pressing = stick.x.hypot(stick.z) > 0.1;
            if a.in_car && streets.blocked(old, next, Some(a.yaw)) {
                if pressing {
                    // A slow push still moves the car it is stuck against.
                    let reach = speed.max(NUDGE_PROBE) * dt;
                    let probe = (old.0 + direction.0 * reach, old.1 + direction.1 * reach);
                    streets.nudge(old, probe, a.yaw, dt);
                    a.stuck_for += real;
                    a.stuck_at = a.at;
                }
                a.drive_speed = 0.;
                a.velocity = (0., 0.);
                continue;
            }
            let open = |to: (f32, f32), from: (f32, f32)| {
                passable(to, a.in_car) && (a.in_car || !city_buildings::body_blocked(from, to))
            };
            if open(next, old) {
                a.at = next;
            } else {
                if a.in_car {
                    a.drive_speed *= (-4. * dt).exp();
                }
                if open((next.0, old.1), old) {
                    a.at.0 = next.0;
                }
                if open((a.at.0, next.1), (a.at.0, old.1)) {
                    a.at.1 = next.1;
                }
            }
            a.velocity = ((a.at.0 - old.0) / dt, (a.at.1 - old.1) / dt);
            if a.in_car {
                // A wedged car creeps a little when its driver lets go; only
                // getting clear of where it stuck withdraws the reset.
                if pressing && distance(old, a.at) < STUCK_PACE * dt {
                    a.stuck_for += real;
                    a.stuck_at = a.at;
                } else if distance(a.at, a.stuck_at) > STUCK_CLEAR {
                    a.stuck_for = 0.;
                }
                a.car_at = a.at;
                if a.drive_speed > 5. {
                    vehicle_sweeps.push((a.seat, old, a.at, a.yaw));
                }
            }
            if !a.in_car && a.immune <= 0. && streets.blocked(old, a.at, None) {
                if carriers.contains(&a.seat) {
                    bumped_bags.push((a.seat, old));
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
        for at in shoves {
            self.impact("shove", at);
        }
        // Bot drivers reset themselves once stuck, as a player can.
        let stuck_bots: Vec<u8> = self
            .actors
            .iter()
            .filter(|a| a.automated && a.in_car && a.stuck_for >= STUCK_SECONDS)
            .map(|a| a.seat)
            .collect();
        for seat in stuck_bots {
            self.reset_car(seat, streets);
        }
        for (seat, at) in bumped_bags {
            self.drop_bag(seat, at);
        }
        for (cop, from, to, yaw) in vehicle_sweeps {
            let victim = self
                .actors
                .iter()
                .find(|a| {
                    a.side == Side::Crew
                        && !a.jailed
                        && a.ram_guard <= 0.
                        && a.height <= 0.
                        && self.role(a.seat).is_none()
                        && segment_distance(a.at, from, to) < 1.55
                })
                .map(|a| (a.seat, a.at));
            if let Some((victim, hit)) = victim {
                let force = distance(from, to) / dt.max(0.001);
                self.impact_on("ram", hit, Some(victim), force);
                let at = self.actors.iter().find(|a| a.seat == victim).unwrap().at;
                self.drop_bag(victim, at);
                let a = self.actors.iter_mut().find(|a| a.seat == victim).unwrap();
                a.stun = RAM_STUN;
                a.ram_guard = 2.5;
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
        if !self.sandbox && self.time >= LIMIT + 3. {
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
        self.tick_items(dt);
        self.repair_flats(dt);
        self.update_arrests(real, eligible);
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
                    && (self.carrying(a.seat) || a.cuff > 0.25 || distance(a.at, JAIL) < 7.)
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
        self.start_checks();
        self.tick_checks(dt);
        self.recharge_scanners();
        // A held cuff action must remain close and visible; packets cannot claim a target.
        let busy: HashSet<u8> = self
            .arrests
            .iter()
            .flat_map(|x| [x.officer, x.robber])
            .collect();
        let mut grabs = Vec::new();
        for p in 0..self.actors.len() {
            let cop = &self.actors[p];
            if cop.side != Side::Police
                || cop.in_car
                || cop.jailed
                || cop.stun > 0.
                || !cop.interact
                || cop.cuff_cooldown > 0.
                || busy.contains(&cop.seat)
            {
                self.actors[p].grab = None;
                continue;
            }
            let target = self
                .actors
                .iter()
                .filter(|r| {
                    r.side == Side::Crew
                        && !r.jailed
                        && r.immune <= 0.
                        && r.height <= 0.4
                        && r.roll <= 0.
                        && !busy.contains(&r.seat)
                        && (cop.automated || cop.dummy == Some(Behavior::Fight) || r.stun > 0.)
                        && self.visible(cop.at, r.at, arrest::GRAB_RANGE)
                })
                .min_by(|a, b| {
                    (!self.carrying(a.seat))
                        .cmp(&!self.carrying(b.seat))
                        .then_with(|| distance(cop.at, a.at).total_cmp(&distance(cop.at, b.at)))
                        .then(a.seat.cmp(&b.seat))
                })
                .map(|r| r.seat);
            let held = match (target, cop.grab) {
                (Some(t), Some((g, held))) if g == t => held + real,
                (Some(_), _) => real,
                (None, _) => 0.,
            };
            self.actors[p].grab = target.map(|t| (t, held));
            if let Some(t) = target.filter(|_| held >= arrest::GRAB_SECONDS) {
                grabs.push((self.actors[p].seat, t));
            }
        }
        let mut taken = HashSet::new();
        for (officer, robber) in grabs {
            if taken.insert(robber) {
                self.start_arrest(officer, robber);
            }
        }
        for a in &mut self.actors {
            if a.side == Side::Crew
                && !a.jailed
                && !busy.contains(&a.seat)
                && !taken.contains(&a.seat)
            {
                a.cuff = 0.;
            }
        }
        if self
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .all(|a| a.jailed)
        {
            if self.job_wait <= 0. {
                if let Some(job) = self.jobs.last_mut() {
                    job.wiped = true;
                }
                self.night_score[0] += 1;
                self.end_job("Crew caught · regrouping in 8s");
            }
        }
    }
}

#[path = "city_hud.rs"]
mod hud;
#[path = "city_items.rs"]
mod items;
#[path = "city_bags.rs"]
mod bags;
#[path = "city_checks.rs"]
mod checks;
pub use checks::{CheckInput, CheckKind, CheckView};
#[path = "city_formats.rs"]
pub mod formats;
#[path = "city_ai.rs"]
mod ai;
#[path = "city_stats.rs"]
pub mod stats;
pub use ai::BotSkill;
pub use items::{BagView, HazardKind, HazardView, ItemAct, ItemKind, PadView};
pub use hud::{ContextAction, Stamp, Step, TargetKind, Verb};
#[cfg(test)]
#[path = "city_heist_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "city_building_play_tests.rs"]
mod building_play_tests;
