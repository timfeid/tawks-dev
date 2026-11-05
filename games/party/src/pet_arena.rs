//! Two pet controlled combat bouts. The room owns every hit, cooldown, and score.
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use crate::ground::{Runner, step_runner};

pub const KEYS: [&str; 2] = ["pet_ringout", "pet_teamfight"];
pub const NAMES: [&str; 2] = ["Skyline Scramble", "Crownfall Clash"];
pub const RULES: [&str; 2] = [
    "Launch rivals out of the ring. Small pets move quickly; broad pets hit farther and harder, and resist launches. Charge a heavy strike for a knockout.",
    "Choose a champion and break the enemy Crown to win. Full teams send one top, two bottom, and one roamer. Clear minions to push a lane, capture relays for Crown pressure, and claim the dragon for a team buff.",
];
pub const CHAMPIONS: [&str; 10] = [
    "Cinderfox",
    "Shadepaw",
    "Runewisp",
    "Brambleback",
    "Misttail",
    "Voltkit",
    "Frostmole",
    "Sunhound",
    "Tidemink",
    "Sporeling",
];
pub const ROLES: [&str; 10] = [
    "charm mage",
    "shadow assassin",
    "rune caster",
    "frontline tank",
    "swift medic",
    "long range striker",
    "frost controller",
    "diving bruiser",
    "wave healer",
    "spore trapper",
];
pub const ABILITIES: [[&str; 4]; 10] = [
    ["Ember Orb", "Charm", "Foxstep", "Starfall"],
    ["Shadow Fang", "Veil", "Blink", "Final Cut"],
    ["Rune Bolt", "Binding Sigil", "Warp", "Arc Storm"],
    ["Thorn Swipe", "Barkskin", "Charge", "Groundbreak"],
    ["Mist Dart", "Mend", "Slipstream", "Sanctuary"],
    ["Volt Shot", "Snap Trap", "Roll", "Overload"],
    ["Frost Shard", "Freeze", "Burrow", "Whiteout"],
    ["Sun Bite", "Roar", "Pounce", "Solar Frenzy"],
    ["Tide Splash", "Bubble", "Surf", "High Tide"],
    ["Spore Dart", "Seed Bomb", "Vine Step", "Wild Bloom"],
];
pub const TEAM_ROLES: [&str; 4] = ["Frontline", "Damage", "Control", "Healer"];
pub const TEAM_POSITIONS: [&str; 4] = ["Top", "Bottom", "Bottom", "Roamer"];
pub const CHAMPION_ROLE: [usize; 10] = [1, 1, 2, 0, 3, 1, 2, 0, 3, 2];
pub const ROLE_DEFAULT: [usize; 4] = [3, 0, 2, 4];
/// Short descriptions match the server's actual targeting and effects.
pub const ABILITY_HELP: [[&str; 4]; 10] = [
    [
        "Orb toward an enemy",
        "Charm and stun",
        "Dash toward your aim",
        "Blast nearby enemies",
    ],
    [
        "Close shadow strike",
        "Brief invulnerability",
        "Long blink",
        "Burst nearby enemies",
    ],
    [
        "Bolt toward an enemy",
        "Bind and stun",
        "Short warp",
        "Storm around you",
    ],
    [
        "Heavy close swipe",
        "Gain a large shield",
        "Charge and hit",
        "Shockwave nearby foes",
    ],
    [
        "Dart toward an enemy",
        "Heal nearest ally",
        "Swift dash",
        "Heal allies; hurt foes",
    ],
    [
        "Long range shot",
        "Trap and stun",
        "Invulnerable roll",
        "Overload nearby foes",
    ],
    [
        "Shard toward an enemy",
        "Freeze and stun",
        "Invulnerable burrow",
        "Whiteout nearby foes",
    ],
    [
        "Heavy close bite",
        "Gain a large shield",
        "Pounce and hit",
        "Frenzy around you",
    ],
    [
        "Splash toward an enemy",
        "Heal nearest ally",
        "Surf and hit",
        "Heal allies; hurt foes",
    ],
    [
        "Spore toward an enemy",
        "Bomb and stun",
        "Vine step",
        "Bloom around you",
    ],
];
pub const DRAFT: f32 = 26.0;
pub const LIMIT: f32 = 360.0;
const TEAM_LIMIT: f32 = 300.0;
const RING_LIMIT: f32 = 60.0;
const RING: f32 = 9.2;
const MAX_HP: f32 = 240.0;
const DRAGON_HP: f32 = 650.0;
pub const DRAGON_WAKE: f32 = 75.0;
const DRAGON_RESPAWN: f32 = 100.0;
pub const CROWN_HP: f32 = 220.0;
pub const CROWN_SHIELD_END: f32 = 60.0;
pub const ARENA_RADIUS: f32 = 46.0;
pub const LANE_Z: [f32; 2] = [20.0, -20.0];
pub const CROWN_POS: [(f32, f32); 2] = [(-39.0, 0.0), (39.0, 0.0)];
pub const RELAYS: [(f32, f32); 2] = [(0.0, LANE_Z[0]), (0.0, LANE_Z[1])];
const RELAY_RADIUS: f32 = 2.7;
const RELAY_PULSE: f32 = 8.0;
const RELAY_DAMAGE: f32 = 12.0;
const CROWN_COLLAPSE_START: f32 = 240.0;
const DUEL_COLLAPSE_START: f32 = 300.0;
const CROWN_COLLAPSE_PER_SECOND: f32 = 4.0;
pub const BRUSH: [(f32, f32); 4] = [(18.0, 11.5), (18.0, -11.5), (-18.0, 11.5), (-18.0, -11.5)];
const MAX_LEVEL: u8 = 9;
const MAX_MANA: f32 = 120.0;
const MINION_WAVE: f32 = 11.0;

#[derive(Clone, Debug)]
struct Minion {
    team: u8,
    lane: usize,
    x: f32,
    z: f32,
    hp: f32,
    attack: f32,
}
#[derive(Clone, Debug)]
struct SparBot {
    x: f32,
    z: f32,
    hp: f32,
    shot: f32,
}

#[derive(Clone, Debug)]
struct Fighter {
    seat: u8,
    is_bot: bool,
    team: u8,
    pet: u8,
    width: f32,
    skill: f32,
    runner: Runner,
    champ: usize,
    role: usize,
    position: Option<usize>,
    selected: bool,
    reveal: f32,
    hp: f32,
    mana: f32,
    level: u8,
    xp: u16,
    points: u8,
    ranks: [u8; 4],
    last_upgrade: u8,
    shield: f32,
    damage: f32,
    stocks: u8,
    score: i32,
    cooldown: [f32; 4],
    lock: f32,
    invuln: f32,
    respawn: f32,
    charge: f32,
    charging: bool,
    held_w: bool,
    held_e: bool,
    knock_x: f32,
    knock_z: f32,
    height: f32,
    vertical_speed: f32,
    last_hit: Option<usize>,
    action_n: u32,
    cast: u8,
    avatar_x: f32,
}

pub struct PetArena {
    kind: usize,
    seed: u64,
    players: Vec<Fighter>,
    time: f32,
    turn: u64,
    team_score: [u8; 2],
    dragon_hp: f32,
    dragon_respawn: f32,
    dragon_breath: f32,
    dragon_claims: [u8; 2],
    team_buff: [f32; 2],
    team_push: [f32; 2],
    crown_hp: [f32; 2],
    relay_progress: [f32; 2],
    relay_owner: [Option<u8>; 2],
    relay_pulse: f32,
    minions: Vec<Minion>,
    minion_wave: f32,
    spar: [SparBot; 2],
    result: Option<Vec<Vec<u8>>>,
}

