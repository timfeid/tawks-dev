//! One lobby's run of Port Run, refereed on the server: who plays which
//! side, where every car and person is, the take, heat, the police and how
//! it ends. Clients drive their own cars and feet and report where they are;
//! this checks those reports and decides everything that matters.

use std::collections::BTreeMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use super::ai::{self, Cop, Mode, Target};
use super::arrest::{self, Hold};
use super::damage::{self, Damage, Handling};
use super::geo::{Frame, LatLon};
use super::heat::{self, Heat};
use super::loader::Loader;
use super::map::{Map, P, RoadClass, dist};
use super::pursuit::{Phase as Chase, Pursuit};
use super::rules::{self, Class, Colour, Contract, GATE, GATE_RADIUS, PORT, PRECINCTS, Settings, TOWNS, Tier};
use super::take::{self, Entry, Odds, Outcome, Take};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Thief,
    Cop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    #[default]
    Walk,
    Run,
    Crouch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Foot {
    pub x: f64,
    pub z: f64,
    pub h: f64,
    pub s: f64,
    #[serde(default)]
    pub stance: Stance,
    #[serde(default)]
    pub flash: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "at", rename_all = "snake_case")]
pub enum Place {
    Foot,
    Seat { vehicle: usize, driver: bool },
}

#[derive(Clone, Debug, Serialize)]
pub struct Player {
    pub name: String,
    pub role: Option<Role>,
    pub place: Place,
    pub arrested: bool,
    #[serde(skip)]
    pub foot: Foot,
    #[serde(skip)]
    caught: Hold,
    #[serde(skip)]
    foot_at: f64,
    #[serde(skip)]
    pub online: bool,
    #[serde(skip)]
    spikes_left: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Crew,
    Parked,
    Stolen,
    Cruiser,
}

