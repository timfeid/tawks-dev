//! A pet ringout bout and an avatar led Crownfall match. The room owns every hit, cooldown, and score.
use serde_json::{Value, json};

use crate::bout::{ArenaTarget, BodyProfile, Bout, Contestant, Input, Stick};
use crate::ground::{Runner, step_runner};

pub const KEYS: [&str; 2] = ["pet_ringout", "pet_teamfight"];
pub const NAMES: [&str; 2] = ["Skyline Scramble", "Crownfall Clash"];
pub const RULES: [&str; 2] = [
    "Launch rivals out of the ring. Small pets move quickly; broad pets hit farther and harder, and resist launches. Charge a heavy strike for a knockout.",
    "Choose a champion and break the enemy Crown. Push pet minions through both side lanes to destroy outer and inner towers, then the last tower before the Crown. Full teams send one flexible top, a bottom carry and support, and a roamer who can gank through jungle passages. Contest the Crown Rift for a team buff.",
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
pub const TEAM_POSITIONS: [&str; 4] = ["Top", "Bottom Carry", "Bottom Support", "Mid Roamer"];
pub const CHAMPION_ROLE: [usize; 10] = [1, 1, 2, 0, 3, 1, 2, 0, 3, 2];
pub const ROLE_DEFAULT: [usize; 4] = [3, 5, 4, 2];
pub const BASIC_RANGE: [f32; 10] = [6.3, 2.9, 6.0, 2.8, 5.5, 8.2, 5.7, 3.0, 5.2, 5.7];
pub fn aimed_range(champion: usize, slot: usize, width: f32) -> f32 {
    let champion = champion % CHAMPIONS.len();
    let reach = BodyProfile::new(width).reach;
    match slot {
        0 => [8.0,4.0,7.0,2.7,7.0,9.0,7.0,2.5,7.0,6.0][champion] * reach,
        1 => if matches!(champion, 0 | 2 | 6) { 6.2 * reach } else { 7.0 * reach },
        2 => [4.5,6.5,3.8,4.0,5.2,5.4,4.3,5.0,5.0,4.1][champion],
        _ => [6.5,3.5,7.5,4.0,6.0,9.0,6.0,4.5,6.5,6.0][champion] * reach,
    }
}
pub const SHOP_ITEMS: [&str; 3] = ["Hourglass", "Renewal Flask", "Wind Charm"];
pub const SHOP_COST: [u16; 3] = [140, 95, 115];
pub fn champion_allowed(position: usize, champion: usize) -> bool {
    match position {
        0 => matches!(champion, 0 | 1 | 3 | 6 | 7),
        1 => matches!(champion, 0 | 5),
        2 => matches!(champion, 4 | 8),
        3 => matches!(champion, 0 | 2 | 6 | 9),
        _ => false,
    }
}
/// Short descriptions match the server's actual targeting and effects.
pub const ABILITY_HELP: [[&str; 4]; 10] = [
    [
        "Orb toward an enemy",
        "Charm and stun",
        "Dash toward your aim",
        "Aim a falling star; it bursts on impact",
    ],
    [
        "Close shadow strike",
        "Brief invulnerability",
        "Long blink",
        "Aim a shadow burst at a vulnerable target",
    ],
    [
        "Bolt toward an enemy",
        "Bind and stun",
        "Short warp",
        "Launch an arc storm that bursts on impact",
    ],
    [
        "Heavy close swipe",
        "Gain a large shield",
        "Charge and hit",
        "Aim a groundbreak shockwave",
    ],
    [
        "Dart toward an enemy",
        "Aim at an ally to mend; self if no ally",
        "Swift dash",
        "Aim a sanctuary burst; heal allies inside",
    ],
    [
        "Long range shot",
        "Trap and stun",
        "Invulnerable roll",
        "Aim an overload blast into the fight",
    ],
    [
        "Shard toward an enemy",
        "Freeze and stun",
        "Invulnerable burrow",
        "Aim a whiteout burst; slow enemies inside",
    ],
    [
        "Heavy close bite",
        "Gain a large shield",
        "Pounce and hit",
        "Aim a solar frenzy burst",
    ],
    [
        "Splash toward an enemy",
        "Aim at an ally to bubble; self if no ally",
        "Surf and hit",
        "Aim a high tide burst; heal allies inside",
    ],
    [
        "Spore toward an enemy",
        "Bomb and stun",
        "Vine step",
        "Aim a wild bloom burst",
    ],
];
pub const DRAFT: f32 = 45.0;
pub const LIMIT: f32 = 360.0;
const TEAM_LIMIT: f32 = 660.0;
const RING_LIMIT: f32 = 60.0;
const RING: f32 = 9.2;
const MAX_HP: f32 = 240.0;
const DRAGON_HP: f32 = 650.0;
pub const DRAGON_WAKE: f32 = 120.0;
const DRAGON_RESPAWN: f32 = 100.0;
pub const CROWN_HP: f32 = 800.0;
pub const CROWN_SHIELD_END: f32 = 90.0;
pub const ARENA_RADIUS: f32 = 50.0;
pub const LANE_Z: [f32; 2] = [31.0, -31.0];
pub const CROWN_POS: [(f32, f32); 2] = [(-39.0, 0.0), (39.0, 0.0)];
pub const RELAYS: [(f32, f32); 2] = [(0.0, LANE_Z[0]), (0.0, LANE_Z[1])];
const RELAY_RADIUS: f32 = 2.7;
const RELAY_PULSE: f32 = 8.0;
const CROWN_COLLAPSE_START: f32 = 540.0;
const DUEL_COLLAPSE_START: f32 = 300.0;
const CROWN_COLLAPSE_PER_SECOND: f32 = 4.0;
pub const BRUSH: [(f32, f32); 4] = [(18.0, 11.5), (18.0, -11.5), (-18.0, 11.5), (-18.0, -11.5)];
// The gaps between these low walls are the walking entrances to the jungle.
// Champion dashes can vault a wall, but ordinary movement cannot pass through it.
pub const JUNGLE_WALLS: [(f32, f32, f32, f32); 6] = [
    (-27.0, -18.0, 18.5, 21.0), (-8.0, 8.0, 18.5, 21.0), (18.0, 27.0, 18.5, 21.0),
    (-27.0, -18.0, -21.0, -18.5), (-8.0, 8.0, -21.0, -18.5), (18.0, 27.0, -21.0, -18.5),
];
pub const PORTALS: [(f32, f32); 2] = [(-12.0, 0.0), (12.0, 0.0)];
pub const RIFT_UNLOCK: f32 = DRAFT + 300.0;
const PORTAL_WAIT: f32 = 5.0;
const RIFT_COUNTDOWN: f32 = 10.0;
const MAX_LEVEL: u8 = 9;
const MAX_MANA: f32 = 120.0;
const MINION_WAVE: f32 = 11.0;

#[derive(Clone, Debug)]
struct Minion {
    id: u32,
    team: u8,
    lane: usize,
    x: f32,
    z: f32,
    hp: f32,
    attack: f32,
}
#[derive(Clone, Debug)]
struct Tower {
    team: u8,
    lane: Option<usize>,
    tier: u8, // 0 outer, 1 inner, 2 last tower before the Crown
    x: f32,
    z: f32,
    hp: f32,
    max_hp: f32,
    shot: f32,
}
#[derive(Clone, Debug)]
struct SparBot {
    x: f32,
    z: f32,
    hp: f32,
    shot: f32,
}
#[derive(Clone, Debug)]
struct Skillshot {
    owner: usize,
    slot: u8,
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    remaining: f32,
    damage: f32,
    stun: f32,
    radius: f32,
    lock: Option<ShotLock>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AutoTarget {
    Minion(usize),
    Champion(usize),
    Spar(usize),
}
#[derive(Clone, Copy, Debug)]
enum ShotLock { Champion(u8), Minion(u32), Spar(usize) }

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideKind { Portal, Rift }
struct SideBout {
    kind: SideKind,
    game: Box<dyn Bout>,
    seats: Vec<u8>,
}
struct PortalQueue {
    seat: u8,
    remaining: f32,
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
    auto_cd: f32,
    arena_target: Option<ArenaTarget>,
    gank_target: Option<u8>,
    gank_until: f32,
    last_hits: u16,
    gold: u16,
    item: u8,
    item_cd: f32,
    stasis: f32,
    haste: f32,
    held_item_key: bool,
    recall: f32,
    busy: bool,
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
    next_minion_id: u32,
    towers: Vec<Tower>,
    minion_wave: f32,
    spar: [SparBot; 2],
    skillshots: Vec<Skillshot>,
    result: Option<Vec<Vec<u8>>>,
    portal_queue: Option<PortalQueue>,
    portal_cd: f32,
    side_bout: Option<SideBout>,
    rift_countdown: Option<f32>,
    rift_used: bool,
}

fn distance(a: &Fighter, b: &Fighter) -> f32 {
    (a.runner.x - b.runner.x).hypot(a.runner.z - b.runner.z)
}
fn direction(yaw: f32) -> (f32, f32) {
    (yaw.sin(), yaw.cos())
}
fn inside_jungle_wall(x: f32, z: f32, margin: f32) -> bool {
    JUNGLE_WALLS.iter().any(|&(x0, x1, z0, z1)|
        x > x0 - margin && x < x1 + margin && z > z0 - margin && z < z1 + margin)
}
fn bot_path(p: &Fighter, target: (f32, f32)) -> (f32, f32) {
    let x = p.runner.x;
    let z = p.runner.z;
    let gate_x = if x < 0.0 { -13.0 } else { 13.0 };
    let crossing_out = z.abs() > 21.0
        && (target.1.abs() < 17.0 || z.signum() != target.1.signum());
    if crossing_out {
        if (x-gate_x).abs() > 1.8 { (gate_x, z) }
        else { (gate_x, z.signum() * 16.0) }
    } else if z.abs() < 18.0 && target.1.abs() > 21.0 {
        if (x-gate_x).abs() > 1.8 { (gate_x, z) }
        else { (gate_x, target.1.signum() * 23.0) }
    } else { target }
}
fn initial_towers(player_count: usize) -> Vec<Tower> {
    let mut towers = Vec::with_capacity(10);
    let durability = match player_count { 0..=2 => 0.35, 3..=4 => 0.55,
        5..=6 => 0.75, _ => 5.0 };
    let final_durability = if player_count >= 8 { 3.0 } else { durability };
    for team in 0..2u8 {
        let sign = if team == 0 { -1.0 } else { 1.0 };
        for lane in 0..2 {
            for tier in 0..2u8 {
                let max_hp = (if tier == 0 { 340.0 } else { 470.0 }) * durability;
                towers.push(Tower { team, lane: Some(lane), tier,
                    x: sign * if tier == 0 { 16.0 } else { 29.0 }, z: LANE_Z[lane],
                    hp: max_hp, max_hp, shot: 0.0 });
            }
        }
        towers.push(Tower { team, lane: None, tier: 2,
            x: sign * 34.0, z: 0.0, hp: 620.0 * final_durability,
            max_hp: 620.0 * final_durability, shot: 0.0 });
    }
    towers
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
fn bot_champion_for_position(position: usize, seed: u64) -> usize {
    let choices: &[usize] = match position {
        0 => &[1, 3, 7],
        1 => &[0, 5],
        2 => &[4, 8],
        _ => &[0, 2, 6, 9],
    };
    choices[(seed as usize).wrapping_add(position) % choices.len()]
}
impl PetArena {
    fn start_side_bout(&mut self, kind: SideKind, seats: Vec<u8>, events: &mut Vec<Value>) {
        let key = if kind == SideKind::Portal { "bumper" } else { crate::tug::KEY };
        let contestants = seats.iter().filter_map(|&seat| {
            if seat == 8 {
                let team = self.players.iter().find(|p| p.seat == seats[0])?.team ^ 1;
                return Some(Contestant::bot(8, 0.65).on_team(team));
            }
            let p = self.players.iter().find(|p| p.seat == seat)?;
            Some(Contestant {
                seat, is_bot: p.is_bot, skill: p.skill, team: p.team,
                ally: None, body_width: p.width, pet: p.pet,
            })
        }).collect::<Vec<_>>();
        let Some(game) = crate::bout::create(key, &contestants, self.seed ^ self.turn) else { return };
        for p in &mut self.players {
            if seats.contains(&p.seat) { p.busy = true; }
        }
        self.portal_queue = None;
        self.side_bout = Some(SideBout { kind, game, seats: seats.clone() });
        events.push(json!({"kind":if kind == SideKind::Portal {"crownfall_portal_match"} else {"crownfall_rift_match"},
            "game":key,"seats":seats}));
    }

    fn tick_side_bout(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        let Some(side) = self.side_bout.as_mut() else { return };
        let mut bout_inputs = Vec::with_capacity(side.seats.len());
        for &seat in &side.seats {
            let input = if seat == 8 {
                side.game.bot_input(seat, dt)
            } else {
                inputs.iter().find(|(s, _)| *s == seat).map(|(_, input)| *input).unwrap_or_default()
            };
            bout_inputs.push((seat, input));
        }
        side.game.step(&bout_inputs, dt, events);
        let Some(placements) = side.game.placements() else { return };
        let side = self.side_bout.take().unwrap();
        let winners = placements.first().cloned().unwrap_or_default();
        for &seat in &side.seats {
            let Some(index) = self.players.iter().position(|p| p.seat == seat) else { continue };
            self.players[index].busy = false;
            self.players[index].invuln = self.players[index].invuln.max(0.8);
            if side.kind == SideKind::Portal {
                let won = winners.contains(&seat);
                self.gain_xp(index, if won { 80 } else { 20 }, events);
                self.players[index].gold = self.players[index].gold.saturating_add(if won { 45 } else { 10 });
            } else {
                let won = winners.iter().any(|winner| self.players.iter().any(|p| p.seat == *winner && p.team == self.players[index].team));
                self.gain_xp(index, if won { 35 } else { 15 }, events);
                self.players[index].gold = self.players[index].gold.saturating_add(if won { 40 } else { 15 });
            }
        }
        if side.kind == SideKind::Rift {
            if let Some(team) = winners.iter().find_map(|seat| self.players.iter().find(|p| p.seat == *seat).map(|p| p.team)) {
                self.team_buff[team as usize] = 60.0;
                self.dragon_claims[team as usize] = self.dragon_claims[team as usize].saturating_add(1);
                events.push(json!({"kind":"crownfall_rift_result","team":team,"buff":60}));
            }
        } else {
            self.portal_cd = 30.0;
            events.push(json!({"kind":"crownfall_portal_result","winners":winners}));
        }
    }

    fn side_events(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.kind != 1 || self.time < DRAFT { return; }
        self.portal_cd = (self.portal_cd - dt).max(0.0);
        self.tick_side_bout(inputs, dt, events);
        if self.side_bout.is_some() { return; }
        if let Some(remaining) = self.rift_countdown.as_mut() {
            *remaining -= dt;
            if *remaining <= 0.0 {
                self.rift_countdown = None;
                let seats = self.players.iter()
                    .filter(|p| p.selected && p.respawn <= 0.0 && p.runner.x.hypot(p.runner.z) < 9.0)
                    .map(|p| p.seat).collect::<Vec<_>>();
                let both = [0, 1].into_iter().all(|team| seats.iter().any(|seat| self.players.iter().any(|p| p.seat == *seat && p.team == team)));
                if both {
                    self.start_side_bout(SideKind::Rift, seats, events);
                } else if let Some(team) = seats.first().and_then(|seat| self.players.iter().find(|p| p.seat == *seat).map(|p| p.team)) {
                    self.team_buff[team as usize] = 60.0;
                    self.dragon_claims[team as usize] = self.dragon_claims[team as usize].saturating_add(1);
                    events.push(json!({"kind":"crownfall_rift_result","team":team,"buff":60,"uncontested":true}));
                } else {
                    self.rift_used = false;
                    events.push(json!({"kind":"crownfall_rift_cancelled"}));
                }
            }
            return;
        }
        if !self.rift_used && self.time >= RIFT_UNLOCK {
            if inputs.iter().any(|(seat, input)| input.pitch >= 1.35 && self.players.iter().any(|p| p.seat == *seat && p.selected && p.respawn <= 0.0 && p.runner.x.hypot(p.runner.z) < 3.2)) {
                self.rift_used = true;
                self.rift_countdown = Some(RIFT_COUNTDOWN);
                self.portal_queue = None;
                events.push(json!({"kind":"crownfall_rift_started","seconds":RIFT_COUNTDOWN}));
                return;
            }
        }
        if self.portal_cd > 0.0 { return; }
        if self.portal_queue.as_ref().is_some_and(|queue| {
            !self.players.iter().any(|p| p.seat == queue.seat && p.selected
                && p.respawn <= 0.0 && !p.busy
                && (p.runner.x - PORTALS[p.team as usize].0)
                    .hypot(p.runner.z - PORTALS[p.team as usize].1) < 3.6)
        }) {
            self.portal_queue = None;
        }
        if let Some(queue) = self.portal_queue.as_mut() {
            queue.remaining -= dt;
            if queue.remaining <= 0.0 {
                let seat = queue.seat;
                self.start_side_bout(SideKind::Portal, vec![seat, 8], events);
                return;
            }
        }
        for &(seat, input) in inputs {
            if input.pitch < 1.35 { continue; }
            let Some(p) = self.players.iter().find(|p| p.seat == seat && p.selected && p.respawn <= 0.0 && !p.busy) else { continue };
            let pad = PORTALS[p.team as usize];
            if (p.runner.x - pad.0).hypot(p.runner.z - pad.1) >= 2.8 { continue; }
            if let Some(queue) = &self.portal_queue {
                let first = self.players.iter().find(|p| p.seat == queue.seat);
                if first.is_some_and(|first| first.team != p.team) {
                    self.start_side_bout(SideKind::Portal, vec![queue.seat, seat], events);
                    return;
                }
            } else {
                self.portal_queue = Some(PortalQueue { seat, remaining: PORTAL_WAIT });
                events.push(json!({"kind":"crownfall_portal_queued","seat":seat,"seconds":PORTAL_WAIT}));
                return;
            }
        }
    }

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
            let pace = ((self.time - DRAFT) / 360.0).clamp(0.0, 1.0);
            self.minion_wave += MINION_WAVE - 4.0 * pace;
            for lane in 0..2 {
                for team in 0..2u8 {
                    for rank in 0..3 {
                        self.minions.push(Minion {
                            id: self.next_minion_id,
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
                        self.next_minion_id = self.next_minion_id.wrapping_add(1);
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
        let mut tower_damage = vec![0.0; self.towers.len()];
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
                let tower = self.towers.iter().enumerate().find(|(_, t)|
                    t.team != m.team && t.lane == Some(m.lane) && t.hp > 0.0
                    && (m.x-t.x).hypot(m.z-t.z) < 4.0).map(|(i, _)| i);
                if let Some(tower) = tower {
                    if m.attack == 0.0 {
                        tower_damage[tower] += 22.0;
                        m.attack = 0.9;
                    }
                } else {
                    let speed = 2.4 + 1.5 * ((self.time - DRAFT) / 360.0).clamp(0.0, 1.0);
                    m.x += if m.team == 0 { speed * dt } else { -speed * dt };
                    if m.x.abs() > 38.0 { m.hp = 0.0; }
                }
            }
        }
        for (index, amount) in tower_damage.into_iter().enumerate() {
            if amount > 0.0 { self.hit_tower(index, amount, events); }
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
                if p.team != team && p.selected && !p.busy && (p.runner.x - x).hypot(p.runner.z - z) < 12.0 {
                    let support = p.position == Some(2);
                    self.gain_xp(index, if support { 16 } else { 10 }, events);
                    self.players[index].gold = self.players[index].gold.saturating_add(if support { 8 } else { 4 });
                }
            }
        }
        self.minions.retain(|m| m.hp > 0.0);
        // Lane pressure opens the tower chain. The Crown is never damaged by
        // minions that merely reach the edge of the map.
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
            self.players[index].last_hits =
                self.players[index].last_hits.saturating_add(kills as u16);
            self.players[index].gold = self.players[index].gold.saturating_add((kills * 25) as u16);
            self.players[index].mana = (self.players[index].mana + kills as f32 * 8.0)
                .min(Self::max_mana(self.players[index].level));
            self.gain_xp(index, kills * 22, events);
            events.push(
                json!({"kind":"pet_minion_farm","seat":self.players[index].seat,
                "count":kills,"xp":kills*22,"mana":kills*8,"x":x,"z":z}),
            );
        }
    }
    fn auto_target(&self, index: usize) -> Option<AutoTarget> {
        let p = &self.players[index];
        if self.kind != 1 || !p.selected || p.respawn > 0.0 || p.busy {
            return None;
        }
        let (x, z, team) = (p.runner.x, p.runner.z, p.team);
        let range = BASIC_RANGE[p.champ] * BodyProfile::new(p.width).reach;
        if self.time < DRAFT {
            if p.is_bot { return None; }
            let spar = &self.spar[team as usize];
            return ((spar.x-x).hypot(spar.z-z) < range).then_some(AutoTarget::Spar(team as usize));
        }
        if let Some(ArenaTarget::Champion { seat }) = p.arena_target {
            return self.players.iter().enumerate().find(|(i, other)| {
                *i != index && other.seat == seat && other.team != team && other.selected
                    && other.respawn <= 0.0 && !other.busy && !self.hidden_from(index,*i)
                    && (other.runner.x-x).hypot(other.runner.z-z) < range
            }).map(|(i,_)| AutoTarget::Champion(i));
        } else if let Some(ArenaTarget::Minion { id }) = p.arena_target {
            return self.minions.iter().enumerate().find(|(_,m)|
                m.id == id && m.team != team && m.hp > 0.0
                    && (m.x-x).hypot(m.z-z) < range
            ).map(|(i,_)| AutoTarget::Minion(i));
        }
        let minion = self
            .minions
            .iter()
            .enumerate()
            .filter(|(_, m)| m.team != team && m.hp > 0.0 && (m.x - x).hypot(m.z - z) < range)
            .min_by(|(_, a), (_, b)| {
                (a.x - x)
                    .hypot(a.z - z)
                    .total_cmp(&(b.x - x).hypot(b.z - z))
            })
            .map(|(i, _)| i);
        if let Some(target) = minion {
            return Some(AutoTarget::Minion(target));
        }
        self.players
            .iter()
            .enumerate()
            .filter(|(i, other)| {
                *i != index
                    && other.team != team
                    && other.selected
                    && other.respawn <= 0.0
                    && !other.busy
                    && !self.hidden_from(index, *i)
                    && (other.runner.x - x).hypot(other.runner.z - z) < range
            })
            .min_by(|(_, a), (_, b)| distance(p, a).total_cmp(&distance(p, b)))
            .map(|(i, _)| AutoTarget::Champion(i))
    }
    fn auto_attack(&mut self, index: usize, events: &mut Vec<Value>) {
        if self.players[index].auto_cd > 0.0 || self.players[index].lock > 0.0
            || self.players[index].stasis > 0.0 {
            return;
        }
        let Some(target) = self.auto_target(index) else {
            let p = &self.players[index];
            if p.arena_target.is_some() { return; }
            let range = BASIC_RANGE[p.champ] * BodyProfile::new(p.width).reach;
            if let Some(tower) = self.towers.iter().enumerate().find(|(i, t)|
                t.team != p.team && t.hp > 0.0 && self.tower_open(*i)
                && (t.x-p.runner.x).hypot(t.z-p.runner.z) <= range)
                .map(|(i, _)| i) {
                self.players[index].auto_cd = 1.0;
                self.players[index].cast = 4;
                self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
                self.hit_tower(tower, 19.0, events);
            }
            return;
        };
        let interval = 1.1 - 0.35 * ((self.time - DRAFT) / 300.0).clamp(0.0, 1.0);
        let seat = self.players[index].seat;
        if self.players[index].champ == 5 {
            let (x,z,lock) = match target {
                AutoTarget::Spar(team) => (self.spar[team].x,self.spar[team].z,ShotLock::Spar(team)),
                AutoTarget::Minion(i) => (self.minions[i].x,self.minions[i].z,ShotLock::Minion(self.minions[i].id)),
                AutoTarget::Champion(i) => (self.players[i].runner.x,self.players[i].runner.z,
                    ShotLock::Champion(self.players[i].seat)),
            };
            let p = &self.players[index];
            let yaw = (x-p.runner.x).atan2(z-p.runner.z);
            let range = BASIC_RANGE[5] * BodyProfile::new(p.width).reach;
            self.launch_skillshot(index, 4, yaw, range, 15.0, 0.0);
            if let Some(shot) = self.skillshots.last_mut() { shot.lock = Some(lock); }
            self.players[index].auto_cd = 0.62 - 0.14 * ((self.time-DRAFT)/300.0).clamp(0.0,1.0);
            self.players[index].cast = 4;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            return;
        }
        match target {
          AutoTarget::Spar(team) => {
            let bot = &mut self.spar[team];
            bot.hp = (bot.hp - 24.0).max(0.0);
            self.players[index].auto_cd = 0.9;
            self.players[index].cast = 4;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            events.push(json!({"kind":"pet_arena_auto","seat":seat,
                "x":bot.x,"z":bot.z,"target":"spar"}));
            if bot.hp == 0.0 { bot.hp = 150.0; }
          }
          AutoTarget::Minion(target) => {
            let (tx, tz) = (self.minions[target].x, self.minions[target].z);
            self.minions[target].hp -= 24.0;
            self.players[index].auto_cd = interval;
            self.players[index].cast = 4;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            events.push(json!({"kind":"pet_arena_auto","seat":seat,
                "x":tx,"z":tz,"target":"minion"}));
            if self.minions[target].hp <= 0.0 {
                self.players[index].last_hits = self.players[index].last_hits.saturating_add(1);
                self.players[index].gold = self.players[index].gold.saturating_add(25);
                self.players[index].mana =
                    (self.players[index].mana + 8.0).min(Self::max_mana(self.players[index].level));
                self.gain_xp(index, 22, events);
                events.push(
                    json!({"kind":"pet_minion_farm","seat":self.players[index].seat,
                    "count":1,"xp":22,"mana":8,"x":tx,"z":tz}),
                );
            }
          }
          AutoTarget::Champion(target) => {
            self.players[index].auto_cd = interval;
            self.players[index].cast = 4;
            self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
            events.push(
                json!({"kind":"pet_arena_auto","seat":self.players[index].seat,
                "x":self.players[target].runner.x,"z":self.players[target].runner.z,
                "target":"champion"}),
            );
            self.hit_team(index, target, 17.0, 0.0, events);
          }
        }
    }
    fn tower_open(&self, index: usize) -> bool {
        let tower = &self.towers[index];
        match tower.tier {
            0 => true,
            1 => !self.towers.iter().any(|other| other.team == tower.team
                && other.lane == tower.lane && other.tier == 0 && other.hp > 0.0),
            _ => !self.towers.iter().any(|other| other.team == tower.team
                && other.tier == 1 && other.hp > 0.0),
        }
    }
    fn hit_tower(&mut self, index: usize, amount: f32, events: &mut Vec<Value>) {
        if self.towers[index].hp <= 0.0 || !self.tower_open(index) { return; }
        let shielded = self.towers[index].tier < 2 && !self.minions.iter().any(|m|
            m.team != self.towers[index].team && m.hp > 0.0
            && (m.x-self.towers[index].x).hypot(m.z-self.towers[index].z) < 8.0);
        let t = &mut self.towers[index];
        t.hp = (t.hp - amount * if shielded { 0.25 } else { 1.0 }).max(0.0);
        events.push(json!({"kind":"crownfall_tower_hit","team":t.team,
            "lane":t.lane,"tier":t.tier,"x":t.x,"z":t.z,"hp":t.hp,"shielded":shielded}));
        if t.hp == 0.0 {
            events.push(json!({"kind":"crownfall_tower_down","team":t.team,
                "lane":t.lane,"tier":t.tier,"x":t.x,"z":t.z}));
        }
    }
    fn damage_towers(&mut self, player: usize, amount: f32, range: f32,
        yaw: f32, cone: f32, events: &mut Vec<Value>) {
        if self.kind != 1 || self.time < DRAFT { return; }
        let p = &self.players[player];
        let (ax, az) = direction(yaw);
        let target = self.towers.iter().enumerate().find(|(i, t)| {
            if t.team == p.team || t.hp <= 0.0 || !self.tower_open(*i) { return false; }
            let (dx, dz) = (t.x - p.runner.x, t.z - p.runner.z);
            let d = dx.hypot(dz);
            d <= range && (dx * ax + dz * az) / d.max(0.01) >= cone
        }).map(|(i, _)| i);
        if let Some(index) = target { self.hit_tower(index, amount, events); }
    }
    fn step_towers(&mut self, dt: f32, events: &mut Vec<Value>) {
        if self.kind != 1 || self.time < DRAFT { return; }
        for index in 0..self.towers.len() {
            if self.towers[index].hp <= 0.0 { continue; }
            self.towers[index].shot = (self.towers[index].shot - dt).max(0.0);
            if self.towers[index].shot > 0.0 { continue; }
            let (team, x, z) = { let t=&self.towers[index]; (t.team,t.x,t.z) };
            if let Some(minion) = self.minions.iter().position(|m| m.team != team
                && m.hp > 0.0 && (m.x-x).hypot(m.z-z) < 7.2) {
                self.minions[minion].hp -= 42.0;
                self.towers[index].shot = 1.35;
                events.push(json!({"kind":"crownfall_tower_shot","x":x,"z":z,
                    "target_x":self.minions[minion].x,"target_z":self.minions[minion].z,"team":team}));
            } else if let Some(champ) = self.players.iter().position(|p| p.team != team
                && p.selected && p.respawn <= 0.0 && !p.busy
                && p.invuln <= 0.0 && p.stasis <= 0.0
                && (p.runner.x-x).hypot(p.runner.z-z) < 7.2) {
                let victim = &mut self.players[champ];
                victim.hp = (victim.hp - 34.0).max(0.0);
                if victim.hp == 0.0 {
                    victim.respawn = 3.5;
                    victim.lock = 0.0;
                    events.push(json!({"kind":"pet_arena_ko","victim":victim.seat,
                        "x":victim.runner.x,"z":victim.runner.z,"tower":true}));
                }
                self.towers[index].shot = 1.35;
                events.push(json!({"kind":"crownfall_tower_shot","x":x,"z":z,
                    "target_x":self.players[champ].runner.x,"target_z":self.players[champ].runner.z,"team":team}));
            }
        }
    }
    fn crown_open(&self, team: usize) -> bool {
        self.kind == 1
            && !self.towers.iter().any(|t| t.team as usize == 1-team && t.hp > 0.0)
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
                let slot = if kind == 1 {
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
                    .then_some(slot);
                let champ = if kind == 1 {
                    if c.is_bot {
                        position.map_or_else(|| bot_champion(slot, seed), |pos| bot_champion_for_position(pos, seed))
                    } else {
                        ROLE_DEFAULT[slot]
                    }
                } else {
                    index % CHAMPIONS.len()
                };
                let role = if kind == 1 { CHAMPION_ROLE[champ] } else { 0 };
                let (x, z) = spawn(index, count, team, kind, role, seed);
                Fighter {
                    seat: c.seat,
                    is_bot: c.is_bot,
                    team,
                    pet: c.pet,
                    width: c.body_width,
                    skill: c.skill.clamp(0.0, 1.0),
                    runner: Runner::at(x, z).with_width(c.body_width),
                    champ,
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
                    auto_cd: 0.0,
                    arena_target: None,
                    gank_target: None,
                    gank_until: 0.0,
                    last_hits: 0,
                    gold: 50,
                    item: 0,
                    item_cd: 0.0,
                    stasis: 0.0,
                    haste: 0.0,
                    held_item_key: false,
                    recall: 0.0,
                    busy: false,
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
            next_minion_id: 1,
            towers: initial_towers(count),
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
            skillshots: Vec::new(),
            result: None,
            portal_queue: None,
            portal_cd: 0.0,
            side_bout: None,
            rift_countdown: None,
            rift_used: false,
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
                if p.selected && !p.busy
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
        let pressure = if self.players.len() == 4 { 0.08 } else { 0.06 };
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
    fn gank_score(&self, index: usize, enemy_index: usize) -> Option<f32> {
        let roamer = &self.players[index];
        if self.kind != 1 || roamer.position != Some(3)
            || self.time < DRAFT + 25.0
            || (self.time >= RIFT_UNLOCK - 20.0 && !self.rift_used)
            || roamer.respawn > 0.0 || roamer.busy
            || roamer.hp < Self::max_hp(roamer.level) * 0.55 || roamer.mana < 20.0 {
            return None;
        }
        let enemy = &self.players[enemy_index];
        if enemy.team == roamer.team || !enemy.selected || enemy.respawn > 0.0
            || enemy.busy || self.hidden_from(index, enemy_index)
            || (enemy.runner.z.abs() - LANE_Z[0]).abs() > 7.0
            || distance(roamer, enemy) > 40.0 { return None; }
        let allies = self.players.iter().filter(|ally| ally.team == roamer.team
                && ally.seat != roamer.seat && ally.selected && ally.respawn <= 0.0
                && ally.hp > Self::max_hp(ally.level) * 0.4
                && distance(ally, enemy) < 15.0).count();
        if allies == 0 { return None; }
        let backup = self.players.iter().filter(|other| other.team == enemy.team
                && other.seat != enemy.seat && other.respawn <= 0.0
                && distance(other, enemy) < 12.0).count();
        if backup > allies { return None; }
        let tower_cover = self.towers.iter().any(|tower| tower.team == enemy.team
                && tower.hp > 0.0 && (tower.x-enemy.runner.x).hypot(tower.z-enemy.runner.z) < 8.0);
        if tower_cover { return None; }
        let vulnerable = 1.0 - enemy.hp / Self::max_hp(enemy.level);
        let overextended = if (roamer.team == 0 && enemy.runner.x < -4.0)
                || (roamer.team == 1 && enemy.runner.x > 4.0) { 0.75 } else { 0.0 };
        let travel = distance(roamer, enemy);
        Some(vulnerable * 1.5 + overextended + allies.min(2) as f32 * 0.5
            - backup as f32 * 0.45 - (travel / 60.0))
    }
    fn best_gank(&self, index: usize) -> Option<usize> {
        self.players.iter().enumerate()
            .filter_map(|(enemy, _)| self.gank_score(index, enemy)
                .filter(|score| *score >= 0.95).map(|score| (enemy, score)))
            .max_by(|a,b| a.1.total_cmp(&b.1)).map(|(index,_)| index)
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
                    && !p.busy
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
    fn aimed_ally(&self, index: usize, yaw: f32, range: f32) -> Option<usize> {
        let (dx,dz) = direction(yaw);
        let caster = &self.players[index];
        self.players.iter().enumerate().filter(|(i,p)| *i != index
            && p.team == caster.team && p.selected && p.respawn <= 0.0)
            .filter_map(|(i,p)| {
                let (x,z) = (p.runner.x-caster.runner.x,p.runner.z-caster.runner.z);
                let dist = x.hypot(z);
                (dist <= range && (x*dx+z*dz)/dist.max(0.01) >= 0.82)
                    .then_some((i,dist))
            })
            .min_by(|a,b| a.1.total_cmp(&b.1)).map(|(i,_)| i)
    }
    fn hit_brawl(&mut self, from: usize, to: usize, power: f32, yaw: f32, events: &mut Vec<Value>) {
        if self.players[to].invuln > 0.0 || self.players[to].respawn > 0.0
            || self.players[to].stasis > 0.0 {
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
        if self.players[to].invuln > 0.0 || self.players[to].respawn > 0.0
            || self.players[to].stasis > 0.0 || self.players[to].busy {
            return;
        }
        let attacker = self.players[from].seat;
        let team = self.players[from].team;
        let rank = self.players[from].ranks.get(self.players[from].cast as usize)
            .copied().unwrap_or(1);
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
        victim.recall = 0.0;
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
            self.players[from].gold = self.players[from].gold.saturating_add(75);
            for ally in 0..self.players.len() {
                if ally != from && self.players[ally].team == team
                    && self.players[ally].selected && self.players[ally].respawn <= 0.0
                    && (self.players[ally].runner.x - victim_at.0)
                        .hypot(self.players[ally].runner.z - victim_at.1) < 9.0
                {
                    self.gain_xp(ally, 20, events);
                    self.players[ally].gold = self.players[ally].gold.saturating_add(30);
                }
            }
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
    fn launch_skillshot(&mut self, owner: usize, slot: u8, yaw: f32,
        range: f32, damage: f32, stun: f32) {
        let (dx, dz) = direction(yaw);
        let p = &self.players[owner];
        self.skillshots.push(Skillshot {
            owner, slot, x: p.runner.x + dx * 0.8, z: p.runner.z + dz * 0.8,
            vx: dx * if slot == 4 { 25.0 } else if slot == 3 { 12.0 }
                else if slot == 0 { 18.0 } else { 15.0 },
            vz: dz * if slot == 4 { 25.0 } else if slot == 3 { 12.0 }
                else if slot == 0 { 18.0 } else { 15.0 },
            remaining: (range - 0.8).max(0.1), damage, stun,
            radius: if slot == 4 { 0.38 } else if slot == 0 { 0.72 } else { 0.9 },
            lock: None,
        });
    }
    fn detonate_skillshot(&mut self, shot: &Skillshot, events: &mut Vec<Value>) {
        let team = self.players[shot.owner].team;
        let champ = self.players[shot.owner].champ;
        let radius = if matches!(champ, 2 | 6 | 8) { 3.7 } else { 3.0 };
        let victims = self.players.iter().enumerate().filter(|(index,p)|
            *index != shot.owner && p.team != team && p.selected && p.respawn <= 0.0
            && (p.runner.x-shot.x).hypot(p.runner.z-shot.z) < radius)
            .map(|(index,_)| index).collect::<Vec<_>>();
        for victim in victims {
            self.hit_team(shot.owner, victim, shot.damage,
                if matches!(champ, 2 | 6 | 9) { 0.8 } else { 0.25 }, events);
        }
        if self.time < DRAFT {
            let spar = &mut self.spar[team as usize];
            if (spar.x-shot.x).hypot(spar.z-shot.z) < radius {
                spar.hp = (spar.hp-shot.damage).max(0.0);
                if spar.hp == 0.0 { spar.hp = 150.0; }
            }
        } else {
            let mut kills = 0u16;
            for minion in &mut self.minions {
                if minion.team != team && minion.hp > 0.0
                    && (minion.x-shot.x).hypot(minion.z-shot.z) < radius {
                    minion.hp -= shot.damage;
                    if minion.hp <= 0.0 { kills += 1; }
                }
            }
            if kills > 0 {
                self.players[shot.owner].last_hits = self.players[shot.owner].last_hits.saturating_add(kills);
                self.players[shot.owner].gold = self.players[shot.owner].gold.saturating_add(kills*25);
                self.gain_xp(shot.owner, kills*22, events);
            }
            if let Some(index) = self.towers.iter().enumerate().find(|(index,t)|
                t.team != team && t.hp > 0.0 && self.tower_open(*index)
                && (t.x-shot.x).hypot(t.z-shot.z) < radius).map(|(index,_)| index) {
                self.hit_tower(index, shot.damage, events);
            }
            if self.crown_open(team as usize) {
                let crown = CROWN_POS[1-team as usize];
                if (crown.0-shot.x).hypot(crown.1-shot.z) < radius {
                    let pressure = if self.players.len() == 4 { 0.08 } else { 0.06 };
                    let damage = shot.damage * pressure
                        * if self.team_buff[team as usize] > 0.0 { 1.25 } else { 1.0 };
                    self.crown_hp[1-team as usize] = (self.crown_hp[1-team as usize]-damage).max(0.0);
                    events.push(json!({"kind":"pet_crown_hit","seat":self.players[shot.owner].seat,
                        "team":team,"target":1-team,"hp":self.crown_hp[1-team as usize],
                        "damage":damage,"x":crown.0,"z":crown.1}));
                }
            }
        }
        if matches!(champ, 4 | 8) {
            for ally in &mut self.players {
                if ally.team == team && ally.selected && ally.respawn <= 0.0
                    && (ally.runner.x-shot.x).hypot(ally.runner.z-shot.z) < radius + 1.5 {
                    ally.hp = (ally.hp+75.0).min(Self::max_hp(ally.level));
                    ally.shield = (ally.shield+35.0).min(150.0);
                }
            }
        }
        events.push(json!({"kind":"pet_arena_blast","seat":self.players[shot.owner].seat,
            "champ":champ,"slot":shot.slot,"x":shot.x,"z":shot.z,"radius":radius}));
    }
    fn step_skillshots(&mut self, dt: f32, events: &mut Vec<Value>) {
        if self.kind != 1 { return; }
        let mut active = Vec::new();
        for mut shot in std::mem::take(&mut self.skillshots) {
            let speed = shot.vx.hypot(shot.vz);
            let distance = (speed * dt).min(shot.remaining);
            let steps = (distance / 0.35).ceil().max(1.0) as usize;
            let mut hit = false;
            for _ in 0..steps {
                if let Some(lock) = shot.lock {
                    let target = match lock {
                        ShotLock::Champion(seat) => self.players.iter().find(|p|
                            p.seat == seat && p.respawn <= 0.0 && p.selected)
                            .map(|p| (p.runner.x,p.runner.z)),
                        ShotLock::Minion(id) => self.minions.iter().find(|m| m.id == id && m.hp > 0.0)
                            .map(|m| (m.x,m.z)),
                        ShotLock::Spar(team) => Some((self.spar[team].x,self.spar[team].z)),
                    };
                    let Some((tx,tz)) = target else { shot.remaining = 0.0; break };
                    let yaw = (tx-shot.x).atan2(tz-shot.z);
                    let (dx,dz) = direction(yaw);
                    shot.vx = dx * speed;
                    shot.vz = dz * speed;
                }
                shot.x += shot.vx / speed * distance / steps as f32;
                shot.z += shot.vz / speed * distance / steps as f32;
                shot.remaining -= distance / steps as f32;
                if self.time >= DRAFT && inside_jungle_wall(shot.x, shot.z, 0.0) {
                    hit = true;
                    break;
                }
                let owner = &self.players[shot.owner];
                let team = owner.team;
                let champ = self.players.iter().enumerate().find(|(index, p)|
                    *index != shot.owner && p.team != team && p.selected
                    && p.respawn <= 0.0 && !p.busy
                    && shot.lock.is_none_or(|lock| matches!(lock, ShotLock::Champion(seat) if seat == p.seat))
                    && (p.runner.x-shot.x).hypot(p.runner.z-shot.z) < shot.radius + 0.45)
                    .map(|(index,_)| index);
                if let Some(index) = champ {
                    if shot.slot != 3 { self.hit_team(shot.owner, index, shot.damage, shot.stun, events); }
                    hit = true;
                } else if self.time < DRAFT && shot.lock.is_none_or(|lock|
                    matches!(lock, ShotLock::Spar(spar_team) if spar_team == team as usize)) {
                    let spar = &mut self.spar[team as usize];
                    if (spar.x-shot.x).hypot(spar.z-shot.z) < shot.radius + 0.55 {
                        if shot.slot != 3 {
                            spar.hp = (spar.hp - shot.damage).max(0.0);
                            if spar.hp == 0.0 { spar.hp = 150.0; }
                        }
                        hit = true;
                    }
                } else if let Some(index) = self.minions.iter().position(|m|
                    m.team != team && m.hp > 0.0
                    && shot.lock.is_none_or(|lock| matches!(lock, ShotLock::Minion(id) if id == m.id))
                    && (m.x-shot.x).hypot(m.z-shot.z) < shot.radius + 0.4) {
                    if shot.slot != 3 { self.minions[index].hp -= shot.damage; }
                    if shot.slot != 3 && self.minions[index].hp <= 0.0 {
                        self.players[shot.owner].last_hits = self.players[shot.owner].last_hits.saturating_add(1);
                        self.players[shot.owner].gold = self.players[shot.owner].gold.saturating_add(25);
                        self.players[shot.owner].mana = (self.players[shot.owner].mana + 8.0)
                            .min(Self::max_mana(self.players[shot.owner].level));
                        self.gain_xp(shot.owner, 22, events);
                        events.push(json!({"kind":"pet_minion_farm","seat":self.players[shot.owner].seat,
                            "count":1,"xp":22,"mana":8,"x":shot.x,"z":shot.z}));
                    }
                    hit = true;
                } else if let Some(index) = self.towers.iter().enumerate().position(|(index,t)|
                    shot.lock.is_none() && t.team != team && t.hp > 0.0 && self.tower_open(index)
                    && (t.x-shot.x).hypot(t.z-shot.z) < shot.radius + 1.0) {
                    if shot.slot != 3 { self.hit_tower(index, shot.damage, events); }
                    hit = true;
                } else if shot.lock.is_none() && self.crown_open(team as usize) {
                    let crown = CROWN_POS[1-team as usize];
                    if (crown.0-shot.x).hypot(crown.1-shot.z) < shot.radius + 1.2 {
                        let pressure = if self.players.len() == 4 { 0.08 } else { 0.06 };
                        let buff = if self.team_buff[team as usize] > 0.0 { 1.25 } else { 1.0 };
                        let damage = shot.damage * pressure * buff;
                        if shot.slot != 3 {
                            self.crown_hp[1-team as usize] = (self.crown_hp[1-team as usize]-damage).max(0.0);
                            events.push(json!({"kind":"pet_crown_hit","seat":self.players[shot.owner].seat,
                                "team":team,"target":1-team,"hp":self.crown_hp[1-team as usize],
                                "damage":damage,"x":crown.0,"z":crown.1}));
                        }
                        hit = true;
                    }
                }
                if hit { break; }
            }
            if shot.slot == 3 && (hit || shot.remaining <= 0.01) {
                self.detonate_skillshot(&shot, events);
            }
            if hit || shot.slot == 3 && shot.remaining <= 0.01 {
                events.push(json!({"kind":"pet_arena_shot_impact","seat":self.players[shot.owner].seat,
                    "slot":shot.slot,"x":shot.x,"z":shot.z}));
            } else if shot.remaining > 0.01 {
                active.push(shot);
            }
        }
        self.skillshots = active;
    }
    fn cast(&mut self, index: usize, slot: usize, yaw: f32, events: &mut Vec<Value>) {
        let mana_cost = [12.0, 25.0, 22.0, 55.0][slot];
        if self.players[index].cooldown[slot] > 0.0
            || self.players[index].respawn > 0.0
            || self.players[index].lock > 0.0
            || self.players[index].mana < mana_cost
            || slot == 3 && self.time >= DRAFT && self.time < RIFT_UNLOCK
        {
            return;
        }
        let champ = self.players[index].champ;
        let base_cd = [0.55, 5.0, 6.0, 18.0][slot];
        self.players[index].cooldown[slot] = base_cd
            * (1.0 - (self.players[index].ranks[slot] - 1) as f32 * 0.08)
            * (1.0 - 0.15 * ((self.time - DRAFT) / 360.0).clamp(0.0, 1.0));
        self.players[index].mana -= mana_cost;
        self.players[index].cast = slot as u8;
        self.players[index].reveal = 2.0;
        self.players[index].action_n = self.players[index].action_n.wrapping_add(1);
        let seat = self.players[index].seat;
        let from = (self.players[index].runner.x, self.players[index].runner.z);
        events.push(json!({"kind":"pet_arena_cast","seat":seat,"champ":champ,
            "slot":slot,"x":from.0,"z":from.1,"yaw":yaw}));
        match slot {
            0 => {
                let range = aimed_range(champ, 0, self.players[index].width);
                let damage = [46.0, 60.0, 43.0, 58.0, 34.0, 52.0, 40.0, 62.0, 36.0, 42.0][champ];
                let slow = if matches!(champ, 2 | 6 | 9) { 0.35 } else { 0.0 };
                self.launch_skillshot(index, 0, yaw, range, damage, slow);
            }
            1 => match champ {
                0 | 2 | 6 => {
                    self.launch_skillshot(index, 1, yaw,
                        aimed_range(champ, 1, self.players[index].width), 28.0, 0.8);
                }
                4 | 8 => {
                    let ally = self.aimed_ally(index, yaw, 6.5).unwrap_or(index);
                    self.players[ally].hp = (self.players[ally].hp + 80.0)
                        .min(Self::max_hp(self.players[ally].level));
                }
                1 => {
                    self.players[index].invuln = 1.15;
                }
                3 | 7 => {
                    self.players[index].shield = (self.players[index].shield + 90.0).min(150.0);
                }
                5 | 9 => {
                    self.launch_skillshot(index, 1, yaw,
                        aimed_range(champ, 1, self.players[index].width), 42.0, 0.55);
                }
                _ => {}
            },
            2 => {
                let (dx, dz) = direction(yaw);
                let dash = [4.5, 6.5, 3.8, 4.0, 5.2, 5.4, 4.3, 5.0, 5.0, 4.1][champ];
                let landing = (from.0 + dx * dash, from.1 + dz * dash);
                if !inside_jungle_wall(landing.0, landing.1, 0.45) {
                    self.players[index].runner.x = landing.0;
                    self.players[index].runner.z = landing.1;
                }
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
                let range = aimed_range(champ, 3, self.players[index].width);
                self.launch_skillshot(index, 3, yaw, range, damage, 0.0);
            }
            _ => {}
        }
        match slot {
            0 => {}
            2 => {
                self.damage_spar(index, 30.0, 2.6, yaw, -1.0, events);
                self.damage_minions(index, 30.0, 2.6, yaw, -1.0, events);
                self.damage_towers(index, 30.0, 2.6, yaw, -1.0, events);
                self.damage_crown(index, 30.0, yaw, 2.6, -1.0, events);
            }
            3 => {}
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
        self.time = if self.kind == 1 && before < DRAFT {
            if before >= 8.0 && self.players.iter().all(|p| p.selected) {
                DRAFT
            } else {
                (before + dt).min(DRAFT - 0.01)
            }
        } else {
            before + dt
        };
        if self.kind == 1 && before < DRAFT && self.time >= DRAFT {
            self.skillshots.clear();
            self.minion_wave = 0.0;
            let count = self.players.len();
            for (index, p) in self.players.iter_mut().enumerate() {
                p.hp = MAX_HP;
                p.mana = MAX_MANA;
                p.shield = 0.0;
                p.cooldown = [0.0; 4];
                p.auto_cd = 0.0;
                p.lock = 0.0;
                p.respawn = 0.0;
                let (x, z) = lane_spawn(index, count, p, self.kind, self.seed);
                p.runner = Runner::at(x, z).with_width(p.width);
            }
        }
        self.step_spar(dt);
        self.side_events(inputs, dt, events);
        if self.kind == 1 {
            for buff in &mut self.team_buff {
                *buff = (*buff - dt).max(0.0);
            }
            for push in &mut self.team_push {
                *push = (*push - dt).max(0.0);
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
            p.arena_target = input.arena_target;
            for cd in &mut p.cooldown {
                *cd = (*cd - dt).max(0.0);
            }
            p.auto_cd = (p.auto_cd - dt).max(0.0);
            p.lock = (p.lock - dt).max(0.0);
            p.invuln = (p.invuln - dt).max(0.0);
            p.reveal = (p.reveal - dt).max(0.0);
            if p.busy { continue; }
            p.item_cd = (p.item_cd - dt).max(0.0);
            p.stasis = (p.stasis - dt).max(0.0);
            p.haste = (p.haste - dt).max(0.0);
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
                    && (open_draft || p.position.is_some_and(|pos| champion_allowed(pos, pick)))
                    && (1.15..1.25).contains(&input.pitch)
                {
                    p.champ = pick;
                    p.role = CHAMPION_ROLE[pick];
                }
                if input.yaw.is_finite()
                    && input.yaw >= 0.0
                    && input.yaw < 10.0
                    && (open_draft || p.position.is_some_and(|pos| champion_allowed(pos, pick)))
                    && (0.95..1.05).contains(&input.pitch)
                {
                    p.champ = pick;
                    p.role = CHAMPION_ROLE[pick];
                    p.selected = true;
                }
                if !p.selected && self.time >= DRAFT {
                    continue;
                }
            }
            if self.kind == 1 {
                p.mana = (p.mana + dt * 6.5).min(Self::max_mana(p.level));
                if self.time >= DRAFT {
                    let home = CROWN_POS[p.team as usize];
                    let at_base = (p.runner.x - home.0).hypot(p.runner.z - home.1) < 8.0;
                    if at_base {
                        p.hp = (p.hp + 72.0 * dt).min(Self::max_hp(p.level));
                        p.mana = (p.mana + 85.0 * dt).min(Self::max_mana(p.level));
                    }
                    let item_key = (0.45..0.95).contains(&input.pitch);
                    if item_key && !p.held_item_key {
                        if (0.65..0.95).contains(&input.pitch) && at_base {
                            let item = ((input.pitch * 10.0).round() as i32 - 6).clamp(1, 3) as usize;
                            let cost = SHOP_COST[item - 1];
                            if p.gold >= cost {
                                p.gold -= cost;
                                p.item = item as u8;
                                p.item_cd = 0.0;
                                events.push(json!({"kind":"crownfall_buy","seat":p.seat,"item":p.item,"gold":p.gold}));
                            }
                        } else if (0.45..0.55).contains(&input.pitch) && p.item_cd <= 0.0 && p.stasis <= 0.0 {
                            match p.item {
                                1 => { p.stasis = 1.8; p.item_cd = 70.0; },
                                2 => {
                                    p.hp = (p.hp + 90.0).min(Self::max_hp(p.level));
                                    p.mana = (p.mana + 55.0).min(Self::max_mana(p.level));
                                    p.item_cd = 48.0;
                                },
                                3 => { p.haste = 4.0; p.item_cd = 55.0; },
                                _ => {},
                            }
                            if p.item != 0 {
                                events.push(json!({"kind":"crownfall_item","seat":p.seat,"item":p.item,
                                    "x":p.runner.x,"z":p.runner.z}));
                            }
                        }
                    }
                    p.held_item_key = item_key;
                    if (0.56..0.64).contains(&input.pitch)
                        && !at_base && p.lock <= 0.0
                        && input.stick.x.hypot(input.stick.z) < 0.15
                    {
                        p.recall += dt;
                        if p.recall >= 3.0 {
                            p.runner = Runner::at(if p.team == 0 { -36.0 } else { 36.0 }, 0.0)
                                .with_width(p.width);
                            p.recall = 0.0;
                            events.push(json!({"kind":"crownfall_recall","seat":p.seat,
                                "x":p.runner.x,"z":p.runner.z}));
                        } else {
                            continue;
                        }
                    } else {
                        p.recall = 0.0;
                    }
                    if p.stasis > 0.0 { continue; }
                }
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
                let before_move = (p.runner.x, p.runner.z);
                step_runner(
                    &mut p.runner,
                    input.stick,
                    dt,
                    if self.kind == 0 {
                        7.0
                    } else {
                        (6.2 + 2.2 * ((self.time - DRAFT) / 300.0).clamp(0.0, 1.0))
                            * if p.haste > 0.0 { 1.35 } else { 1.0 }
                    },
                );
                if self.kind == 1 && self.time >= DRAFT
                    && inside_jungle_wall(p.runner.x, p.runner.z, 0.45) {
                    p.runner.x = before_move.0;
                    p.runner.z = before_move.1;
                    p.runner.vx = 0.0;
                    p.runner.vz = 0.0;
                }
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
                self.auto_attack(index, events);
            }
        }
        self.step_skillshots(dt, events);
        self.step_minions(dt, events);
        self.step_towers(dt, events);
        self.step_relays(dt, events);
        if self.kind == 1 && self.time >= self.crown_collapse_start() {
            for team in 0..2 {
                if self.crown_open(1-team) {
                    self.crown_hp[team] = (self.crown_hp[team] - CROWN_COLLAPSE_PER_SECOND * dt).max(0.0);
                }
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
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        if let Some(side) = self.side_bout.as_mut() {
            if side.seats.contains(&seat) {
                return side.game.bot_input(seat, dt);
            }
        }
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
        if self.kind == 1 && p.position == Some(3) && self.time >= RIFT_UNLOCK
            && !self.rift_used && self.side_bout.is_none() {
            let path = bot_path(p, (0.0,0.0));
            let dx = path.0-p.runner.x;
            let dz = path.1-p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            return Input {
                stick: if dist > 2.5 { Stick { x: dx / dist, z: dz / dist } } else { Stick::default() },
                yaw: dx.atan2(dz),
                pitch: if dist < 3.0 { 1.4 } else { 0.0 },
                ..Default::default()
            };
        }
        if self.kind == 1 && self.rift_countdown.is_some() {
            let path = bot_path(p, (0.0,0.0));
            let dx = path.0-p.runner.x;
            let dz = path.1-p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            return Input {
                stick: if dist > 6.0 { Stick { x: dx / dist, z: dz / dist } } else { Stick::default() },
                yaw: dx.atan2(dz),
                ..Default::default()
            };
        }
        if self.kind == 1 && p.position == Some(3) && p.respawn <= 0.0 && !p.busy {
            let hp_ratio = p.hp / Self::max_hp(p.level);
            if hp_ratio < 0.34 {
                let home = CROWN_POS[p.team as usize];
                let gate_x = if p.team == 0 { -13.0 } else { 13.0 };
                let waypoint = if p.runner.z.abs() > 21.0 {
                    (gate_x, p.runner.z.signum() * 23.0)
                } else if p.runner.z.abs() > 2.0 { (gate_x, 0.0) } else { home };
                let (dx,dz) = (waypoint.0-p.runner.x, waypoint.1-p.runner.z);
                let d = dx.hypot(dz).max(0.01);
                return Input { stick: Stick { x:dx/d, z:dz/d }, yaw:dx.atan2(dz), ..Default::default() };
            }
            let queued = self.portal_queue.as_ref().is_some_and(|queue| queue.seat == seat);
            let locked = p.gank_target.filter(|_| p.gank_until > self.time)
                .and_then(|target| self.players.iter().enumerate()
                    .find(|(other_index,other)| other.seat == target
                        && other.team != p.team
                        && self.gank_score(index, *other_index).is_some_and(|score| score >= 0.45))
                    .map(|(other_index,_)| other_index));
            let gank = if queued { None } else { locked.or_else(|| self.best_gank(index)) };
            if let Some(target_index) = gank {
                let enemy = &self.players[target_index];
                let enemy_seat = enemy.seat;
                let target = (enemy.runner.x, enemy.runner.z);
                let gap_x = if p.team == 0 { -13.0 } else { 13.0 };
                let brush_x = if p.team == 0 { -18.0 } else { 18.0 };
                let waypoint = if p.runner.z.abs() < 9.0 && target.1.abs() > 21.0 {
                    (brush_x, target.1.signum() * 11.5)
                } else if p.runner.z.abs() < 21.0 && target.1.abs() > 21.0 {
                    (gap_x, target.1.signum() * 23.0)
                } else { target };
                let (dx,dz) = (waypoint.0-p.runner.x, waypoint.1-p.runner.z);
                let dist = dx.hypot(dz).max(0.01);
                let enemy_dist = (target.0-p.runner.x).hypot(target.1-p.runner.z);
                let attack = enemy_dist < 8.0;
                let input = Input {
                    stick: if enemy_dist > 4.2 { Stick { x:dx/dist, z:dz/dist } } else { Stick::default() },
                    yaw: (target.0-p.runner.x).atan2(target.1-p.runner.z),
                    arena_target: Some(ArenaTarget::Champion { seat: enemy_seat }),
                    pitch: 0.0,
                    fire: attack && p.cooldown[0] <= 0.0,
                    jump: attack && p.cooldown[1] <= 0.0,
                    crouch: enemy_dist > 5.0 && enemy_dist < 9.0 && p.cooldown[2] <= 0.0,
                    press: (enemy_dist < 5.8 && p.cooldown[3] <= 0.0)
                        .then_some(crate::bout::Press::Both),
                };
                let lock_until = if p.gank_target == Some(enemy_seat)
                    && p.gank_until > self.time { p.gank_until } else { self.time + 8.0 };
                self.players[index].gank_target = Some(enemy_seat);
                self.players[index].gank_until = lock_until;
                return input;
            }
        }
        if self.kind == 1 && p.position == Some(3) && self.portal_cd <= 0.0
            && self.side_bout.is_none() && self.time >= DRAFT + 20.0
            && self.time < RIFT_UNLOCK - 25.0 {
            let pad = PORTALS[p.team as usize];
            let path = bot_path(p, pad);
            let dx = path.0 - p.runner.x;
            let dz = path.1 - p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            let queued = self.portal_queue.as_ref().is_some_and(|q| q.seat == seat);
            if self.portal_queue.is_none() || queued || self.portal_queue.as_ref().is_some_and(|q| {
                self.players.iter().find(|other| other.seat == q.seat).is_some_and(|other| other.team != p.team)
            }) {
                return Input {
                    stick: if dist > 1.8 { Stick { x: dx / dist, z: dz / dist } } else { Stick::default() },
                    yaw: dx.atan2(dz),
                    pitch: if dist < 2.5 && !queued { 1.4 } else { 0.0 },
                    ..Default::default()
                };
            }
        }
        if self.kind == 1 {
            let threat = self.players.iter().enumerate()
                .filter(|(other_index, other)| *other_index != index
                    && other.team != p.team && other.selected && other.respawn <= 0.0
                    && !other.busy && !self.hidden_from(index, *other_index)
                    && distance(p, other) < 9.0)
                .filter(|(other_index, other)|
                    p.last_hit == Some(*other_index) && p.reveal > 0.0
                    || other.hp < Self::max_hp(other.level) * 0.3)
                .filter(|(_, other)| !self.towers.iter().any(|tower| tower.team == other.team
                    && tower.hp > 0.0
                    && (tower.x-other.runner.x).hypot(tower.z-other.runner.z) < 8.0))
                .min_by(|a, b| distance(p, a.1).total_cmp(&distance(p, b.1)));
            if let Some((_, enemy)) = threat {
                let dx = enemy.runner.x - p.runner.x;
                let dz = enemy.runner.z - p.runner.z;
                let dist = dx.hypot(dz).max(0.01);
                let range = BASIC_RANGE[p.champ] * BodyProfile::new(p.width).reach;
                return Input {
                    stick: if dist > range * 0.75 { Stick { x:dx/dist, z:dz/dist } }
                        else { Stick::default() },
                    yaw: dx.atan2(dz),
                    arena_target: Some(ArenaTarget::Champion { seat: enemy.seat }),
                    fire: dist < range && p.cooldown[0] <= 0.0,
                    jump: dist < 6.0 && p.cooldown[1] <= 0.0,
                    crouch: dist > 4.0 && p.cooldown[2] <= 0.0,
                    press: (dist < 5.8 && p.cooldown[3] <= 0.0)
                        .then_some(crate::bout::Press::Both),
                    ..Default::default()
                };
            }
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
            let preferred_lane = if p.position == Some(0) { 0 } else { 1 };
            let enemy = 1 - p.team;
            if let Some(tower) = self.towers.iter().enumerate()
                .filter(|(i, t)| t.team == enemy && t.hp > 0.0 && self.tower_open(*i))
                .min_by_key(|(_, t)| (if t.lane == Some(preferred_lane) { 0 } else { 1 }, t.tier))
                .map(|(_, t)| t) {
                let path = bot_path(p, (tower.x,tower.z));
                let (dx, dz) = (path.0 - p.runner.x, path.1 - p.runner.z);
                let dist = dx.hypot(dz).max(0.01);
                let range = BASIC_RANGE[p.champ] * BodyProfile::new(p.width).reach;
                let tower_dist = (tower.x-p.runner.x).hypot(tower.z-p.runner.z);
                return Input {
                    stick: if tower_dist > range * 0.78 || path != (tower.x, tower.z) {
                        Stick { x: dx / dist, z: dz / dist }
                    } else { Stick::default() },
                    yaw: (tower.x-p.runner.x).atan2(tower.z-p.runner.z),
                    fire: tower_dist < range && p.cooldown[0] <= 0.0,
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
            let path = bot_path(p, site);
            let dx = path.0 - p.runner.x;
            let dz = path.1 - p.runner.z;
            let dist = dx.hypot(dz).max(0.01);
            let crown_dist = (site.0-p.runner.x).hypot(site.1-p.runner.z);
            return Input {
                stick: if crown_dist > 4.8 {
                    Stick {
                        x: dx / dist,
                        z: dz / dist,
                    }
                } else {
                    Stick::default()
                },
                yaw: (site.0-p.runner.x).atan2(site.1-p.runner.z),
                fire: crown_dist < 8.0 && p.cooldown[0] <= 0.0,
                crouch: crown_dist > 5.0 && p.cooldown[2] <= 0.0,
                press: (crown_dist < 6.5 && p.cooldown[3] <= 0.0).then_some(crate::bout::Press::Both),
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
        let players = self.players.iter().map(|p| {
            let mut player = json!({
                "seat":p.seat,"team":p.team,"pet":p.pet,"body_width":p.width,
                "x":if self.kind == 1 { p.runner.x } else { p.avatar_x },
                "y":if self.kind == 1 { 0.0 } else { 1.05 },
                "z":if self.kind == 1 { p.runner.z } else { -13.3 },
                "vx":if self.kind == 1 { p.runner.vx } else { 0.0 },
                "vz":if self.kind == 1 { p.runner.vz } else { 0.0 },
                "pet_x":p.runner.x,"pet_z":p.runner.z,"pet_y":if p.respawn > 0.0 { -2.0 } else { p.height },
                "score":p.score,"hp":p.hp,"max_hp":Self::max_hp(p.level),
                "mana":p.mana,"max_mana":Self::max_mana(p.level),
                "last_hits":p.last_hits,
                "level":p.level,"xp":p.xp,"xp_next":45+u16::from(p.level)*20,
                "points":p.points,"ranks":p.ranks,
                "shield":p.shield,"damage":p.damage,
                "stocks":p.stocks,"respawn":p.respawn,"lock":p.lock,
                "invuln":p.invuln,"charge":p.charge,
                "champ":p.champ,"role":p.role,"position":p.position,
                "selected":p.selected,"reveal":p.reveal,
                "hidden":p.busy,
                "cooldowns":p.cooldown,"action_n":p.action_n,"cast":p.cast
            });
            player["gold"] = json!(p.gold);
            player["item"] = json!(p.item);
            player["item_cd"] = json!(p.item_cd);
            player["stasis"] = json!(p.stasis);
            player["haste"] = json!(p.haste);
            player["recall"] = json!(p.recall);
            player["at_base"] = json!(self.kind == 1 && (p.runner.x - CROWN_POS[p.team as usize].0)
                .hypot(p.runner.z - CROWN_POS[p.team as usize].1) < 8.0);
            player
        }).collect::<Vec<_>>();
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
            "x":m.x,"z":m.z,"hp":m.hp,"id":m.id})
            })
            .collect::<Vec<_>>();
        let towers = self.towers.iter().enumerate().map(|(index,t)| json!({"team":t.team,
            "lane":t.lane,"tier":t.tier,"x":t.x,"z":t.z,
            "hp":t.hp,"max_hp":t.max_hp,"open":self.tower_open(index),
            "shielded":t.tier < 2 && !self.minions.iter().any(|m| m.team != t.team
                && m.hp > 0.0 && (m.x-t.x).hypot(m.z-t.z) < 8.0)}))
            .collect::<Vec<_>>();
        let mut snapshot = json!({"game":self.key(),"time":self.time,"limit":self.limit(),
            "draft":self.kind == 1 && self.time < DRAFT,
            "team_score":self.team_score,"champions":CHAMPIONS,
            "push":self.team_push,"crown_open":[self.crown_open(0),self.crown_open(1)],
            "crown_hp":self.crown_hp,"crown_max_hp":CROWN_HP,
            "crown_shield":(CROWN_SHIELD_END-self.time).max(0.0),
            "crown_collapse_in":(self.crown_collapse_start()-self.time).max(0.0),
            "relays":relays,"relay_pulse":self.relay_pulse,
            "dragon":dragon,"players":players,"minions":minions,"towers":towers,
            "skillshots":self.skillshots.iter().map(|shot| json!({
                "seat":self.players[shot.owner].seat,"slot":shot.slot,
                "champ":self.players[shot.owner].champ,
                "x":shot.x,"z":shot.z,"yaw":shot.vx.atan2(shot.vz)
            })).collect::<Vec<_>>(),
            "spar":self.spar.iter().map(|bot|json!({"x":bot.x,"z":bot.z,"hp":bot.hp})).collect::<Vec<_>>(),
            "props":[]});
        snapshot["portal"] = json!({
            "queue":self.portal_queue.as_ref().map(|queue| queue.seat),
            "remaining":self.portal_queue.as_ref().map(|queue| queue.remaining),
            "cooldown":self.portal_cd,
        });
        snapshot["rift"] = json!({
            "unlocks_in":(RIFT_UNLOCK-self.time).max(0.0),
            "countdown":self.rift_countdown,
            "used":self.rift_used,
            "available":self.time >= RIFT_UNLOCK && !self.rift_used,
            "buff":self.team_buff,
            "claims":self.dragon_claims,
        });
        snapshot
    }
    fn snapshot_for(&self, seat: Option<u8>) -> Option<Value> {
        if self.kind != 1 {
            return None;
        }
        if let Some(side) = self.side_bout.as_ref().filter(|side| seat.is_some_and(|seat| side.seats.contains(&seat))) {
            let mut view = side.game.snapshot_for(seat).unwrap_or_else(|| side.game.snapshot());
            view["crownfall_side"] = json!(if side.kind == SideKind::Portal { "portal" } else { "rift" });
            view["nested_setup"] = side.game.setup();
            return Some(view);
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
                    player["x"] = Value::Null;
                    player["z"] = Value::Null;
                    player["hp"] = Value::Null;
                    player["mana"] = Value::Null;
                    player["shield"] = Value::Null;
                }
            }
        }
        view["auto_target"] = match self.auto_target(observer) {
            Some(AutoTarget::Minion(index)) => json!({"kind":"minion","index":index}),
            Some(AutoTarget::Champion(index)) => json!({"kind":"champion","seat":self.players[index].seat}),
            Some(AutoTarget::Spar(team)) => json!({"kind":"spar","team":team}),
            None => Value::Null,
        };
        view["auto_ready_in"] = json!(self.players[observer].auto_cd.max(self.players[observer].lock).max(self.players[observer].stasis));
        view["auto_range"] = json!(BASIC_RANGE[self.players[observer].champ] * BodyProfile::new(self.players[observer].width).reach);
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
        game.players[1].selected = true;
        for _ in 0..((DRAFT * 60.0) as usize + 1) {
            game.step(
                &[(
                    0,
                    Input {
                        yaw: 7.0,
                        pitch: 1.0,
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
        game.time = RIFT_UNLOCK;
        game.cast(0, 3, std::f32::consts::FRAC_PI_2, &mut events);
        game.step_skillshots(0.3, &mut events);
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
    fn smaller_teams_can_pick_any_champion_but_full_teams_fill_positions() {
        let duel = [
            Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(1),
        ];
        let mut small = PetArena::new(1, &duel, 2);
        small.step(
            &[(
                0,
                Input {
                    yaw: 0.0, pitch: 1.0,
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
                    yaw: 4.0, pitch: 1.0,
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
                    yaw: 3.0, pitch: 1.0,
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
                    yaw: 7.0, pitch: 1.0,
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
    fn each_full_team_has_top_carry_support_and_roaming_mage() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let game = PetArena::new(1, &contestants, 3);
        for team in 0..2 {
            assert!(
                game.players
                    .iter()
                    .filter(|p| p.team == team)
                    .all(|p| CHAMPION_ROLE[p.champ] == p.role
                        && p.position.is_some_and(|pos| champion_allowed(pos, p.champ)))
            );
            let carry = game.players.iter().find(|p| p.team == team && p.position == Some(1)).unwrap();
            let support = game.players.iter().find(|p| p.team == team && p.position == Some(2)).unwrap();
            let roamer = game.players.iter().find(|p| p.team == team && p.position == Some(3)).unwrap();
            assert_eq!(carry.role, 1);
            assert_eq!(support.role, 3);
            assert!(BASIC_RANGE[roamer.champ] >= 5.7);
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
    fn buff_does_not_bypass_tower_defenses() {
        let contestants = (0..2).map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2)).collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 3);
        game.time = CROWN_SHIELD_END + 1.0;
        game.team_buff[0] = 45.0;
        game.relay_owner[0] = Some(0);
        game.relay_pulse = 0.01;
        game.step(&[], 1.0 / 60.0, &mut Vec::new());
        assert_eq!(game.crown_hp[1], CROWN_HP);
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
    fn both_side_lanes_and_last_tower_guard_the_crown() {
        let contestants = [Contestant::bot(0, 0.7).on_team(0), Contestant::bot(1, 0.7).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = DRAFT;
        game.players[0].runner.x = 13.0;
        game.players[0].runner.z = LANE_Z[0];
        let mut events = Vec::new();
        let outer_north = game.towers.iter().position(|t| t.team == 1 && t.lane == Some(0) && t.tier == 0).unwrap();
        let inner_north = game.towers.iter().position(|t| t.team == 1 && t.lane == Some(0) && t.tier == 1).unwrap();
        let final_tower = game.towers.iter().position(|t| t.team == 1 && t.tier == 2).unwrap();
        game.hit_tower(inner_north, 99999.0, &mut events);
        assert!(game.towers[inner_north].hp > 0.0);
        game.hit_tower(outer_north, 99999.0, &mut events);
        game.hit_tower(inner_north, 99999.0, &mut events);
        assert_eq!(game.towers[inner_north].hp, 0.0);
        game.hit_tower(final_tower, 99999.0, &mut events);
        assert!(game.towers[final_tower].hp > 0.0);
        for i in 0..game.towers.len() {
            if game.towers[i].team == 1 && game.towers[i].tier < 2 {
                game.towers[i].hp = 0.0;
            }
        }
        game.hit_tower(final_tower, 99999.0, &mut events);
        assert_eq!(game.towers[final_tower].hp, 0.0);
        assert!(game.crown_open(0));
        game.players[0].runner.x = CROWN_POS[1].0 - 4.5;
        game.players[0].runner.z = 0.0;
        game.damage_crown(0, 46.0, std::f32::consts::FRAC_PI_2, 8.0, 0.72, &mut events);
        assert!(game.crown_hp[1] < CROWN_HP);
    }
    #[test]
    fn jungle_walls_have_passages_and_dash_landings() {
        assert!(inside_jungle_wall(0.0, 20.0, 0.45));
        assert!(!inside_jungle_wall(13.0, 20.0, 0.45));
        let contestants = [Contestant::bot(0, 0.7).on_team(0), Contestant::bot(1, 0.7).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 9);
        game.time = DRAFT;
        game.players[0].runner.x = 0.0;
        game.players[0].runner.z = 17.5;
        game.step(&[(0, Input { stick: Stick { x: 0.0, z: 1.0 }, ..Default::default() })], 0.1, &mut Vec::new());
        assert!(game.players[0].runner.z < 18.1);
        game.players[0].cooldown[2] = 0.0;
        game.players[0].mana = MAX_MANA;
        game.cast(0, 2, 0.0, &mut Vec::new());
        assert!(game.players[0].runner.z > 21.0);
        game.players[0].runner = Runner::at(16.0, 31.0);
        assert_eq!(bot_path(&game.players[0], (34.0, 0.0)), (13.0, 31.0));
        game.players[0].runner = Runner::at(13.0, 23.0);
        assert_eq!(bot_path(&game.players[0], (34.0, 0.0)), (13.0, 16.0));
    }
    #[test]
    fn roamer_ganks_a_vulnerable_lane_through_brush_and_retreats_when_low() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 17);
        game.time = DRAFT + 60.0;
        let roamer = game.players.iter().position(|p| p.team == 0 && p.position == Some(3)).unwrap();
        let ally = game.players.iter().position(|p| p.team == 0 && p.position == Some(0)).unwrap();
        let enemy = game.players.iter().position(|p| p.team == 1 && p.position == Some(0)).unwrap();
        for (i,p) in game.players.iter_mut().enumerate() {
            p.runner = Runner::at(if p.team == 0 { -34.0 } else { 34.0 },
                if i % 2 == 0 { 31.0 } else { -31.0 });
        }
        game.players[roamer].runner = Runner::at(-13.0, 0.0);
        game.players[ally].runner = Runner::at(-9.0, 31.0);
        game.players[enemy].runner = Runner::at(-8.0, 31.0);
        game.players[enemy].hp = 60.0;
        let seat = game.players[roamer].seat;
        assert_eq!(game.best_gank(roamer), Some(enemy));
        let approach = game.bot_input(seat, 1.0/60.0);
        assert!(approach.stick.x < 0.0 && approach.stick.z > 0.0);
        assert_eq!(game.players[roamer].gank_target, Some(game.players[enemy].seat));
        game.players[roamer].runner = Runner::at(-8.0, 26.5);
        let engage = game.bot_input(seat, 1.0/60.0);
        assert!(engage.fire);
        assert_eq!(engage.arena_target, Some(ArenaTarget::Champion { seat: game.players[enemy].seat }));
        game.players[roamer].hp = 20.0;
        let retreat = game.bot_input(seat, 1.0/60.0);
        assert!(retreat.stick.z < 0.0 && !retreat.fire);
        game.players[roamer].hp = MAX_HP;
        game.players[roamer].gank_target = None;
        game.players[ally].runner = Runner::at(15.0, 31.0);
        game.players[enemy].runner = Runner::at(16.0, 31.0);
        assert_eq!(game.best_gank(roamer), None, "do not dive an active enemy tower");
    }
    #[test]
    fn roamer_rechecks_a_gank_and_can_gank_after_the_rift() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 17);
        game.time = RIFT_UNLOCK + 15.0;
        game.rift_used = true;
        let roamer = game.players.iter().position(|p| p.team == 0 && p.position == Some(3)).unwrap();
        let ally = game.players.iter().position(|p| p.team == 0 && p.position == Some(0)).unwrap();
        let enemy = game.players.iter().position(|p| p.team == 1 && p.position == Some(0)).unwrap();
        for p in &mut game.players {
            p.runner = Runner::at(if p.team == 0 { -34.0 } else { 34.0 }, -31.0);
        }
        game.players[roamer].runner = Runner::at(-13.0, 0.0);
        game.players[ally].runner = Runner::at(-9.0, 31.0);
        game.players[enemy].runner = Runner::at(-8.0, 31.0);
        game.players[enemy].hp = 60.0;
        let roamer_seat = game.players[roamer].seat;
        let enemy_seat = game.players[enemy].seat;
        assert_eq!(game.best_gank(roamer), Some(enemy));
        assert_eq!(game.bot_input(roamer_seat, 1.0/60.0).arena_target,
            Some(ArenaTarget::Champion { seat: enemy_seat }));
        game.players[enemy].runner = Runner::at(16.0, 31.0);
        assert_ne!(game.bot_input(roamer_seat, 1.0/60.0).arena_target,
            Some(ArenaTarget::Champion { seat: enemy_seat }),
            "a target reaching tower cover must end a locked gank");
    }
    #[test]
    fn laner_defends_against_recent_attacker_then_resumes_last_hits() {
        let contestants = [Contestant::bot(0, 0.7).on_team(0),
            Contestant::bot(1, 0.7).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 23);
        game.time = DRAFT + 40.0;
        game.players[0].runner = Runner::at(-10.0, 31.0);
        game.players[1].runner = Runner::at(-8.0, 31.0);
        game.players[0].last_hit = Some(1);
        game.players[0].reveal = 1.0;
        game.minions.push(Minion { id: 77, team: 1, lane: 0,
            x: -9.0, z: 31.0, hp: 30.0, attack: 0.0 });
        let defend = game.bot_input(0, 1.0/60.0);
        assert_eq!(defend.arena_target, Some(ArenaTarget::Champion { seat: 1 }));
        assert!(defend.fire);
        game.players[0].reveal = 0.0;
        let farm = game.bot_input(0, 1.0/60.0);
        assert_eq!(farm.arena_target, None);
        assert!(farm.fire, "the bot should resume farming after the threat passes");
    }
    #[test]
    fn ultimate_unlocks_when_the_crown_rift_opens() {
        let contestants = [Contestant::person(0).on_team(0), Contestant::bot(1, 0.7).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 11);
        game.players[0].selected = true;
        game.players[0].mana = MAX_MANA;
        let mut events = Vec::new();
        game.time = RIFT_UNLOCK - 0.1;
        game.cast(0, 3, 0.0, &mut events);
        assert_eq!(game.players[0].cooldown[3], 0.0);
        assert_eq!(game.players[0].mana, MAX_MANA);
        game.time = RIFT_UNLOCK;
        game.cast(0, 3, 0.0, &mut events);
        assert!(game.players[0].cooldown[3] > 0.0);
        assert!(game.players[0].mana < MAX_MANA);
    }
    #[test]
    fn rune_bolt_is_a_dodgeable_travelling_skillshot() {
        let contestants = [Contestant::person(0).on_team(0), Contestant::person(1).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 19);
        game.time = DRAFT + 20.0;
        game.players[0].selected = true;
        game.players[1].selected = true;
        game.players[0].champ = 2;
        game.players[0].runner = Runner::at(0.0, 0.0);
        game.players[1].runner = Runner::at(4.0, 0.0);
        let mut events = Vec::new();
        game.cast(0, 0, 0.0, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP, "casting alone must not apply damage");
        game.step_skillshots(0.4, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP, "a sideways shot must miss");
        game.players[0].cooldown[0] = 0.0;
        game.cast(0, 0, std::f32::consts::FRAC_PI_2, &mut events);
        game.step_skillshots(0.08, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP, "the bolt has flight time");
        game.step_skillshots(0.3, &mut events);
        assert!(game.players[1].hp < MAX_HP);
        assert!(events.iter().any(|event| event["kind"] == "pet_arena_shot_impact"));
    }
    #[test]
    fn voltkit_auto_shots_follow_the_ordered_target() {
        let contestants = [Contestant::person(0).on_team(0), Contestant::person(1).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 21);
        game.time = DRAFT + 35.0;
        game.players[0].selected = true;
        game.players[1].selected = true;
        game.players[0].champ = 5;
        game.players[0].runner = Runner::at(0.0, 0.0);
        game.players[1].runner = Runner::at(4.0, 0.0);
        game.players[0].arena_target = Some(ArenaTarget::Champion { seat: 1 });
        let mut events = Vec::new();
        game.auto_attack(0, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP);
        assert_eq!(game.skillshots.len(), 1);
        game.players[1].runner = Runner::at(4.0, 1.0);
        game.step_skillshots(0.3, &mut events);
        assert!(game.players[1].hp < MAX_HP);
        assert!(game.players[0].auto_cd < 0.7);
    }
    #[test]
    fn ultimate_blast_goes_where_it_is_aimed() {
        let contestants = [Contestant::person(0).on_team(0), Contestant::person(1).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 26);
        game.time = RIFT_UNLOCK;
        game.players[0].selected = true;
        game.players[1].selected = true;
        game.players[0].champ = 2;
        game.players[0].runner = Runner::at(0.0, 0.0);
        game.players[1].runner = Runner::at(0.0, 5.0);
        let mut events = Vec::new();
        game.cast(0, 3, std::f32::consts::FRAC_PI_2, &mut events);
        game.step_skillshots(0.7, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP, "a badly aimed ultimate must miss");
        game.players[0].mana = MAX_MANA;
        game.players[0].cooldown[3] = 0.0;
        game.cast(0, 3, 0.0, &mut events);
        assert_eq!(game.players[1].hp, MAX_HP);
        game.step_skillshots(0.7, &mut events);
        assert!(game.players[1].hp < MAX_HP);
        assert!(events.iter().any(|event| event["kind"] == "pet_arena_blast"));
    }
    #[test]
    fn healer_aims_mend_at_one_ally() {
        let contestants = [Contestant::person(0).on_team(0),
            Contestant::person(1).on_team(0), Contestant::person(2).on_team(0)];
        let mut game = PetArena::new(1, &contestants, 29);
        game.time = DRAFT + 25.0;
        for p in &mut game.players { p.selected = true; p.hp = 100.0; }
        game.players[0].champ = 4;
        game.players[0].runner = Runner::at(0.0,0.0);
        game.players[1].runner = Runner::at(4.0,0.0);
        game.players[2].runner = Runner::at(0.0,4.0);
        game.cast(0,1,std::f32::consts::FRAC_PI_2,&mut Vec::new());
        assert_eq!(game.players[1].hp,180.0);
        assert_eq!(game.players[2].hp,100.0);
    }
    #[test]
    fn explicit_attack_order_tracks_the_exact_enemy() {
        let contestants = [Contestant::person(0).on_team(0), Contestant::bot(1,0.7).on_team(1)];
        let mut game = PetArena::new(1, &contestants, 8);
        game.time = DRAFT;
        game.players[0].selected = true;
        game.players[0].runner = Runner::at(0.0,0.0);
        game.players[1].runner = Runner::at(2.0,0.0);
        game.minions.push(Minion { id: 42, team: 1, lane: 0,
            x: 1.0, z: 0.0, hp: 80.0, attack: 0.0 });
        game.players[0].arena_target = Some(ArenaTarget::Champion { seat: 1 });
        assert_eq!(game.auto_target(0), Some(AutoTarget::Champion(1)));
        game.auto_attack(0, &mut Vec::new());
        assert!(game.players[1].hp < MAX_HP);
        assert_eq!(game.minions[0].hp,80.0);
        game.players[0].auto_cd = 0.0;
        game.players[0].arena_target = Some(ArenaTarget::Minion { id: 42 });
        assert_eq!(game.auto_target(0), Some(AutoTarget::Minion(0)));
        game.auto_attack(0, &mut Vec::new());
        assert!(game.minions[0].hp < 80.0);
        game.players[0].arena_target = Some(ArenaTarget::Minion { id: 99 });
        assert_eq!(game.auto_target(0), None);
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
        assert_eq!(game.players[0].points, game.players[0].level - 1);
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
        assert_eq!(game.players[0].points, game.players[0].level - 2);
    }
    #[test]
    fn a_minion_breakthrough_cannot_skip_the_tower_chain() {
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
            id: 1,
            team: 0,
            lane: 0,
            x: 37.99,
            z: LANE_Z[0],
            hp: 80.0,
            attack: 0.0,
        });
        let mut events = Vec::new();
        game.step_minions(0.1, &mut events);
        assert_eq!(game.crown_hp[1], CROWN_HP);
        assert_eq!(game.team_push[0], 0.0);
        assert_eq!(game.players[1].xp, 0);
        assert!(game.minions.is_empty());
        assert!(!events.iter().any(|event| event["kind"] == "pet_crown_hit"));
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
        game.players[0].champ = 0;
        game.players[0].mana = 0.0;
        game.cast(0, 0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        assert_eq!(game.spar[0].hp, 150.0);
        game.players[0].mana = 120.0;
        game.cast(0, 0, std::f32::consts::FRAC_PI_2, &mut Vec::new());
        assert_eq!(game.spar[0].hp, 150.0, "the shot must travel before it hits");
        game.step_skillshots(0.3, &mut Vec::new());
        assert!(game.spar[0].hp < 150.0);
        game.time = DRAFT - 0.01;
        game.step(&[], 0.02, &mut Vec::new());
        assert_eq!(game.players[0].mana, MAX_MANA);
        assert_eq!(game.players[0].hp, MAX_HP);
        assert_eq!(game.players[0].cooldown, [0.0; 4]);
    }
    #[test]
    fn crown_rift_requires_a_player_trigger_after_five_minutes() {
        let mut game = PetArena::new(
            1,
            &[
                Contestant::person(0).on_team(0),
                Contestant::person(1).on_team(1),
            ],
            3,
        );
        game.time = RIFT_UNLOCK - 1.0;
        game.players[0].selected = true;
        game.players[0].runner.x = 1.0;
        game.players[0].runner.z = 0.0;
        let mut events = Vec::new();
        let trigger = Input { pitch: 1.4, ..Default::default() };
        game.step(&[(0, trigger)], 1.0 / 60.0, &mut events);
        assert!(!game.rift_used);
        game.time = RIFT_UNLOCK;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert!(!game.rift_used);
        game.step(&[(0, trigger)], 1.0 / 60.0, &mut events);
        assert!(game.rift_used);
        assert!(game.rift_countdown.is_some());
        assert!(events.iter().any(|e| e["kind"] == "crownfall_rift_started"));
    }
    #[test]
    fn base_shop_and_recall_restore_the_fighter() {
        let mut game = PetArena::new(1, &[Contestant::person(0).on_team(0)], 16);
        game.time = DRAFT + 1.0;
        game.players[0].selected = true;
        game.players[0].gold = 160;
        game.players[0].runner.x = CROWN_POS[0].0;
        game.step(&[(0, Input { pitch: 0.7, ..Default::default() })], 0.05, &mut Vec::new());
        assert_eq!(game.players[0].item, 1);
        assert_eq!(game.players[0].gold, 20);
        game.players[0].runner.x = 0.0;
        game.players[0].gold = 160;
        game.step(&[], 0.05, &mut Vec::new());
        game.step(&[(0, Input { pitch: 0.8, ..Default::default() })], 0.05, &mut Vec::new());
        assert_eq!(game.players[0].item, 1);
        assert_eq!(game.players[0].gold, 160);
        game.step(&[], 0.05, &mut Vec::new());
        game.step(&[(0, Input { pitch: 0.5, ..Default::default() })], 0.05, &mut Vec::new());
        assert!(game.players[0].stasis > 1.7);
        game.players[0].hp = 50.0;
        game.players[0].mana = 10.0;
        for _ in 0..31 {
            game.step(&[(0, Input { pitch: 0.6, ..Default::default() })], 0.1, &mut Vec::new());
        }
        assert!(game.players[0].runner.x < -35.0);
        for _ in 0..25 { game.step(&[], 0.1, &mut Vec::new()); }
        assert!(game.players[0].hp > 200.0);
        assert!(game.players[0].mana > 100.0);
    }
    #[test]
    fn portal_waits_five_seconds_then_starts_existing_minigame() {
        let mut game = PetArena::new(1, &[Contestant::person(0).on_team(0)], 17);
        game.time = DRAFT + 1.0;
        game.players[0].selected = true;
        game.players[0].runner.x = PORTALS[0].0;
        game.players[0].runner.z = PORTALS[0].1;
        let mut events = Vec::new();
        game.step(&[(0, Input { pitch: 1.4, ..Default::default() })], 0.1, &mut events);
        assert!(game.portal_queue.is_some());
        for _ in 0..52 { game.step(&[], 0.1, &mut events); }
        assert!(game.side_bout.is_some());
        assert_eq!(game.snapshot_for(Some(0)).unwrap()["crownfall_side"], "portal");
        assert!(events.iter().any(|e| e["kind"] == "crownfall_portal_match"));
    }
    #[test]
    fn rift_collects_only_players_who_arrive_before_countdown() {
        let contestants = (0..8)
            .map(|seat| Contestant::person(seat).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 18);
        game.time = RIFT_UNLOCK;
        for p in &mut game.players {
            p.selected = true;
            p.runner.x = if p.seat < 4 { 1.0 } else { 20.0 };
            p.runner.z = 0.0;
        }
        let mut events = Vec::new();
        game.side_events(&[(0, Input { pitch: 1.4, ..Default::default() })], 0.1, &mut events);
        assert!(game.rift_countdown.is_some());
        for _ in 0..101 { game.side_events(&[], 0.1, &mut events); }
        let side = game.side_bout.as_ref().expect("Rift starts a nested team contest");
        assert!(side.kind == SideKind::Rift);
        assert_eq!(side.seats, vec![0, 1, 2, 3]);
        assert_eq!(game.snapshot_for(Some(0)).unwrap()["crownfall_side"], "rift");
        assert_eq!(game.snapshot_for(Some(4)).unwrap()["game"], "pet_teamfight");
    }
    #[test]
    fn portal_result_returns_players_with_xp_and_gold() {
        let mut game = PetArena::new(1, &[
            Contestant::bot(0, 0.7).on_team(0),
            Contestant::bot(1, 0.7).on_team(1),
        ], 19);
        game.time = DRAFT + 1.0;
        let mut events = Vec::new();
        game.start_side_bout(SideKind::Portal, vec![0, 1], &mut events);
        for _ in 0..(120 * 60) {
            let inputs = [0, 1].map(|seat| (seat, game.bot_input(seat, 1.0 / 60.0)));
            game.tick_side_bout(&inputs, 1.0 / 60.0, &mut events);
            if game.side_bout.is_none() { break; }
        }
        assert!(game.side_bout.is_none(), "portal minigame must end");
        assert!(game.players.iter().all(|p| !p.busy && p.gold > 50));
        assert!(game.players.iter().all(|p| p.xp > 0 || p.level > 1));
        assert!(events.iter().any(|e| e["kind"] == "crownfall_portal_result"));
    }
    #[test]
    fn rift_team_contest_returns_a_buff_to_the_moba() {
        let contestants = (0..8)
            .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
            .collect::<Vec<_>>();
        let mut game = PetArena::new(1, &contestants, 20);
        game.time = RIFT_UNLOCK + 1.0;
        let seats = (0..8).collect::<Vec<_>>();
        let mut events = Vec::new();
        game.start_side_bout(SideKind::Rift, seats.clone(), &mut events);
        for _ in 0..(120 * 60) {
            let inputs = seats.iter().map(|&seat| (seat, game.bot_input(seat, 1.0 / 60.0))).collect::<Vec<_>>();
            game.tick_side_bout(&inputs, 1.0 / 60.0, &mut events);
            if game.side_bout.is_none() { break; }
        }
        assert!(game.side_bout.is_none(), "Rift team minigame must end");
        assert!(game.team_buff.iter().any(|&buff| buff > 0.0));
        assert!(game.players.iter().all(|p| !p.busy));
        assert!(events.iter().any(|e| e["kind"] == "crownfall_rift_result"));
    }
    #[test]
    fn both_arena_modes_finish_with_active_bots() {
        for kind in 0..2 {
            let contestants = (0..4)
                .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
                .collect::<Vec<_>>();
            let mut game = PetArena::new(kind, &contestants, 11);
            let mut events = Vec::new();
            for _ in 0..(game.limit() as usize * 60 + 2) {
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
                        "pet_minion_wave"
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
                    for _ in 0..(game.limit() as usize * 60 + 120) {
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
                        assert!(events.iter().any(|e| e["kind"] == "pet_minion_wave"));
                        assert!(
                            events
                                .iter()
                                .any(|e| e["kind"] == "crownfall_tower_hit"
                                    || e["kind"] == "pet_minion_farm")
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
        let samples = std::env::var("CROWNFALL_PACING_SAMPLES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(128)
            .max(2);
        for count in [2, 4, 6, 8]
            .into_iter()
            .filter(|&count| only.is_none_or(|wanted| wanted == count))
        {
            let mut times = Vec::new();
            let mut endings = [0u32; 2];
            let mut first = [0u32; 2];
            let mut direct_crown_hits = 0u32;
            for seed in 0..samples as u64 {
                let contestants = (0..count)
                    .map(|seat| Contestant::bot(seat, 0.7).on_team(seat % 2))
                    .collect::<Vec<_>>();
                let mut game = PetArena::new(1, &contestants, seed + 100);
                let mut events = Vec::new();
                let mut crown_exposed = None;
                while game.placements().is_none() {
                    let inputs = contestants
                        .iter()
                        .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                        .collect::<Vec<_>>();
                    game.step(&inputs, 1.0 / 60.0, &mut events);
                    if crown_exposed.is_none() && (game.crown_open(0) || game.crown_open(1)) { crown_exposed = Some(game.time); }
                }
                if seed == 0 { println!("Crownfall {count} crown exposed {:?}; tower HP {:?}; tower hits {}", crown_exposed, game.towers.iter().map(|t| t.hp.round() as i32).collect::<Vec<_>>(), events.iter().filter(|e| e["kind"] == "crownfall_tower_hit").count()); }
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
                times[samples / 2],
                times[(samples * 9 / 10).min(samples - 1)]
            );
        }
        for count in [4, 6, 8]
            .into_iter()
            .filter(|&count| only.is_none_or(|wanted| wanted == count))
        {
            for flipped in [false, true] {
                let mut side = [0u32; 2];
                let mut seat_parity = [0u32; 2];
                for seed in 0..(samples / 2) as u64 {
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
    #[test]
    fn champion_preview_can_cast_before_confirmation_and_all_ready_starts_match() {
        let mut game = PetArena::new(
            1,
            &[
                Contestant::person(0).on_team(0),
                Contestant::bot(1, 0.5).on_team(1),
            ],
            13,
        );
        let mut events = Vec::new();
        game.step(
            &[(
                0,
                Input {
                    yaw: 0.0,
                    pitch: 1.2,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut events,
        );
        assert_eq!(game.players[0].champ, 0);
        assert!(!game.players[0].selected);
        let action = game.players[0].action_n;
        game.step(
            &[(
                0,
                Input {
                    yaw: 0.0,
                    pitch: 1.3,
                    fire: true,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut events,
        );
        assert!(game.players[0].action_n > action);
        game.step(
            &[(
                0,
                Input {
                    yaw: 0.0,
                    pitch: 1.0,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut events,
        );
        assert!(game.players[0].selected);
        game.time = 8.0;
        game.step(&[], 1.0 / 60.0, &mut events);
        assert_eq!(game.time, DRAFT);
        assert!(!game.snapshot()["draft"].as_bool().unwrap());
    }
    #[test]
    fn auto_attack_last_hit_grants_extra_xp_mana_and_counter() {
        let mut game = PetArena::new(
            1,
            &[
                Contestant::person(0).on_team(0),
                Contestant::bot(1, 0.5).on_team(1),
            ],
            14,
        );
        game.time = DRAFT + 1.0;
        game.players[0].selected = true;
        game.players[0].runner = Runner::at(0.0, 0.0);
        game.players[0].mana = 30.0;
        game.minions.push(Minion {
            id: 1,
            team: 1,
            lane: 0,
            x: 2.0,
            z: 0.0,
            hp: 20.0,
            attack: 0.0,
        });
        let mut events = Vec::new();
        game.auto_attack(0, &mut events);
        assert_eq!(game.players[0].last_hits, 1);
        assert_eq!(game.players[0].xp, 22);
        assert_eq!(game.players[0].mana, 38.0);
        assert!(events.iter().any(|e| e["kind"] == "pet_minion_farm"));
    }
}