fn distance(a: &Fighter, b: &Fighter) -> f32 {
    (a.runner.x - b.runner.x).hypot(a.runner.z - b.runner.z)
}
fn direction(yaw: f32) -> (f32, f32) {
    (yaw.sin(), yaw.cos())
}
fn spawn(index: usize, count: usize, team: u8, kind: usize, role: usize, seed: u64) -> (f32, f32) {
    if kind == 1 {
        let side = if team == 0 { -1.0 } else { 1.0 };
        (side * 36.0, (role as f32 - 1.5) * 3.4)
    } else {
        let angle = index as f32 * std::f32::consts::TAU / count.max(1) as f32
            + (seed % 32) as f32 * std::f32::consts::TAU / 32.0;
        (angle.cos() * 4.4, angle.sin() * 4.4)
    }
}
fn lane_spawn(index: usize, count: usize, p: &Fighter, kind: usize, seed: u64) -> (f32, f32) {
    match p.position {
        Some(position) if kind == 1 => {
            let side = if p.team == 0 { -1.0 } else { 1.0 };
            let z = match position {
                0 => LANE_Z[0] - 2.0,
                1 => LANE_Z[1] + 2.0,
                2 => LANE_Z[1] - 2.0,
                _ => 0.0,
            };
            (side * 32.0, z)
        }
        _ => spawn(index, count, p.team, kind, p.role, seed),
    }
}
fn bot_champion(role: usize, seed: u64) -> usize {
    let options: &[usize] = match role {
        0 => &[3, 7],
        1 => &[0, 1, 5],
        2 => &[2, 6, 9],
        _ => &[4, 8],
    };
    options[(seed as usize).wrapping_add(role * 2) % options.len()]
}
impl PetArena {
    fn damage_spar(
        &mut self,
        index: usize,
        amount: f32,
        range: f32,
        yaw: f32,
        cone: f32,
        events: &mut Vec<Value>,
    ) {
        if self.kind != 1 || self.time >= DRAFT {
            return;
        }
        let p = &self.players[index];
        let bot = &mut self.spar[p.team as usize];
        let (dx, dz) = (bot.x - p.runner.x, bot.z - p.runner.z);
        let distance = dx.hypot(dz);
        let (ax, az) = direction(yaw);
        if distance > range || (dx * ax + dz * az) / distance.max(0.01) < cone {
            return;
        }
        bot.hp = (bot.hp - amount).max(0.0);
        events.push(json!({"kind":"pet_arena_spar_hit","seat":p.seat,
            "x":bot.x,"z":bot.z,"hp":bot.hp}));
        if bot.hp == 0.0 {
            bot.hp = 150.0;
        }
    }
    fn step_spar(&mut self, dt: f32) {
        if self.kind != 1 || self.time >= DRAFT {
            return;
        }
        for team in 0..2 {
            let bot = &mut self.spar[team];
            bot.z = 5.0 + (self.time * 1.6 + team as f32).sin() * 2.5;
            bot.shot -= dt;
            if bot.shot > 0.0 {
                continue;
            }
            bot.shot = 2.0;
            for p in &mut self.players {
                if p.team as usize == team
                    && p.selected
                    && p.respawn <= 0.0
                    && (p.runner.x - bot.x).hypot(p.runner.z - bot.z) < 8.0
                {
                    p.hp = (p.hp - 12.0).max(1.0);
                }
            }
        }
    }
    fn max_hp(level: u8) -> f32 {
        MAX_HP + (level.saturating_sub(1) as f32) * 19.0
    }
    fn max_mana(level: u8) -> f32 {
        MAX_MANA + (level.saturating_sub(1) as f32) * 8.0
    }
    fn gain_xp(&mut self, index: usize, amount: u16, events: &mut Vec<Value>) {
        let p = &mut self.players[index];
        if p.level >= MAX_LEVEL {
            return;
        }
        let amount = if p.position == Some(3) {
            amount.saturating_add(((u32::from(amount) * 3) / 10) as u16)
        } else {
            amount
        };
        p.xp = p.xp.saturating_add(amount);
        while p.level < MAX_LEVEL {
            let needed = 45 + u16::from(p.level) * 20;
            if p.xp < needed {
                break;
            }
            p.xp -= needed;
            p.level += 1;
            p.points += 1;
            p.hp = (p.hp + 35.0).min(Self::max_hp(p.level));
            p.mana = (p.mana + 25.0).min(Self::max_mana(p.level));
            events.push(
                json!({"kind":"pet_arena_level","seat":p.seat,"level":p.level,
                "x":p.runner.x,"z":p.runner.z}),
            );
        }
    }
    fn step_minions(&mut self, dt: f32, events: &mut Vec<Value>) {
        if self.kind != 1 || self.time < DRAFT {
            return;
        }
        self.minion_wave -= dt;
        if self.minion_wave <= 0.0 {
            self.minion_wave += MINION_WAVE;
            for lane in 0..2 {
                for team in 0..2u8 {
                    for rank in 0..3 {
                        self.minions.push(Minion {
                            team,
                            lane,
                            x: if team == 0 { -33.0 } else { 33.0 }
                                + if team == 0 {
                                    -rank as f32 * 1.3
                                } else {
                                    rank as f32 * 1.3
                                },
                            z: LANE_Z[lane] + (rank as f32 - 1.0) * 1.8,
                            hp: 80.0,
                            attack: 0.0,
                        });
                    }
                }
            }
            events.push(json!({"kind":"pet_minion_wave"}));
        }
        let positions = self
            .minions
            .iter()
            .map(|m| (m.team, m.lane, m.x, m.z, m.hp))
            .collect::<Vec<_>>();
        let mut damage = vec![0.0; self.minions.len()];
        let mut breaches = [0u8; 2];
        for (index, m) in self.minions.iter_mut().enumerate() {
            if m.hp <= 0.0 {
                continue;
            }
            m.attack = (m.attack - dt).max(0.0);
            let opponent = positions
                .iter()
                .enumerate()
                .find(|(j, (team, lane, x, z, hp))| {
                    *j != index
                        && *team != m.team
                        && *lane == m.lane
                        && *hp > 0.0
                        && (m.x - x).hypot(m.z - z) < 2.5
                })
                .map(|(j, _)| j);
            if let Some(other) = opponent {
                if m.attack == 0.0 {
                    damage[other] += 14.0;
                    m.attack = 0.8;
                }
            } else {
                m.x += if m.team == 0 { 2.9 * dt } else { -2.9 * dt };
                if m.x.abs() > 38.0 {
                    breaches[m.team as usize] += 1;
                    m.hp = 0.0;
                }
            }
        }
        for (m, damage) in self.minions.iter_mut().zip(damage) {
            m.hp -= damage;
        }
        let dead = self
            .minions
            .iter()
            .filter(|m| m.hp <= 0.0 && m.x.abs() <= 38.0)
            .map(|m| (m.team, m.x, m.z))
            .collect::<Vec<_>>();
        for (team, x, z) in dead {
            for index in 0..self.players.len() {
                let p = &self.players[index];
                if p.team != team && p.selected && (p.runner.x - x).hypot(p.runner.z - z) < 12.0 {
                    self.gain_xp(index, 10, events);
                }
            }
        }
        self.minions.retain(|m| m.hp > 0.0);
        if self.time >= CROWN_SHIELD_END {
            for (team, count) in breaches.into_iter().enumerate() {
                if count == 0 {
                    continue;
                }
                self.team_push[team] = self.team_push[team].max(8.0);
                let target = 1 - team;
                let damage = 4.0 * f32::from(count);
                self.crown_hp[target] = (self.crown_hp[target] - damage).max(0.0);
                events.push(json!({"kind":"pet_crown_hit","team":team,
                    "target":target,"hp":self.crown_hp[target],"damage":damage,
                    "x":CROWN_POS[target].0,"z":CROWN_POS[target].1}));
            }
        }
    }
    fn damage_minions(
        &mut self,
        index: usize,
        damage: f32,
        range: f32,
        yaw: f32,
        cone: f32,
        events: &mut Vec<Value>,
    ) {
        let p = &self.players[index];
        let (dx, dz) = direction(yaw);
        let (x, z, team) = (p.runner.x, p.runner.z, p.team);
        let mut kills = 0;
        for m in &mut self.minions {
            if m.team == team || m.hp <= 0.0 {
                continue;
            }
            let (mx, mz) = (m.x - x, m.z - z);
            let distance = mx.hypot(mz);
            if distance > range || (mx * dx + mz * dz) / distance.max(0.01) < cone {
                continue;
            }
            m.hp -= damage;
            if m.hp <= 0.0 {
                kills += 1;
            }
        }
        if kills > 0 {
            self.gain_xp(index, kills * 14, events);
            events.push(
                json!({"kind":"pet_minion_farm","seat":self.players[index].seat,
                "count":kills,"x":x,"z":z}),
            );
        }
    }
    fn relay_lead(&self, team: usize) -> bool {
        let own = self
            .relay_owner
            .iter()
            .filter(|owner| **owner == Some(team as u8))
            .count();
        let enemy = self
            .relay_owner
            .iter()
            .filter(|owner| **owner == Some((1 - team) as u8))
            .count();
        own > enemy
    }
    fn crown_open(&self, team: usize) -> bool {
        self.kind == 1
            && self.time >= CROWN_SHIELD_END
            && self.team_push[team] > 0.0
            && self.relay_lead(team)
    }
    fn limit(&self) -> f32 {
        if self.kind == 0 {
            RING_LIMIT
        } else if self.players.len() < 4 {
            LIMIT
        } else {
            TEAM_LIMIT
        }
    }
    fn crown_collapse_start(&self) -> f32 {
        if self.players.len() < 4 {
            DUEL_COLLAPSE_START
        } else {
            CROWN_COLLAPSE_START
        }
    }
    pub fn new(kind: usize, contestants: &[Contestant], seed: u64) -> Self {
        assert!(kind < 2);
        let count = contestants.len();
        let players = contestants
            .iter()
            .enumerate()
            .map(|(index, c)| {
                let team = if kind == 1 {
                    c.team.min(1)
                } else {
                    index as u8 % 2
                };
                let role = if kind == 1 {
                    contestants[..index]
                        .iter()
                        .filter(|prior| prior.team.min(1) == team)
                        .count()
                        % 4
                } else {
                    0
                };
                let position = (kind == 1
                    && contestants
                        .iter()
                        .filter(|other| other.team.min(1) == team)
                        .count()
                        == 4)
                    .then_some(role);
                let (x, z) = spawn(index, count, team, kind, role, seed);
                Fighter {
                    seat: c.seat,
                    is_bot: c.is_bot,
                    team,
                    pet: c.pet,
                    width: c.body_width,
                    skill: c.skill.clamp(0.0, 1.0),
                    runner: Runner::at(x, z).with_width(c.body_width),
                    champ: if kind == 1 {
                        if c.is_bot {
                            bot_champion(role, seed)
                        } else {
                            ROLE_DEFAULT[role]
                        }
                    } else {
                        index % CHAMPIONS.len()
                    },
                    role,
                    position,
                    selected: c.is_bot || kind == 0,
                    reveal: 0.0,
                    hp: MAX_HP,
                    mana: MAX_MANA,
                    level: 1,
                    xp: 0,
                    points: 0,
                    ranks: [1; 4],
                    last_upgrade: 0,
                    shield: 0.0,
                    damage: 0.0,
                    stocks: 3,
                    score: 0,
                    cooldown: [0.0; 4],
                    lock: 0.0,
                    invuln: 0.0,
                    respawn: 0.0,
                    charge: 0.0,
                    charging: false,
                    held_w: false,
                    held_e: false,
                    knock_x: 0.0,
                    knock_z: 0.0,
                    height: 0.0,
                    vertical_speed: 0.0,
                    last_hit: None,
                    action_n: 0,
                    cast: 0,
                    avatar_x: (index as f32 - (count as f32 - 1.0) * 0.5) * 2.8,
                }
            })
            .collect();
        Self {
            kind,
            seed,
            players,
            time: 0.0,
            turn: 0,
            team_score: [0, 0],
            dragon_hp: DRAGON_HP,
            dragon_respawn: 0.0,
            dragon_breath: 5.0,
            dragon_claims: [0, 0],
            team_buff: [0.0, 0.0],
            team_push: [0.0, 0.0],
            crown_hp: [CROWN_HP; 2],
            relay_progress: [0.0; 2],
            relay_owner: [None; 2],
            relay_pulse: RELAY_PULSE,
            minions: Vec::new(),
            minion_wave: MINION_WAVE,
            spar: [
                SparBot {
                    x: -31.0,
                    z: 5.0,
                    hp: 150.0,
                    shot: 2.0,
                },
                SparBot {
                    x: 31.0,
                    z: 5.0,
                    hp: 150.0,
                    shot: 2.0,
                },
            ],
            result: None,
        }
    }
    fn brush_at(x: f32, z: f32) -> Option<usize> {
        BRUSH
            .iter()
            .position(|&(bx, bz)| (x - bx).hypot(z - bz) < 3.6)
    }
    fn hidden_from(&self, observer: usize, target: usize) -> bool {
        let a = &self.players[observer];
        let b = &self.players[target];
        if a.team == b.team || b.reveal > 0.0 {
            return false;
        }
        let Some(brush) = Self::brush_at(b.runner.x, b.runner.z) else {
            return false;
        };
        Self::brush_at(a.runner.x, a.runner.z) != Some(brush) && distance(a, b) > 3.0
    }
    fn step_relays(&mut self, dt: f32, events: &mut Vec<Value>) {
        if self.kind != 1 || self.time < DRAFT {
            return;
        }
        for (site, &(x, z)) in RELAYS.iter().enumerate() {
            let mut nearby = [0u8; 2];
            for p in &self.players {
                if p.selected
                    && p.respawn <= 0.0
                    && (p.runner.x - x).hypot(p.runner.z - z) < RELAY_RADIUS
                {
                    nearby[p.team as usize] += 1;
                }
            }
            let team = if nearby[0] > 0 && nearby[1] == 0 {
                Some(0)
            } else if nearby[1] > 0 && nearby[0] == 0 {
                Some(1)
            } else {
                None
            };
            if let Some(team) = team {
                let trailing = self.crown_hp[team] < self.crown_hp[1 - team];
                let rate = 0.18
                    * (1.0 + (nearby[team] - 1).min(2) as f32 * 0.35)
                    * if trailing { 1.2 } else { 1.0 };
                let rate = rate
                    * if self.team_push[team] > 0.0 { 1.3 } else { 1.0 }
                    * if self.team_buff[team] > 0.0 { 1.3 } else { 1.0 };
                self.relay_progress[site] = (self.relay_progress[site]
                    + if team == 0 { rate * dt } else { -rate * dt })
                .clamp(-1.0, 1.0);
            }
            let old = self.relay_owner[site];
            let progress = self.relay_progress[site];
            let owner = if progress >= 0.999 {
                Some(0)
            } else if progress <= -0.999 {
                Some(1)
            } else if progress.abs() < 0.2 {
                None
            } else {
                old
            };
            self.relay_owner[site] = owner;
            if owner != old {
                events.push(json!({"kind":"pet_relay_change","site":site,"owner":owner,
                    "x":x,"z":z}));
                if let Some(team) = owner {
                    for index in 0..self.players.len() {
                        let p = &self.players[index];
                        if p.position == Some(3)
                            && p.team == team
                            && p.respawn <= 0.0
                            && (p.runner.x - x).hypot(p.runner.z - z) < RELAY_RADIUS
                        {
                            self.gain_xp(index, 18, events);
                        }
                    }
                }
            }
        }
        self.relay_pulse -= dt;
        if self.relay_pulse <= 0.0 {
            self.relay_pulse += RELAY_PULSE;
            if self.time >= CROWN_SHIELD_END {
                let count = [0, 1].map(|team| {
                    self.relay_owner
                        .iter()
                        .filter(|owner| **owner == Some(team))
                        .count()
                        + usize::from(self.players.len() < 4 && self.team_buff[team as usize] > 0.0)
                });
                if count[0] != count[1] {
                    let team = if count[0] > count[1] { 0 } else { 1 };
                    let target = 1 - team;
                    let site = self
                        .relay_owner
                        .iter()
                        .position(|owner| *owner == Some(team as u8));
                    let base = if self.players.len() <= 4 {
                        20.0
                    } else {
                        RELAY_DAMAGE
                    };
                    let damage = base
                        * count[team].abs_diff(count[target]) as f32
                        * if self.team_push[team] > 0.0 { 1.5 } else { 1.0 }
                        * if self.team_buff[team] > 0.0 { 1.5 } else { 1.0 };
                    self.crown_hp[target] = (self.crown_hp[target] - damage).max(0.0);
                    events.push(json!({"kind":"pet_crown_hit","site":site,"team":team,
                        "target":target,"hp":self.crown_hp[target],"damage":damage,
                        "x":site.map_or(0.0, |site|RELAYS[site].0),
                        "z":site.map_or(0.0, |site|RELAYS[site].1)}));
                }
            }
        }
    }
    fn damage_dragon(
        &mut self,
        index: usize,
        amount: f32,
        yaw: f32,
        range: f32,
        cone: f32,
        events: &mut Vec<Value>,
    ) {
        if self.kind != 1
            || self.time < DRAGON_WAKE
            || self.dragon_respawn > 0.0
            || self.dragon_hp <= 0.0
        {
            return;
        }
        let p = &self.players[index];
        let dx = -p.runner.x;
        let dz = -p.runner.z;
        let dist = dx.hypot(dz);
        let (ax, az) = direction(yaw);
        if dist > range || (dx * ax + dz * az) / dist.max(0.01) < cone {
            return;
        }
        let team = p.team as usize;
        let power = amount
            * if self.team_buff[team] > 0.0 {
                1.25
            } else {
                1.0
            };
        self.dragon_hp = (self.dragon_hp - power).max(0.0);
        events.push(json!({"kind":"pet_dragon_hit","seat":p.seat,"team":team,
            "hp":self.dragon_hp,"x":0.0,"z":0.0}));
        if self.dragon_hp == 0.0 {
            self.dragon_claims[team] = self.dragon_claims[team].saturating_add(1);
            self.team_buff[team] = 45.0;
            self.dragon_respawn = if self.players.len() >= 6 {
                135.0
            } else if self.players.len() >= 4 {
                120.0
            } else {
                DRAGON_RESPAWN
            };
            events.push(json!({"kind":"pet_dragon_claim","seat":p.seat,"team":team,
                "claims":self.dragon_claims[team],"x":0.0,"z":0.0}));
        }
    }
    fn damage_crown(
        &mut self,
        index: usize,
        amount: f32,
        yaw: f32,
        range: f32,
        cone: f32,
        events: &mut Vec<Value>,
    ) {
        let team = self.players[index].team as usize;
        if !self.crown_open(team) {
            return;
        }
        let target = 1 - team;
        let (x, z) = CROWN_POS[target];
        let p = &self.players[index];
        let dx = x - p.runner.x;
        let dz = z - p.runner.z;
        let dist = dx.hypot(dz);
        let (ax, az) = direction(yaw);
        if dist > range || (dx * ax + dz * az) / dist.max(0.01) < cone {
            return;
        }
        let pressure = if self.players.len() == 4 { 0.08 } else { 0.045 };
        let damage = amount
            * pressure
            * if self.team_buff[team] > 0.0 {
                1.25
            } else {
                1.0
            };
        self.crown_hp[target] = (self.crown_hp[target] - damage).max(0.0);
        events.push(json!({"kind":"pet_crown_hit","seat":p.seat,"team":team,
            "target":target,"hp":self.crown_hp[target],"damage":damage,"x":x,"z":z}));
    }
    fn target(&self, index: usize, yaw: f32, range: f32, cone: f32) -> Option<usize> {
        let (dx, dz) = direction(yaw);
        self.players
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                *i != index
                    && p.respawn <= 0.0
                    && (self.kind == 0 || p.selected)
                    && (self.kind == 0 || !self.hidden_from(index, *i))
                    && p.stocks > 0
                    && (self.kind != 0 || (p.height - self.players[index].height).abs() < 1.5)
                    && (self.kind == 0 || p.team != self.players[index].team)
            })
            .filter_map(|(i, p)| {
                let ox = p.runner.x - self.players[index].runner.x;
                let oz = p.runner.z - self.players[index].runner.z;
                let d = ox.hypot(oz);
                (d <= range && (ox * dx + oz * dz) / d.max(0.01) >= cone).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    fn hit_brawl(&mut self, from: usize, to: usize, power: f32, yaw: f32, events: &mut Vec<Value>) {
        if self.players[to].invuln > 0.0 || self.players[to].respawn > 0.0 {
            return;
        }
        let attacker = self.players[from].seat;
        let striking_body = BodyProfile::new(self.players[from].width);
        let power = power * (1.0 + (striking_body.mass - 1.0) * 0.4);
        let body = BodyProfile::new(self.players[to].width);
        let target = &mut self.players[to];
        target.damage += power * 2.2;
        let launch = (power * 0.25 + target.damage * 0.07) / body.mass;
        let (dx, dz) = direction(yaw);
        target.knock_x += dx * launch;
        target.knock_z += dz * launch;
        target.lock = 0.22 * body.stun;
        target.invuln = 0.16;
        target.last_hit = Some(from);
        target.action_n = target.action_n.wrapping_add(1);
        events.push(json!({"kind":"pet_ring_hit","seat":attacker,
            "victim":target.seat,"power":power,"x":target.runner.x,"z":target.runner.z}));
    }
    fn hit_team(
        &mut self,
        from: usize,
        to: usize,
        amount: f32,
        stun: f32,
        events: &mut Vec<Value>,
    ) {
        if self.players[to].invuln > 0.0 || self.players[to].respawn > 0.0 {
            return;
        }
        let attacker = self.players[from].seat;
        let team = self.players[from].team;
        let rank = self.players[from].ranks[self.players[from].cast as usize];
        let amount = amount
            * (1.0 + (rank.saturating_sub(1) as f32) * 0.14)
            * if self.team_buff[team as usize] > 0.0 {
                1.25
            } else {
                1.0
            };
        let victim = &mut self.players[to];
        let through = (amount - victim.shield).max(0.0);
        victim.shield = (victim.shield - amount).max(0.0);
        victim.hp -= through;
        victim.lock = victim.lock.max(stun * BodyProfile::new(victim.width).stun);
        victim.last_hit = Some(from);
        victim.reveal = 2.0;
        victim.action_n = victim.action_n.wrapping_add(1);
        let victim_seat = victim.seat;
        let victim_at = (victim.runner.x, victim.runner.z);
        let killed = victim.hp <= 0.0;
        if killed {
            victim.hp = 0.0;
            victim.respawn = 3.5;
        }
        events.push(json!({"kind":"pet_arena_hit","seat":attacker,
            "victim":victim.seat,"amount":amount,"x":victim.runner.x,"z":victim.runner.z}));
        if killed {
            self.players[from].score += 1;
            self.gain_xp(from, 45, events);
            self.team_score[team as usize] += 1;
            self.team_push[team as usize] = self.team_push[team as usize].max(12.0);
            events.push(json!({"kind":"pet_arena_ko","seat":attacker,
                "victim":victim_seat,"team":team,"x":victim_at.0,"z":victim_at.1}));
        }
    }
    fn brawl_action(&mut self, index: usize, input: Input, dt: f32, events: &mut Vec<Value>) {
        if self.players[index].respawn > 0.0
            || self.players[index].stocks == 0
            || self.players[index].lock > 0.0
        {
            return;
        }
        let p = &mut self.players[index];
        let hop = input.jump && !p.held_w;
        p.held_w = input.jump;
        if hop && p.cooldown[2] <= 0.0 && p.vertical_speed == 0.0 {
            p.vertical_speed = 6.6;
            p.invuln = 0.2;
            p.cooldown[2] = 2.8;
            p.action_n = p.action_n.wrapping_add(1);
            p.cast = 2;
            events.push(json!({"kind":"pet_ring_hop","seat":p.seat}));
        }
        if input.crouch {
            p.charging = true;
            p.charge = (p.charge + dt).min(1.4);
        } else if p.charging {
            p.charging = false;
            if p.cooldown[1] <= 0.0 {
                let power = 13.0 + p.charge * 13.0;
                p.cooldown[1] = 1.1;
                p.action_n = p.action_n.wrapping_add(1);
                p.cast = 1;
                let reach = BodyProfile::new(self.players[index].width).reach;
                if let Some(to) = self.target(index, input.yaw, 2.6 * reach, 0.45) {
                    self.hit_brawl(index, to, power, input.yaw, events);
                }
                events.push(
                    json!({"kind":"pet_ring_smash","seat":self.players[index].seat,
                    "power":power}),
                );
            }
            self.players[index].charge = 0.0;
        }
        if input.fire && self.players[index].cooldown[0] <= 0.0 && !self.players[index].charging {
            self.players[index].cooldown[0] = 0.42;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            self.players[index].cast = 0;
            let reach = BodyProfile::new(self.players[index].width).reach;
            if let Some(to) = self.target(index, input.yaw, 2.2 * reach, 0.4) {
                self.hit_brawl(index, to, 7.0, input.yaw, events);
            }
            events.push(json!({"kind":"pet_ring_swing","seat":self.players[index].seat}));
        }
    }
    fn cast(&mut self, index: usize, slot: usize, yaw: f32, events: &mut Vec<Value>) {
        let mana_cost = [12.0, 25.0, 22.0, 55.0][slot];
        if self.players[index].cooldown[slot] > 0.0
            || self.players[index].respawn > 0.0
            || self.players[index].lock > 0.0
            || self.players[index].mana < mana_cost
        {
            return;
        }
        let champ = self.players[index].champ;
        let base_cd = [0.55, 5.0, 6.0, 18.0][slot];
        self.players[index].cooldown[slot] =
            base_cd * (1.0 - (self.players[index].ranks[slot] - 1) as f32 * 0.08);
        self.players[index].mana -= mana_cost;
        self.players[index].cast = slot as u8;
        self.players[index].reveal = 2.0;
        self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
        let seat = self.players[index].seat;
        let from = (self.players[index].runner.x, self.players[index].runner.z);
        events.push(json!({"kind":"pet_arena_cast","seat":seat,"champ":champ,
            "slot":slot,"x":from.0,"z":from.1,"yaw":yaw}));
        let reach = BodyProfile::new(self.players[index].width).reach;
        match slot {
            0 => {
                let range = [8.0, 4.0, 7.0, 2.7, 7.0, 9.0, 7.0, 2.5, 7.0, 6.0][champ] * reach;
                let damage = [46.0, 60.0, 43.0, 58.0, 34.0, 52.0, 40.0, 62.0, 36.0, 42.0][champ];
                if let Some(to) = self.target(index, yaw, range, 0.72) {
                    let slow = if matches!(champ, 2 | 6 | 9) {
                        0.35
                    } else {
                        0.0
                    };
                    self.hit_team(index, to, damage, slow, events);
                }
            }
            1 => match champ {
                0 | 2 | 6 => {
                    if let Some(to) = self.target(index, yaw, 6.2, 0.55) {
                        self.hit_team(index, to, 28.0, 0.8, events);
                    }
                }
                4 | 8 => {
                    let ally = self
                        .players
                        .iter()
                        .enumerate()
                        .filter(|(i, p)| {
                            *i != index
                                && p.team == self.players[index].team
                                && p.selected
                                && p.respawn <= 0.0
                                && distance(&self.players[index], p) <= 6.5
                        })
                        .min_by(|a, b| {
                            distance(&self.players[index], a.1)
                                .total_cmp(&distance(&self.players[index], b.1))
                        })
                        .map(|(i, _)| i)
                        .unwrap_or(index);
                    self.players[ally].hp = (self.players[ally].hp + 80.0).min(MAX_HP);
                }
                1 => {
                    self.players[index].invuln = 1.15;
                }
                3 | 7 => {
                    self.players[index].shield = (self.players[index].shield + 90.0).min(150.0);
                }
                5 | 9 => {
                    if let Some(to) = self.target(index, yaw, 7.0, 0.45) {
                        self.hit_team(index, to, 42.0, 0.55, events);
                    }
                }
                _ => {}
            },
            2 => {
                let (dx, dz) = direction(yaw);
                let dash = [4.5, 6.5, 3.8, 4.0, 5.2, 5.4, 4.3, 5.0, 5.0, 4.1][champ];
                self.players[index].runner.x = from.0 + dx * dash;
                self.players[index].runner.z = from.1 + dz * dash;
                self.players[index].invuln = if matches!(champ, 1 | 5 | 6) {
                    0.25
                } else {
                    0.0
                };
                if matches!(champ, 3 | 7 | 8) {
                    if let Some(to) = self.target(index, yaw, 2.6, -0.2) {
                        self.hit_team(index, to, 35.0, 0.35, events);
                    }
                }
            }
            3 => {
                let damage = [
                    110.0, 145.0, 105.0, 95.0, 70.0, 120.0, 100.0, 115.0, 75.0, 105.0,
                ][champ];
                let range = [6.5, 3.5, 7.5, 4.0, 6.0, 9.0, 6.0, 4.5, 6.5, 6.0][champ];
                let targets = self
                    .players
                    .iter()
                    .enumerate()
                    .filter(|(i, p)| {
                        *i != index
                            && p.team != self.players[index].team
                            && p.selected
                            && p.respawn <= 0.0
                            && distance(&self.players[index], p) < range
                    })
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                for to in targets {
                    self.hit_team(
                        index,
                        to,
                        damage,
                        if champ == 6 { 0.85 } else { 0.25 },
                        events,
                    );
                }
                if matches!(champ, 4 | 8) {
                    let team = self.players[index].team;
                    for p in &mut self.players {
                        if p.team == team && p.selected && p.respawn <= 0.0 {
                            p.hp = (p.hp + 75.0).min(MAX_HP);
                            p.shield = (p.shield + 35.0).min(150.0);
                        }
                    }
                }
            }
            _ => {}
        }
        match slot {
            0 => {
                self.damage_spar(index, 46.0, 8.0 * reach, yaw, 0.6, events);
                self.damage_minions(index, 46.0, 8.0 * reach, yaw, 0.6, events);
                self.damage_dragon(index, 46.0, yaw, 8.0 * reach, 0.72, events);
                let range = [8.0, 4.0, 7.0, 2.7, 7.0, 9.0, 7.0, 2.5, 7.0, 6.0][champ] * reach;
                self.damage_crown(index, 46.0, yaw, range, 0.72, events);
            }
            2 => {
                self.damage_spar(index, 30.0, 2.6, yaw, -1.0, events);
                self.damage_minions(index, 30.0, 2.6, yaw, -1.0, events);
                self.damage_dragon(index, 30.0, yaw, 2.6, -1.0, events);
                self.damage_crown(index, 30.0, yaw, 2.6, -1.0, events);
            }
            3 => {
                self.damage_spar(index, 110.0, 6.5, yaw, -1.0, events);
                self.damage_minions(index, 110.0, 6.5, yaw, -1.0, events);
                self.damage_dragon(index, 110.0, yaw, 6.5, -1.0, events);
                let range = [6.5, 3.5, 7.5, 4.0, 6.0, 9.0, 6.0, 4.5, 6.5, 6.0][champ];
                self.damage_crown(index, 110.0, yaw, range, -1.0, events);
            }
            _ => {}
        }
    }
    fn finish(&mut self) {
        if self.kind == 1 {
            let winner = match self.crown_hp[0].total_cmp(&self.crown_hp[1]) {
                std::cmp::Ordering::Greater => Some(0),
                std::cmp::Ordering::Less => Some(1),
                std::cmp::Ordering::Equal => None,
            };
            let first = self
                .players
                .iter()
                .filter(|p| winner.is_none_or(|w| p.team == w))
                .map(|p| p.seat)
                .collect::<Vec<_>>();
            let second = self
                .players
                .iter()
                .filter(|p| winner.is_some_and(|w| p.team != w))
                .map(|p| p.seat)
                .collect::<Vec<_>>();
            self.result = Some(if second.is_empty() {
                vec![first]
            } else {
                vec![first, second]
            });
        } else {
            let mut ranked = self
                .players
                .iter()
                .map(|p| (p.seat, p.score, p.stocks))
                .collect::<Vec<_>>();
            ranked.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));
            let mut groups: Vec<Vec<u8>> = Vec::new();
            let mut previous = None;
            for (seat, score, stocks) in ranked {
                if previous != Some((score, stocks)) {
                    groups.push(Vec::new());
                }
                groups.last_mut().unwrap().push(seat);
                previous = Some((score, stocks));
            }
            self.result = Some(groups);
        }
    }
}
impl Bout for PetArena {
    fn key(&self) -> &'static str {
        KEYS[self.kind]
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.turn = self.turn.wrapping_add(1);
        let before = self.time;
        self.time += dt;
        if self.kind == 1 && before < DRAFT && self.time >= DRAFT {
            self.minion_wave = 0.0;
            let count = self.players.len();
            for (index, p) in self.players.iter_mut().enumerate() {
                p.hp = MAX_HP;
                p.mana = MAX_MANA;
                p.shield = 0.0;
                p.cooldown = [0.0; 4];
                p.lock = 0.0;
                p.respawn = 0.0;
                let (x, z) = lane_spawn(index, count, p, self.kind, self.seed);
                p.runner = Runner::at(x, z).with_width(p.width);
            }
        }
        self.step_spar(dt);
        if self.kind == 1 {
            for buff in &mut self.team_buff {
                *buff = (*buff - dt).max(0.0);
            }
            for push in &mut self.team_push {
                *push = (*push - dt).max(0.0);
            }
            if self.dragon_respawn > 0.0 {
                self.dragon_respawn = (self.dragon_respawn - dt).max(0.0);
                if self.dragon_respawn == 0.0 {
                    self.dragon_hp = DRAGON_HP;
                    self.dragon_breath = 4.0;
                    events.push(json!({"kind":"pet_dragon_respawn","x":0.0,"z":0.0}));
                }
            }
            if self.time >= DRAGON_WAKE && self.dragon_hp > 0.0 {
                self.dragon_breath -= dt;
                if self.dragon_breath <= 0.0 {
                    self.dragon_breath = 6.0;
                    events.push(json!({"kind":"pet_dragon_breath","x":0.0,"z":0.0}));
                    for p in &mut self.players {
                        if !p.selected || p.respawn > 0.0 || p.runner.x.hypot(p.runner.z) > 4.4 {
                            continue;
                        }
                        let through = (32.0 - p.shield).max(0.0);
                        p.shield = (p.shield - 32.0).max(0.0);
                        p.hp = (p.hp - through).max(0.0);
                        p.lock = p.lock.max(0.35 * BodyProfile::new(p.width).stun);
                        p.reveal = 2.0;
                        if p.hp == 0.0 {
                            p.respawn = 3.5;
                        }
                        events.push(json!({"kind":"pet_dragon_burn","victim":p.seat,
                            "x":p.runner.x,"z":p.runner.z}));
                    }
                }
            }
        }
        let count = self.players.len();
        let mut order = (0..count).collect::<Vec<_>>();
        if self.kind == 1 && count == 4 {
            let first_team = (self.turn.wrapping_add(self.seed) % 2) as usize;
            let first_role = (self.turn / 2 % 4) as usize;
            order.sort_by_key(|&index| {
                let p = &self.players[index];
                (
                    (p.role + 4 - first_role) % 4,
                    (p.team as usize + 2 - first_team) % 2,
                )
            });
        } else if count > 0 {
            let first = ((self.time * 60.0) as usize).wrapping_add(self.seed as usize) % count;
            order.rotate_left(first);
        }
        for index in order {
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == self.players[index].seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let open_draft = self.kind == 1
                && self
                    .players
                    .iter()
                    .filter(|other| other.team == self.players[index].team)
                    .count()
                    < 4;
            let p = &mut self.players[index];
            for cd in &mut p.cooldown {
                *cd = (*cd - dt).max(0.0);
            }
            p.lock = (p.lock - dt).max(0.0);
            p.invuln = (p.invuln - dt).max(0.0);
            p.reveal = (p.reveal - dt).max(0.0);
            if p.respawn > 0.0 {
                p.respawn = (p.respawn - dt).max(0.0);
                if p.respawn == 0.0 && p.stocks > 0 {
                    let (x, z) = lane_spawn(index, count, p, self.kind, self.seed);
                    p.runner = Runner::at(x, z).with_width(p.width);
                    p.hp = Self::max_hp(p.level);
                    p.mana = Self::max_mana(p.level);
                    p.shield = 0.0;
                    p.damage = 0.0;
                    p.invuln = 1.0;
                    p.height = 0.0;
                    p.vertical_speed = 0.0;
                }
                continue;
            }
            if p.stocks == 0 {
                continue;
            }
            if self.kind == 1 && !p.selected {
                let pick = input.yaw.round().clamp(0.0, 9.0) as usize;
                if input.yaw.is_finite()
                    && input.yaw >= 0.0
                    && input.yaw < 10.0
                    && (open_draft || CHAMPION_ROLE[pick] == p.role)
                {
                    p.champ = pick;
                    if open_draft {
                        p.role = CHAMPION_ROLE[pick];
                    }
                    p.selected = true;
                }
                if !p.selected {
                    continue;
                }
            }
            if self.kind == 1 {
                p.mana = (p.mana + dt * 6.5).min(Self::max_mana(p.level));
                // Top-down aim has no pitch. Tenths encode Q/W/E/R through the room's pitch clamp.
                let upgrade = if (-0.45..=-0.05).contains(&input.pitch) {
                    (-input.pitch * 10.0).round() as u8
                } else {
                    0
                };
                if p.points > 0 && upgrade != 0 && upgrade != p.last_upgrade {
                    let slot = upgrade as usize - 1;
                    if p.ranks[slot] < 5 {
                        p.ranks[slot] += 1;
                        p.points -= 1;
                        events.push(json!({"kind":"pet_arena_upgrade","seat":p.seat,
                            "slot":slot,"rank":p.ranks[slot]}));
                    }
                }
                p.last_upgrade = upgrade;
                if p.is_bot && p.points > 0 {
                    let slot = [0, 1, 2, 3][(p.level as usize + p.role) % 4];
                    p.ranks[slot] = p.ranks[slot].saturating_add(1).min(5);
                    p.points -= 1;
                }
            }
            if p.lock <= 0.0 {
                step_runner(
                    &mut p.runner,
                    input.stick,
                    dt,
                    if self.kind == 0 { 7.0 } else { 7.6 },
                );
            }
            if self.kind == 0 {
                let old_height = p.height;
                p.vertical_speed -= 14.0 * dt;
                p.height += p.vertical_speed * dt;
                let platform = [(0.0, 3.4), (3.4, -1.7), (-3.4, -1.7)]
                    .iter()
                    .any(|(x, z)| (p.runner.x - x).hypot(p.runner.z - z) < 1.5);
                let ground = if platform && old_height >= 0.84 && p.vertical_speed <= 0.0 {
                    0.85
                } else {
                    0.0
                };
                if p.height <= ground {
                    p.height = ground;
                    p.vertical_speed = 0.0;
                }
                p.runner.x += p.knock_x * dt;
                p.runner.z += p.knock_z * dt;
                let drag = (-2.5 * dt).exp();
                p.knock_x *= drag;
                p.knock_z *= drag;
                if p.runner.x.hypot(p.runner.z) > RING {
                    p.stocks -= 1;
                    p.respawn = if p.stocks > 0 { 1.2 } else { LIMIT };
                    let last_hit = p.last_hit.filter(|from| *from != index);
                    let victim = p.seat;
                    let victim_at = (p.runner.x, p.runner.z);
                    let stocks = p.stocks;
                    p.last_hit = None;
                    if let Some(from) = last_hit {
                        self.players[from].score += 1;
                        events.push(json!({"kind":"pet_ring_ko","seat":self.players[from].seat,
                            "victim":victim,"stocks":stocks,"x":victim_at.0,"z":victim_at.1}));
                    }
                    continue;
                }
                self.brawl_action(index, input, dt, events);
            } else {
                if self.time < DRAFT {
                    let home = if p.team == 0 { -36.0 } else { 36.0 };
                    p.runner.x = p.runner.x.clamp(home - 9.0, home + 9.0);
                    p.runner.z = p.runner.z.clamp(-9.0, 9.0);
                }
                let radius = p.runner.x.hypot(p.runner.z);
                if radius > ARENA_RADIUS {
                    p.runner.x *= ARENA_RADIUS / radius;
                    p.runner.z *= ARENA_RADIUS / radius;
                }
                let w = input.jump && !p.held_w;
                let e = input.crouch && !p.held_e;
                p.held_w = input.jump;
                p.held_e = input.crouch;
                if input.fire {
                    self.cast(index, 0, input.yaw, events);
                }
                if w {
                    self.cast(index, 1, input.yaw, events);
                }
                if e {
                    self.cast(index, 2, input.yaw, events);
                }
                if input.press.is_some() {
                    self.cast(index, 3, input.yaw, events);
                }
            }
        }
        self.step_minions(dt, events);
        self.step_relays(dt, events);
        if self.kind == 1 && self.time >= self.crown_collapse_start() {
            for hp in &mut self.crown_hp {
                *hp = (*hp - CROWN_COLLAPSE_PER_SECOND * dt).max(0.0);
            }
        }
        if self.time >= self.limit()
            || self.kind == 1 && self.crown_hp.iter().any(|&hp| hp <= 0.0)
            || self.kind == 0
                && self.players.iter().filter(|p| p.stocks > 0).count() <= 1
                && self.players.len() > 1
        {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let p = &self.players[index];
        if self.kind == 1 && self.time < DRAFT {
            return Input {
                yaw: p.champ as f32,
                ..Default::default()
            };
        }
        if self.kind == 1 {
            if let Some(minion) = self
                .minions
                .iter()
                .filter(|m| m.team != p.team && m.hp > 0.0)
                .filter(|m| (m.x - p.runner.x).hypot(m.z - p.runner.z) < 7.0)
                .min_by(|a, b| {
                    (a.x - p.runner.x)
                        .hypot(a.z - p.runner.z)
                        .total_cmp(&(b.x - p.runner.x).hypot(b.z - p.runner.z))
                })
            {
                return Input {
                    yaw: (minion.x - p.runner.x).atan2(minion.z - p.runner.z),
                    fire: p.cooldown[0] <= 0.0,
                    ..Default::default()
                };
            }
        }
        let team_size = self
            .players
            .iter()
            .filter(|other| other.team == p.team)
            .count();
        if self.kind == 1
            && self.crown_open(p.team as usize)
            && (team_size == 1 || p.position != Some(3) || self.players.len() == 4)
        {
            let site = CROWN_POS[1 - p.team as usize];
            let dx = site.0 - p.runner.x;
            let dz = site.1 - p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            return Input {
                stick: if dist > 4.8 {
                    Stick {
                        x: dx / dist,
                        z: dz / dist,
                    }
                } else {
                    Stick::default()
                },
                yaw: dx.atan2(dz),
                fire: dist < 8.0 && p.cooldown[0] <= 0.0,
                crouch: dist > 5.0 && p.cooldown[2] <= 0.0,
                press: (dist < 6.5 && p.cooldown[3] <= 0.0).then_some(crate::bout::Press::Both),
                ..Default::default()
            };
        }
        let relay_site = if team_size == 1 {
            let preferred = (p.team as usize) ^ (self.seed as usize & 1);
            if self.relay_owner[preferred] != Some(p.team) {
                Some(preferred)
            } else {
                self.relay_owner
                    .iter()
                    .position(|owner| *owner != Some(p.team))
            }
        } else {
            Some(match p.position {
                Some(0) => 0,
                Some(1 | 2) => 1,
                Some(3) => {
                    let first = self.seed as usize % 2;
                    if self.relay_owner[first] != Some(p.team) {
                        first
                    } else {
                        1 - first
                    }
                }
                _ => p.role % 2,
            })
        };
        if self.kind == 1
            && relay_site.is_some_and(|site| self.relay_owner[site] != Some(p.team))
            && (team_size == 1 || self.time < DRAGON_WAKE || p.position != Some(3))
        {
            let site = RELAYS[relay_site.unwrap()];
            let dx = site.0 - p.runner.x;
            let dz = site.1 - p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            let enemy = self
                .players
                .iter()
                .enumerate()
                .filter(|(i, o)| {
                    *i != index
                        && o.team != p.team
                        && o.respawn <= 0.0
                        && o.selected
                        && !self.hidden_from(index, *i)
                        && distance(p, o) < 8.0
                })
                .min_by(|a, b| distance(p, a.1).total_cmp(&distance(p, b.1)))
                .map(|(_, o)| o);
            let (yaw, attack) = if let Some(enemy) = enemy {
                (
                    (enemy.runner.x - p.runner.x).atan2(enemy.runner.z - p.runner.z),
                    true,
                )
            } else {
                (dx.atan2(dz), false)
            };
            return Input {
                stick: if dist > 1.2 {
                    Stick {
                        x: dx / dist,
                        z: dz / dist,
                    }
                } else {
                    Stick::default()
                },
                yaw,
                fire: attack && p.cooldown[0] <= 0.0,
                jump: attack && p.cooldown[1] <= 0.0,
                press: (attack && p.cooldown[3] <= 0.0).then_some(crate::bout::Press::Both),
                ..Default::default()
            };
        }
        if self.kind == 1
            && (p.position == Some(3) || (p.position.is_none() && p.role == 1) || team_size == 1)
            && self.dragon_hp > 0.0
            && self.time >= DRAGON_WAKE
            && self.team_buff[p.team as usize] <= 0.0
        {
            let dx = -p.runner.x;
            let dz = -p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            return Input {
                stick: if dist > 5.0 {
                    Stick {
                        x: dx / dist,
                        z: dz / dist,
                    }
                } else {
                    Stick::default()
                },
                yaw: dx.atan2(dz),
                fire: dist < 8.0 && p.cooldown[0] <= 0.0,
                press: (dist < 6.5 && p.cooldown[3] <= 0.0).then_some(crate::bout::Press::Both),
                ..Default::default()
            };
        }
        let target = self
            .players
            .iter()
            .enumerate()
            .filter(|(i, o)| {
                *i != index
                    && o.respawn <= 0.0
                    && o.stocks > 0
                    && (self.kind == 0 || !self.hidden_from(index, *i))
                    && (self.kind == 0 || o.team != p.team)
            })
            .min_by(|a, b| distance(p, a.1).total_cmp(&distance(p, b.1)))
            .map(|(_, o)| o);
        let Some(target) = target else {
            return Input::default();
        };
        let dx = target.runner.x - p.runner.x;
        let dz = target.runner.z - p.runner.z;
        let dist = dx.hypot(dz).max(0.01);
        let yaw = dx.atan2(dz);
        let bot_phase = if self.kind == 1 && self.players.len() >= 4 {
            p.role as u32
        } else {
            u32::from(seat)
        };
        let ready =
            ((self.time * 10.0) as u32 + bot_phase * 17 + (self.seed as u32).wrapping_mul(37)) % 10
                < (3.0 + p.skill * 7.0) as u32;
        Input {
            stick: if dist > if self.kind == 0 { 1.7 } else { 4.0 } {
                Stick {
                    x: dx / dist,
                    z: dz / dist,
                }
            } else {
                Stick::default()
            },
            yaw,
            fire: ready && dist < if self.kind == 0 { 2.0 } else { 8.0 } && p.cooldown[0] <= 0.0,
            jump: (self.kind == 1 && dist < 5.0 && p.cooldown[1] <= 0.0)
                || (self.kind == 0 && p.runner.x.hypot(p.runner.z) > 7.0 && p.cooldown[2] <= 0.0),
            crouch: (self.kind == 1 && dist > 5.0 && p.cooldown[2] <= 0.0)
                || (self.kind == 0 && dist < 2.5 && self.time % 4.0 < 1.1),
            press: (self.kind == 1 && dist < 6.0 && p.cooldown[3] <= 0.0)
                .then_some(crate::bout::Press::Both),
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        let players = self.players.iter().map(|p| json!({
                "seat":p.seat,"team":p.team,"pet":p.pet,"body_width":p.width,
                "x":p.avatar_x,"y":1.05,"z":if self.kind == 1 { -50.0 } else { -13.3 },
                "pet_x":p.runner.x,"pet_z":p.runner.z,"pet_y":if p.respawn > 0.0 { -2.0 } else { p.height },
                "score":p.score,"hp":p.hp,"max_hp":Self::max_hp(p.level),
                "mana":p.mana,"max_mana":Self::max_mana(p.level),
                "level":p.level,"xp":p.xp,"xp_next":45+u16::from(p.level)*20,
                "points":p.points,"ranks":p.ranks,
                "shield":p.shield,"damage":p.damage,
                "stocks":p.stocks,"respawn":p.respawn,"lock":p.lock,
                "invuln":p.invuln,"charge":p.charge,
                "champ":p.champ,"role":p.role,"position":p.position,
                "selected":p.selected,"reveal":p.reveal,
                "cooldowns":p.cooldown,"action_n":p.action_n,"cast":p.cast
            })).collect::<Vec<_>>();
        let relays = RELAYS
            .iter()
            .enumerate()
            .map(|(site, &(x, z))| {
                json!({
            "site":site,"x":x,"z":z,"progress":self.relay_progress[site],
            "owner":self.relay_owner[site]})
            })
            .collect::<Vec<_>>();
        let dragon = json!({"hp":self.dragon_hp,"max_hp":DRAGON_HP,
            "respawn":self.dragon_respawn,"breath":self.dragon_breath,
            "wakes_in":(DRAGON_WAKE-self.time).max(0.0),
            "claims":self.dragon_claims,"buff":self.team_buff});
        let minions = self
            .minions
            .iter()
            .map(|m| {
                json!({"team":m.team,"lane":m.lane,
            "x":m.x,"z":m.z,"hp":m.hp})
            })
            .collect::<Vec<_>>();
        json!({"game":self.key(),"time":self.time,"limit":self.limit(),
            "draft":self.kind == 1 && self.time < DRAFT,
            "team_score":self.team_score,"champions":CHAMPIONS,
            "push":self.team_push,"crown_open":[self.crown_open(0),self.crown_open(1)],
            "crown_hp":self.crown_hp,"crown_max_hp":CROWN_HP,
            "crown_shield":(CROWN_SHIELD_END-self.time).max(0.0),
            "crown_collapse_in":(self.crown_collapse_start()-self.time).max(0.0),
            "relays":relays,"relay_pulse":self.relay_pulse,
            "dragon":dragon,"players":players,"minions":minions,
            "spar":self.spar.iter().map(|bot|json!({"x":bot.x,"z":bot.z,"hp":bot.hp})).collect::<Vec<_>>(),
            "props":[]})
    }
    fn snapshot_for(&self, seat: Option<u8>) -> Option<Value> {
        if self.kind != 1 {
            return None;
        }
        let observer = self.players.iter().position(|p| Some(p.seat) == seat)?;
        let mut view = self.snapshot();
        if let Some(players) = view["players"].as_array_mut() {
            for (index, player) in players.iter_mut().enumerate() {
                if self.hidden_from(observer, index) {
                    player["hidden"] = json!(true);
                    player["pet_x"] = Value::Null;
                    player["pet_y"] = Value::Null;
                    player["pet_z"] = Value::Null;
                    player["hp"] = Value::Null;
                }
            }
        }
        Some(view)
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::Press;

    #[test]
    fn charged_hit_launches_a_pet_out_and_credits_the_attacker() {
        let mut game = PetArena::new(0, &[Contestant::person(0), Contestant::person(1)], 2);
        game.players[0].runner.x = 4.0;
        game.players[0].runner.z = 0.0;
        game.players[1].runner.x = 6.0;
        game.players[1].runner.z = 0.0;
        game.players[0].charging = true;
        game.players[0].charge = 1.4;
        let mut events = Vec::new();
        game.brawl_action(
            0,
            Input {
                yaw: std::f32::consts::FRAC_PI_2,
                ..Default::default()
            },
            1.0 / 60.0,
            &mut events,
        );
        assert_eq!(game.players[1].damage, 68.64);
        for _ in 0..90 {
            game.step(&[], 1.0 / 60.0, &mut events);
        }
        assert_eq!(game.players[1].stocks, 2);
        assert_eq!(game.players[0].score, 1);
        assert!(events.iter().any(|event| event["kind"] == "pet_ring_ko"));
    }
    #[test]
    fn remaining_stocks_beat_knockout_score_in_ringout() {
        let mut game = PetArena::new(0, &[Contestant::person(0), Contestant::person(1)], 4);
        game.players[0].score = 9;
        game.players[0].stocks = 0;
        game.players[1].score = 1;
        game.players[1].stocks = 1;
        game.finish();
        assert_eq!(game.placements().unwrap()[0], vec![1]);
    }

    #[test]
    fn heavier_pets_resist_the_same_launch() {
        let mut light = PetArena::new(0, &[Contestant::person(0), Contestant::person(1)], 2);
        let mut heavy = PetArena::new(0, &[Contestant::person(0), Contestant::person(1)], 2);
        light.players[1].width = 0.7;
        heavy.players[1].width = 1.4;
        light.hit_brawl(0, 1, 20.0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        heavy.hit_brawl(0, 1, 20.0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        assert!(light.players[1].knock_x > heavy.players[1].knock_x);
        assert!(light.players[1].lock > heavy.players[1].lock);
    }

    #[test]
    fn broad_pets_trade_speed_for_strike_force_and_reach() {
        let contestants = [Contestant::person(0), Contestant::person(1)];
        let mut small = PetArena::new(0, &contestants, 2);
        let mut broad = PetArena::new(0, &contestants, 2);
        small.players[0].width = 0.7;
        broad.players[0].width = 1.4;
        for game in [&mut small, &mut broad] {
            game.players[0].runner.x = 0.0;
            game.players[0].runner.z = 0.0;
            game.players[1].runner.x = 2.22;
            game.players[1].runner.z = 0.0;
        }
        let strike = Input {
            fire: true,
            yaw: std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        small.brawl_action(0, strike, 1.0 / 60.0, &mut Vec::new());
        broad.brawl_action(0, strike, 1.0 / 60.0, &mut Vec::new());
        assert_eq!(small.players[1].damage, 0.0);
        assert!(broad.players[1].damage > 0.0);
        small.players[1].runner.x = 1.5;
        small.players[0].cooldown[0] = 0.0;
        small.brawl_action(0, strike, 1.0 / 60.0, &mut Vec::new());
        assert!(broad.players[1].knock_x > small.players[1].knock_x);
        assert!(
            BodyProfile::new(small.players[0].width).speed
                > BodyProfile::new(broad.players[0].width).speed
        );
    }

    #[test]
    fn champion_is_chosen_before_full_power_teamfight() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 2);
        assert!(!game.players[0].selected);
        let mut events = Vec::new();
        for _ in 0..((DRAFT * 60.0) as usize + 1) {
            game.step(
                &[(
                    0,
                    Input {
                        yaw: 7.0,
                        ..Default::default()
                    },
                )],
                1.0 / 60.0,
                &mut events,
            );
        }
        assert_eq!(game.players[0].champ, 7);
        assert!(game.players[0].selected);
        assert!(!game.snapshot()["draft"].as_bool().unwrap());
        game.players[0].runner.x = -1.0;
        game.players[0].runner.z = 0.0;
        game.players[1].runner.x = 1.0;
        game.players[1].runner.z = 0.0;
        game.players[1].selected = true;
        game.cast(0, 3, std::f32::consts::FRAC_PI_2, &mut events);
        assert!(game.players[1].hp < 150.0);
        game.players[1].hp = 30.0;
        game.step(
            &[(
                0,
                Input {
                    yaw: std::f32::consts::FRAC_PI_2,
                    fire: true,
                    press: Some(Press::Both),
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut events,
        );
        assert_eq!(game.team_score[0], 1);
        assert!(game.players[1].respawn > 3.4);
        assert_eq!(game.players[0].score, 1);
    }
    #[test]
    fn smaller_teams_can_pick_any_champion_but_full_teams_fill_roles() {
        let duel = [
            Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(1),
        ];
        let mut small = PetArena::new(1, &duel, 2);
        small.step(
            &[(
                0,
                Input {
                    yaw: 0.0,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut Vec::new(),
        );
        assert!(small.players[0].selected);
        assert_eq!(small.players[0].champ, 0);
        assert_eq!(small.players[0].role, CHAMPION_ROLE[0]);

        let full = (0..8)
            .map(|seat| Contestant::person(seat).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut large = PetArena::new(1, &full, 2);
        large.step(
            &[(
                0,
                Input {
                    yaw: 0.0,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut Vec::new(),
        );
        assert!(!large.players[0].selected);
        large.step(
            &[(
                0,
                Input {
                    yaw: 3.0,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut Vec::new(),
        );
        assert!(large.players[0].selected);
        assert_eq!(large.players[0].role, 0);
    }
    #[test]
    fn an_unpicked_champion_cannot_fight_and_can_choose_after_the_clock() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::bot(1, 0.8).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 2);
        let mut events = Vec::new();
        for _ in 0..1201 {
            game.step(
                &[(
                    0,
                    Input {
                        yaw: -1.0,
                        ..Default::default()
                    },
                )],
                1.0 / 60.0,
                &mut events,
            );
        }
        assert!(!game.players[0].selected);
        assert_eq!(game.players[0].action_n, 0);
        game.step(
            &[(
                0,
                Input {
                    yaw: 7.0,
                    fire: true,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut events,
        );
        assert!(game.players[0].selected);
        assert_eq!(game.players[0].champ, 7);
    }
    #[test]
    fn each_full_team_has_one_of_each_combat_role() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let game = PetArena::new(1, &contestants, 3);
        for team in 0..2 {
            let mut roles = game
                .players
                .iter()
                .filter(|p| p.team == team)
                .map(|p| p.role)
                .collect::<Vec<_>>();
            roles.sort_unstable();
            assert_eq!(roles, vec![0, 1, 2, 3]);
            assert!(
                game.players
                    .iter()
                    .filter(|p| p.team == team)
                    .all(|p| CHAMPION_ROLE[p.champ] == p.role)
            );
            let mut positions = game
                .players
                .iter()
                .filter(|p| p.team == team)
                .map(|p| p.position)
                .collect::<Vec<_>>();
            positions.sort_unstable();
            assert_eq!(positions, vec![Some(0), Some(1), Some(2), Some(3)]);
        }
    }
    #[test]
    fn full_team_starts_in_lanes_and_roamer_gains_faster_xp() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 3);
        game.time = DRAFT - 0.01;
        game.step(&[], 0.02, &mut Vec::new());
        let team = game
            .players
            .iter()
            .filter(|p| p.team == 0)
            .collect::<Vec<_>>();
        assert!(team[0].runner.z > 15.0);
        assert!(team[1].runner.z < -15.0);
        assert!(team[2].runner.z < -15.0);
        assert!(team[3].runner.z.abs() < 1.0);
        for index in [0, 2, 4, 6] {
            game.gain_xp(index, 10, &mut Vec::new());
        }
        assert_eq!(game.players[0].xp, 10);
        assert_eq!(game.players[6].xp, 13);
        let snapshot = game.snapshot();
        assert_eq!(snapshot["players"][6]["position"], 3);
    }
    #[test]
    fn roamer_earns_xp_for_helping_capture_a_relay() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 3);
        game.time = DRAFT + 1.0;
        game.relay_progress[0] = 0.98;
        game.players[6].runner.x = RELAYS[0].0;
        game.players[6].runner.z = RELAYS[0].1;
        game.step_relays(0.2, &mut Vec::new());
        assert_eq!(game.relay_owner[0], Some(0));
        assert_eq!(game.players[6].xp, 23);
        assert_eq!(game.players[0].xp, 0);
    }
    #[test]
    fn brush_hides_from_enemies_until_they_approach_or_the_pet_fights() {
        let mut game = PetArena::new(
            1,
            &[
                Contestant::person(0).on_team(0),
                Contestant::person(1).on_team(1),
            ],
            3,
        );
        game.players[0].runner.x = BRUSH[0].0;
        game.players[0].runner.z = BRUSH[0].1;
        game.players[1].runner.x = 0.0;
        game.players[1].runner.z = 0.0;
        let hidden = game.snapshot_for(Some(1)).unwrap();
        assert_eq!(hidden["players"][0]["hidden"], true);
        assert!(hidden["players"][0]["pet_x"].is_null());
        assert!(game.snapshot_for(Some(0)).unwrap()["players"][0]["pet_x"].is_number());
        game.players[1].runner.x = BRUSH[0].0 - 0.2;
        game.players[1].runner.z = BRUSH[0].1 - 0.2;
        assert_ne!(
            game.snapshot_for(Some(1)).unwrap()["players"][0]["hidden"],
            true
        );
        game.players[1].runner.x = 0.0;
        game.players[1].runner.z = 0.0;
        game.players[0].reveal = 2.0;
        assert_ne!(
            game.snapshot_for(Some(1)).unwrap()["players"][0]["hidden"],
            true
        );
    }
    #[test]
    fn dragon_claim_grants_buff_but_only_the_crown_ends_the_match() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 3);
        game.time = DRAGON_WAKE + 1.0;
        game.players[0].runner.x = -2.0;
        game.players[0].runner.z = 0.0;
        let mut events = Vec::new();
        game.damage_dragon(
            0,
            DRAGON_HP,
            std::f32::consts::FRAC_PI_2,
            8.0,
            0.7,
            &mut events,
        );
        assert_eq!(game.dragon_claims, [1, 0]);
        assert!(game.team_buff[0] > 20.0);
        game.dragon_hp = DRAGON_HP;
        game.dragon_respawn = 0.0;
        game.damage_dragon(
            0,
            DRAGON_HP,
            std::f32::consts::FRAC_PI_2,
            8.0,
            0.7,
            &mut events,
        );
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(game.dragon_claims, [2, 0]);
        assert!(game.placements().is_none());
        assert!(events.iter().any(|e| e["kind"] == "pet_dragon_claim"));
        game.crown_hp[1] = 0.0;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(game.placements().unwrap()[0], vec![0, 2, 4, 6]);
    }
    #[test]
    fn dragon_buff_adds_crown_pressure_in_a_duel_without_ending_it() {
        let contestants = (0..2)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 3);
        game.time = CROWN_SHIELD_END + 1.0;
        game.team_buff[0] = 45.0;
        game.relay_pulse = 0.01;
        game.step(&[], 1.0 / 60.0, &mut Vec::new());
        assert!(game.crown_hp[1] < CROWN_HP);
        assert!(game.placements().is_none());
    }
    #[test]
    fn time_limit_awards_the_team_with_more_crown_health() {
        let contestants = (0..2)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 8);
        game.time = LIMIT - 0.01;
        game.crown_hp = [50.0, 20.0];
        game.relay_pulse = RELAY_PULSE;
        game.step(&[], 0.02, &mut Vec::new());
        assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
    }
    #[test]
    fn dragon_buff_strengthens_team_relay_pressure() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = CROWN_SHIELD_END + 1.0;
        game.relay_owner[0] = Some(0);
        game.relay_progress[0] = 1.0;
        game.team_buff[0] = 30.0;
        game.relay_pulse = 0.01;
        game.step(&[], 1.0 / 60.0, &mut Vec::new());
        assert_eq!(game.crown_hp[1], CROWN_HP - RELAY_DAMAGE * 1.5);
    }
    #[test]
    fn relays_open_crown_pressure_and_a_ko_creates_a_push() {
        let contestants = [
            Contestant::bot(0, 0.7).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = DRAFT;
        game.players[0].runner.x = RELAYS[0].0;
        game.players[0].runner.z = RELAYS[0].1;
        game.players[1].runner.x = 14.8;
        game.players[1].runner.z = 0.0;
        let mut events = Vec::new();
        for _ in 0..400 {
            game.step(&[], 1.0 / 60.0, &mut events);
        }
        assert_eq!(game.relay_owner[0], Some(0));
        assert_eq!(game.crown_hp, [CROWN_HP; 2]);
        game.players[1].hp = 20.0;
        game.hit_team(0, 1, 46.0, 0.0, &mut events);
        assert!(game.team_push[0] > 11.0);
        game.time = CROWN_SHIELD_END + 1.0;
        game.relay_pulse = 0.01;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(game.crown_hp[1], CROWN_HP - 20.0 * 1.5);
        assert!(events.iter().any(|e| e["kind"] == "pet_crown_hit"));
    }
    #[test]
    fn breaking_the_enemy_crown_wins_without_a_dragon_claim() {
        let contestants = [
            Contestant::bot(0, 0.7).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = CROWN_SHIELD_END + 1.0;
        game.relay_owner[0] = Some(0);
        game.relay_progress[0] = 1.0;
        game.crown_hp[1] = RELAY_DAMAGE;
        game.relay_pulse = 0.01;
        game.step(&[], 1.0 / 60.0, &mut Vec::new());
        assert_eq!(game.crown_hp[1], 0.0);
        assert_eq!(game.dragon_claims, [0, 0]);
        assert_eq!(game.placements().unwrap()[0], vec![0]);
    }
    #[test]
    fn direct_crown_damage_requires_a_ko_push_and_relay_lead() {
        let contestants = [
            Contestant::bot(0, 0.7).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = CROWN_SHIELD_END + 1.0;
        game.players[0].runner.x = CROWN_POS[1].0 - 4.5;
        game.players[0].runner.z = 0.0;
        let mut events = Vec::new();
        game.damage_crown(0, 46.0, std::f32::consts::FRAC_PI_2, 8.0, 0.72, &mut events);
        assert_eq!(game.crown_hp[1], CROWN_HP);
        game.relay_owner[0] = Some(0);
        game.relay_progress[0] = 1.0;
        game.team_push[0] = 12.0;
        game.damage_crown(0, 46.0, std::f32::consts::FRAC_PI_2, 8.0, 0.72, &mut events);
        assert!(game.crown_hp[1] < CROWN_HP);
        assert_eq!(events.last().unwrap()["kind"], "pet_crown_hit");
        let after = game.crown_hp[1];
        game.team_push[0] = 0.0;
        game.damage_crown(0, 46.0, std::f32::consts::FRAC_PI_2, 8.0, 0.72, &mut events);
        assert_eq!(game.crown_hp[1], after);
    }
    #[test]
    fn two_lanes_spawn_minions_and_farming_levels_champions() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 4);
        game.time = DRAFT;
        game.players[0].selected = true;
        game.minion_wave = 0.0;
        game.step(&[], 1.0 / 60.0, &mut Vec::new());
        assert_eq!(game.minions.len(), 12);
        assert_eq!(game.minions.iter().filter(|m| m.lane == 0).count(), 6);
        assert_eq!(game.minions.iter().filter(|m| m.lane == 1).count(), 6);
        let target = game
            .minions
            .iter()
            .find(|m| m.team == 1 && m.lane == 0)
            .unwrap()
            .clone();
        game.players[0].runner.x = target.x - 3.0;
        game.players[0].runner.z = target.z;
        game.damage_minions(
            0,
            200.0,
            8.0,
            std::f32::consts::FRAC_PI_2,
            0.6,
            &mut Vec::new(),
        );
        assert!(game.players[0].xp > 0);
        game.gain_xp(0, 90, &mut Vec::new());
        assert!(game.players[0].level >= 2);
        assert_eq!(game.players[0].points, 1);
        game.minions.clear();
        game.step(
            &[(
                0,
                Input {
                    pitch: -0.2,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut Vec::new(),
        );
        assert_eq!(game.players[0].ranks[1], 2);
        assert_eq!(game.players[0].points, 0);
    }
    #[test]
    fn a_minion_breakthrough_damages_the_crown_without_awarding_farm_xp() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 4);
        game.time = CROWN_SHIELD_END;
        game.players[1].selected = true;
        game.players[1].runner.x = 38.0;
        game.players[1].runner.z = LANE_Z[0];
        game.minions.push(Minion {
            team: 0,
            lane: 0,
            x: 37.99,
            z: LANE_Z[0],
            hp: 80.0,
            attack: 0.0,
        });
        let mut events = Vec::new();
        game.step_minions(0.1, &mut events);
        assert_eq!(game.crown_hp[1], CROWN_HP - 4.0);
        assert_eq!(game.team_push[0], 8.0);
        assert_eq!(game.players[1].xp, 0);
        assert!(game.minions.is_empty());
        assert!(events.iter().any(|event| event["kind"] == "pet_crown_hit"));
    }
    #[test]
    fn crown_health_decides_a_timed_match() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 4);
        game.crown_hp = [120.0, 80.0];
        game.finish();
        assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
        game.result = None;
        game.crown_hp = [120.0, 120.0];
        game.finish();
        assert_eq!(game.placements(), Some(vec![vec![0, 1]]));
    }
    #[test]
    fn mana_blocks_cast_until_it_regenerates_and_sparring_resets() {
        let contestants = [
            Contestant::person(0).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ];
        let mut game = PetArena::new(1, &contestants, 4);
        game.players[0].selected = true;
        game.players[0].runner.x = -36.0;
        game.players[0].runner.z = 5.0;
        game.players[0].mana = 0.0;
        game.cast(0, 0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        assert_eq!(game.spar[0].hp, 150.0);
        game.players[0].mana = 120.0;
        game.cast(0, 0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        assert!(game.spar[0].hp < 150.0);
        game.time = DRAFT - 0.01;
        game.step(&[], 0.02, &mut Vec::new());
        assert_eq!(game.players[0].mana, MAX_MANA);
        assert_eq!(game.players[0].hp, MAX_HP);
        assert_eq!(game.players[0].cooldown, [0.0; 4]);
    }
    #[test]
    fn dragon_breath_warns_then_burns_pets_near_the_center() {
        let mut game = PetArena::new(
            1,
            &[
                Contestant::bot(0, 0.7).on_team(0),
                Contestant::bot(1, 0.7).on_team(1),
            ],
            3,
        );
        game.time = DRAGON_WAKE + 1.0;
        game.dragon_breath = 0.01;
        game.players[0].runner.x = 1.0;
        game.players[0].runner.z = 0.0;
        let mut events = Vec::new();
        game.step(&[], 1.0 / 60.0, &mut events);
        assert!(game.players[0].hp < MAX_HP);
        assert_eq!(game.dragon_breath, 6.0);
        assert!(events.iter().any(|e| e["kind"] == "pet_dragon_breath"));
    }
    #[test]
    fn both_arena_modes_finish_with_active_bots() {
        for kind in 0..2 {
            let contestants = (0..4)
                .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
                .collect::<Vec<_>>();
            let mut game = PetArena::new(kind, &contestants, 11);
            let mut events = Vec::new();
            for _ in 0..(LIMIT as usize * 60 + 2) {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut events);
            }
            assert_eq!(game.placements().unwrap().iter().flatten().count(), 4);
            assert!(
                events.iter().any(|event| event["kind"]
                    == if kind == 0 {
                        "pet_ring_hit"
                    } else {
                        "pet_arena_hit"
                    }),
                "{}",
                game.key()
            );
            println!(
                "{} scores {:?}",
                game.key(),
                game.players.iter().map(|p| p.score).collect::<Vec<_>>()
            );
        }
    }
    #[test]
    fn pet_arena_balance_audit() {
        for kind in 0..2 {
            for count in [2, 4, 8] {
                let mut duel_winners = [false; 2];
                for seed in 0..4 {
                    let contestants = (0..count)
                        .map(|seat| {
                            let mut c = Contestant::bot(seat, 0.7).on_team(seat % 2);
                            c.body_width = [0.7, 0.9, 1.2, 1.4][seat as usize % 4];
                            c
                        })
                        .collect::<Vec<_>>();
                    let mut game = PetArena::new(kind, &contestants, seed);
                    let mut events = Vec::new();
                    for _ in 0..(LIMIT as usize * 60 + 120) {
                        let inputs = contestants
                            .iter()
                            .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                            .collect::<Vec<_>>();
                        game.step(&inputs, 1.0 / 60.0, &mut events);
                        if game.placements().is_some() {
                            break;
                        }
                    }
                    println!(
                        "{} count={count} seed={seed} time={:.1} score={:?} personal={:?} stocks={:?} dragon={:?} crown={:?} relays={:?} ring_hits={} ring_kos={} hits={} placements={:?}",
                        game.key(),
                        game.time,
                        game.team_score,
                        game.players.iter().map(|p| p.score).collect::<Vec<_>>(),
                        game.players.iter().map(|p| p.stocks).collect::<Vec<_>>(),
                        game.dragon_claims,
                        game.crown_hp,
                        game.relay_owner,
                        events
                            .iter()
                            .filter(|e| e["kind"] == "pet_ring_hit")
                            .count(),
                        events.iter().filter(|e| e["kind"] == "pet_ring_ko").count(),
                        events
                            .iter()
                            .filter(|e| e["kind"] == "pet_dragon_hit")
                            .count(),
                        game.placements()
                    );
                    let placements = game.placements().unwrap();
                    assert_eq!(placements.iter().flatten().count(), count as usize);
                    if kind == 0 {
                        assert!(events.iter().any(|e| e["kind"] == "pet_ring_hit"));
                        assert!(events.iter().any(|e| e["kind"] == "pet_ring_ko"));
                    } else {
                        assert!(events.iter().any(|e| e["kind"] == "pet_relay_change"));
                        assert!(
                            events
                                .iter()
                                .any(|e| e["kind"] == "pet_dragon_hit"
                                    || e["kind"] == "pet_crown_hit")
                        );
                    }
                    if count == 2 {
                        duel_winners[placements[0][0] as usize] = true;
                    }
                }
                if kind == 0 && count == 2 {
                    assert_eq!(duel_winners, [true, true]);
                }
            }
        }
    }
    #[test]
    #[ignore]
    fn quick_match_pacing_report() {
        let only = std::env::var("CROWNFALL_PACING_ONLY")
            .ok()
            .and_then(|value| value.parse::<u8>().ok());
        for count in [2, 4, 6, 8]
            .into_iter()
            .filter(|&count| only.is_none_or(|wanted| wanted == count))
        {
            let mut times = Vec::new();
            let mut endings = [0u32; 2];
            let mut first = [0u32; 2];
            let mut direct_crown_hits = 0u32;
            for seed in 0..128u64 {
                let contestants = (0..count)
                    .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
                    .collect::<Vec<_>>();
                let mut game = PetArena::new(1, &contestants, seed + 100);
                let mut events = Vec::new();
                while game.placements().is_none() {
                    let inputs = contestants
                        .iter()
                        .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1.0 / 60.0, &mut events);
                }
                times.push(game.time);
                direct_crown_hits += events
                    .iter()
                    .filter(|e| e["kind"] == "pet_crown_hit" && e["seat"].is_number())
                    .count() as u32;
                let ending = if game.crown_hp.iter().any(|&hp| hp <= 0.0) {
                    0
                } else {
                    1
                };
                endings[ending] += 1;
                if ending == 0 {
                    let winner = game
                        .players
                        .iter()
                        .find(|p| p.seat == game.placements().unwrap()[0][0])
                        .unwrap()
                        .team as usize;
                    first[winner] += 1;
                }
            }
            times.sort_by(|a, b| a.total_cmp(b));
            println!(
                "Crownfall {count} players: median={:.1}s p90={:.1}s endings(crown,draw)={endings:?} wins={first:?} direct_crown_hits={direct_crown_hits}",
                times[64], times[115]
            );
        }
        for count in [4, 6, 8]
            .into_iter()
            .filter(|&count| only.is_none_or(|wanted| wanted == count))
        {
            for flipped in [false, true] {
                let mut side = [0u32; 2];
                let mut seat_parity = [0u32; 2];
                for seed in 0..64u64 {
                    let contestants = (0..count)
                        .map(|seat| {
                            Contestant::bot(seat, 0.7).on_team((seat % 2) ^ u8::from(flipped))
                        })
                        .collect::<Vec<_>>();
                    let mut game = PetArena::new(1, &contestants, seed + 100);
                    let mut events = Vec::new();
                    while game.placements().is_none() {
                        let inputs = contestants
                            .iter()
                            .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                            .collect::<Vec<_>>();
                        game.step(&inputs, 1.0 / 60.0, &mut events);
                    }
                    let winner = game.placements().unwrap()[0][0];
                    side[(winner % 2 ^ u8::from(flipped)) as usize] += 1;
                    seat_parity[(winner % 2) as usize] += 1;
                }
                println!(
                    "Crownfall {count}p flipped={flipped}: side={side:?} seat_parity={seat_parity:?}"
                );
            }
        }
    }
    #[test]
    fn bots_choose_different_champions_within_their_combat_role() {
        for role in 0..4 {
            let choices = (0..10)
                .map(|seed| bot_champion(role, seed))
                .collect::<Vec<_>>();
            assert!(choices.iter().all(|&champ| CHAMPION_ROLE[champ] == role));
            assert!(choices.iter().any(|&champ| champ != choices[0]));
        }
    }
}