#[derive(Clone, Debug, Serialize)]
pub struct Vehicle {
    pub id: usize,
    pub kind: Kind,
    pub class: Class,
    pub colour: Colour,
    pub tier: Tier,
    #[serde(skip)]
    pub town: Option<usize>,
    pub x: f64,
    pub z: f64,
    pub h: f64,
    #[serde(skip)]
    pub s: f64,
    pub lights: bool,
    pub siren: bool,
    pub spotlight: bool,
    pub damage: Damage,
    pub driver: Option<String>,
    pub riders: Vec<String>,
    pub owner: Option<String>,
    #[serde(skip)]
    heat: Heat,
    #[serde(skip)]
    pose_at: f64,
    #[serde(skip)]
    prev: P,
    #[serde(skip)]
    junction: Option<usize>,
    #[serde(skip)]
    stopped_for: f64,
    #[serde(skip)]
    flagged: bool,
    #[serde(skip)]
    horn: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PingKind {
    Mark,
    Pickup,
    Danger,
    Radio,
    Witness,
    Reader,
    Report,
    Flagged,
    Seen,
    Alarm,
}

impl PingKind {
    pub fn for_cops(self) -> bool {
        !matches!(self, PingKind::Mark | PingKind::Pickup | PingKind::Danger)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Ping {
    pub id: u32,
    pub kind: PingKind,
    pub x: f64,
    pub z: f64,
    pub text: String,
    pub at: f64,
    pub by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Spike {
    pub id: u32,
    pub a: P,
    pub b: P,
    pub until: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome2 {
    Delivered,
    Arrested,
    Timeout,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Stats {
    pub driven: f64,
    pub pursuits: u32,
    pub spikes_hit: u32,
    pub spin_outs: u32,
    pub witnesses: u32,
    pub plate_hits: u32,
    pub foot_arrests: u32,
    pub peak_heat: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Results {
    pub outcome: Outcome2,
    pub payout: i64,
    pub condition: f64,
    pub seconds: f64,
    pub stats: Stats,
    pub thieves: Vec<String>,
    pub cops: Vec<String>,
    pub car: Option<String>,
}

impl Results {
    pub fn winners(&self) -> &[String] {
        if self.outcome == Outcome2::Delivered {
            &self.thieves
        } else {
            &self.cops
        }
    }

    pub fn losers(&self) -> &[String] {
        if self.outcome == Outcome2::Delivered {
            &self.cops
        } else {
            &self.thieves
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    Setup,
    Loading { since: f64 },
    Running { started: f64, ends: f64 },
    Ended { results: Results },
}

/// Something that just happened, for one side or everyone.
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub to: Audience,
    #[serde(flatten)]
    pub what: What,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    All,
    Thieves,
    Cops,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum What {
    Notice {
        text: String,
    },
    Alarm {
        vehicle: usize,
        until: f64,
    },
    PorchLight {
        x: f64,
        z: f64,
    },
    Bark {
        x: f64,
        z: f64,
    },
    Take {
        result: String,
    },
    Pursuit {
        started: bool,
    },
    Spin {
        vehicle: usize,
        side: f64,
    },
    Spiked {
        vehicle: usize,
    },
    Arrested {
        user_id: String,
    },
    Correction {
        vehicle: Option<usize>,
        x: f64,
        z: f64,
        h: f64,
    },
    Ended {
        outcome: Outcome2,
    },
    Started,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct VehiclePose {
    pub x: f64,
    pub z: f64,
    pub h: f64,
    pub s: f64,
    #[serde(default)]
    pub lights: bool,
    #[serde(default)]
    pub horn: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Msg {
    Role {
        role: Option<Role>,
    },
    Settings {
        settings: Settings,
    },
    Start,
    Rematch,
    Vehicle {
        id: usize,
        pose: VehiclePose,
    },
    Foot {
        foot: Foot,
    },
    Enter {
        vehicle: usize,
        #[serde(default)]
        driver: bool,
    },
    Exit,
    TakeStart {
        vehicle: usize,
    },
    TakeChoose {
        entry: Entry,
    },
    TakePress {
        t: f64,
    },
    TakeHold {
        t: f64,
        hold: bool,
    },
    TakeAbandon,
    Impact {
        vehicle: usize,
        x: f64,
        z: f64,
        speed: f64,
    },
    Siren {
        on: bool,
    },
    Spotlight {
        on: bool,
    },
    Spike,
    Ping {
        kind: PingKind,
        x: f64,
        z: f64,
        #[serde(default)]
        text: String,
    },
    Dev {
        cmd: Dev,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Dev {
    Vehicle { id: usize, x: f64, z: f64, h: f64 },
    Me { x: f64, z: f64 },
    ReportNow,
    EndIn { seconds: f64 },
}

const MAX_JUMP: f64 = 20.0;
const PING_FOR: f64 = 75.0;
const SPIKES_PER_COP: u8 = 2;
const COP_SIGHT: f64 = 90.0;
const SPOTLIGHT_SIGHT: f64 = 160.0;
const SPOTLIGHT_CONE: f64 = 0.28;
const REPORT_AFTER: f64 = 120.0;
const LOOKOUT_WARN: f64 = 160.0;
const READER_RANGE: f64 = 30.0;
/// Road refs plate readers watch: Routes 1 and 9, the Turnpike, Route 22, the Parkway and McCarter Highway.
const WATCHED: &[&str] = &["US 1", "US 9", "NJTP", "I 95", "US 22", "GSP", "NJ 21"];

pub struct Heist {
    pub settings: Settings,
    pub phase: Phase,
    pub contract: Option<Contract>,
    pub players: BTreeMap<String, Player>,
    pub vehicles: Vec<Vehicle>,
    pub cops: Vec<Cop>,
    pub take: Option<Take>,
    pub spikes: Vec<Spike>,
    pub readers: Vec<P>,
    pub pings: Vec<Ping>,
    pub pursuit: Pursuit,
    pub report_at: Option<f64>,
    pub reported: bool,
    pub stats: Stats,
    pub frame: Frame,
    pub dev: bool,
    pub events: Vec<Event>,
    /// Goes up when the parked car list changes, so clients refetch it.
    pub world_version: u32,
    loader: Option<Loader>,
    rng: StdRng,
    next_id: u32,
    box_hold: Hold,
    disabled_hold: Hold,
    warn_at: f64,
    want_at: f64,
    readers_from: usize,
    spike_at: f64,
    block_at: f64,
    stolen_from: Option<P>,
    witness_seen: bool,
}

fn heading(dx: f64, dz: f64) -> f64 {
    dx.atan2(-dz)
}

impl Heist {
    pub fn new(dev: bool, seed: u64) -> Heist {
        Heist {
            settings: Settings::default(),
            phase: Phase::Setup,
            contract: None,
            players: BTreeMap::new(),
            vehicles: Vec::new(),
            cops: Vec::new(),
            take: None,
            spikes: Vec::new(),
            readers: Vec::new(),
            pings: Vec::new(),
            pursuit: Pursuit::default(),
            report_at: None,
            reported: false,
            stats: Stats::default(),
            frame: Frame::new(PORT.into()),
            dev,
            events: Vec::new(),
            world_version: 0,
            loader: None,
            rng: StdRng::seed_from_u64(seed),
            next_id: 1,
            box_hold: Hold::default(),
            disabled_hold: Hold::default(),
            warn_at: 0.0,
            want_at: 0.0,
            readers_from: 0,
            spike_at: -100.0,
            block_at: -100.0,
            stolen_from: None,
            witness_seen: false,
        }
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn say(&mut self, to: Audience, what: What) {
        self.events.push(Event { to, what });
    }

    fn notice(&mut self, to: Audience, text: impl Into<String>) {
        self.say(to, What::Notice { text: text.into() });
    }

    pub fn running(&self) -> bool {
        matches!(self.phase, Phase::Running { .. })
    }

    pub fn join(&mut self, user_id: &str, name: &str) {
        let p = self.players.entry(user_id.to_string()).or_insert_with(|| Player {
            name: name.to_string(),
            role: None,
            place: Place::Foot,
            arrested: false,
            foot: Foot::default(),
            caught: Hold::default(),
            foot_at: 0.0,
            online: true,
            spikes_left: SPIKES_PER_COP,
        });
        p.online = true;
        p.name = name.to_string();
    }

    pub fn leave(&mut self, user_id: &str) {
        if let Some(p) = self.players.get_mut(user_id) {
            p.online = false;
            if matches!(self.phase, Phase::Setup) {
                self.players.remove(user_id);
            }
        }
        for v in &mut self.vehicles {
            if v.driver.as_deref() == Some(user_id) {
                v.driver = None;
            }
        }
    }

    pub fn count(&self, role: Role) -> usize {
        self.players
            .values()
            .filter(|p| p.role == Some(role) && p.online)
            .count()
    }

    fn thieves(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|(_, p)| p.role == Some(Role::Thief))
            .map(|(k, _)| k.clone())
            .collect()
    }

    fn cop_players(&self) -> Vec<String> {
        self.players
            .iter()
            .filter(|(_, p)| p.role == Some(Role::Cop))
            .map(|(k, _)| k.clone())
            .collect()
    }

    fn map(&self) -> Option<std::sync::MutexGuard<'_, Map>> {
        self.loader.as_ref().map(|l| l.map.lock().unwrap())
    }

    pub fn handle(&mut self, user: &str, host: bool, msg: Msg, now: f64) -> Result<(), &'static str> {
        match msg {
            Msg::Role { role } => {
                if !matches!(self.phase, Phase::Setup) {
                    return Err("Sides are picked before the run starts");
                }
                let room = match role {
                    Some(Role::Thief) => self.count(Role::Thief) < self.settings.max_thieves as usize,
                    Some(Role::Cop) => self.count(Role::Cop) < self.settings.max_cops as usize,
                    None => true,
                };
                let p = self.players.get_mut(user).ok_or("Join first")?;
                if p.role == role {
                    return Ok(());
                }
                if !room {
                    return Err("That side is full");
                }
                p.role = role;
                Ok(())
            }
            Msg::Settings { settings } => {
                if !host || !matches!(self.phase, Phase::Setup) {
                    return Err("Only the host sets up the run");
                }
                self.settings = settings.checked();
                Ok(())
            }
            Msg::Start => {
                if !host || !matches!(self.phase, Phase::Setup) {
                    return Err("Only the host starts the run");
                }
                if self.count(Role::Thief) == 0 {
                    return Err("Someone has to be a thief");
                }
                self.begin(now);
                Ok(())
            }
            Msg::Rematch => {
                if !host || !matches!(self.phase, Phase::Ended { .. }) {
                    return Err("Only the host starts another run");
                }
                let keep: Vec<(String, String, Option<Role>)> = self
                    .players
                    .iter()
                    .filter(|(_, p)| p.online)
                    .map(|(k, p)| (k.clone(), p.name.clone(), p.role))
                    .collect();
                let settings = self.settings;
                *self = Heist::new(self.dev, self.rng.random());
                self.settings = settings;
                for (id, name, role) in keep {
                    self.join(&id, &name);
                    self.players.get_mut(&id).unwrap().role = role;
                }
                Ok(())
            }
            // Cars and feet still reporting after the whistle are not an error.
            Msg::Vehicle { .. } | Msg::Foot { .. } | Msg::Impact { .. } if !self.running() => Ok(()),
            _ if !self.running() => Err("The run is not on"),
            Msg::Vehicle { id, pose } => self.vehicle_pose(user, id, pose, now),
            Msg::Foot { foot } => self.foot_pose(user, foot, now),
            Msg::Enter { vehicle, driver } => self.enter(user, vehicle, driver),
            Msg::Exit => self.exit(user),
            Msg::TakeStart { vehicle } => self.take_start(user, vehicle),
            Msg::TakeChoose { entry } => self.take_choose(user, entry, now),
            Msg::TakePress { t } => {
                let take = self
                    .take
                    .as_mut()
                    .filter(|t| t.thief == user)
                    .ok_or("You are not taking a car")?;
                let out = take.press(t, now, &mut self.rng);
                self.after_take(out, now);
                Ok(())
            }
            Msg::TakeHold { t, hold } => {
                let assist = self.assist();
                let take = self
                    .take
                    .as_mut()
                    .filter(|t| t.thief == user)
                    .ok_or("You are not taking a car")?;
                let out = take.hold(t, hold, now, assist);
                self.after_take(out, now);
                Ok(())
            }
            Msg::TakeAbandon => {
                if self.take.as_ref().is_some_and(|t| t.thief == user) {
                    self.take = None;
                }
                Ok(())
            }
            Msg::Impact { vehicle, x, z, speed } => {
                let v = self.vehicles.get_mut(vehicle).ok_or("No such car")?;
                if v.driver.as_deref() != Some(user) || !speed.is_finite() {
                    return Ok(());
                }
                v.damage.hit(x, z, speed.clamp(0.0, 60.0));
                if speed > 4.0 {
                    v.heat.add(speed * 1.2);
                }
                Ok(())
            }
            Msg::Siren { on } | Msg::Spotlight { on } if self.role(user) != Some(Role::Cop) => {
                let _ = on;
                Err("Only cops have those")
            }
            Msg::Siren { on } => {
                if let Some(v) = self.cruiser_of(user) {
                    self.vehicles[v].siren = on;
                }
                Ok(())
            }
            Msg::Spotlight { on } => {
                if let Some(v) = self.cruiser_of(user) {
                    self.vehicles[v].spotlight = on;
                }
                Ok(())
            }
            Msg::Spike => self.deploy_spike(user, now),
            Msg::Ping { kind, x, z, text } => self.ping_from(user, kind, x, z, text, now),
            Msg::Dev { cmd } => {
                if !self.dev {
                    return Err("Not here");
                }
                match cmd {
                    Dev::Vehicle { id, x, z, h } => {
                        if let Some(v) = self.vehicles.get_mut(id) {
                            v.x = x;
                            v.z = z;
                            v.h = h;
                            v.prev = (x, z);
                            self.say(
                                Audience::All,
                                What::Correction {
                                    vehicle: Some(id),
                                    x,
                                    z,
                                    h,
                                },
                            );
                        }
                    }
                    Dev::Me { x, z } => {
                        if let Some(p) = self.players.get_mut(user) {
                            p.foot.x = x;
                            p.foot.z = z;
                        }
                        self.say(
                            Audience::All,
                            What::Correction {
                                vehicle: None,
                                x,
                                z,
                                h: 0.0,
                            },
                        );
                    }
                    Dev::ReportNow => self.report_at = Some(now),
                    Dev::EndIn { seconds } => {
                        if let Phase::Running { ends, .. } = &mut self.phase {
                            *ends = now + seconds;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn role(&self, user: &str) -> Option<Role> {
        self.players.get(user).and_then(|p| p.role)
    }

    fn cruiser_of(&self, user: &str) -> Option<usize> {
        self.vehicles
            .iter()
            .position(|v| v.kind == Kind::Cruiser && v.owner.as_deref() == Some(user))
    }

    fn begin(&mut self, now: f64) {
        self.contract = Some(Contract::pick(&mut self.rng));
        let loader = Loader::new(self.frame);
        let c = self.contract.clone().unwrap();
        for &t in &c.towns {
            let (x, z) = self.frame.to_world(TOWNS[t].centre.into());
            loader.want_circle(x, z, TOWNS[t].radius + 200.0);
        }
        loader.want_around(GATE.0, GATE.1, 1);
        for p in PRECINCTS {
            let (x, z) = self.frame.to_world((*p).into());
            loader.want_around(x, z, 0);
        }
        self.loader = Some(loader);
        self.phase = Phase::Loading { since: now };
        self.notice(Audience::All, "Loading the map for the run");
    }

    fn vehicle(&mut self, kind: Kind, class: Class, colour: Colour, tier: Tier, at: P, h: f64) -> usize {
        let id = self.vehicles.len();
        self.vehicles.push(Vehicle {
            id,
            kind,
            class,
            colour,
            tier,
            town: None,
            x: at.0,
            z: at.1,
            h,
            s: 0.0,
            lights: kind != Kind::Parked,
            siren: false,
            spotlight: false,
            damage: Damage::default(),
            driver: None,
            riders: Vec::new(),
            owner: None,
            heat: Heat::default(),
            pose_at: 0.0,
            prev: at,
            junction: None,
            stopped_for: 0.0,
            flagged: false,
            horn: false,
        });
        id
    }

    /// A spot on the road near a point, facing along it.
    fn road_spot(map: &Map, p: P, r: f64) -> Option<(P, f64)> {
        let (seg, q, _) = map.nearest_road(p, r)?;
        let s = &map.segs[seg];
        let (dx, dz) = (s.b.0 - s.a.0, s.b.1 - s.a.1);
        let len = dx.hypot(dz).max(0.01);
        let lane = s.half * 0.45;
        Some(((q.0 - dz / len * lane, q.1 + dx / len * lane), heading(dx, dz)))
    }

    fn spawn(&mut self, now: f64) {
        let loader = self.loader.clone().unwrap();
        let map = loader.map.lock().unwrap();
        let contract = self.contract.clone().unwrap();
        let mut spots: Vec<(usize, P, f64, bool)> = Vec::new();
        for &t in &contract.towns {
            let town = TOWNS[t];
            let centre = self.frame.to_world(town.centre.into());
            spots.extend(
                parking_spots(&map, centre, town.radius, &mut self.rng)
                    .into_iter()
                    .map(|(p, h, drive)| (t, p, h, drive)),
            );
        }
        let gate = Self::road_spot(&map, GATE, 300.0).unwrap_or((GATE, 0.0));
        let precincts: Vec<(P, f64)> = PRECINCTS
            .iter()
            .map(|p| {
                let w = self.frame.to_world((*p).into());
                Self::road_spot(&map, w, 400.0).unwrap_or((w, 0.0))
            })
            .collect();
        let towns: Vec<P> = contract
            .towns
            .iter()
            .map(|&t| self.frame.to_world(TOWNS[t].centre.into()))
            .collect();
        drop(map);

        // The parked cars: a mix, with a handful that fit the contract in each town.
        let mut per_town: BTreeMap<usize, usize> = BTreeMap::new();
        for (town, at, h, _) in &spots {
            let fits = *per_town.get(town).unwrap_or(&0) < 6 && self.rng.random_bool(0.18);
            let class = if fits {
                contract.class
            } else {
                Class::PARKED[self.rng.random_range(0..4)]
            };
            let colour = if fits {
                *Colour::ALL
                    .iter()
                    .filter(|c| c.group() == contract.shade)
                    .nth(self.rng.random_range(0..2))
                    .unwrap_or(&Colour::Black)
            } else {
                Colour::ALL[self.rng.random_range(0..Colour::ALL.len())]
            };
            if contract.matches(class, colour) {
                *per_town.entry(*town).or_default() += 1;
            }
            let tier = Tier([1, 1, 2, 2, 3][self.rng.random_range(0..5)]);
            let id = self.vehicle(Kind::Parked, class, colour, tier, *at, *h);
            self.vehicles[id].town = Some(*town);
        }

        let van = self.vehicle(Kind::Crew, Class::Van, Colour::Grey, Tier(1), gate.0, gate.1);
        for (i, t) in self.thieves().iter().enumerate() {
            let p = self.players.get_mut(t).unwrap();
            p.place = Place::Seat {
                vehicle: van,
                driver: i == 0,
            };
            p.foot = Foot {
                x: gate.0.0,
                z: gate.0.1,
                ..Foot::default()
            };
            if i == 0 {
                self.vehicles[van].driver = Some(t.clone());
            } else {
                self.vehicles[van].riders.push(t.clone());
            }
        }
        for (i, c) in self.cop_players().iter().enumerate() {
            let (at, h) = precincts[i % precincts.len()];
            let at = (at.0 + (i / precincts.len()) as f64 * 9.0, at.1);
            let id = self.vehicle(Kind::Cruiser, Class::Cruiser, Colour::White, Tier(1), at, h);
            self.vehicles[id].owner = Some(c.clone());
            self.vehicles[id].driver = Some(c.clone());
            let p = self.players.get_mut(c).unwrap();
            p.place = Place::Seat {
                vehicle: id,
                driver: true,
            };
            p.foot = Foot {
                x: at.0,
                z: at.1,
                ..Foot::default()
            };
        }
        let map = loader.map.lock().unwrap();
        for i in 0..self.settings.difficulty.ai_cops() {
            let at = if i % 2 == 0 {
                precincts[i % precincts.len()].0
            } else {
                towns[i % towns.len()]
            };
            self.cops.push(Cop::new(format!("ai-{}", i + 1), &map, at));
        }
        drop(map);
        let minutes = self.settings.minutes as f64;
        self.phase = Phase::Running {
            started: now,
            ends: now + minutes * 60.0,
        };
        self.world_version += 1;
        self.say(Audience::All, What::Started);
        self.notice(Audience::Thieves, format!("The buyer wants a {}", contract.describe()));
        self.notice(Audience::Cops, "Night shift. Patrol and wait for reports");
    }

    fn vehicle_pose(&mut self, user: &str, id: usize, pose: VehiclePose, now: f64) -> Result<(), &'static str> {
        let v = self.vehicles.get_mut(id).ok_or("No such car")?;
        if v.driver.as_deref() != Some(user) {
            // A pose sent just before getting out lands after it.
            return Ok(());
        }
        let finite = [pose.x, pose.z, pose.h, pose.s].iter().all(|n| n.is_finite());
        let dt = (now - v.pose_at).clamp(0.05, 2.0);
        let cap = v.damage.handling().top_speed.max(4.0) * 1.3 + 6.0;
        let moved = dist((v.x, v.z), (pose.x, pose.z));
        if !finite || moved > cap * dt + MAX_JUMP {
            let (x, z, h) = (v.x, v.z, v.h);
            self.say(
                Audience::All,
                What::Correction {
                    vehicle: Some(id),
                    x,
                    z,
                    h,
                },
            );
            return Ok(());
        }
        v.prev = (v.x, v.z);
        v.x = pose.x;
        v.z = pose.z;
        v.h = pose.h;
        v.s = pose.s.clamp(-15.0, cap);
        v.lights = pose.lights;
        v.horn |= pose.horn;
        v.pose_at = now;
        self.stats.driven += moved;
        for p in self.players.values_mut() {
            if p.place
                == (Place::Seat {
                    vehicle: id,
                    driver: true,
                })
                || matches!(p.place, Place::Seat { vehicle, .. } if vehicle == id)
            {
                p.foot.x = pose.x;
                p.foot.z = pose.z;
            }
        }
        Ok(())
    }

    fn foot_pose(&mut self, user: &str, foot: Foot, now: f64) -> Result<(), &'static str> {
        let p = self.players.get_mut(user).ok_or("Join first")?;
        if p.place != Place::Foot || p.arrested {
            return Ok(());
        }
        let dt = (now - p.foot_at).clamp(0.05, 2.0);
        let moved = dist((p.foot.x, p.foot.z), (foot.x, foot.z));
        if ![foot.x, foot.z, foot.h, foot.s].iter().all(|n| n.is_finite()) || moved > 8.0 * dt + 3.0 {
            let (x, z, h) = (p.foot.x, p.foot.z, p.foot.h);
            self.say(Audience::All, What::Correction { vehicle: None, x, z, h });
            return Ok(());
        }
        p.foot = Foot {
            s: foot.s.clamp(0.0, 7.0),
            ..foot
        };
        p.foot_at = now;
        Ok(())
    }

    fn enter(&mut self, user: &str, id: usize, want_driver: bool) -> Result<(), &'static str> {
        let p = self.players.get(user).ok_or("Join first")?;
        if p.place != Place::Foot || p.arrested {
            return Err("You are not on foot");
        }
        let role = p.role;
        let at = (p.foot.x, p.foot.z);
        let v = self.vehicles.get(id).ok_or("No such car")?;
        if dist(at, (v.x, v.z)) > 5.0 {
            return Err("Get closer to the car");
        }
        let allowed = match (role, v.kind) {
            (Some(Role::Thief), Kind::Crew | Kind::Stolen) => true,
            (Some(Role::Cop), Kind::Cruiser) => v.owner.as_deref() == Some(user),
            _ => false,
        };
        if !allowed {
            return Err(if v.kind == Kind::Parked {
                "It is locked. Take it first"
            } else {
                "That car is not yours"
            });
        }
        let driver = v.driver.is_none() && (want_driver || v.riders.len() >= 3 || v.riders.is_empty());
        if !driver && v.riders.len() >= 3 {
            return Err("No seats left");
        }
        let v = &mut self.vehicles[id];
        if driver {
            v.driver = Some(user.to_string());
            v.pose_at = 0.0;
        } else {
            v.riders.push(user.to_string());
        }
        self.players.get_mut(user).unwrap().place = Place::Seat { vehicle: id, driver };
        Ok(())
    }

    fn exit(&mut self, user: &str) -> Result<(), &'static str> {
        let p = self.players.get(user).ok_or("Join first")?;
        let Place::Seat { vehicle, driver } = p.place else {
            return Ok(());
        };
        let v = &mut self.vehicles[vehicle];
        if driver {
            v.driver = None;
            v.s = 0.0;
        }
        v.riders.retain(|r| r != user);
        let side = if driver { -2.4 } else { 2.4 };
        let (x, z, h) = (v.x + v.h.cos() * side, v.z + v.h.sin() * side, v.h);
        let p = self.players.get_mut(user).unwrap();
        p.place = Place::Foot;
        p.foot = Foot {
            x,
            z,
            h,
            ..Foot::default()
        };
        p.foot_at = 0.0;
        self.say(Audience::All, What::Correction { vehicle: None, x, z, h });
        Ok(())
    }

    fn take_start(&mut self, user: &str, id: usize) -> Result<(), &'static str> {
        let p = self.players.get(user).ok_or("Join first")?;
        if p.role != Some(Role::Thief) || p.place != Place::Foot || p.arrested {
            return Err("Only a thief on foot can do that");
        }
        let v = self.vehicles.get(id).ok_or("No such car")?;
        if v.kind != Kind::Parked {
            return Err("That car is not parked");
        }
        if dist((p.foot.x, p.foot.z), (v.x, v.z)) > 3.5 {
            return Err("Get to the car door");
        }
        if self.take.as_ref().is_some_and(|t| t.thief != user) {
            return Err("Someone in the crew is already at a car");
        }
        let tier = v.tier;
        self.take = Some(Take::new(id, user, tier));
        Ok(())
    }

    fn take_choose(&mut self, user: &str, entry: Entry, now: f64) -> Result<(), &'static str> {
        let car = self
            .take
            .as_ref()
            .filter(|t| t.thief == user)
            .map(|t| t.car)
            .ok_or("You are not at a car")?;
        let town = self.vehicles[car].town;
        let odds = Odds {
            unlocked_bonus: town.map(|t| TOWNS[t].unlocked_bonus).unwrap_or(0.0) + 0.02,
        };
        let take = self.take.as_mut().unwrap();
        let out = take.choose(entry, now, odds, &mut self.rng);
        self.after_take(out, now);
        Ok(())
    }

    fn after_take(&mut self, out: Outcome, now: f64) {
        let Some(take) = self.take.clone() else { return };
        let car = take.car;
        let label = match out {
            Outcome::Opened => "open",
            Outcome::StillLocked => "locked",
            Outcome::LockStarted => "lock",
            Outcome::Hit => "hit",
            Outcome::Miss => "miss",
            Outcome::Smashed { .. } => "smashed",
            Outcome::SpareKey => "spare_key",
            Outcome::Started => "started",
            Outcome::Ignored => return,
        };
        self.say(Audience::Thieves, What::Take { result: label.into() });
        // A smash can also turn up a spare key, which reports as the key.
        if take.window_broken && !self.vehicles[car].damage.window_broken {
            self.vehicles[car].damage.break_window();
            if let Some(alarm) = take.alarm {
                self.say(
                    Audience::All,
                    What::Alarm {
                        vehicle: car,
                        until: alarm.until,
                    },
                );
            }
        }
        if matches!(out, Outcome::Miss) {
            let (x, z) = (self.vehicles[car].x, self.vehicles[car].z);
            self.noise(x, z);
        }
        if take.started() {
            self.car_taken(&take, now);
        }
    }

    fn car_taken(&mut self, take: &Take, now: f64) {
        let car = take.car;
        let v = &mut self.vehicles[car];
        v.kind = Kind::Stolen;
        v.lights = true;
        v.driver = Some(take.thief.clone());
        v.pose_at = 0.0;
        let at = (v.x, v.z);
        self.stolen_from = Some(at);
        if let Some(p) = self.players.get_mut(&take.thief) {
            p.place = Place::Seat {
                vehicle: car,
                driver: true,
            };
        }
        let soon = if self.witness_seen {
            20.0
        } else {
            REPORT_AFTER + self.rng.random_range(-30.0..30.0)
        };
        self.report_at = Some(match (self.report_at, take.witness_due()) {
            (Some(r), _) => r.min(now + soon),
            (None, Some(w)) => (w + 10.0).min(now + soon),
            (None, None) => now + soon,
        });
        self.take = None;
        self.world_version += 1;
        self.notice(Audience::Thieves, "It is running. Get it to the port");
    }

    fn assist(&self) -> f64 {
        let Some(take) = &self.take else { return 1.0 };
        let car = &self.vehicles[take.car];
        let lookouts = self
            .players
            .iter()
            .filter(|(id, p)| {
                **id != take.thief
                    && p.role == Some(Role::Thief)
                    && p.place == Place::Foot
                    && !p.arrested
                    && dist((p.foot.x, p.foot.z), (car.x, car.z)) <= take::LOOKOUT_RANGE
            })
            .count();
        take::assist(lookouts)
    }

    fn deploy_spike(&mut self, user: &str, now: f64) -> Result<(), &'static str> {
        if self.role(user) != Some(Role::Cop) {
            return Err("Only cops carry spikes");
        }
        let p = self.players.get(user).unwrap();
        if p.spikes_left == 0 {
            return Err("No spike strips left");
        }
        let (x, z, h) = match p.place {
            Place::Seat { vehicle, .. } => {
                let v = &self.vehicles[vehicle];
                (v.x - v.h.sin() * 6.0, v.z + v.h.cos() * 6.0, v.h)
            }
            Place::Foot => (p.foot.x, p.foot.z, p.foot.h),
        };
        let (rx, rz) = (h.cos() * 5.0, h.sin() * 5.0);
        let id = self.id();
        self.spikes.push(Spike {
            id,
            a: (x - rx, z - rz),
            b: (x + rx, z + rz),
            until: now + 120.0,
        });
        self.players.get_mut(user).unwrap().spikes_left -= 1;
        self.ping(PingKind::Radio, x, z, "Spikes down", now, Some(user.to_string()));
        Ok(())
    }

    fn ping_from(
        &mut self,
        user: &str,
        kind: PingKind,
        x: f64,
        z: f64,
        text: String,
        now: f64,
    ) -> Result<(), &'static str> {
        let role = self.role(user).ok_or("Pick a side first")?;
        let ok = match role {
            Role::Thief => matches!(kind, PingKind::Mark | PingKind::Pickup | PingKind::Danger),
            Role::Cop => kind == PingKind::Radio,
        };
        if !ok || !x.is_finite() || !z.is_finite() {
            return Err("You cannot send that");
        }
        let text: String = text.chars().take(40).collect();
        self.ping(kind, x, z, &text, now, Some(user.to_string()));
        Ok(())
    }

    fn ping(&mut self, kind: PingKind, x: f64, z: f64, text: &str, now: f64, by: Option<String>) {
        let id = self.id();
        self.pings.push(Ping {
            id,
            kind,
            x,
            z,
            text: text.to_string(),
            at: now,
            by,
        });
        if self.pings.len() > 24 {
            self.pings.remove(0);
        }
    }

    /// Sends the nearest AI cops to look at a spot.
    fn dispatch(&mut self, x: f64, z: f64, how_many: usize) {
        let mut order: Vec<usize> = (0..self.cops.len())
            .filter(|&i| !matches!(self.cops[i].mode, Mode::Chase | Mode::Block { .. }))
            .collect();
        order.sort_by(|a, b| {
            dist((self.cops[*a].x, self.cops[*a].z), (x, z))
                .total_cmp(&dist((self.cops[*b].x, self.cops[*b].z), (x, z)))
        });
        for i in order.into_iter().take(how_many) {
            self.cops[i].mode = Mode::Respond { x, z };
            self.cops[i].siren = false;
        }
    }

    fn noise(&mut self, x: f64, z: f64) {
        let houses = self.map().map(|m| m.buildings_near((x, z), 30.0).len()).unwrap_or(0);
        if houses > 0 {
            self.say(Audience::All, What::Bark { x, z });
            if self.rng.random_bool(0.25) {
                self.witness(x, z, 0.0);
            }
        }
    }

    fn witness(&mut self, x: f64, z: f64, now: f64) {
        if self.witness_seen && now > 0.0 && self.rng.random_bool(0.6) {
            return;
        }
        self.witness_seen = true;
        self.stats.witnesses += 1;
        let (ox, oz) = (self.rng.random_range(-90.0..90.0), self.rng.random_range(-90.0..90.0));
        self.ping(
            PingKind::Witness,
            x + ox,
            z + oz,
            "A neighbour reports someone at the cars",
            now,
            None,
        );
        self.dispatch(x + ox, z + oz, 2);
        self.notice(Audience::Thieves, "A light came on. Someone saw you");
        if self.vehicles.iter().any(|v| v.kind == Kind::Stolen) {
            self.report_at = Some(self.report_at.map_or(now + 20.0, |r| r.min(now + 20.0)));
        }
    }

    /// The car cops are after: the stolen one once it moves with the crew, else whatever the crew drives.
    fn target(&self) -> Option<usize> {
        let stolen = self.vehicles.iter().position(|v| v.kind == Kind::Stolen);
        let driven = self
            .vehicles
            .iter()
            .position(|v| matches!(v.kind, Kind::Crew | Kind::Stolen) && v.driver.is_some());
        match (stolen, driven) {
            (Some(s), Some(d)) if self.vehicles[d].kind == Kind::Crew && self.vehicles[s].driver.is_none() => Some(d),
            (Some(s), _) => Some(s),
            (None, d) => d,
        }
    }

    /// Every police position: AI cars, player cruisers with a driver, and cops on foot.
    fn police(&self) -> Vec<(P, f64, bool, f64)> {
        let mut out: Vec<(P, f64, bool, f64)> = self.cops.iter().map(|c| ((c.x, c.z), c.h, false, c.speed)).collect();
        for v in &self.vehicles {
            if v.kind == Kind::Cruiser && v.driver.is_some() {
                out.push(((v.x, v.z), v.h, v.spotlight, v.s));
            }
        }
        for p in self.players.values() {
            if p.role == Some(Role::Cop) && p.place == Place::Foot {
                out.push(((p.foot.x, p.foot.z), p.foot.h, p.foot.flash, p.foot.s));
            }
        }
        out
    }

    pub fn tick(&mut self, now: f64, dt: f64) {
        match self.phase {
            Phase::Loading { since } => {
                let ready = self.loader.as_ref().is_some_and(|l| l.settled());
                if ready || now - since > 25.0 {
                    self.spawn(now);
                }
            }
            Phase::Running { started, ends } => {
                self.run(now, dt, started);
                if now >= ends && self.running() {
                    self.end(Outcome2::Timeout, now, started);
                }
            }
            _ => {}
        }
    }

    fn run(&mut self, now: f64, dt: f64, started: f64) {
        self.pings.retain(|p| now - p.at < PING_FOR);
        self.spikes.retain(|s| now < s.until);
        if now >= self.want_at {
            self.want_at = now + 2.0;
            if let Some(l) = &self.loader {
                for v in self.vehicles.iter().filter(|v| v.driver.is_some()) {
                    l.want_around(v.x, v.z, 1);
                }
                for p in self
                    .players
                    .values()
                    .filter(|p| p.place == Place::Foot && p.role.is_some())
                {
                    l.want_around(p.foot.x, p.foot.z, 0);
                }
                for c in &self.cops {
                    l.want_around(c.x, c.z, 0);
                }
            }
            self.find_readers();
        }

        self.drive_heat(now, dt);
        self.take_tick(now);
        self.witnesses(dt, now);
        if let Some(at) = self.report_at
            && now >= at
            && !self.reported
        {
            self.report(now);
        }
        self.readers_check(now);
        self.police_tick(now, dt);
        self.spikes_check();
        self.contacts();
        self.flag_check(now, dt);
        self.arrests(now, dt, started);
        self.delivery(now, started);
    }

    fn drive_heat(&mut self, _now: f64, dt: f64) {
        let map = self.loader.clone();
        let map = map.as_ref().map(|l| l.map.lock().unwrap());
        for v in self.vehicles.iter_mut() {
            if v.driver.is_none() && v.kind != Kind::Cruiser {
                v.s *= 0.5;
            }
            v.damage.tick(dt, v.s.abs());
            if !matches!(v.kind, Kind::Crew | Kind::Stolen) {
                continue;
            }
            let (limit, junction) = map
                .as_ref()
                .and_then(|m| {
                    let (seg, _, _) = m.nearest_road((v.x, v.z), 25.0)?;
                    let s = &m.segs[seg];
                    let near = [s.from, s.to]
                        .into_iter()
                        .find(|n| m.edges[*n].len() >= 3 && dist(m.nodes[*n], (v.x, v.z)) < 7.0);
                    Some((s.class.limit(), near))
                })
                .unwrap_or((RoadClass::Minor.limit(), None));
            let ran = match junction {
                Some(n) if v.junction != Some(n) => {
                    v.junction = Some(n);
                    v.s > 14.0
                }
                Some(_) => false,
                None => {
                    v.junction = None;
                    false
                }
            };
            v.heat.step(
                heat::Sample {
                    speed: v.s.abs(),
                    limit,
                    lights: v.lights,
                    ran_junction: ran,
                    impact: 0.0,
                    horn: std::mem::take(&mut v.horn),
                    alarm: false,
                },
                dt,
            );
            self.stats.peak_heat = self.stats.peak_heat.max(v.heat.value);
        }
    }

    fn take_tick(&mut self, now: f64) {
        let assist = self.assist();
        let Some(take) = self.take.as_mut() else { return };
        let out = take.tick(now, assist);
        let car = take.car;
        let thief = take.thief.clone();
        let due = take.witness_due();
        if let Some(w) = due
            && now >= w
            && !self.witness_seen
        {
            let (x, z) = (self.vehicles[car].x, self.vehicles[car].z);
            self.ping(PingKind::Alarm, x, z, "A car alarm is going off", now, None);
            self.witness(x, z, now);
        }
        if self.players.get(&thief).is_none_or(|p| {
            p.arrested
                || p.place != Place::Foot
                || dist((p.foot.x, p.foot.z), (self.vehicles[car].x, self.vehicles[car].z)) > 6.0
        }) {
            self.take = None;
            return;
        }
        self.after_take(out, now);
        if now >= self.warn_at {
            self.warn_at = now + 2.0;
            let at = (self.vehicles[car].x, self.vehicles[car].z);
            let lookout = self.assist() > 1.0;
            if lookout
                && let Some(d) = self.police().iter().map(|c| dist(c.0, at)).min_by(f64::total_cmp)
                && d < LOOKOUT_WARN
            {
                self.notice(Audience::Thieves, format!("Lookout: police {} m away", d.round()));
            }
        }
    }

    fn witnesses(&mut self, dt: f64, now: f64) {
        let Some(loader) = self.loader.clone() else { return };
        let mut rolls: Vec<(f64, f64, f64, bool)> = Vec::new();
        {
            let map = loader.map.lock().unwrap();
            for (id, p) in &self.players {
                if p.role != Some(Role::Thief) || p.place != Place::Foot || p.arrested {
                    continue;
                }
                let at = (p.foot.x, p.foot.z);
                let houses = map.buildings_near(at, 25.0).len();
                if houses == 0 {
                    continue;
                }
                let dark = map.lamp_distance(at, 40.0) > 16.0;
                let stance = match p.foot.stance {
                    Stance::Run => 2.4,
                    Stance::Walk => 1.0,
                    Stance::Crouch => 0.35,
                };
                let taking = self
                    .take
                    .as_ref()
                    .filter(|t| t.thief == *id)
                    .map(|t| 1.5 + t.noise as f64 * 0.4)
                    .unwrap_or(1.0);
                let rate = 0.0035
                    * (houses.min(4) as f64 / 4.0)
                    * if dark { 0.45 } else { 1.6 }
                    * stance
                    * if p.foot.flash { 2.2 } else { 1.0 }
                    * taking;
                let near_door = map.buildings_near(at, 11.0).len() > 0;
                rolls.push((at.0, at.1, rate, near_door && p.foot.stance != Stance::Crouch));
            }
        }
        for (x, z, rate, porch) in rolls {
            if porch && self.rng.random_bool((0.25 * dt).min(1.0)) {
                self.say(Audience::All, What::PorchLight { x, z });
            }
            if self.rng.random_bool((rate * dt).min(1.0)) {
                self.witness(x, z, now);
            }
        }
        let barking: Vec<P> = self
            .players
            .values()
            .filter(|p| {
                p.role == Some(Role::Thief) && p.place == Place::Foot && p.foot.stance == Stance::Run && p.foot.s > 3.0
            })
            .map(|p| (p.foot.x, p.foot.z))
            .collect();
        for (x, z) in barking {
            if self.rng.random_bool((0.08 * dt).min(1.0)) {
                self.noise(x, z);
            }
        }
    }

    pub fn description(&self) -> Option<String> {
        let v = self.vehicles.iter().find(|v| v.kind == Kind::Stolen)?;
        let damaged = if v.damage.visibly_damaged() {
            ", visibly damaged"
        } else {
            ""
        };
        Some(format!("{} {}{}", v.colour.group().label(), v.class.label(), damaged))
    }

    fn report(&mut self, now: f64) {
        self.reported = true;
        let Some((x, z)) = self.stolen_from else { return };
        let (ox, oz) = (
            self.rng.random_range(-150.0..150.0),
            self.rng.random_range(-150.0..150.0),
        );
        let text = format!("Stolen: {}", self.description().unwrap_or_default());
        self.ping(PingKind::Report, x + ox, z + oz, &text, now, None);
        self.dispatch(x + ox, z + oz, 3);
        self.notice(Audience::Cops, text);
        self.notice(Audience::Thieves, "The owner called it in. Cops have a description");
    }

    fn find_readers(&mut self) {
        let Some(loader) = self.loader.clone() else { return };
        let map = loader.map.lock().unwrap();
        for n in &map.named[self.readers_from..] {
            let watched = WATCHED.iter().any(|w| n.refs.split(';').any(|r| r.trim() == *w));
            if !watched {
                continue;
            }
            let mid = ((n.a.0 + n.b.0) / 2.0, (n.a.1 + n.b.1) / 2.0);
            if self.readers.iter().all(|r| dist(*r, mid) > 1500.0) {
                self.readers.push(mid);
                self.world_version += 1;
            }
        }
        self.readers_from = map.named.len();
    }

    fn readers_check(&mut self, now: f64) {
        if !self.reported {
            return;
        }
        let Some(v) = self.vehicles.iter().find(|v| v.kind == Kind::Stolen && v.s.abs() > 3.0) else {
            return;
        };
        let (x, z, h) = (v.x, v.z, v.h);
        let hit = self.readers.iter().any(|r| dist(*r, (x, z)) < READER_RANGE);
        let recently = self
            .pings
            .iter()
            .any(|p| p.kind == PingKind::Reader && now - p.at < 20.0);
        if hit && !recently {
            self.stats.plate_hits += 1;
            let compass = [
                "north",
                "northeast",
                "east",
                "southeast",
                "south",
                "southwest",
                "west",
                "northwest",
            ];
            let dir =
                compass[((h.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * 8.0).round() as usize) % 8];
            self.ping(
                PingKind::Reader,
                x,
                z,
                &format!("Plate reader hit, heading {dir}"),
                now,
                None,
            );
            self.dispatch(x + h.sin() * 400.0, z - h.cos() * 400.0, 2);
        }
    }

    fn police_tick(&mut self, now: f64, dt: f64) {
        let Some(loader) = self.loader.clone() else { return };
        let target = self.target();
        let tpose = target.map(|t| {
            let v = &self.vehicles[t];
            Target {
                x: v.x,
                z: v.z,
                h: v.h,
                speed: v.s,
            }
        });
        let hot = target.is_some_and(|t| {
            let v = &self.vehicles[t];
            (self.reported && v.kind == Kind::Stolen)
                || v.heat.suspicious()
                || v.damage.visibly_damaged() && self.reported
        });
        let map = loader.map.lock().unwrap();
        let mut seen: Option<P> = None;
        if let Some(t) = tpose {
            for c in &self.cops {
                if c.sees(&map, (t.x, t.z), ai::SIGHT, ai::CONE) {
                    seen = Some((t.x, t.z));
                }
            }
            for (at, h, spot, _) in self.police().into_iter().skip(self.cops.len()) {
                let probe = Cop::new(String::new(), &Map::default(), at);
                let mut probe = probe;
                probe.x = at.0;
                probe.z = at.1;
                probe.h = h;
                let (range, cone) = if spot {
                    (SPOTLIGHT_SIGHT, SPOTLIGHT_CONE)
                } else {
                    (COP_SIGHT, ai::CONE)
                };
                if probe.sees(&map, (t.x, t.z), range, cone)
                    || (spot && probe.sees(&map, (t.x, t.z), COP_SIGHT, ai::CONE))
                {
                    seen = Some((t.x, t.z));
                }
            }
        }
        let started = self.pursuit.update(now, seen, hot);
        let lost = !self.pursuit.active();
        let mut events = Vec::new();
        if started {
            self.stats.pursuits += 1;
            events.push(What::Pursuit { started: true });
            if let Some(t) = tpose {
                let mut order: Vec<usize> = (0..self.cops.len()).collect();
                order.sort_by(|a, b| {
                    dist((self.cops[*a].x, self.cops[*a].z), (t.x, t.z))
                        .total_cmp(&dist((self.cops[*b].x, self.cops[*b].z), (t.x, t.z)))
                });
                for (rank, i) in order.into_iter().enumerate() {
                    let d = dist((self.cops[i].x, self.cops[i].z), (t.x, t.z));
                    if rank < 2 {
                        self.cops[i].mode = Mode::Chase;
                        self.cops[i].siren = true;
                    } else if d < 2500.0 {
                        self.cops[i].mode = Mode::Respond { x: t.x, z: t.z };
                        self.cops[i].siren = true;
                    }
                }
            }
        }
        if lost {
            let last = self.pursuit.last_known;
            for c in &mut self.cops {
                if c.mode == Mode::Chase || matches!(c.mode, Mode::Block { .. }) {
                    c.mode = match last {
                        Some((x, z)) if matches!(self.pursuit.phase, Chase::Search { .. }) => Mode::Respond { x, z },
                        _ => Mode::Patrol,
                    };
                    c.siren = matches!(self.pursuit.phase, Chase::Search { .. });
                }
            }
        }
        let chasing = self.pursuit.chasing();
        for c in &mut self.cops {
            let t = if c.mode == Mode::Chase { tpose } else { None };
            c.step(&map, dt, t, &mut self.rng);
        }
        if chasing && let Some(t) = tpose {
            // Spikes laid well ahead of the car, now and then.
            if now - self.spike_at > 45.0 && t.speed > 12.0 && self.spikes.len() < 3 {
                if let Some((a, b)) = ai::spike_ahead(&map, t, 380.0) {
                    self.spike_at = now;
                    let id = self.next_id + 1;
                    self.next_id = id;
                    self.spikes.push(Spike {
                        id,
                        a,
                        b,
                        until: now + 90.0,
                    });
                }
            }
            // Ramps blocked when the car is on a fast road.
            let fast = map
                .nearest_road((t.x, t.z), 20.0)
                .is_some_and(|(s, _, _)| map.segs[s].class.major());
            if fast && now - self.block_at > 90.0 {
                if let Some((p, h)) = ai::ramp_ahead(&map, t, 1500.0) {
                    self.block_at = now;
                    let mut free: Vec<usize> = (0..self.cops.len())
                        .filter(|&i| self.cops[i].mode != Mode::Chase)
                        .collect();
                    free.sort_by(|a, b| {
                        dist((self.cops[*a].x, self.cops[*a].z), p)
                            .total_cmp(&dist((self.cops[*b].x, self.cops[*b].z), p))
                    });
                    for (k, i) in free.into_iter().take(2).enumerate() {
                        let off = if k == 0 { -2.6 } else { 2.6 };
                        self.cops[i].mode = Mode::Block {
                            x: p.0 + h.sin() * off,
                            z: p.1 - h.cos() * off,
                            h,
                        };
                        self.cops[i].siren = true;
                    }
                    events.push(What::Notice {
                        text: "Dispatch: units blocking the ramp ahead".into(),
                    });
                }
            }
            // A spin-out from the rear quarter.
            if let Some(target) = target {
                for c in &mut self.cops {
                    if c.mode == Mode::Chase
                        && now >= c.pit_ready
                        && let Some(side) = c.pit_position(t)
                    {
                        c.pit_ready = now + 12.0;
                        self.stats.spin_outs += 1;
                        self.vehicles[target].damage.hit(side, -0.3, 11.0);
                        events.push(What::Spin { vehicle: target, side });
                    }
                }
            }
        }
        drop(map);
        if started && let Some(t) = tpose {
            self.ping(PingKind::Seen, t.x, t.z, "Units in pursuit", now, None);
        }
        for e in events {
            let to = match e {
                What::Notice { .. } => Audience::Cops,
                _ => Audience::All,
            };
            self.say(to, e);
        }
    }

    fn spikes_check(&mut self) {
        let mut hits = Vec::new();
        for v in self.vehicles.iter_mut() {
            if v.driver.is_none() || v.kind == Kind::Cruiser {
                continue;
            }
            for s in &self.spikes {
                if ai::crossed(v.prev, (v.x, v.z), (s.a, s.b)) {
                    v.damage.spike(true, v.s.abs() > 8.0);
                    hits.push(v.id);
                }
            }
            v.prev = (v.x, v.z);
        }
        for id in hits {
            self.stats.spikes_hit += 1;
            self.say(Audience::All, What::Spiked { vehicle: id });
        }
    }

    fn contacts(&mut self) {
        let Some(t) = self.target() else { return };
        let (tx, tz, th, ts) = {
            let v = &self.vehicles[t];
            (v.x, v.z, v.h, v.s)
        };
        let police: Vec<(P, f64, f64)> = self.police().into_iter().map(|(p, h, _, s)| (p, h, s)).collect();
        for (p, h, s) in police {
            let d = dist(p, (tx, tz));
            if d < 3.4 {
                let (vx, vz) = (th.sin() * ts - h.sin() * s, -th.cos() * ts + h.cos() * s);
                let closing = vx.hypot(vz);
                if closing > 5.0 {
                    let (dx, dz) = (p.0 - tx, p.1 - tz);
                    let (lx, lz) = (dx * th.cos() + dz * th.sin(), dx * th.sin() - dz * th.cos());
                    self.vehicles[t].damage.hit(lx, lz, closing * 0.5);
                    self.vehicles[t].heat.add(4.0);
                }
            }
        }
    }

    fn flag_check(&mut self, now: f64, dt: f64) {
        let mut flagged = Vec::new();
        for v in self.vehicles.iter_mut() {
            if !matches!(v.kind, Kind::Stolen | Kind::Crew) {
                continue;
            }
            if v.driver.is_none() && v.s.abs() < 0.5 {
                v.stopped_for += dt;
            } else {
                v.stopped_for = 0.0;
                v.flagged = false;
            }
            let hot = v.heat.value > 30.0 || (v.kind == Kind::Stolen && self.reported);
            if hot && v.stopped_for > 90.0 && !v.flagged {
                v.flagged = true;
                flagged.push((v.x, v.z));
            }
        }
        for (x, z) in flagged {
            self.ping(
                PingKind::Flagged,
                x,
                z,
                "A parked car matching a report was flagged",
                now,
                None,
            );
            self.dispatch(x, z, 2);
        }
    }

    fn arrests(&mut self, now: f64, dt: f64, started: f64) {
        let police = self.police();
        if let Some(t) = self.target() {
            let v = &self.vehicles[t];
            let close: Vec<f64> = police.iter().map(|c| dist(c.0, (v.x, v.z))).collect();
            let wanted = self.pursuit.active() || (self.reported && v.kind == Kind::Stolen);
            let boxed = wanted && v.driver.is_some() && arrest::boxed(v.s.abs(), &close);
            let disabled = arrest::disabled_with_cop(v.damage.disabled(), &close);
            if self.box_hold.step(boxed, dt, arrest::BOXED_FOR)
                || self.disabled_hold.step(disabled, dt, arrest::DISABLED_FOR)
            {
                let inside: Vec<String> = v.driver.iter().cloned().chain(v.riders.iter().cloned()).collect();
                for id in inside {
                    if let Some(p) = self.players.get_mut(&id) {
                        p.arrested = true;
                    }
                }
                self.end(Outcome2::Arrested, now, started);
                return;
            }
        }
        let cops_on_foot: Vec<P> = self
            .players
            .values()
            .filter(|p| p.role == Some(Role::Cop) && p.place == Place::Foot)
            .map(|p| (p.foot.x, p.foot.z))
            .collect();
        let mut caught = Vec::new();
        for (id, p) in self.players.iter_mut() {
            if p.role != Some(Role::Thief) || p.place != Place::Foot || p.arrested {
                continue;
            }
            let d: Vec<f64> = cops_on_foot.iter().map(|c| dist(*c, (p.foot.x, p.foot.z))).collect();
            if p.caught.step(arrest::caught(&d), dt, arrest::CAUGHT_FOR) {
                p.arrested = true;
                caught.push(id.clone());
            }
        }
        for id in caught {
            self.stats.foot_arrests += 1;
            self.say(Audience::All, What::Arrested { user_id: id });
        }
        let free = self
            .players
            .values()
            .filter(|p| p.role == Some(Role::Thief) && !p.arrested)
            .count();
        if free == 0 && self.count(Role::Thief) > 0 {
            self.end(Outcome2::Arrested, now, started);
        }
    }

    fn delivery(&mut self, now: f64, started: f64) {
        let contract = self.contract.clone();
        let at_gate = self
            .vehicles
            .iter()
            .find(|v| {
                v.kind == Kind::Stolen
                    && dist((v.x, v.z), GATE) <= GATE_RADIUS
                    && (v.driver.is_some() || !v.riders.is_empty())
            })
            .map(|v| (v.class, v.colour));
        if let (Some((class, colour)), Some(c)) = (at_gate, contract) {
            if c.matches(class, colour) {
                self.end(Outcome2::Delivered, now, started);
            } else if !self
                .events
                .iter()
                .any(|e| matches!(&e.what, What::Notice { text } if text.starts_with("The buyer")))
            {
                self.notice(Audience::Thieves, format!("The buyer only wants a {}", c.describe()));
            }
        }
    }

    fn end(&mut self, outcome: Outcome2, now: f64, started: f64) {
        let car = self.vehicles.iter().find(|v| v.kind == Kind::Stolen);
        let condition = car.map(|v| v.damage.condition()).unwrap_or(0.0);
        let total = self.settings.minutes as f64 * 60.0;
        let seconds = now - started;
        let payout = match (outcome, car) {
            (Outcome2::Delivered, Some(v)) => damage::payout(
                (v.class.value() as f64 * v.tier.value_factor()) as i64,
                condition,
                1.0 - seconds / total,
                self.stats.peak_heat,
            ),
            _ => 0,
        };
        let results = Results {
            outcome,
            payout,
            condition,
            seconds,
            stats: self.stats.clone(),
            thieves: self.thieves(),
            cops: self.cop_players(),
            car: self.description(),
        };
        self.phase = Phase::Ended { results };
        self.take = None;
        self.say(Audience::All, What::Ended { outcome });
    }

    /// The heat of the car cops would be after, for the crew's meter.
    pub fn crew_heat(&self) -> f64 {
        self.target().map(|t| self.vehicles[t].heat.value).unwrap_or(0.0)
    }

    pub fn handling(&self, vehicle: usize) -> Option<Handling> {
        self.vehicles.get(vehicle).map(|v| v.damage.handling())
    }

    pub fn time_left(&self, now: f64) -> f64 {
        match self.phase {
            Phase::Running { ends, .. } => (ends - now).max(0.0),
            _ => 0.0,
        }
    }

    pub fn loader_frame(&self) -> Frame {
        self.frame
    }

    pub fn port(&self) -> LatLon {
        PORT.into()
    }
}

/// Curbside and driveway spots along a town's side streets, next to houses.
pub fn parking_spots(map: &Map, centre: P, radius: f64, rng: &mut impl Rng) -> Vec<(P, f64, bool)> {
    let mut out: Vec<(P, f64, bool)> = Vec::new();
    for s in &map.segs {
        if s.class != RoadClass::Minor {
            continue;
        }
        let mid = ((s.a.0 + s.b.0) / 2.0, (s.a.1 + s.b.1) / 2.0);
        if dist(mid, centre) > radius {
            continue;
        }
        let len = dist(s.a, s.b);
        let (dx, dz) = ((s.b.0 - s.a.0) / len.max(0.01), (s.b.1 - s.a.1) / len.max(0.01));
        let mut t = rng.random_range(5.0..30.0);
        while t < len - 4.0 && out.len() < 90 {
            let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
            let on = (s.a.0 + dx * t, s.a.1 + dz * t);
            let curb = (on.0 - dz * (s.half + 1.4) * side, on.1 + dx * (s.half + 1.4) * side);
            let houses = map.buildings_near(curb, 22.0);
            if let Some(&b) = houses.first() {
                let house = map.buildings[b].centre;
                let driveway = rng.random_bool(0.45) && dist(house, curb) > 10.0;
                let (at, h, drive) = if driveway {
                    let toward = (
                        (house.0 - curb.0) / dist(house, curb),
                        (house.1 - curb.1) / dist(house, curb),
                    );
                    (
                        (curb.0 + toward.0 * 5.0, curb.1 + toward.1 * 5.0),
                        heading(toward.0, toward.1),
                        true,
                    )
                } else {
                    (curb, heading(dx * side, dz * side), false)
                };
                if out.iter().all(|o| dist(o.0, at) > 7.0) {
                    out.push((at, h, drive));
                }
            }
            t += rng.random_range(30.0..55.0);
        }
    }
    out
}

pub use rules::Difficulty;

#[cfg(test)]
mod tests {
    use super::super::mvt::{
        decode,
        testing::{V, tile},
    };
    use super::*;

    /// A street grid around the port origin, every 256 tile units, with a row of houses.
    fn heist() -> Heist {
        let mut h = Heist::new(true, 7);
        let (tx, ty) = h.frame.tile_at(0.0, 0.0);
        let mut roads = Vec::new();
        for i in 0..=16 {
            let c = (i * 256) as i64;
            let along: Vec<i64> = (0..=16).flat_map(|j| [c, j * 256]).collect();
            let across: Vec<i64> = (0..=16).flat_map(|j| [j * 256, c]).collect();
            roads.push((2, vec![("class", V::S("minor"))], vec![along]));
            roads.push((2, vec![("class", V::S("minor"))], vec![across]));
        }
        let houses: Vec<(u32, Vec<(&str, V)>, Vec<Vec<i64>>)> = (0..10)
            .map(|k| {
                let x = 40 + k * 22;
                (3, vec![], vec![vec![x, 280, x + 12, 280, x + 12, 296, x, 296, x, 280]])
            })
            .collect();
        let buf = tile(&[("transportation", roads), ("building", houses)]);
        let mut map = Map::default();
        map.add_tile(&h.frame, tx, ty, &decode(&buf, &[]));
        h.loader = Some(Loader::offline(h.frame, map));
        for (id, role) in [("ana", Role::Thief), ("ben", Role::Thief), ("cop", Role::Cop)] {
            h.join(id, id);
            h.players.get_mut(id).unwrap().role = Some(role);
        }
        h.contract = Some(Contract {
            class: Class::Suv,
            shade: rules::Shade::Dark,
            towns: vec![0, 1],
        });
        h.spawn(0.0);
        h.cops.clear();
        h
    }

    fn parked(h: &mut Heist, at: P, class: Class, colour: Colour) -> usize {
        h.vehicle(Kind::Parked, class, colour, Tier(1), at, 0.0)
    }

    fn step(h: &mut Heist, from: f64, secs: f64) -> f64 {
        let mut t = from;
        while t < from + secs {
            t += 0.1;
            h.tick(t, 0.1);
        }
        t
    }

    /// Puts someone on foot at a spot, as if they had walked there.
    fn walk(h: &mut Heist, who: &str, at: P, _now: f64) {
        let p = h.players.get_mut(who).unwrap();
        p.foot.x = at.0;
        p.foot.z = at.1;
    }

    #[test]
    fn a_step_too_far_on_foot_is_refused() {
        let mut h = heist();
        h.handle("ben", false, Msg::Exit, 1.0).unwrap();
        let start = (h.players["ben"].foot.x, h.players["ben"].foot.z);
        h.handle(
            "ben",
            false,
            Msg::Foot {
                foot: Foot {
                    x: start.0 + 1.0,
                    z: start.1,
                    s: 1.5,
                    ..Foot::default()
                },
            },
            1.5,
        )
        .unwrap();
        assert_eq!(h.players["ben"].foot.x, start.0 + 1.0);
        h.handle(
            "ben",
            false,
            Msg::Foot {
                foot: Foot {
                    x: start.0 + 90.0,
                    z: start.1,
                    ..Foot::default()
                },
            },
            2.0,
        )
        .unwrap();
        assert_eq!(h.players["ben"].foot.x, start.0 + 1.0);
    }

    fn outcome(h: &Heist) -> Option<Outcome2> {
        match &h.phase {
            Phase::Ended { results } => Some(results.outcome),
            _ => None,
        }
    }

    #[test]
    fn the_crew_starts_together_in_one_car_and_each_cop_in_a_cruiser() {
        let h = heist();
        let van = h.vehicles.iter().find(|v| v.kind == Kind::Crew).unwrap();
        assert_eq!(van.driver.as_deref(), Some("ana"));
        assert_eq!(van.riders, vec!["ben".to_string()]);
        let cruiser = h.vehicles.iter().find(|v| v.kind == Kind::Cruiser).unwrap();
        assert_eq!(cruiser.owner.as_deref(), Some("cop"));
        assert!(h.running());
    }

    #[test]
    fn a_smash_that_turns_up_a_spare_key_still_shows_on_the_car() {
        let mut found = false;
        for seed in 0..2000 {
            let mut h = heist();
            h.rng = StdRng::seed_from_u64(seed);
            let car = parked(&mut h, (60.0, 60.0), Class::Suv, Colour::Black);
            h.handle("ben", false, Msg::Exit, 1.0).unwrap();
            walk(&mut h, "ben", (61.0, 60.0), 1.0);
            h.handle("ben", false, Msg::TakeStart { vehicle: car }, 1.0).unwrap();
            h.handle("ben", false, Msg::TakeChoose { entry: Entry::Window }, 1.0)
                .unwrap();
            assert!(h.vehicles[car].damage.window_broken, "seed {seed}");
            if h.vehicles[car].kind == Kind::Stolen {
                found = true;
                break;
            }
        }
        assert!(found, "no seed turned up a spare key");
    }

    #[test]
    fn a_take_on_foot_then_a_report_and_a_pickup_by_the_crew_car() {
        let mut h = heist();
        let car = parked(&mut h, (60.0, 60.0), Class::Suv, Colour::Black);
        let mut now = 1.0;
        h.handle("ben", false, Msg::Exit, now).unwrap();
        walk(&mut h, "ben", (61.0, 60.0), now);
        assert!(
            h.handle(
                "ben",
                false,
                Msg::Enter {
                    vehicle: car,
                    driver: true
                },
                now
            )
            .is_err(),
            "locked"
        );
        h.handle("ben", false, Msg::TakeStart { vehicle: car }, now).unwrap();
        h.handle("ben", false, Msg::TakeChoose { entry: Entry::Window }, now)
            .unwrap();
        assert!(h.vehicles[car].damage.window_broken);
        // Play the start: hold below the band, let go above it.
        let mut t = 0.0;
        while h.take.is_some() && t < 40.0 {
            let hold = match &h.take.as_ref().unwrap().stage {
                take::Stage::Ignition { meter, .. } => meter.needle < meter.centre(t),
                _ => false,
            };
            h.handle("ben", false, Msg::TakeHold { t, hold }, now + t).unwrap();
            t += 0.05;
        }
        now += t;
        assert_eq!(h.vehicles[car].kind, Kind::Stolen);
        assert_eq!(h.vehicles[car].driver.as_deref(), Some("ben"));
        assert!(h.report_at.is_some());
        while !h.reported && now < 400.0 {
            now = step(&mut h, now, 0.5);
        }
        assert!(h.reported);
        assert!(h.description().unwrap().contains("visibly damaged"));
        assert!(h.pings.iter().any(|p| p.kind == PingKind::Report));

        // The van picks a walker up: out, walk over, in as a rider.
        let van = h.vehicles.iter().position(|v| v.kind == Kind::Crew).unwrap();
        let (vx, vz) = (h.vehicles[van].x, h.vehicles[van].z);
        h.handle("ben", false, Msg::Exit, now).unwrap();
        walk(&mut h, "ben", (vx + 1.0, vz), now + 1.0);
        h.handle(
            "ben",
            false,
            Msg::Enter {
                vehicle: van,
                driver: false,
            },
            now + 1.0,
        )
        .unwrap();
        assert_eq!(
            h.players["ben"].place,
            Place::Seat {
                vehicle: van,
                driver: false
            }
        );
    }

    #[test]
    fn a_matching_stolen_car_at_the_gate_is_delivered_and_paid_by_condition() {
        let mut h = heist();
        let car = parked(&mut h, GATE, Class::Suv, Colour::Navy);
        h.vehicles[car].kind = Kind::Stolen;
        h.vehicles[car].driver = Some("ana".into());
        h.vehicles[car].damage.hit(0.0, 1.0, 20.0);
        step(&mut h, 1.0, 0.5);
        assert_eq!(outcome(&h), Some(Outcome2::Delivered));
        let Phase::Ended { results } = &h.phase else { panic!() };
        assert!(results.payout > 0 && results.payout < 14_000, "{}", results.payout);
        assert_eq!(results.winners(), &["ana".to_string(), "ben".to_string()]);
    }

    #[test]
    fn the_wrong_car_at_the_gate_is_turned_away() {
        let mut h = heist();
        let car = parked(&mut h, GATE, Class::Hatch, Colour::White);
        h.vehicles[car].kind = Kind::Stolen;
        h.vehicles[car].driver = Some("ana".into());
        step(&mut h, 1.0, 1.0);
        assert_eq!(outcome(&h), None);
    }

    #[test]
    fn a_reported_car_boxed_in_by_a_cop_is_an_arrest() {
        let mut h = heist();
        let car = parked(&mut h, (300.0, 300.0), Class::Suv, Colour::Black);
        h.vehicles[car].kind = Kind::Stolen;
        h.vehicles[car].driver = Some("ana".into());
        h.reported = true;
        let cruiser = h.vehicles.iter().position(|v| v.kind == Kind::Cruiser).unwrap();
        h.vehicles[cruiser].x = 304.0;
        h.vehicles[cruiser].z = 300.0;
        let t = step(&mut h, 1.0, 3.0);
        assert_eq!(outcome(&h), None, "not yet");
        step(&mut h, t, 2.5);
        assert_eq!(outcome(&h), Some(Outcome2::Arrested));
        assert!(h.players["ana"].arrested);
    }

    #[test]
    fn a_cop_on_foot_catches_a_thief_on_foot_and_the_last_catch_ends_it() {
        let mut h = heist();
        let mut now = 1.0;
        for who in ["ana", "ben", "cop"] {
            h.handle(who, false, Msg::Exit, now).unwrap();
        }
        walk(&mut h, "ana", (500.0, 500.0), now);
        walk(&mut h, "ben", (900.0, 900.0), now);
        walk(&mut h, "cop", (501.0, 500.0), now);
        now = step(&mut h, now, 3.2);
        assert!(h.players["ana"].arrested && !h.players["ben"].arrested);
        assert_eq!(outcome(&h), None);
        walk(&mut h, "cop", (700.0, 700.0), now);
        walk(&mut h, "cop", (899.0, 900.0), now + 40.0);
        step(&mut h, now + 40.0, 3.2);
        assert_eq!(outcome(&h), Some(Outcome2::Arrested));
    }

    #[test]
    fn the_clock_running_out_is_a_win_for_the_cops() {
        let mut h = heist();
        let ends = match h.phase {
            Phase::Running { ends, .. } => ends,
            _ => panic!(),
        };
        step(&mut h, ends - 0.5, 1.0);
        assert_eq!(outcome(&h), Some(Outcome2::Timeout));
    }

    #[test]
    fn teleporting_poses_are_refused() {
        let mut h = heist();
        let van = h.vehicles.iter().position(|v| v.kind == Kind::Crew).unwrap();
        let (x, z) = (h.vehicles[van].x, h.vehicles[van].z);
        h.handle(
            "ana",
            false,
            Msg::Vehicle {
                id: van,
                pose: VehiclePose {
                    x: x + 2000.0,
                    z,
                    h: 0.0,
                    s: 10.0,
                    ..VehiclePose::default()
                },
            },
            1.0,
        )
        .unwrap();
        assert_eq!(h.vehicles[van].x, x);
        h.handle(
            "ben",
            false,
            Msg::Vehicle {
                id: van,
                pose: VehiclePose {
                    x: x + 3.0,
                    z,
                    ..VehiclePose::default()
                },
            },
            1.0,
        )
        .unwrap();
        assert_eq!(h.vehicles[van].x, x, "riders do not drive");
    }
}
