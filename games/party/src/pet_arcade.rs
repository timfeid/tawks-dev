//! Ten short contests where a player's companion performs the scored action.
//! The room fixes pet species and body build before the bout starts. A pet's
//! species changes its cue and animation; every species has the same scoring
//! windows and cooldowns, including pets adopted through the store.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use crate::ground::{RUNNER_RADIUS, Runner, step_runner};

pub const KEYS: [&str; 10] = [
    "pet_fetch",
    "pet_scent",
    "pet_herd",
    "pet_choir",
    "pet_garden",
    "pet_bubbles",
    "pet_rescue",
    "pet_dig",
    "pet_clouds",
    "pet_parade",
];
pub const NAMES: [&str; 10] = [
    "Fetch Festival",
    "Scent Seekers",
    "Woolly Whistle",
    "Companion Choir",
    "Pocket Garden",
    "Bubble Patrol",
    "Nest Rescue",
    "Dig Detectives",
    "Cloudbound",
    "Pet Parade",
];
pub const RULES: [&str; 10] = [
    "Touch a ball to carry it. Aim a shot at the center basket, or dive toward a carrier to knock theirs loose. Broad pets carry faster in single-ball rounds.",
    "Guide your pet by its scent pulse and sniff out hidden snacks. Broad pets sense farther; small pets search faster.",
    "Get behind a sheep and call to push it into the center pen. Broad pets earn more for each pen, while small pets call again sooner.",
    "Choose the highlighted note and let your pet sing on the beat.",
    "Plant your beds, touch the center well for water, then return to water and harvest. The first pet to claim golden water earns a bonus on its next bloom. Broad pets carry more and tend farther; small pets grow blooms sooner.",
    "Choose the requested color and let your pet pop a drifting bubble. Broad pets reach farther; small pets recover faster in crowds.",
    "Your pet carries a lost hatchling back to the center nest. Broad pets slow down less while carrying.",
    "Use your pet's sniff meter, then dig the right tile of the treasure grid. Broad pets sense farther and dig faster.",
    "Time a leap near a drifting cloud hoop to send your pet through it.",
    "Move your pet in the shown direction, then call while its feet are stepping that way. Small pets earn a downbeat point for an early call every fourth cue.",
];
pub const LIMIT: f32 = 65.0;
const ACTION_CD: f32 = 0.65;
const ARENA: f32 = 11.0;
fn sniff_range(width: f32) -> f32 {
    16.0 * BodyProfile::new(width).reach.powi(3)
}
// Duels leave room for a broad pet's full reach; crowded lanes favor quick repeat pops.
fn bubble_reach(width: f32, seats: usize) -> f32 {
    let reach = 1.0 + (BodyProfile::new(width).width - 1.0) * 0.12;
    if seats <= 2 { reach * reach } else { reach }
}
fn bubble_cooldown(width: f32, seats: usize) -> f32 {
    if seats <= 2 {
        ACTION_CD
    } else {
        0.635 + (BodyProfile::new(width).width - 0.7) / 0.7 * 0.03
    }
}
fn scent_crowd_agility(width: f32, seats: usize) -> f32 {
    let compactness = (1.4 - BodyProfile::new(width).width) / 0.7;
    // Compact pets keep some search agility even in a duel.
    let crowd = (seats.saturating_sub(2) as f32 / 6.0).min(1.0).sqrt().max(0.5);
    1.0 + compactness * crowd * 0.16
}
fn rescue_carry_speed(width: f32) -> f32 {
    let size = (BodyProfile::new(width).width - 0.7) / 0.7;
    0.72 + size * 0.18
}
fn fetch_carry_speed(width: f32, balls: usize) -> f32 {
    if balls > 1 {
        0.88
    } else {
        let size = (BodyProfile::new(width).width - 0.7) / 0.7;
        0.84 + size * 0.08
    }
}
fn garden_capacity(width: f32) -> u8 {
    if width >= 1.1 { 2 } else { 1 }
}
fn garden_tend_range(width: f32) -> f32 {
    let size = (BodyProfile::new(width).width - 0.7) / 0.7;
    2.0 + size * 2.0
}
fn garden_growth_times(width: f32) -> (f32, f32) {
    let size = (BodyProfile::new(width).width - 0.7) / 0.7;
    (2.6 + size * 0.4, 2.2 + size * 0.3)
}
fn dig_cooldown(width: f32) -> f32 {
    let size = (BodyProfile::new(width).width - 0.7) / 0.7;
    0.65 - size * 0.15
}
pub fn parade_step_threshold(width: f32) -> f32 {
    1.15 - (1.4 - BodyProfile::new(width).width) / 0.7 * 0.18
}
fn bark_profile(width: f32) -> (f32, f32) {
    let size = (width.clamp(0.7, 1.4) - 0.7) / 0.7;
    (10.5 + size * 3.5, 0.50 + size * 0.30)
}
fn cloud_leap_profile(width: f32) -> (f32, f32, f32) {
    let size = (width.clamp(0.7, 1.4) - 0.7) / 0.7;
    (1.35 - size * 0.35, 1.7 + size * 0.5, 0.72)
}
pub fn choir_window(width: f32) -> (f32, f32) {
    let spread = (width.clamp(0.7, 1.4)-0.7)/0.7;
    let half = 0.26 + spread*0.10;
    (0.5-half, 0.5+half)
}

#[derive(Clone, Debug)]
pub struct Player {
    seat: u8,
    runner: Runner,
    width: f32,
    pet: u8,
    skill: f32,
    score: i32,
    combo: u8,
    last_scored_beat: Option<usize>,
    cooldown: f32,
    pounce_for: f32,
    pounce_cooldown: f32,
    impact_lock: f32,
    leap_for: f32,
    leap_hit: bool,
    aim_yaw: f32,
    facing_x: f32,
    facing_z: f32,
    streak_left: f32,
    holding_toy: Option<usize>,
    carrying: bool,
    carrying_chick: Option<usize>,
    water: u8,
    golden_water: bool,
    avatar_x: f32,
    avatar_z: f32,
    pet_x: f32,
    pet_z: f32,
    pet_action: f32,
    action_n: u32,
    scent_waypoint: usize,
}
#[derive(Clone, Debug)]
pub struct Prop {
    x: f32,
    z: f32,
    owner: usize,
    state: u8,
    timer: f32,
    phase: f32,
    vx: f32,
    vz: f32,
}

pub struct PetArcade {
    kind: usize,
    seed: u64,
    players: Vec<Player>,
    props: Vec<Prop>,
    time: f32,
    beat: usize,
    target: usize,
    target_color: u8,
    secret: usize,
    dug: u16,
    golden_water_at: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn move_toward(from: (f32, f32), to: (f32, f32)) -> Stick {
    let dx = to.0 - from.0;
    let dz = to.1 - from.1;
    let length = dx.hypot(dz).max(0.001);
    Stick {
        x: dx / length,
        z: dz / length,
    }
}
fn circle(index: usize, count: usize, radius: f32) -> (f32, f32) {
    let angle = index as f32 * std::f32::consts::TAU / count as f32;
    (angle.cos() * radius, angle.sin() * radius)
}
/// Hide a cache away from pets and other caches so the round starts with a search.
fn scent_spot(rng: &mut StdRng, pets: &[(f32, f32)], caches: &[(f32, f32)]) -> (f32, f32) {
    for _ in 0..96 {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let radius = rng.random_range(2.4..8.5);
        let spot = (angle.cos() * radius, angle.sin() * radius);
        if pets.iter().all(|&pet| distance(spot, pet) >= 2.7)
            && caches.iter().all(|&cache| distance(spot, cache) >= 2.1) {
            return spot;
        }
    }
    (0.0, 0.0)
}
fn scent_waypoint(index: usize) -> (f32, f32) {
    let row = index / 9 % 9;
    let column = index % 9;
    let column = if row % 2 == 0 { column } else { 8 - column };
    (-8.0 + column as f32 * 2.0, -8.0 + row as f32 * 2.0)
}
fn bot_reacts(seat: u8, tick: u32, seed: u64, skill: f32) -> bool {
    let mut hash = tick.wrapping_mul(0x9e37_79b9)
        ^ u32::from(seat).wrapping_mul(0x85eb_ca6b)
        ^ (seed as u32).wrapping_mul(0xc2b2_ae35)
        ^ ((seed >> 32) as u32).rotate_left(13);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb_352d);
    hash ^= hash >> 15;
    ((hash % 100) as f32) < 37.0 + skill * 49.0
}
impl PetArcade {
    pub fn new(kind: usize, contestants: &[Contestant], seed: u64) -> Self {
        assert!(kind < KEYS.len());
        let mut rng = StdRng::seed_from_u64(seed);
        let players: Vec<Player> = contestants
            .iter()
            .enumerate()
            .map(|(index, c)| {
                let body = BodyProfile::new(c.body_width);
                let (x, z) = circle(
                    index,
                    contestants.len().max(1),
                    if kind == 9 { 3.5 } else { 7.4 },
                );
                let avatar_x = (index as f32 - (contestants.len() as f32 - 1.0) * 0.5) * 2.8;
                let avatar_z = -13.3;
                Player {
                    seat: c.seat,
                    runner: Runner::at(x, z).with_width(body.width),
                    width: body.width,
                    pet: c.pet.min(4),
                    skill: c.skill.clamp(0.0, 1.0),
                    score: 0,
                    combo: 0,
                    last_scored_beat: None,
                    cooldown: 0.0,
                    pounce_for: 0.0,
                    pounce_cooldown: 0.0,
                    impact_lock: 0.0,
                    leap_for: 0.0,
                    leap_hit: false,
                    aim_yaw: 0.0,
                    facing_x: 0.0,
                    facing_z: -1.0,
                    streak_left: 0.0,
                    holding_toy: None,
                    carrying: false,
                    carrying_chick: None,
                    water: 0,
                    golden_water: false,
                    avatar_x,
                    avatar_z,
                    pet_x: x,
                    pet_z: z,
                    pet_action: 0.0,
                    action_n: 0,
                    scent_waypoint: index * 17 % 81,
                }
            })
            .collect();
        let seats = contestants.len().max(2);
        let count = match kind {
            0 => (contestants.len() / 2).max(1),
            1 => 8,
            2 => 9,
            3 => 4,
            4 => seats * 2,
            5 => seats * 3,
            6 => seats * 3,
            7 => 16,
            8 => seats * 2,
            _ => 4,
        };
        let pet_starts = players.iter().map(|p| (p.runner.x, p.runner.z)).collect::<Vec<_>>();
        let mut cache_spots = Vec::new();
        let props = (0..count)
            .map(|i| {
                let (x, z) = if kind == 7 {
                    (((i % 4) as f32 - 1.5) * 4.1, ((i / 4) as f32 - 1.5) * 4.1)
                } else if kind == 1 {
                    let spot = scent_spot(&mut rng, &pet_starts, &cache_spots);
                    cache_spots.push(spot);
                    spot
                } else if kind == 4 {
                    let angle = i as f32 * std::f32::consts::TAU / count as f32
                        + rng.random_range(-0.12..0.12);
                    let radius = 6.8 + rng.random_range(-0.6..0.6);
                    (angle.cos() * radius, angle.sin() * radius)
                } else if kind == 3 {
                    circle((i + 3) % 4, 4, 5.2)
                } else {
                    circle(i, count, if kind == 3 || kind == 9 { 5.2 } else { 6.8 })
                };
                let drift = rng.random_range(0.0..std::f32::consts::TAU);
                Prop {
                    x,
                    z,
                    owner: if kind == 4 { i / 2 % contestants.len().max(1) }
                        else { i % contestants.len().max(1) },
                    state: if kind == 5 {
                        ((i / seats) % 3) as u8
                    } else {
                        0
                    },
                    timer: 0.0,
                    phase: rng.random_range(0.0..6.28),
                    vx: if kind == 0 { drift.cos() * 3.2 } else { 0.0 },
                    vz: if kind == 0 { drift.sin() * 3.2 } else { 0.0 },
                }
            })
            .collect();
        Self {
            kind,
            seed,
            players,
            props,
            time: 0.0,
            beat: 0,
            target: 0,
            target_color: 0,
            secret: rng.random_range(0..16),
            dug: 0,
            golden_water_at: 5.0,
            rng,
            result: None,
        }
    }
    fn nearest(
        &self,
        index: usize,
        range: f32,
        predicate: impl Fn(&Prop) -> bool,
    ) -> Option<usize> {
        let at = (self.players[index].runner.x, self.players[index].runner.z);
        self.props
            .iter()
            .enumerate()
            .filter(|(_, p)| predicate(p))
            .filter_map(|(i, p)| {
                let d = distance(at, (p.x, p.z));
                (d <= range).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    fn basket(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn award(&mut self, index: usize, points: i32, prop: Option<usize>, events: &mut Vec<Value>) {
        let player = &mut self.players[index];
        player.score += points;
        player.combo = if points > 0 {
            player.combo.saturating_add(1)
        } else {
            0
        };
        if !(self.kind == 8 && player.leap_for > 0.0) {
            player.pet_action = 0.8;
            player.action_n = player.action_n.wrapping_add(1);
        }
        events.push(
            json!({"kind":"pet_score", "seat":player.seat, "pet":player.pet,
            "points":points, "score":player.score, "mode":KEYS[self.kind], "prop":prop}),
        );
    }
    fn act(&mut self, index: usize, input: Input, events: &mut Vec<Value>) {
        if self.kind == 0 {
            let p = &mut self.players[index];
            if p.impact_lock > 0.0 {
                return;
            }
            if let Some(toy) = p.holding_toy.take() {
                let yaw = if input.yaw.is_finite() {
                    input.yaw
                } else {
                    p.aim_yaw
                };
                let (dx, dz) = (yaw.sin(), yaw.cos());
                let prop = &mut self.props[toy];
                prop.x = p.runner.x + dx * 0.8;
                prop.z = p.runner.z + dz * 0.8;
                prop.vx = dx * 17.0;
                prop.vz = dz * 17.0;
                prop.state = 3;
                prop.timer = 1.2;
                prop.phase = 0.0;
                p.pet_action = 0.55;
                p.action_n = p.action_n.wrapping_add(1);
                events.push(
                    json!({"kind":"pet_fetch_shot","seat":p.seat,"prop":toy,"x":prop.x,"z":prop.z}),
                );
                return;
            }
            if p.pounce_cooldown <= 0.0 {
                let stick = input.stick.clamped();
                let length = stick.x.hypot(stick.z);
                let (dx, dz) = if length > 0.15 {
                    (stick.x / length, stick.z / length)
                } else {
                    (p.facing_x, p.facing_z)
                };
                p.facing_x = dx;
                p.facing_z = dz;
                p.runner.vx = dx * 15.0;
                p.runner.vz = dz * 15.0;
                p.pounce_for = 0.32;
                p.pounce_cooldown = 3.0;
                p.pet_action = 0.48;
                p.action_n = p.action_n.wrapping_add(1);
                events.push(
                    json!({"kind":"pet_fetch_pounce","seat":p.seat,"x":p.runner.x,"z":p.runner.z}),
                );
            }
            return;
        }
        if self.players[index].cooldown > 0.0 {
            return;
        }
        if matches!(self.kind, 3 | 9)
            && self.players[index].last_scored_beat == Some(self.beat) {
            return;
        }
        let at = (self.players[index].runner.x, self.players[index].runner.z);
        let reach = 1.0 + (self.players[index].width.clamp(0.7, 1.4) - 1.0) * 0.12;
        self.players[index].cooldown = if self.kind == 3 {
            0.35 + (self.players[index].width.clamp(0.7,1.4)-0.7)/0.7*0.18
        } else if self.kind == 2 {
            bark_profile(self.players[index].width).1
        } else if self.kind == 5 {
            bubble_cooldown(self.players[index].width, self.players.len())
        } else if self.kind == 7 {
            dig_cooldown(self.players[index].width)
        } else if self.kind == 8 {
            cloud_leap_profile(self.players[index].width).2
        } else if self.kind == 9 {
            0.53 + (self.players[index].width.clamp(0.7,1.4)-0.7)/0.7*0.10
        } else { ACTION_CD };
        match self.kind {
            0 => unreachable!(),
            1 => {
                // Scent: the clue is a distance pulse, not the cache position.
                if let Some(i) = self.nearest(index, 1.5 * reach, |p| p.timer <= 0.0) {
                    self.props[i].timer = 2.2;
                    self.award(index, 3, Some(i), events);
                }
            }
            2 => {
                // The sheep runs away from the calling pet. Position matters.
                if let Some(i) = self.nearest(index, 3.3 * reach, |p| p.timer <= 0.0) {
                    let bark_speed = bark_profile(self.players[index].width).0;
                    let p = &mut self.props[i];
                    let dx = p.x - at.0;
                    let dz = p.z - at.1;
                    let d = dx.hypot(dz).max(0.1);
                    p.vx = dx / d * bark_speed;
                    p.vz = dz / d * bark_speed;
                    p.owner = index;
                    p.state = 1;
                    p.timer = 0.8;
                    self.players[index].pet_action = 0.5;
                    self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                    events.push(
                        json!({"kind":"pet_herd_bark","seat":self.players[index].seat,
                            "prop":i,"x":at.0,"z":at.1,"power":bark_speed}),
                    );
                }
            }
            3 => {
                // Choir: note selection and beat timing, with escalating phrases.
                let note = input.yaw.round() as isize;
                let target = (self.beat + self.target) % 4;
                let phase = (self.time * 1.5).fract();
                let (early,late) = choir_window(self.players[index].width);
                if note == target as isize && (early..=late).contains(&phase) {
                    self.award(
                        index,
                        1 + i32::from(self.players[index].combo.min(3)),
                        Some(target),
                        events,
                    );
                    self.players[index].last_scored_beat = Some(self.beat);
                } else {
                    self.players[index].combo = 0;
                }
            }
            4 => {
                // Garden: the pet carries water from the well to its own beds.
                let i = self
                    .props
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.owner == index)
                    .filter(|(_,p)|p.state==0 || p.timer<=0.0
                        && (p.state==2 || self.players[index].water>0))
                    .filter_map(|(i, p)| {
                        let gap=distance(at,(p.x,p.z));
                        (gap <= garden_tend_range(self.players[index].width)).then_some((i,gap))
                    })
                    .min_by(|a,b|a.1.total_cmp(&b.1)).map(|(i,_)|i);
                if let Some(i) = i {
                    match self.props[i].state {
                        0 => {
                            self.props[i].state = 1;
                            self.props[i].timer = garden_growth_times(self.players[index].width).0;
                            self.players[index].pet_action = 0.5;
                            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                            events.push(json!({"kind":"pet_garden_plant","seat":self.players[index].seat,"prop":i}));
                        }
                        1 if self.props[i].timer <= 0.0 && self.players[index].water > 0 => {
                            self.players[index].water -= 1;
                            self.props[i].state = 2;
                            self.props[i].timer = garden_growth_times(self.players[index].width).1;
                            self.players[index].pet_action = 0.6;
                            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                            events.push(json!({"kind":"pet_garden_water","seat":self.players[index].seat,"prop":i}));
                        }
                        2 if self.props[i].timer <= 0.0 => {
                            self.props[i].state = 0;
                            self.props[i].timer = 0.0;
                            let bonus = if self.players[index].golden_water { 2 } else { 0 };
                            self.players[index].golden_water = false;
                            self.award(index, 4 + bonus, Some(i), events);
                        }
                        _ => {}
                    }
                }
            }
            5 => {
                // Bubbles: point a color choice at a drifting bubble.
                let pop_reach = 3.7 * bubble_reach(self.players[index].width, self.players.len());
                if let Some(i) = self.nearest(index, pop_reach, |p| {
                    p.timer <= 0.0 && p.state == self.target_color
                }) {
                    let choice = input.yaw.round() as i32;
                    if choice == i32::from(self.props[i].state)
                        && choice == i32::from(self.target_color)
                    {
                        self.props[i].timer = 2.0;
                        self.award(index, 3, Some(i), events);
                        self.target_color =
                            (self.target_color + 1 + self.rng.random_range(0..2)) % 3;
                    }
                }
            }
            6 => {
                // Rescue: pick up a hatchling, then bring it to the safe nest.
                if self.players[index].carrying {
                    if distance(at, (0.0, 0.0)) < 2.4 * reach {
                        self.players[index].carrying = false;
                        if let Some(chick) = self.players[index].carrying_chick.take() {
                            self.props[chick].x = 0.0;
                            self.props[chick].z = 0.0;
                            self.props[chick].state = 2;
                            self.props[chick].timer = 2.5;
                            self.award(index, 5, Some(chick), events);
                        }
                    }
                } else if let Some(i) =
                    self.nearest(index, 1.7 * reach, |p| p.timer <= 0.0 && p.state == 0)
                {
                    self.props[i].state = 1;
                    self.props[i].owner = index;
                    self.players[index].carrying = true;
                    self.players[index].carrying_chick = Some(i);
                    self.players[index].pet_action = 0.8;
                    self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                    events.push(json!({"kind":"pet_rescue_pickup","seat":self.players[index].seat,"prop":i}));
                }
            }
            7 => {
                // Dig: one hidden tile, shared clues and exhausted squares.
                if let Some(i) = self.nearest(index, 1.7 * reach * reach, |_| true) {
                    if self.dug & (1 << i) != 0 {
                        return;
                    }
                    self.dug |= 1 << i;
                    if i == self.secret {
                        self.award(index, 6, Some(i), events);
                        self.secret = self.rng.random_range(0..16);
                        self.dug = 0;
                    } else {
                        self.players[index].pet_action = 0.6;
                        self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                    }
                }
            }
            8 => {
                // A leap has an airborne window. Contact with a passing hoop
                // during that window scores; clicking beside one does not.
                let p = &mut self.players[index];
                p.leap_for = 0.6;
                p.leap_hit = false;
                p.pet_action = 0.6;
                p.action_n = p.action_n.wrapping_add(1);
                events.push(json!({"kind":"pet_cloud_leap","seat":p.seat,
                    "x":at.0,"z":at.1}));
            }
            _ => {
                // Parade: a call only counts while stepping the shown direction.
                let dir = self.target % 4;
                let expected = match dir {
                    0 => Stick { x: 1.0, z: 0.0 },
                    1 => Stick { x: 0.0, z: 1.0 },
                    2 => Stick { x: -1.0, z: 0.0 },
                    _ => Stick { x: 0.0, z: -1.0 },
                };
                let moving = input.stick.clamped();
                let feet = self.players[index].runner.vx * expected.x
                    + self.players[index].runner.vz * expected.z;
                if moving.x * expected.x + moving.z * expected.z > 0.6
                    && feet > parade_step_threshold(self.players[index].width) {
                    let quick_step = self.players[index].width <= 0.9
                        && self.beat % 4 == 0 && (self.time * 0.8).fract() < 0.5;
                    self.award(
                        index,
                        1 + i32::from(self.players[index].combo.min(4))
                            + i32::from(quick_step),
                        Some(dir),
                        events,
                    );
                    self.players[index].last_scored_beat = Some(self.beat);
                } else {
                    self.players[index].combo = 0;
                }
            }
        }
    }
    fn fetch_tackle(&mut self, index: usize, events: &mut Vec<Value>) {
        if self.players[index].pounce_for <= 0.0 || self.players[index].holding_toy.is_some() {
            return;
        }
        let at = (self.players[index].runner.x, self.players[index].runner.z);
        let attacker_body = BodyProfile::new(self.players[index].width);
        let facing = (self.players[index].facing_x, self.players[index].facing_z);
        let Some(victim) = self.players.iter().enumerate()
            .filter_map(|(other, player)| {
                if other == index || player.holding_toy.is_none() {
                    return None;
                }
                let delta = (player.runner.x - at.0, player.runner.z - at.1);
                let gap = delta.0.hypot(delta.1);
                let defender_body = BodyProfile::new(player.width);
                let reach = RUNNER_RADIUS * (attacker_body.radius + defender_body.radius) + 0.12;
                (gap <= reach && (gap < 0.05
                    || (delta.0 * facing.0 + delta.1 * facing.1) / gap > 0.35))
                    .then_some((other, gap))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(other, _)| other) else {
            return;
        };
        let toy = self.players[victim].holding_toy.take().unwrap();
        let defender = (self.players[victim].runner.x, self.players[victim].runner.z);
        let delta = (defender.0 - at.0, defender.1 - at.1);
        let length = delta.0.hypot(delta.1);
        let (dx, dz) = if length > 0.01 {
            (delta.0 / length, delta.1 / length)
        } else {
            (self.players[index].facing_x, self.players[index].facing_z)
        };
        let defender_body = crate::bout::BodyProfile::new(self.players[victim].width);
        let total_mass = attacker_body.mass + defender_body.mass;
        let impact = (
            (at.0 * attacker_body.mass + defender.0 * defender_body.mass) / total_mass,
            (at.1 * attacker_body.mass + defender.1 * defender_body.mass) / total_mass,
        );
        for (seat, offset) in [(index, -0.42), (victim, 0.42)] {
            let player = &mut self.players[seat];
            player.runner.x = (impact.0 + dx * offset).clamp(-ARENA, ARENA);
            player.runner.z = (impact.1 + dz * offset).clamp(-ARENA, ARENA);
            player.runner.vx = 0.0;
            player.runner.vz = 0.0;
            player.pet_x = player.runner.x;
            player.pet_z = player.runner.z;
            player.pet_action = 0.55;
            player.impact_lock = 0.8 * crate::bout::BodyProfile::new(player.width).stun;
            player.action_n = player.action_n.wrapping_add(1);
        }
        self.players[index].pounce_for = 0.0;
        self.props[toy].x = (self.players[victim].runner.x + dx * 1.2).clamp(-ARENA, ARENA);
        self.props[toy].z = (self.players[victim].runner.z + dz * 1.2).clamp(-ARENA, ARENA);
        let loose_speed = (7.0 * attacker_body.mass / defender_body.mass).clamp(5.0, 9.0);
        self.props[toy].vx = dx * loose_speed;
        self.props[toy].vz = dz * loose_speed;
        self.props[toy].state = 0;
        self.props[toy].timer = 0.23;
        events.push(
            json!({"kind":"pet_fetch_fumble","seat":self.players[index].seat,
            "victim":self.players[victim].seat,"prop":toy,"x":impact.0,"z":impact.1}),
        );
    }
    fn fetch_pickup(&mut self, index: usize, events: &mut Vec<Value>) {
        if self.players[index].holding_toy.is_some() || self.players[index].pounce_for > 0.0 {
            return;
        }
        let reach = 0.95 * crate::bout::BodyProfile::new(self.players[index].width).reach;
        if let Some(toy) = self.nearest(index, reach, |p| p.state == 0 && p.timer <= 0.0) {
            self.props[toy].state = 1;
            self.props[toy].owner = index;
            self.players[index].holding_toy = Some(toy);
            self.players[index].pet_action = 0.5;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            events.push(
                json!({"kind":"pet_fetch_pickup","seat":self.players[index].seat,"prop":toy}),
            );
        }
    }
    fn update_props(&mut self, dt: f32, events: &mut Vec<Value>) {
        let count = self.props.len();
        let seats = self.players.len().max(1);
        let pet_positions = if self.kind == 1 {
            self.players.iter().map(|p| (p.runner.x, p.runner.z)).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut goals = Vec::new();
        let mut penned = Vec::new();
        for (i, p) in self.props.iter_mut().enumerate() {
            let was_waiting = p.timer > 0.0;
            p.timer = (p.timer - dt).max(0.0);
            if was_waiting
                && p.timer == 0.0
                && (self.kind == 0 && p.state == 2 || matches!(self.kind, 1 | 6))
            {
                let spawn_range = if self.kind == 0 { 7.2 } else { 8.2 };
                if self.kind == 1 {
                    (p.x, p.z) = scent_spot(&mut self.rng, &pet_positions, &[]);
                } else {
                    p.x = self.rng.random_range(-spawn_range..spawn_range);
                    p.z = self.rng.random_range(-spawn_range..spawn_range);
                }
                if self.kind == 0 || self.kind == 6 {
                    p.state = 0;
                    p.owner = i % seats;
                    let drift = self.rng.random_range(0.0..std::f32::consts::TAU);
                    p.vx = if self.kind == 0 {
                        drift.cos() * 3.2
                    } else {
                        0.0
                    };
                    p.vz = if self.kind == 0 {
                        drift.sin() * 3.2
                    } else {
                        0.0
                    };
                }
            }
            if self.kind == 0 && p.state == 0 {
                p.x += p.vx * dt;
                p.z += p.vz * dt;
                let speed = p.vx.hypot(p.vz);
                if speed > 2.7 {
                    let rolled = 2.7 + (speed - 2.7) * (-2.8 * dt).exp();
                    p.vx *= rolled / speed;
                    p.vz *= rolled / speed;
                }
                let away = p.x.hypot(p.z);
                if away > 9.1 {
                    let (nx, nz) = (p.x / away, p.z / away);
                    p.x = nx * 9.1;
                    p.z = nz * 9.1;
                    let toward = p.vx * nx + p.vz * nz;
                    p.vx -= 2.0 * toward * nx;
                    p.vz -= 2.0 * toward * nz;
                }
            }
            if self.kind == 0 && p.state == 3 {
                let start = (p.x, p.z);
                let step = (p.vx * dt, p.vz * dt);
                p.x += step.0;
                p.z += step.1;
                p.phase += dt;
                let length_squared = step.0 * step.0 + step.1 * step.1;
                let t = if length_squared > 0.0001 {
                    (-(start.0 * step.0 + start.1 * step.1) / length_squared).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if (start.0 + step.0 * t).hypot(start.1 + step.1 * t) < 0.85 {
                    goals.push((p.owner, i));
                    p.x = 0.0;
                    p.z = 0.0;
                    p.state = 2;
                    p.timer = 0.35;
                } else if p.timer <= 0.0 || p.x.hypot(p.z) > 10.0 {
                    p.state = 0;
                    p.timer = 0.2;
                    p.vx *= 0.2;
                    p.vz *= 0.2;
                }
            }
            if self.kind == 2 {
                p.x += p.vx * dt;
                p.z += p.vz * dt;
                let drag = (-1.0 * dt).exp();
                p.vx *= drag;
                p.vz *= drag;
                let radius = p.x.hypot(p.z);
                if p.state == 1 && radius < 2.8 {
                    penned.push((p.owner, i));
                    let (x, z) = circle(i, count, 7.0);
                    p.x = x;
                    p.z = z;
                    p.vx = 0.0;
                    p.vz = 0.0;
                    p.state = 0;
                    p.timer = 0.6;
                } else if radius > 9.1 {
                    let (nx, nz) = (p.x / radius, p.z / radius);
                    p.x = nx * 9.1;
                    p.z = nz * 9.1;
                    let outward = p.vx * nx + p.vz * nz;
                    if outward > 0.0 {
                        p.vx -= 1.6 * outward * nx;
                        p.vz -= 1.6 * outward * nz;
                    }
                }
            }
            if self.kind == 5 || self.kind == 8 {
                p.phase += dt * if self.kind == 5 { 0.7 } else { 0.45 };
                let radius = if self.kind == 5 { 6.5 } else { 6.8 };
                p.x = (p.phase + i as f32).cos() * radius;
                p.z = (p.phase + i as f32).sin() * radius;
            }
        }
        for (owner, ball) in goals {
            if owner < self.players.len() {
                if self.players[owner].streak_left <= 0.0 {
                    self.players[owner].combo = 0;
                }
                let points = 5 + i32::from(self.players[owner].combo.min(3));
                self.award(owner, points, Some(ball), events);
                self.players[owner].streak_left = 6.0;
            }
        }
        for (owner, sheep) in penned {
            if owner < self.players.len() {
                let power_bonus = ((self.players[owner].width - 0.7) / 0.7 * 3.0).round() as i32;
                self.award(owner, 5 + power_bonus, Some(sheep), events);
            }
        }
        if self.kind == 3 || self.kind == 9 {
            let beat = (self.time * if self.kind == 3 { 1.5 } else { 0.8 }) as usize;
            if beat != self.beat {
                self.beat = beat;
                if self.kind == 9 {
                    self.target = self.rng.random_range(0..4);
                }
                if self.kind == 3 && beat % 4 == 0 {
                    self.target = self.rng.random_range(0..4);
                }
            }
        }
    }
    fn finish(&mut self) {
        let mut ranked: Vec<_> = self.players.iter().map(|p| (p.seat, p.score)).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (seat, score) in ranked {
            if last != Some(score) {
                groups.push(Vec::new());
                last = Some(score);
            }
            groups.last_mut().unwrap().push(seat);
        }
        self.result = Some(groups);
    }
    fn hint(&self, player: &Player) -> Value {
        let at = (player.runner.x, player.runner.z);
        match self.kind {
            1 => {
                let near = self
                    .props
                    .iter()
                    .filter(|p| p.timer <= 0.0)
                    .map(|p| distance(at, (p.x, p.z)))
                    .fold(f32::INFINITY, f32::min);
                json!({"strength": (1.0 - near / sniff_range(player.width)).clamp(0.0, 1.0)})
            }
            7 => {
                let p = &self.props[self.secret];
                json!({"strength": (1.0 - distance(at, (p.x, p.z)) / (sniff_range(player.width) * 1.25)).clamp(0.0, 1.0)})
            }
            _ => Value::Null,
        }
    }
}
impl Bout for PetArcade {
    fn key(&self) -> &'static str {
        KEYS[self.kind]
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        self.update_props(dt, events);
        let seats = self.players.len();
        for i in 0..self.players.len() {
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == self.players[i].seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let p = &mut self.players[i];
            p.cooldown = (p.cooldown - dt).max(0.0);
            p.pet_action = (p.pet_action - dt).max(0.0);
            p.pounce_for = (p.pounce_for - dt).max(0.0);
            p.pounce_cooldown = (p.pounce_cooldown - dt).max(0.0);
            p.impact_lock = (p.impact_lock - dt).max(0.0);
            let was_leaping = p.leap_for > 0.0;
            p.leap_for = (p.leap_for - dt).max(0.0);
            if self.kind == 8 && was_leaping && p.leap_for == 0.0 && !p.leap_hit {
                p.combo = 0;
            }
            p.streak_left = (p.streak_left - dt).max(0.0);
            if input.yaw.is_finite() {
                p.aim_yaw = input.yaw;
            }
            let moving = input.stick.clamped();
            let magnitude = moving.x.hypot(moving.z);
            if magnitude > 0.15 && p.pounce_for <= 0.0 {
                p.facing_x = moving.x / magnitude;
                p.facing_z = moving.z / magnitude;
            }
            let carry = if p.carrying {
                rescue_carry_speed(p.width)
            } else if p.holding_toy.is_some() {
                fetch_carry_speed(p.width, self.props.len())
            } else if self.kind == 4 && p.water > 0 {
                1.0 - f32::from(p.water) / f32::from(garden_capacity(p.width)) * 0.07
            } else {
                1.0
            };
            if self.kind != 3 && p.impact_lock <= 0.0 {
                let stick = if self.kind == 0 && p.pounce_for > 0.0 {
                    Stick {
                        x: p.facing_x,
                        z: p.facing_z,
                    }
                } else {
                    input.stick
                };
                step_runner(
                    &mut p.runner,
                    stick,
                    dt,
                    if self.kind == 9 {
                        2.8
                    } else if self.kind == 0 && p.pounce_for > 0.0 {
                        16.0
                    } else {
                        6.8 * carry * if self.kind == 1 {
                            scent_crowd_agility(p.width, seats)
                        } else {
                            1.0
                        }
                    },
                );
            }
            let edge = if self.kind == 9 { 6.5 } else { ARENA };
            p.runner.x = p.runner.x.clamp(-edge, edge);
            p.runner.z = p.runner.z.clamp(-edge, edge);
            p.pet_x = p.runner.x;
            p.pet_z = p.runner.z;
            if self.kind == 4 && p.runner.x.hypot(p.runner.z) < 1.6
                && p.water < garden_capacity(p.width) {
                p.water = garden_capacity(p.width);
                if self.time >= self.golden_water_at && !p.golden_water {
                    p.golden_water = true;
                    self.golden_water_at = self.time + 9.0;
                    events.push(json!({"kind":"pet_garden_golden_water","seat":p.seat,
                        "x":p.runner.x,"z":p.runner.z}));
                }
                p.pet_action = 0.35;
                p.action_n = p.action_n.wrapping_add(1);
                events.push(json!({"kind":"pet_garden_fill","seat":p.seat,
                    "water":p.water,"x":p.runner.x,"z":p.runner.z}));
            }
            if input.fire {
                self.act(i, input, events);
            }
        }
        if self.kind == 0 {
            let mut runners = self.players.iter().map(|p| p.runner).collect::<Vec<_>>();
            for i in 0..runners.len() {
                for j in (i + 1)..runners.len() {
                    if runners[i].distance_to(runners[j].x, runners[j].z) < 0.01 {
                        runners[j].x += 0.02;
                    }
                }
            }
            crate::ground::separate(&mut runners);
            for (player, runner) in self.players.iter_mut().zip(runners) {
                player.runner = runner;
                player.pet_x = runner.x;
                player.pet_z = runner.z;
            }
            for i in 0..self.players.len() {
                self.fetch_tackle(i, events);
            }
            for i in 0..self.players.len() {
                self.fetch_pickup(i, events);
            }
        }
        if self.kind == 8 {
            for i in 0..self.players.len() {
                let leap = self.players[i].leap_for;
                if !(0.12..=0.48).contains(&leap) || self.players[i].leap_hit {
                    continue;
                }
                let reach = cloud_leap_profile(self.players[i].width).1;
                if let Some(hoop) = self.nearest(i, reach, |p| p.timer <= 0.0) {
                    self.props[hoop].timer = 1.5;
                    self.players[i].leap_hit = true;
                    let points = 1 + i32::from(self.players[i].combo.min(4));
                    self.award(i, points, Some(hoop), events);
                }
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.kind == 1 {
            let pet = &mut self.players[index];
            if distance((pet.runner.x, pet.runner.z), scent_waypoint(pet.scent_waypoint)) < 0.9 {
                pet.scent_waypoint = (pet.scent_waypoint + 1) % 81;
            }
        }
        let p = &self.players[index];
        let at = (p.runner.x, p.runner.z);
        let loose = self
            .props
            .iter()
            .filter(|prop| prop.state == 0 && prop.timer <= 0.0)
            .map(|prop| (prop.x, prop.z))
            .min_by(|a, b| distance(at, *a).total_cmp(&distance(at, *b)));
        let carrier = self
            .players
            .iter()
            .filter(|other| other.seat != seat && other.holding_toy.is_some())
            .map(|other| (other.runner.x, other.runner.z))
            .min_by(|a, b| distance(at, *a).total_cmp(&distance(at, *b)));
        let (fetch_target, fetch_carrier) = if p.holding_toy.is_some() {
            if distance(at, (0.0, 0.0)) > 5.5 {
                ((0.0, 0.0), false)
            } else {
                (at, false)
            }
        } else {
            match (loose, carrier) {
                (Some(ball), Some(holder)) if distance(at, ball) <= distance(at, holder) => {
                    (ball, false)
                }
                (_, Some(holder)) => (holder, true),
                (Some(ball), None) => (ball, false),
                (None, None) => ((0.0, 0.0), false),
            }
        };
        let target = match self.kind {
            0 => fetch_target,
            8 => self
                .props
                .iter()
                .filter(|prop| prop.timer <= 0.0)
                .min_by(|a, b| distance(at, (a.x, a.z)).total_cmp(&distance(at, (b.x, b.z))))
                .map(|prop| (prop.x, prop.z))
                .unwrap_or((0.0, 0.0)),
            2 => self
                .props
                .iter()
                .filter(|prop| prop.timer <= 0.0)
                .min_by(|a, b| distance(at, (a.x, a.z)).total_cmp(&distance(at, (b.x, b.z))))
                .map(|prop| {
                    let length = prop.x.hypot(prop.z).max(0.1);
                    (
                        prop.x + prop.x / length * 2.0,
                        prop.z + prop.z / length * 2.0,
                    )
                })
                .unwrap_or((0.0, 0.0)),
            4 => {
                let owned = self.props.iter().filter(|prop|prop.owner==index);
                let harvest = owned.clone().find(|prop|prop.state==2 && prop.timer<=0.0);
                let water = owned.clone().find(|prop|prop.state==1 && prop.timer<=0.0);
                let plant = owned.clone().find(|prop|prop.state==0);
                if let Some(prop) = harvest.or_else(|| if p.water>0 { water } else { None })
                    .or(plant) {
                    (prop.x,prop.z)
                } else if p.water < garden_capacity(p.width)
                    && owned.clone().any(|prop|prop.state==1) {
                    (0.0,0.0)
                } else {
                    owned.min_by(|a,b|distance(at,(a.x,a.z))
                        .total_cmp(&distance(at,(b.x,b.z))))
                        .map(|prop|(prop.x,prop.z)).unwrap_or((0.0,0.0))
                }
            }
            // Sweep the field and use the public sniff cue to know when to
            // search. Hidden cache positions never enter the bot target.
            1 => scent_waypoint(p.scent_waypoint),
            5 => self
                .props
                .iter()
                .filter(|prop| {
                    prop.timer <= 0.0 && prop.state == self.target_color
                })
                .min_by(|a, b| distance(at, (a.x, a.z)).total_cmp(&distance(at, (b.x, b.z))))
                .map(|prop| (prop.x, prop.z))
                .unwrap_or((0.0, 0.0)),
            6 => {
                if p.carrying {
                    (0.0, 0.0)
                } else {
                    self.props
                        .iter()
                        .filter(|prop| prop.timer <= 0.0 && prop.state == 0)
                        .min_by(|a, b| {
                            distance(at, (a.x, a.z)).total_cmp(&distance(at, (b.x, b.z)))
                        })
                        .map(|prop| (prop.x, prop.z))
                        .unwrap_or((0.0, 0.0))
                }
            }
            7 => {
                let start = ((self.time / 5.5) as usize + usize::from(seat) * 5) % 16;
                let tile = (0..16)
                    .map(|offset| (start + offset) % 16)
                    .find(|tile| self.dug & (1 << tile) == 0)
                    .unwrap_or(start);
                (self.props[tile].x, self.props[tile].z)
            }
            _ => (0.0, 0.0),
        };
        let dist = distance(at, target);
        let mut input = Input {
            stick: move_toward(at, target),
            ..Default::default()
        };
        input.stick.x *= 0.58 + p.skill * 0.42;
        input.stick.z *= 0.58 + p.skill * 0.42;
        let action_range = match self.kind {
            2 => 3.0 * (1.0 + (p.width - 1.0) * 0.12),
            4 => garden_tend_range(p.width) * 0.85,
            5 => 3.5 * bubble_reach(p.width, self.players.len()),
            7 => 1.7 * (1.0 + (p.width - 1.0) * 0.12).powi(2) * 0.85,
            8 => cloud_leap_profile(p.width).1 * 0.85,
            _ => 1.35,
        };
        input.fire = p.cooldown <= 0.0
            && (self.kind == 3 || self.kind == 9 || dist < action_range);
        if self.kind == 1 {
            let clue = self.hint(p)["strength"].as_f64().unwrap_or(0.0) as f32;
            let reach = 1.0 + (p.width - 1.0) * 0.12;
            input.fire = p.cooldown <= 0.0
                && clue >= 1.0 - 1.45 * reach / sniff_range(p.width);
        }
        if self.kind == 0 {
            input.fire = if p.holding_toy.is_some() {
                distance(at, (0.0, 0.0)) < 7.0 && p.impact_lock <= 0.0
            } else {
                fetch_carrier && p.pounce_cooldown <= 0.0 && dist < 2.7
            };
        }
        let reaction_tick = if self.kind == 3 || self.kind == 9 {
            self.beat as u32
        } else {
            (self.time / 0.8) as u32
        };
        input.fire &= bot_reacts(seat, reaction_tick, self.seed, p.skill);
        if self.kind == 0 && p.holding_toy.is_some() {
            let variation =
                ((reaction_tick.wrapping_mul(37) + u32::from(seat) * 53) % 101) as f32 / 50.0 - 1.0;
            input.yaw = (-at.0).atan2(-at.1) + variation * 0.32 * (1.0 - p.skill * 0.35);
        }
        if self.kind == 3 {
            let (early,late) = choir_window(p.width);
            input.fire &= (early..=late).contains(&(self.time * 1.5).fract());
        }
        if self.kind == 3 {
            input.yaw = ((self.beat + self.target) % 4) as f32;
        }
        if self.kind == 5 {
            input.yaw = self.target_color as f32;
        }
        if self.kind == 9 {
            input.stick = match self.target % 4 {
                0 => Stick { x: 1.0, z: 0.0 },
                1 => Stick { x: 0.0, z: 1.0 },
                2 => Stick { x: -1.0, z: 0.0 },
                _ => Stick { x: 0.0, z: -1.0 },
            };
            let feet = p.runner.vx * input.stick.x + p.runner.vz * input.stick.z;
            input.fire &= feet > parade_step_threshold(p.width);
        }
        if self.kind == 6 && p.carrying {
            input.fire = p.cooldown <= 0.0 && dist < 1.8;
        }
        input
    }
    fn snapshot(&self) -> Value {
        let basket = self.basket();
        json!({"game":self.key(), "time":self.time, "limit":LIMIT, "beat":self.beat,
            "target":self.target, "target_color":self.target_color, "dug":self.dug,
            "golden_water_in":if self.kind == 4 {
                (self.golden_water_at-self.time).max(0.0)
            } else { 0.0 },
            "basket":{"x":basket.0,"z":basket.1},
            "players":self.players.iter().map(|p| json!({"seat":p.seat,"x":p.avatar_x,"y":1.05,"z":p.avatar_z,
                "score":p.score,"combo":p.combo,"cooldown":p.cooldown,"pounce_for":p.pounce_for,
                "cue_scored":matches!(self.kind, 3 | 9) && p.last_scored_beat == Some(self.beat),
                "pounce_cooldown":p.pounce_cooldown,"impact_lock":p.impact_lock,"streak_left":p.streak_left,
                "aim_yaw":p.aim_yaw,
                "holding_toy":p.holding_toy,"carrying":p.carrying,
                "water":p.water,"water_capacity":garden_capacity(p.width),
                "golden_water":p.golden_water,
                "pet":p.pet,"body_width":p.width,"pet_x":p.pet_x,"pet_z":p.pet_z,
                "pet_vx":p.runner.vx,"pet_vz":p.runner.vz,
                "pet_y":if self.kind == 8 && p.leap_for > 0.0 {
                    let phase = 1.0 - p.leap_for / 0.6;
                    cloud_leap_profile(p.width).0 * (phase * std::f32::consts::PI).sin().max(0.0)
                } else { 0.0 },
                "leap_for":p.leap_for,
                "pet_action":p.pet_action,"action_n":p.action_n})).collect::<Vec<_>>(),
            "props":self.props.iter().enumerate().map(|(i,p)| {
                // Scent caches and treasure stay secret until found.
                if self.kind == 1 && p.timer <= 0.0 {
                    json!({"index":i,"hidden":true,"state":p.state,"owner":p.owner})
                } else {
                    json!({"index":i,"x":p.x,"z":p.z,"state":p.state,"timer":p.timer,"phase":p.phase,"owner":p.owner})
                }
            }).collect::<Vec<_>>()})
    }
    fn snapshot_for(&self, seat: Option<u8>) -> Option<Value> {
        if self.kind != 1 && self.kind != 7 {
            return None;
        }
        let mut view = self.snapshot();
        let hint = seat
            .and_then(|seat| self.players.iter().find(|p| p.seat == seat))
            .map(|p| self.hint(p))
            .unwrap_or(Value::Null);
        view["hint"] = hint;
        Some(view)
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bot_reactions_vary_by_match_without_erasing_skill() {
        let reactions = |seed, skill| (0..64)
            .map(|tick| bot_reacts(0, tick, seed, skill))
            .collect::<Vec<_>>();
        let first = reactions(11, 0.7);
        assert_eq!(first, reactions(11, 0.7));
        assert_ne!(first, reactions(12, 0.7));
        let relaxed = reactions(11, 0.15);
        let sharp = reactions(11, 0.9);
        assert!(relaxed.iter().zip(&sharp).all(|(slow, fast)| !slow || *fast));
        assert!(sharp.iter().filter(|&&reacts| reacts).count()
            > relaxed.iter().filter(|&&reacts| reacts).count());
    }
    #[test]
    fn rhythm_games_award_only_one_success_per_cue() {
        for kind in [3, 9] {
            let mut game = PetArcade::new(kind,
                &[Contestant::person(0), Contestant::person(1)], 4);
            game.time = if kind == 3 { 1.0 / 3.0 } else { 0.3 };
            game.players[0].runner.vx = 2.0;
            let mut input = Input { stick: Stick { x: 1.0, z: 0.0 },
                ..Default::default() };
            game.act(0, input, &mut Vec::new());
            let first = game.players[0].score;
            assert!(first > 0, "{}", game.key());
            assert_eq!(game.snapshot()["players"][0]["cue_scored"], true);
            game.players[0].cooldown = 0.0;
            game.act(0, input, &mut Vec::new());
            assert_eq!(game.players[0].score, first, "{} repeated a cue", game.key());
            game.beat = 1;
            game.time = if kind == 3 { 1.0 } else { 1.55 };
            assert_eq!(game.snapshot()["players"][0]["cue_scored"], false);
            input.yaw = if kind == 3 { 1.0 } else { 0.0 };
            game.act(0, input, &mut Vec::new());
            assert!(game.players[0].score > first, "{} blocked the next cue", game.key());
        }
    }
    #[test]
    fn scent_caches_start_and_respawn_outside_immediate_pickup_range() {
        for count in [2, 4, 8] {
            for seed in 0..128 {
                let field = (0..count).map(Contestant::person).collect::<Vec<_>>();
                let mut game = PetArcade::new(1, &field, seed);
                let starts = game.players.iter().map(|p| (p.runner.x, p.runner.z)).collect::<Vec<_>>();
                for cache in &game.props {
                    assert!(starts.iter().all(|&pet| distance(pet, (cache.x, cache.z)) >= 2.7));
                }
                for (index, cache) in game.props.iter().enumerate() {
                    assert!(game.props.iter().skip(index + 1).all(|other| {
                        distance((cache.x, cache.z), (other.x, other.z)) >= 2.1
                    }));
                }
                game.props[0].timer = 0.01;
                game.update_props(0.02, &mut Vec::new());
                assert!(starts.iter().all(|&pet| {
                    distance(pet, (game.props[0].x, game.props[0].z)) >= 2.7
                }));
            }
        }
    }
    #[test]
    fn broad_pets_get_a_stronger_search_clue_at_the_same_spot() {
        for kind in [1, 7] {
            let mut small = Contestant::person(0);
            small.body_width = 0.7;
            let mut broad = Contestant::person(1);
            broad.body_width = 1.4;
            let mut game = PetArcade::new(kind, &[small, broad], 17);
            game.players[1].runner = game.players[0].runner;
            game.props[0].x = game.players[0].runner.x + 6.0;
            game.props[0].z = game.players[0].runner.z;
            if kind == 1 {
                for prop in game.props.iter_mut().skip(1) {
                    prop.timer = 10.0;
                }
            } else {
                game.secret = 0;
            }
            let small_hint = game.snapshot_for(Some(0)).unwrap()["hint"]["strength"]
                .as_f64().unwrap();
            let broad_hint = game.snapshot_for(Some(1)).unwrap()["hint"]["strength"]
                .as_f64().unwrap();
            assert!(broad_hint > small_hint, "{}: {broad_hint} <= {small_hint}", KEYS[kind]);
        }
    }

    #[test]
    fn broad_pets_can_pop_and_dig_from_a_little_farther_away() {
        for kind in [5, 7] {
            let mut small = Contestant::person(0);
            small.body_width = 0.7;
            let mut broad = Contestant::person(1);
            broad.body_width = 1.4;
            let mut game = PetArcade::new(kind, &[small, broad], 23);
            for player in &mut game.players {
                player.runner.x = 0.0;
                player.runner.z = 0.0;
            }
            for prop in &mut game.props {
                prop.timer = 10.0;
            }
            game.props[0].timer = 0.0;
            game.props[0].x = if kind == 5 { 3.75 } else { 1.8 };
            game.props[0].z = 0.0;
            game.props[0].state = game.target_color;
            game.secret = 0;
            game.act(0, Input::default(), &mut Vec::new());
            assert_eq!(game.players[0].score, 0, "{}", KEYS[kind]);
            game.act(1, Input::default(), &mut Vec::new());
            assert!(game.players[1].score > 0, "{}", KEYS[kind]);
            if kind == 7 {
                assert!(game.players[1].cooldown < game.players[0].cooldown);
            } else {
                assert_eq!(game.players[0].cooldown, game.players[1].cooldown);
            }
        }
    }

    #[test]
    fn broad_rescuers_keep_more_speed_while_carrying() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = PetArcade::new(6, &[small, broad], 3);
        for player in &mut game.players {
            player.runner = Runner::at(0.0, 0.0).with_width(player.width);
            player.carrying = true;
        }
        let input = Input::from_stick(Stick { x: 0.0, z: 1.0 });
        for _ in 0..60 {
            game.step(&[(0, input), (1, input)], 1.0 / 60.0, &mut Vec::new());
        }
        assert!(game.players[1].runner.z > game.players[0].runner.z);
    }
    #[test]
    fn herding_scores_only_when_a_sheep_enters_the_pen() {
        let mut game = PetArcade::new(2, &[Contestant::person(0)], 19);
        let mut events = Vec::new();
        game.players[0].runner.x = game.props[0].x + 1.5;
        game.players[0].runner.z = game.props[0].z;
        game.act(0, Input::default(), &mut events);
        assert_eq!(game.players[0].score, 0);
        for _ in 0..90 {
            game.update_props(1.0 / 60.0, &mut events);
        }
        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.props[0].state, 0);
    }
    #[test]
    fn barking_from_inside_does_not_score() {
        let mut game = PetArcade::new(2, &[Contestant::person(0)], 19);
        let mut events = Vec::new();
        game.players[0].runner.x = game.props[0].x - 1.5;
        game.players[0].runner.z = game.props[0].z;
        game.act(0, Input::default(), &mut events);
        for _ in 0..120 {
            game.update_props(1.0 / 60.0, &mut events);
        }
        assert_eq!(game.players[0].score, 0);
    }
    #[test]
    fn large_pets_bark_harder_and_small_pets_recover_faster() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut large = Contestant::person(1);
        large.body_width = 1.4;
        let mut game = PetArcade::new(2, &[small, large], 19);
        let mut events = Vec::new();
        for index in 0..2 {
            game.players[index].runner.x = game.props[index].x + 1.5;
            game.players[index].runner.z = game.props[index].z;
            game.act(index, Input::default(), &mut events);
        }
        assert!(game.players[0].cooldown < game.players[1].cooldown);
        assert!(game.props[1].vx.hypot(game.props[1].vz)
            > game.props[0].vx.hypot(game.props[0].vz));
        assert_eq!(events.iter().filter(|event| event["kind"] == "pet_herd_bark").count(), 2);
    }
    #[test]
    fn cloudbound_scores_during_a_leap_and_size_changes_its_arc() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut large = Contestant::person(1);
        large.body_width = 1.4;
        let mut game = PetArcade::new(8, &[small, large], 19);
        let mut events = Vec::new();
        let (x, z) = (game.props[0].x, game.props[0].z);
        game.players[0].runner = Runner::at(x, z).with_width(0.7);
        game.act(0, Input::default(), &mut events);
        assert_eq!(game.players[0].score, 0, "the click only starts a leap");
        assert!(game.snapshot()["players"][0]["pet_y"].as_f64().unwrap() < 0.01);
        for _ in 0..15 {
            game.step(&[], 1.0 / 60.0, &mut events);
        }
        assert!(game.players[0].score > 0, "airborne contact scores");
        assert!(game.snapshot()["players"][0]["pet_y"].as_f64().unwrap() > 0.5);
        let small_arc = cloud_leap_profile(0.7);
        let large_arc = cloud_leap_profile(1.4);
        assert!(small_arc.0 > large_arc.0);
        assert!(small_arc.1 < large_arc.1);
        assert_eq!(small_arc.2, large_arc.2);
    }
    #[test]
    fn pet_collects_by_touch_but_must_aim_and_shoot_to_score() {
        let mut game = PetArcade::new(0, &[Contestant::person(0)], 2);
        assert_eq!(game.props.len(), 1);
        assert_eq!(game.basket(), (0.0, 0.0));
        game.players[0].runner.x = game.props[0].x;
        game.players[0].runner.z = game.props[0].z;
        let mut events = Vec::new();
        game.fetch_pickup(0, &mut events);
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.players[0].holding_toy, Some(0));
        assert_eq!(game.props[0].state, 1);
        game.players[0].runner.x = 0.0;
        game.players[0].runner.z = 0.0;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(
            game.players[0].score, 0,
            "entering the basket does not score without a shot"
        );
        game.players[0].runner.x = 5.0;
        game.players[0].runner.z = 0.0;
        game.act(
            0,
            Input {
                fire: true,
                yaw: -std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            &mut events,
        );
        assert_eq!(game.players[0].holding_toy, None);
        assert_eq!(game.props[0].state, 3);
        assert_eq!(
            game.players[0].score, 0,
            "a shot scores only on reaching the basket"
        );
        for _ in 0..20 {
            game.update_props(1.0 / 60.0, &mut events);
        }
        assert_eq!(game.players[0].score, 5);
        assert_eq!(game.props[0].state, 2);
        assert!(game.props[0].timer <= 0.35);
    }
    #[test]
    fn inaccurate_fetch_shot_leaves_a_recoverable_ball() {
        let mut game = PetArcade::new(0, &[Contestant::person(0)], 3);
        let mut events = Vec::new();
        game.players[0].runner.x = 5.0;
        game.players[0].runner.z = 0.0;
        game.players[0].holding_toy = Some(0);
        game.props[0].state = 1;
        game.act(
            0,
            Input {
                fire: true,
                yaw: std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            &mut events,
        );
        for _ in 0..90 {
            game.update_props(1.0 / 60.0, &mut events);
        }
        assert_eq!(game.players[0].score, 0);
        assert_eq!(game.props[0].state, 0);
        assert_eq!(game.props[0].timer, 0.0);
    }
    #[test]
    fn fetch_dive_cannot_be_spammed() {
        let mut game = PetArcade::new(0, &[Contestant::person(0), Contestant::person(1)], 5);
        let mut events = Vec::new();
        game.act(0, Input::default(), &mut events);
        let first = game.players[0].action_n;
        assert_eq!(game.players[0].pounce_cooldown, 3.0);
        game.act(0, Input::default(), &mut events);
        assert_eq!(game.players[0].action_n, first);
    }
    #[test]
    fn dive_knocks_the_ball_loose_and_ends_with_both_pets_at_impact() {
        let mut game = PetArcade::new(0, &[Contestant::person(0), Contestant::person(1)], 7);
        let mut events = Vec::new();
        game.players[1].runner.x = game.props[0].x;
        game.players[1].runner.z = game.props[0].z;
        game.fetch_pickup(1, &mut events);
        assert_eq!(game.players[1].holding_toy, Some(0));
        let basket = game.basket();
        game.players[1].runner.x = basket.0;
        game.players[1].runner.z = basket.1;
        game.players[0].runner.x = basket.0 - 0.8;
        game.players[0].runner.z = basket.1;
        game.players[0].facing_x = 1.0;
        game.players[0].facing_z = 0.0;
        game.players[0].pounce_for = 0.25;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(game.players[1].holding_toy, None);
        assert_eq!(game.players[1].score, 0);
        assert_eq!(game.props[0].state, 0);
        assert_eq!(game.props[0].timer, 0.23);
        assert!(
            game.players[0]
                .runner
                .distance_to(game.players[1].runner.x, game.players[1].runner.z)
                < 1.0
        );
        assert_eq!(game.players[0].runner.vx, 0.0);
        assert_eq!(game.players[1].runner.vx, 0.0);
        game.fetch_pickup(0, &mut events);
        assert_eq!(
            game.players[0].holding_toy, None,
            "the knocked ball has a short pickup delay"
        );
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "pet_fetch_fumble")
        );
    }
    #[test]
    fn fetch_dive_hits_the_nearest_carrier_in_front() {
        let contestants = (0..6).map(Contestant::person).collect::<Vec<_>>();
        let mut game = PetArcade::new(0, &contestants, 13);
        game.players[0].runner = Runner::at(0.0, 0.0);
        game.players[0].facing_x = 1.0;
        game.players[0].facing_z = 0.0;
        game.players[0].pounce_for = 0.2;
        for (seat, toy, x) in [(1, 0, -0.4), (2, 1, 0.8), (3, 2, 0.5)] {
            game.players[seat].runner = Runner::at(x, 0.0);
            game.players[seat].holding_toy = Some(toy);
            game.props[toy].state = 1;
            game.props[toy].owner = seat;
        }
        game.fetch_tackle(0, &mut Vec::new());
        assert_eq!(game.players[1].holding_toy, Some(0));
        assert_eq!(game.players[2].holding_toy, Some(1));
        assert_eq!(game.players[3].holding_toy, None);
        assert_eq!(game.props[2].state, 0);
    }
    #[test]
    fn garden_water_relay_rewards_capacity_and_requires_a_well_trip() {
        let mut heavy=Contestant::person(0);
        heavy.body_width=1.4;
        let mut light=Contestant::person(1);
        light.body_width=0.7;
        let mut game=PetArcade::new(4,&[heavy,light],3);
        let mut events=Vec::new();
        for p in &mut game.players { p.runner=Runner::at(0.0,0.0).with_width(p.width); }
        game.step(&[],1.0/60.0,&mut events);
        assert_eq!(game.players[0].water,2);
        assert_eq!(game.players[1].water,1);
        let (x,z)=(game.props[0].x,game.props[0].z);
        game.players[0].runner=Runner::at(x,z).with_width(1.4);
        game.act(0,Input::default(),&mut events);
        assert_eq!(game.props[0].state,1);
        let broad_growth = game.props[0].timer;
        game.props[0].timer=0.0;
        game.players[0].cooldown=0.0;
        game.act(0,Input::default(),&mut events);
        assert_eq!(game.props[0].state,2);
        assert_eq!(game.players[0].water,1);
        game.props[0].timer=0.0;
        game.players[0].cooldown=0.0;
        game.act(0,Input::default(),&mut events);
        assert_eq!(game.players[0].score,4);
        let (x,z)=(game.props[2].x,game.props[2].z);
        game.players[1].runner=Runner::at(x,z).with_width(0.7);
        game.players[1].water=0;
        game.act(1,Input::default(),&mut events);
        assert!(game.props[2].timer < broad_growth);
        game.props[2].timer=0.0;
        game.players[1].cooldown=0.0;
        game.act(1,Input::default(),&mut events);
        assert_eq!(game.props[2].state,1);
        assert!(events.iter().any(|event|event["kind"]=="pet_garden_fill"));
    }
    #[test]
    fn golden_well_is_shared_and_rewards_only_the_next_harvest() {
        let mut game = PetArcade::new(4,
            &[Contestant::person(0), Contestant::person(1)], 9);
        game.time = 5.0;
        for pet in &mut game.players {
            pet.runner = Runner::at(0.0, 0.0).with_width(pet.width);
        }
        let mut events = Vec::new();
        game.step(&[], 1.0 / 60.0, &mut events);
        assert!(game.players[0].golden_water);
        assert!(!game.players[1].golden_water);
        assert!(game.snapshot()["golden_water_in"].as_f64().unwrap() > 8.0);
        assert_eq!(events.iter().filter(|event|
            event["kind"] == "pet_garden_golden_water").count(), 1);

        for (index, bed) in [(0, 0), (1, 2)] {
            game.props[bed].state = 2;
            game.props[bed].timer = 0.0;
            let (x, z) = (game.props[bed].x, game.props[bed].z);
            game.players[index].runner = Runner::at(x, z).with_width(game.players[index].width);
            game.act(index, Input::default(), &mut events);
        }
        assert_eq!(game.players[0].score, 6);
        assert_eq!(game.players[1].score, 4);
        assert!(!game.players[0].golden_water);
    }
    #[test]
    fn broad_garden_pets_can_tend_a_bed_from_farther_away() {
        for (width, planted) in [(0.7, false), (1.4, true)] {
            let mut pet = Contestant::person(0);
            pet.body_width = width;
            let mut game = PetArcade::new(4, &[pet], 23);
            game.props[1].state = 2;
            game.props[1].timer = 10.0;
            let (x, z) = (game.props[0].x, game.props[0].z);
            game.players[0].runner = Runner::at(x + 2.3, z).with_width(width);
            game.act(0, Input::default(), &mut Vec::new());
            assert_eq!(game.props[0].state == 1, planted);
        }
    }
    #[test]
    fn larger_choir_pets_have_wider_beat_but_longer_recovery() {
        let (small_early,small_late)=choir_window(0.7);
        let (large_early,large_late)=choir_window(1.4);
        assert!(large_early<small_early && large_late>small_late);
        let mut small=Contestant::person(0);
        small.body_width=0.7;
        let mut large=Contestant::person(1);
        large.body_width=1.4;
        let mut game=PetArcade::new(3,&[small,large],3);
        game.time=0.12;
        game.act(0,Input::default(),&mut Vec::new());
        game.act(1,Input::default(),&mut Vec::new());
        assert_eq!(game.players[0].score,0);
        assert!(game.players[1].score>0);
        assert!(game.players[0].cooldown<game.players[1].cooldown);
    }
    #[test]
    fn choir_notes_follow_the_controller_dpad_compass() {
        let game = PetArcade::new(3,
            &[Contestant::person(0), Contestant::person(1)], 3);
        assert!(game.props[0].z < -4.0);
        assert!(game.props[1].x > 4.0);
        assert!(game.props[2].z > 4.0);
        assert!(game.props[3].x < -4.0);
    }
    #[test]
    fn parade_requires_real_footwork_and_size_changes_the_step_tradeoff() {
        let mut small=Contestant::person(0);
        small.body_width=0.7;
        let mut large=Contestant::person(1);
        large.body_width=1.4;
        let mut game=PetArcade::new(9,&[small,large],3);
        let input=Input { stick:Stick { x:1.0,z:0.0 }, ..Default::default() };
        game.act(0,input,&mut Vec::new());
        assert_eq!(game.players[0].score,0);
        game.players[0].cooldown=0.0;
        game.players[0].runner.vx=2.0;
        game.players[1].runner.vx=2.0;
        game.act(0,input,&mut Vec::new());
        game.act(1,input,&mut Vec::new());
        assert_eq!(game.players[0].score,2);
        assert_eq!(game.players[1].score,1);
        assert!(game.players[0].cooldown<game.players[1].cooldown);
    }
    #[test]
    fn mixed_size_garden_choir_and_parade_bots_all_stay_in_the_game() {
        let contestants=[0.7,0.9,1.2,1.4].into_iter().enumerate().map(|(seat,width)| {
            let mut c=Contestant::bot(seat as u8,0.7);
            c.body_width=width;
            c
        }).collect::<Vec<_>>();
        for kind in [3,4,9] {
            let mut game=PetArcade::new(kind,&contestants,29);
            let mut events=Vec::new();
            for _ in 0..(LIMIT as usize * 60+2) {
                let inputs=contestants.iter().map(|c|
                    (c.seat,game.bot_input(c.seat,1.0/60.0))).collect::<Vec<_>>();
                game.step(&inputs,1.0/60.0,&mut events);
            }
            let scores=game.players.iter().map(|p|p.score).collect::<Vec<_>>();
            println!("{} mixed widths {:?}",game.key(),scores);
            assert!(scores.iter().all(|&score|score>0),"{}",game.key());
        }
    }
    #[test]
    fn avatar_stays_at_the_edge_while_its_pet_moves() {
        let mut game = PetArcade::new(0, &[Contestant::person(0), Contestant::person(1)], 2);
        let before = game.snapshot();
        assert!((before["players"][0]["y"].as_f64().unwrap() - 1.05).abs() < 0.01);
        assert!((before["players"][0]["z"].as_f64().unwrap() + 13.3).abs() < 0.01);
        assert_ne!(before["players"][0]["x"], before["players"][1]["x"]);
        game.step(
            &[(
                0,
                Input {
                    stick: Stick { x: 0.0, z: -1.0 },
                    ..Default::default()
                },
            )],
            0.1,
            &mut Vec::new(),
        );
        let after = game.snapshot();
        assert_eq!(before["players"][0]["x"], after["players"][0]["x"]);
        assert_eq!(before["players"][0]["z"], after["players"][0]["z"]);
        assert_ne!(before["players"][0]["pet_z"], after["players"][0]["pet_z"]);
    }
    #[test]
    fn every_pet_contest_is_live_and_ends_with_all_seats() {
        for kind in 0..KEYS.len() {
            let contestants = (0..4)
                .map(|seat| Contestant::bot(seat, 0.7))
                .collect::<Vec<_>>();
            let mut game = PetArcade::new(kind, &contestants, 23);
            let mut events = Vec::new();
            for _ in 0..(LIMIT as usize * 60 + 2) {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut events);
            }
            let placements = game.placements().unwrap();
            println!(
                "{} scores {:?}",
                game.key(),
                game.players.iter().map(|p| p.score).collect::<Vec<_>>()
            );
            if kind == 0 {
                assert!(
                    events
                        .iter()
                        .filter(|event| event["kind"] == "pet_score")
                        .count()
                        >= 2
                );
                assert!(
                    events
                        .iter()
                        .any(|event| event["kind"] == "pet_fetch_fumble")
                );
            }
            assert_eq!(placements.iter().flatten().count(), 4, "{}", game.key());
            assert!(game.players.iter().any(|p| p.score > 0), "{}", game.key());
        }
    }
    #[test]
    #[ignore = "manual multi-seed pet roster balance audit"]
    fn pet_roster_balance_audit() {
        for count in [2, 4, 8] {
            for kind in 0..KEYS.len() {
                let mut least_active = usize::from(count);
                let mut lowest_winner = i32::MAX;
                let mut highest_winner = 0;
                for seed in 0..8 {
                    let contestants = (0..count)
                        .map(|seat| {
                            let mut pet = Contestant::bot(seat, 0.7);
                            pet.body_width = [0.7, 0.9, 1.2, 1.4][seat as usize % 4];
                            pet
                        })
                        .collect::<Vec<_>>();
                    let mut game = PetArcade::new(kind, &contestants, seed);
                    let mut events = Vec::new();
                    for _ in 0..(LIMIT as usize * 60 + 2) {
                        let inputs = contestants.iter().map(|c| {
                            (c.seat, game.bot_input(c.seat, 1.0 / 60.0))
                        }).collect::<Vec<_>>();
                        game.step(&inputs, 1.0 / 60.0, &mut events);
                    }
                    let active = game.players.iter().filter(|p| p.score > 0).count();
                    let winner = game.players.iter().map(|p| p.score).max().unwrap_or(0);
                    least_active = least_active.min(active);
                    lowest_winner = lowest_winner.min(winner);
                    highest_winner = highest_winner.max(winner);
                    assert!(game.placements().is_some());
                }
                println!("{} count={count} least_active={least_active} winner_range={lowest_winner}..{highest_winner}", KEYS[kind]);
                assert!(least_active >= usize::from(count) - 1,
                    "{} left too many pets without a scoring chance at {count} players", KEYS[kind]);
                assert!(lowest_winner > 0, "{} stopped producing winners", KEYS[kind]);
            }
        }
    }
    #[test]
    #[ignore = "manual rotated body build audit across the pet roster"]
    fn pet_body_build_balance_audit() {
        for kind in 0..KEYS.len() {
            let mut small_wins = 0;
            let mut broad_wins = 0;
            let mut ties = 0;
            let mut small_score = 0;
            let mut broad_score = 0;
            for seed in 0..96 {
                let mut contestants = [Contestant::bot(0, 0.7), Contestant::bot(1, 0.7)];
                let broad_seat = (seed % 2) as usize;
                contestants[broad_seat].body_width = 1.4;
                contestants[1 - broad_seat].body_width = 0.7;
                let mut game = PetArcade::new(kind, &contestants, seed);
                let mut events = Vec::new();
                for _ in 0..(LIMIT as usize * 60 + 2) {
                    let inputs = contestants.iter().map(|c| {
                        (c.seat, game.bot_input(c.seat, 1.0 / 60.0))
                    }).collect::<Vec<_>>();
                    game.step(&inputs, 1.0 / 60.0, &mut events);
                }
                let broad = game.players[broad_seat].score;
                let small = game.players[1 - broad_seat].score;
                broad_score += broad;
                small_score += small;
                if broad > small { broad_wins += 1; }
                else if small > broad { small_wins += 1; }
                else { ties += 1; }
            }
            println!("{} small_wins={small_wins} broad_wins={broad_wins} ties={ties} mean_scores={:.1}/{:.1}",
                KEYS[kind], small_score as f32 / 96.0, broad_score as f32 / 96.0);
        }
    }
    #[test]
    #[ignore = "manual mixed-size audit for crowded pet arcade matches"]
    fn pet_crowded_body_build_balance_audit() {
        for count in [4, 8] {
            for kind in 0..KEYS.len() {
                let mut small_score = 0i64;
                let mut broad_score = 0i64;
                let mut small_winners = 0;
                let mut broad_winners = 0;
                let mut inactive = 0;
                for seed in 0..64 {
                    let contestants = (0..count)
                        .map(|seat| {
                            let mut contestant = Contestant::bot(seat, 0.7);
                            contestant.body_width = if (usize::from(seat) + seed as usize) % 2 == 0 {
                                0.7
                            } else {
                                1.4
                            };
                            contestant
                        })
                        .collect::<Vec<_>>();
                    let mut game = PetArcade::new(kind, &contestants, seed);
                    let mut events = Vec::new();
                    for _ in 0..(LIMIT as usize * 60 + 2) {
                        let inputs = contestants
                            .iter()
                            .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                            .collect::<Vec<_>>();
                        game.step(&inputs, 1.0 / 60.0, &mut events);
                    }
                    let best = game.players.iter().map(|p| p.score).max().unwrap_or(0);
                    for player in &game.players {
                        let broad = player.width >= 1.1;
                        if broad {
                            broad_score += i64::from(player.score);
                            broad_winners += usize::from(player.score == best);
                        } else {
                            small_score += i64::from(player.score);
                            small_winners += usize::from(player.score == best);
                        }
                        inactive += usize::from(player.score == 0);
                    }
                }
                let per_build = (64 * usize::from(count) / 2) as f32;
                println!("{} count={count} mean_small={:.1} mean_broad={:.1} winners_small={small_winners} winners_broad={broad_winners} inactive={inactive}",
                    KEYS[kind], small_score as f32 / per_build, broad_score as f32 / per_build);
            }
        }
    }
    #[test]
    fn shared_ball_remains_scoreable_with_two_four_or_eight_pets() {
        for count in [2, 4, 8] {
            let contestants = (0..count)
                .map(|seat| Contestant::bot(seat, 0.7))
                .collect::<Vec<_>>();
            let mut game = PetArcade::new(0, &contestants, 23);
            let mut events = Vec::new();
            for _ in 0..(LIMIT as usize * 60 + 2) {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut events);
            }
            let goals = events
                .iter()
                .filter(|event| event["kind"] == "pet_score")
                .count();
            assert!(goals >= 2, "{count} pets scored only {goals} goals");
            assert_eq!(game.props.len(), (count as usize / 2).max(1));
        }
    }
    #[test]
    fn secret_locations_are_not_sent_to_other_players() {
        let scent = PetArcade::new(1, &[Contestant::person(0), Contestant::person(1)], 5);
        let view = scent.snapshot_for(Some(0)).unwrap();
        assert!(view["hint"]["strength"].is_number());
        assert!(
            view["props"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"] == true)
        );
        let dig = PetArcade::new(7, &[Contestant::person(0), Contestant::person(1)], 5);
        let view = dig.snapshot_for(Some(0)).unwrap();
        assert!(view.get("secret").is_none());
        assert!(
            view["props"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["x"].is_number())
        );
    }
}
