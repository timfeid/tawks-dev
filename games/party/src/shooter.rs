//! Paint Blasters: a first person round in a walled arena. Splat a rival three
//! times to take them out, they come back a couple of seconds later, and
//! whoever has the most splats when the clock runs out wins.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::fpmove::{self, Block, Body, EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_RADIUS};
use crate::bout::{Contestant, Input, Bout, Skill};

pub const ARENA_HALF: f32 = 26.0;
pub const MAX_HP: i16 = 150;
pub const SHOT_DAMAGE: i16 = 25;
pub const SHOT_RANGE: f32 = 60.0;
pub const FIRE_COOLDOWN: f32 = 0.28;
pub const RESPAWN_SECONDS: f32 = 2.5;
pub const ROUND_SECONDS: f32 = 90.0;

const BOT_REACTION: f32 = 0.38;

/// A big open yard: a stepped platform in the middle, long walls with gaps,
/// crates and pillars, laid out the same way whichever corner you spawn in.
pub fn arena() -> Vec<Block> {
    let mut blocks = vec![Block {
        x: 0.0,
        z: 0.0,
        hx: 5.0,
        hz: 5.0,
        top: 1.6,
        bottom: 0.0,
    }];
    // Steps up onto the middle platform, low enough to hop.
    for (x, z) in [(8.0, 0.0), (-8.0, 0.0), (0.0, 8.0), (0.0, -8.0)] {
        blocks.push(Block {
            x,
            z,
            hx: 1.8,
            hz: 1.8,
            top: 0.8,
            bottom: 0.0,
        });
    }
    for (sx, sz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        blocks.push(Block {
            x: 14.0 * sx,
            z: 14.0 * sz,
            hx: 1.8,
            hz: 1.8,
            top: 2.4,
            bottom: 0.0,
        });
        blocks.push(Block {
            x: 21.0 * sx,
            z: 8.5 * sz,
            hx: 1.0,
            hz: 5.0,
            top: 1.7,
            bottom: 0.0,
        });
        blocks.push(Block {
            x: 8.5 * sx,
            z: 21.0 * sz,
            hx: 5.0,
            hz: 1.0,
            top: 1.7,
            bottom: 0.0,
        });
        blocks.push(Block {
            x: 10.5 * sx,
            z: 6.0 * sz,
            hx: 1.5,
            hz: 1.5,
            top: 0.9,
            bottom: 0.0,
        });
        blocks.push(Block {
            x: 19.0 * sx,
            z: 19.0 * sz,
            hx: 0.9,
            hz: 0.9,
            top: 3.2,
            bottom: 0.0,
        });
        blocks.push(Block {
            x: 23.0 * sx,
            z: 0.0,
            hx: 1.2,
            hz: 2.4,
            top: 1.2,
            bottom: 0.0,
        });
    }
    blocks
}

pub fn spawn_points() -> Vec<(f32, f32)> {
    vec![
        (-22.0, -22.0),
        (22.0, 22.0),
        (22.0, -22.0),
        (-22.0, 22.0),
        (0.0, -23.0),
        (0.0, 23.0),
        (-23.5, 4.0),
        (23.5, -4.0),
    ]
}

pub fn target_lane_centers(count: usize) -> Vec<f32> {
    let center = (count.saturating_sub(1) as f32) * 3.5;
    (0..count)
        .map(|index| index as f32 * 7.0 - center)
        .collect()
}

/// A long shooting range with one fully divided lane per contestant.
pub fn target_range(count: usize) -> (Vec<Block>, Vec<(f32, f32)>, f32) {
    let lanes = target_lane_centers(count);
    let half_x = (count.max(1) as f32 * 3.5 + 1.0).max(8.0);
    let mut blocks = Vec::new();
    for pair in lanes.windows(2) {
        blocks.push(Block {
            x: (pair[0] + pair[1]) * 0.5,
            z: -1.5,
            hx: 0.18,
            hz: 33.0,
            top: 3.8,
            bottom: 0.0,
        });
    }
    blocks.push(Block {
        x: 0.0,
        z: -32.2,
        hx: half_x - 0.45,
        hz: 0.55,
        top: 4.0,
        bottom: 0.0,
    });
    let spawns = lanes.into_iter().map(|x| (x, 27.0)).collect();
    (blocks, spawns, half_x)
}

/// A compact, circular popgun pit.  Players get a moment of cover after the
/// bell, then need to move through the gaps to take a clean one-shot hit.
/// Popgun Knockout's map: Timberline, a meadow between two ridges. A lookout
/// in the middle joins the ridges by bridges you can walk under, the flanks are
/// woods whose canopies stop paint, and every level has a way up and down.
/// It is point-symmetric so no spawn has the better side.
pub struct Timberline {
    pub blocks: Vec<Block>,
    pub kinds: Vec<&'static str>,
    pub spawns: Vec<(f32, f32)>,
}

const RIDGE_TOP: f32 = 2.4;
const STEP_RISE: f32 = 0.2;
const STEP_RUN: f32 = 0.45;

impl Timberline {
    fn solid(&mut self, x: f32, z: f32, hx: f32, hz: f32, top: f32, kind: &'static str) {
        self.blocks.push(Block { x, z, hx, hz, top, bottom: 0.0 });
        self.kinds.push(kind);
    }

    fn floating(&mut self, x: f32, z: f32, hx: f32, hz: f32, bottom: f32, top: f32, kind: &'static str) {
        self.blocks.push(Block::floating(x, z, hx, hz, bottom, top));
        self.kinds.push(kind);
    }

    /// Stairs climbing to `top`, ending against an edge at `edge` on one axis.
    /// `outward` is the direction the stairs run away from that edge; risers
    /// are low enough to walk up without jumping.
    fn stairs(&mut self, along_x: bool, edge: f32, outward: f32, across: f32, half_width: f32, top: f32) {
        let steps = (top / STEP_RISE).round() as usize;
        for i in 1..=steps {
            let height = STEP_RISE * i as f32;
            let offset = (steps - i) as f32 * STEP_RUN + STEP_RUN / 2.0;
            let at = edge + outward * offset;
            if along_x {
                self.solid(at, across, STEP_RUN / 2.0, half_width, height, "step");
            } else {
                self.solid(across, at, half_width, STEP_RUN / 2.0, height, "step");
            }
        }
    }

    fn tree(&mut self, x: f32, z: f32) {
        self.solid(x, z, 0.32, 0.32, 4.4, "trunk");
        self.floating(x, z, 1.7, 1.7, 2.5, 4.3, "leaves");
        self.floating(x, z, 1.0, 1.0, 4.3, 5.6, "leaves");
    }

    pub fn build() -> Self {
        let mut map = Timberline { blocks: Vec::new(), kinds: Vec::new(), spawns: Vec::new() };
        // Everything below is placed once for the north half and mirrored
        // through the centre for the south.
        for sign in [-1.0f32, 1.0] {
            let m = |x: f32, z: f32| (x * sign, z * sign);

            // The ridge, reached by two flights of stairs from the meadow.
            let (x, z) = m(0.0, -21.5);
            map.solid(x, z, 14.0, 4.5, RIDGE_TOP, "earth");
            for across in [-9.5f32, 9.5] {
                map.stairs(false, -17.0 * sign, sign, across * sign, 1.2, RIDGE_TOP);
            }
            // Rocks on the ridge to duck behind.
            for (rx, rz, hx, hz) in [(-5.5, -19.6, 1.3, 0.7), (4.0, -23.5, 1.0, 1.0), (12.0, -19.5, 0.8, 1.2)] {
                let (x, z) = m(rx, rz);
                map.solid(x, z, hx, hz, RIDGE_TOP + 1.05, "rock");
            }

            // The bridge from the ridge to the lookout, high enough to walk under.
            let (x, z) = m(0.0, -9.75);
            map.floating(x, z, 1.1, 7.25, 1.95, RIDGE_TOP, "wood");

            // Meadow cover: low rocks and a fallen log, good for crouching.
            for (rx, rz, hx, hz, top, kind) in [
                (-6.0, -6.5, 1.1, 0.8, 1.0, "rock"),
                (-15.0, 2.5, 0.9, 1.2, 1.1, "rock"),
                (8.5, -3.5, 2.3, 0.35, 0.6, "log"),
                (4.5, -13.0, 0.9, 0.9, 0.95, "rock"),
            ] {
                let (x, z) = m(rx, rz);
                map.solid(x, z, hx, hz, top, kind);
            }

            // The woods on this side's flank.
            for (tx, tz) in [(-20.5, -9.0), (-23.0, -1.5), (-18.5, 5.5), (-22.5, 11.5), (-15.5, -4.0), (-7.0, 12.5)] {
                let (x, z) = m(tx, tz);
                map.tree(x, z);
            }

            for (sx, sz) in [(-11.5, -23.5), (11.0, -24.0), (-24.0, 4.0), (-4.0, -11.0)] {
                map.spawns.push(m(sx, sz));
            }
        }

        // The lookout in the middle, with stairs up from east and west.
        map.solid(0.0, 0.0, 2.5, 2.5, RIDGE_TOP, "wood");
        map.stairs(true, 2.5, 1.0, 0.0, 1.1, RIDGE_TOP);
        map.stairs(true, -2.5, -1.0, 0.0, 1.1, RIDGE_TOP);
        map
    }
}

/// The height of the ground under a point: the floor or the highest block
/// resting there.
pub fn ground_height(blocks: &[Block], x: f32, z: f32) -> f32 {
    blocks
        .iter()
        .filter(|b| b.bottom == 0.0 && b.contains_xz(x, z, 0.0))
        .fold(0.0, |h, b| h.max(b.top))
}
/// A wide pinball-table layout for Ricochet Rumble.  The staggered banks make
/// side approaches and rebound shots meaningful instead of a central brawl.
pub fn ricochet_table() -> (Vec<Block>, Vec<(f32, f32)>) {
    let mut blocks = Vec::new();
    for (x, z, hx, hz, top) in [
        (-22.0, -10.0, 1.0, 7.0, 2.4),
        (22.0, 10.0, 1.0, 7.0, 2.4),
        (-11.0, 8.0, 5.5, 1.0, 1.8),
        (11.0, -8.0, 5.5, 1.0, 1.8),
        (-4.5, -15.0, 2.0, 2.0, 2.8),
        (4.5, 15.0, 2.0, 2.0, 2.8),
        (0.0, 0.0, 2.3, 2.3, 1.1),
    ] {
        blocks.push(Block { x, z, hx, hz, top, bottom: 0.0 });
    }
    (
        blocks,
        vec![
            (-30.0, -18.0),
            (30.0, 18.0),
            (30.0, -18.0),
            (-30.0, 18.0),
            (0.0, -20.0),
            (0.0, 20.0),
        ],
    )
}

/// A full sized bomb site: the planted core sits behind a low wall, with two
/// approach lanes, crates to break sight lines and a raised ledge over each
/// entrance. Big enough to rotate around rather than trade shots across.
pub fn retake_site() -> Vec<Block> {
    [
        // The site itself: a low wall behind the bomb and two shoulders.
        (0.0, 0.0, 4.0, 1.1, 1.25),
        (-7.5, -2.0, 1.2, 3.0, 1.6),
        (7.5, -2.0, 1.2, 3.0, 1.6),
        (0.0, -9.0, 2.6, 1.2, 1.0),
        // Lane walls funnelling the two approaches.
        (-16.0, -6.0, 1.2, 9.0, 3.0),
        (16.0, -6.0, 1.2, 9.0, 3.0),
        (-16.0, 12.0, 1.2, 6.0, 3.0),
        (16.0, 12.0, 1.2, 6.0, 3.0),
        // Mid cover between the site and the retake spawns.
        (-8.0, 9.0, 3.2, 1.2, 1.7),
        (8.0, 9.0, 3.2, 1.2, 1.7),
        (0.0, 16.0, 2.4, 2.4, 2.2),
        (-24.0, 2.0, 2.2, 2.2, 2.6),
        (24.0, 2.0, 2.2, 2.2, 2.6),
        // Ledges either side of the site, reachable with a jump from the crates.
        (-11.0, 5.5, 2.4, 1.6, 0.9),
        (11.0, 5.5, 2.4, 1.6, 0.9),
        // Back of the site, where the defenders hold.
        (-20.0, -17.0, 3.0, 1.2, 2.0),
        (20.0, -17.0, 3.0, 1.2, 2.0),
        (0.0, -22.0, 5.0, 1.2, 2.4),
    ]
    .into_iter()
    .map(|(x, z, hx, hz, top)| Block { x, z, hx, hz, top, bottom: 0.0 })
    .collect()
}

#[derive(Clone, Debug)]
pub struct Fighter {
    pub seat: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub hp: i16,
    pub kills: u32,
    pub deaths: u32,
    pub on_ground: bool,
    pub cooldown: f32,
    pub respawn_in: f32,
    pub crouching: bool,
}

impl Fighter {
    fn alive(&self) -> bool {
        self.hp > 0
    }

    fn eye(&self) -> (f32, f32, f32) {
        (self.x, self.y + fpmove::stance(self.crouching).1, self.z)
    }

    pub fn height(&self) -> f32 {
        fpmove::stance(self.crouching).0
    }

    fn look(&self) -> (f32, f32, f32) {
        fpmove::look(self.yaw, self.pitch)
    }
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    target: Option<u8>,
    think_in: f32,
    saw_target_for: f32,
    strafe: f32,
    aim_error: (f32, f32),
}

pub struct PaintBlasters {
    pub fighters: Vec<Fighter>,
    pub time: f32,
    blocks: Vec<Block>,
    block_kinds: Vec<&'static str>,
    spawns: Vec<(f32, f32)>,
    rng: StdRng,
    brains: HashMap<u8, BotBrain>,
    placements: Option<Vec<Vec<u8>>>,
    damage: i16,
    respawn_seconds: f32,
    round_seconds: f32,
    half_x: f32,
    half_z: f32,
    player_damage: bool,
    allow_movement: bool,
    teams: HashMap<u8, u8>,
    skills: HashMap<u8, Skill>,
}

impl PaintBlasters {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let spawns = spawn_points();
        let fighters = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (x, z) = spawns[i % spawns.len()];
                let mut fighter = blank(c.seat);
                fighter.x = x;
                fighter.z = z;
                fighter.yaw = facing_center(x, z);
                fighter
            })
            .collect();
        let brains = contestants
            .iter()
            .filter(|c| c.is_bot)
            .map(|c| {
                (
                    c.seat,
                    BotBrain {
                        think_in: 0.3,
                        ..Default::default()
                    },
                )
            })
            .collect();
        PaintBlasters {
            fighters,
            time: 0.0,
            blocks: arena(),
            block_kinds: Vec::new(),
            spawns,
            rng: StdRng::seed_from_u64(seed),
            brains,
            placements: None,
            damage: SHOT_DAMAGE,
            respawn_seconds: RESPAWN_SECONDS,
            round_seconds: ROUND_SECONDS,
            half_x: ARENA_HALF,
            half_z: ARENA_HALF,
            player_damage: true,
            allow_movement: true,
            teams: HashMap::new(),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
        }
    }

    /// The skill a playlist wrapper needs when it drives its own bots.
    pub fn skill_of(&self, seat: u8) -> Skill {
        self.skill(seat)
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Reconfigure this otherwise normal first-person simulation as a target
    /// range.  Keeping movement and shot authority shared avoids a client-only
    /// "map" that disagrees with collisions.
    pub fn use_target_range(&mut self) {
        let (blocks, spawns, half_x) = target_range(self.fighters.len());
        self.blocks = blocks;
        self.spawns = spawns;
        self.half_x = half_x;
        self.half_z = 36.0;
        self.player_damage = false;
        self.allow_movement = false;
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            let (x, z) = self.spawns[index % self.spawns.len()];
            fighter.x = x;
            fighter.z = z;
            fighter.y = 0.0;
            fighter.vx = 0.0;
            fighter.vy = 0.0;
            fighter.vz = 0.0;
            fighter.yaw = 0.0;
        }
    }

    /// Popgun Knockout: every direct hit is decisive and there are no
    /// respawns.  The wrapper owns the shorter party-round finish/ranking.
    pub fn use_popgun_knockout(&mut self) {
        let map = Timberline::build();
        self.blocks = map.blocks;
        self.block_kinds = map.kinds;
        self.spawns = map.spawns;
        self.half_x = 26.0;
        self.half_z = 26.0;
        self.damage = MAX_HP;
        self.respawn_seconds = f32::INFINITY;
        self.round_seconds = f32::INFINITY;
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            let (x, z) = self.spawns[index % self.spawns.len()];
            fighter.x = x;
            fighter.z = z;
            fighter.y = ground_height(&self.blocks, x, z);
            fighter.vx = 0.0;
            fighter.vy = 0.0;
            fighter.vz = 0.0;
            fighter.yaw = facing_center(x, z);
        }
    }

    pub fn use_ricochet_table(&mut self) {
        let (blocks, spawns) = ricochet_table();
        self.blocks = blocks;
        self.spawns = spawns;
        self.half_x = 34.0;
        self.half_z = 24.0;
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            let (x, z) = self.spawns[index % self.spawns.len()];
            fighter.x = x;
            fighter.z = z;
            fighter.y = 0.0;
            fighter.vx = 0.0;
            fighter.vy = 0.0;
            fighter.vz = 0.0;
            fighter.yaw = facing_center(x, z);
        }
    }

    pub fn use_retake_site(&mut self) {
        self.blocks = retake_site();
        self.spawns = vec![
            (-10.0, -26.0),
            (-26.0, 26.0),
            (10.0, -26.0),
            (26.0, 26.0),
            (0.0, -28.0),
            (0.0, 27.0),
        ];
        self.half_x = 34.0;
        self.half_z = 32.0;
        self.damage = 50;
        self.respawn_seconds = f32::INFINITY;
        self.round_seconds = f32::INFINITY;
        self.teams.clear();
        // On your own you get the retake, so there is a bomb to defuse.
        let solo = self.fighters.len() == 1;
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            let team = if solo { 1 } else { (index % 2) as u8 };
            self.teams.insert(fighter.seat, team);
            let team_index = index / 2;
            let candidates = if team == 0 {
                [(-10.0, -26.0), (10.0, -26.0), (0.0, -28.0), (-18.0, -24.0)]
            } else {
                [(-26.0, 26.0), (26.0, 26.0), (0.0, 27.0), (18.0, 27.0)]
            };
            let (x, z) = candidates[team_index % candidates.len()];
            fighter.x = x;
            fighter.z = z;
            fighter.y = 0.0;
            fighter.vx = 0.0;
            fighter.vy = 0.0;
            fighter.vz = 0.0;
            fighter.yaw = facing_center(x, z);
        }
    }

    pub fn team_of(&self, seat: u8) -> Option<u8> {
        self.teams.get(&seat).copied()
    }

    fn fighter(&self, seat: u8) -> Option<&Fighter> {
        self.fighters.iter().find(|f| f.seat == seat)
    }

    /// The spawn furthest from everyone still standing.
    fn respawn_spot(&self, seat: u8) -> (f32, f32) {
        let live: Vec<(f32, f32)> = self
            .fighters
            .iter()
            .filter(|f| f.seat != seat && f.alive())
            .map(|f| (f.x, f.z))
            .collect();
        self.spawns
            .iter()
            .copied()
            .max_by(|a, b| {
                let score = |p: (f32, f32)| {
                    live.iter()
                        .map(|o| (o.0 - p.0).powi(2) + (o.1 - p.1).powi(2))
                        .fold(f32::INFINITY, f32::min)
                };
                score(*a)
                    .partial_cmp(&score(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or((0.0, 0.0))
    }

    fn move_fighter(&mut self, index: usize, input: Input, dt: f32) {
        let blocks = self.blocks.clone();
        let fighter = &mut self.fighters[index];
        fighter.yaw = input.yaw;
        fighter.pitch = input.pitch.clamp(-1.4, 1.4);
        fighter.crouching = input.crouch && fighter.on_ground;
        let mut body = Body {
            x: fighter.x,
            y: fighter.y,
            z: fighter.z,
            vx: fighter.vx,
            vy: fighter.vy,
            vz: fighter.vz,
            on_ground: fighter.on_ground,
        };
        fpmove::step_body(&mut body, input, dt, &blocks, self.half_x, self.half_z);
        fighter.x = body.x;
        fighter.y = body.y;
        fighter.z = body.z;
        fighter.vx = body.vx;
        fighter.vy = body.vy;
        fighter.vz = body.vz;
        fighter.on_ground = body.on_ground;
    }

    fn fire(&mut self, seat: u8, events: &mut Vec<Value>) {
        let Some(shooter) = self.fighter(seat).cloned() else {
            return;
        };
        let (ox, oy, oz) = shooter.eye();
        let (dx, dy, dz) = shooter.look();
        let wall = self.wall_distance((ox, oy, oz), (dx, dy, dz));
        // Target Frenzy uses the same authoritative blaster simulation, but
        // shots pass through other contestants so the round stays about aim.
        let hit = if self.player_damage {
            self.first_target(seat, (ox, oy, oz), (dx, dy, dz), wall)
        } else {
            None
        };
        let range = hit.map(|(_, t)| t).unwrap_or(wall.min(SHOT_RANGE));
        events.push(json!({
            "kind": "shot",
            "seat": seat,
            "from": [round(ox), round(oy), round(oz)],
            "to": [round(ox + dx * range), round(oy + dy * range), round(oz + dz * range)],
            "hit": hit.is_some(),
        }));
        let Some((target_seat, travel)) = hit else {
            return;
        };
        let (hx, hy, hz) = (ox + dx * travel, oy + dy * travel, oz + dz * travel);
        let mut down = false;
        if let Some(target) = self.fighters.iter_mut().find(|f| f.seat == target_seat) {
            target.hp -= self.damage;
            if target.hp <= 0 {
                target.hp = 0;
                target.deaths += 1;
                target.respawn_in = self.respawn_seconds;
                target.vx = 0.0;
                target.vz = 0.0;
                down = true;
            }
        }
        events.push(json!({
            "kind": "splat",
            "seat": target_seat,
            "by": seat,
            "x": round(hx),
            "y": round(hy),
            "z": round(hz),
            "down": down,
        }));
        if down {
            if let Some(shooter) = self.fighters.iter_mut().find(|f| f.seat == seat) {
                shooter.kills += 1;
            }
            events.push(json!({ "kind": "down", "seat": target_seat, "by": seat }));
        }
    }

    /// How far a shot travels before a block or a wall stops it.
    fn wall_distance(&self, origin: (f32, f32, f32), dir: (f32, f32, f32)) -> f32 {
        fpmove::clear_distance(
            origin,
            dir,
            &self.blocks,
            self.half_x,
            self.half_z,
            SHOT_RANGE,
        )
    }

    /// The closest rival the shot reaches before it hits something solid.
    fn first_target(
        &self,
        seat: u8,
        origin: (f32, f32, f32),
        dir: (f32, f32, f32),
        limit: f32,
    ) -> Option<(u8, f32)> {
        self.fighters
            .iter()
            .filter(|f| f.seat != seat && f.alive())
            .filter(|f| self.team_of(seat).is_none() || self.team_of(seat) != self.team_of(f.seat))
            .filter_map(|f| {
                fpmove::ray_player(origin, dir, (f.x, f.y, f.z), PLAYER_RADIUS, f.height())
                    .map(|t| (f.seat, t))
            })
            .filter(|(_, t)| *t <= limit)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    fn can_see(&self, from: &Fighter, to: &Fighter) -> bool {
        fpmove::can_see(
            from.eye(),
            (to.x, to.y + to.height() * 0.6, to.z),
            &self.blocks,
            self.half_x,
            self.half_z,
        )
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < self.round_seconds {
            return;
        }
        let mut rows: Vec<(u32, i64, u8)> = self
            .fighters
            .iter()
            .map(|f| (f.kills, -(f.deaths as i64), f.seat))
            .collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, i64)> = None;
        for (kills, deaths, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (kills, deaths) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((kills, deaths));
        }
        self.placements = Some(groups);
    }
}

fn blank(seat: u8) -> Fighter {
    Fighter {
        seat,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        vx: 0.0,
        vy: 0.0,
        vz: 0.0,
        yaw: 0.0,
        pitch: 0.0,
        hp: MAX_HP,
        kills: 0,
        deaths: 0,
        on_ground: true,
        cooldown: 0.0,
        respawn_in: 0.0,
        crouching: false,
    }
}

/// The yaw that points a fighter at the middle of the arena.
fn facing_center(x: f32, z: f32) -> f32 {
    x.atan2(z)
}

fn round(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

impl Bout for PaintBlasters {
    fn key(&self) -> &'static str {
        "shooter"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "shooter",
            "half": self.half_x,
            "half_z": self.half_z,
            "hp": MAX_HP,
            "eye": EYE_HEIGHT,
            "height": PLAYER_HEIGHT,
            "radius": PLAYER_RADIUS,
            "blocks": self.blocks.iter().enumerate().map(|(i, b)| json!({
                "x": b.x, "z": b.z, "hx": b.hx, "hz": b.hz, "top": b.top, "bottom": b.bottom,
                "kind": self.block_kinds.get(i).copied().unwrap_or("wall"),
            })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        for index in 0..self.fighters.len() {
            let seat = self.fighters[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, i)| *i)
                .unwrap_or_default();
            let fighter = &mut self.fighters[index];
            fighter.cooldown = (fighter.cooldown - dt).max(0.0);
            if !fighter.alive() {
                fighter.respawn_in -= dt;
                fighter.yaw = input.yaw;
                fighter.pitch = input.pitch.clamp(-1.4, 1.4);
                if fighter.respawn_in <= 0.0 {
                    let (x, z) = self.respawn_spot(seat);
                    let fighter = &mut self.fighters[index];
                    fighter.x = x;
                    fighter.z = z;
                    fighter.y = 0.0;
                    fighter.vx = 0.0;
                    fighter.vy = 0.0;
                    fighter.vz = 0.0;
                    fighter.yaw = facing_center(x, z);
                    fighter.pitch = 0.0;
                    fighter.hp = MAX_HP;
                    fighter.on_ground = true;
                    events.push(json!({ "kind": "respawn", "seat": seat }));
                }
                continue;
            }
            if self.allow_movement {
                self.move_fighter(index, input, dt);
            } else {
                let fighter = &mut self.fighters[index];
                fighter.yaw = input.yaw;
                fighter.pitch = input.pitch.clamp(-1.4, 1.4);
                fighter.vx = 0.0;
                fighter.vy = 0.0;
                fighter.vz = 0.0;
            }
            let fighter = &self.fighters[index];
            if input.fire && fighter.cooldown <= 0.0 {
                if let Some(fighter) = self.fighters.get_mut(index) {
                    fighter.cooldown = FIRE_COOLDOWN;
                }
                self.fire(seat, events);
            }
        }

        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.fighter(seat).cloned() else {
            return Input::default();
        };
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = self.rng.random_range(0.4..1.1);
            brain.strafe = [-1.0, 0.0, 1.0][self.rng.random_range(0..3)];
            let wobble = self.skill(seat).aim_error();
            brain.aim_error = (
                self.rng.random_range(-wobble..wobble),
                self.rng.random_range(-wobble * 0.5..wobble * 0.5),
            );
            let rivals: Vec<(u8, f32, bool)> = self
                .fighters
                .iter()
                .filter(|f| f.seat != seat && f.alive())
                .filter(|f| {
                    self.team_of(seat).is_none() || self.team_of(seat) != self.team_of(f.seat)
                })
                .map(|f| {
                    let distance = ((f.x - me.x).powi(2) + (f.z - me.z).powi(2)).sqrt();
                    (f.seat, distance, self.can_see(&me, f))
                })
                .collect();
            brain.target = rivals
                .iter()
                .min_by(|a, b| {
                    let score = |r: &(u8, f32, bool)| r.1 * if r.2 { 1.0 } else { 2.5 };
                    score(a)
                        .partial_cmp(&score(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(s, _, _)| *s);
        }

        let mut input = Input {
            yaw: me.yaw,
            pitch: me.pitch,
            ..Default::default()
        };
        let target = brain
            .target
            .and_then(|t| self.fighter(t).cloned())
            .filter(|t| t.alive());
        if let Some(target) = target {
            let (dx, dz) = (target.x - me.x, target.z - me.z);
            let distance = (dx * dx + dz * dz).sqrt().max(0.01);
            let visible = me.alive() && self.can_see(&me, &target);
            let want_yaw = (-dx).atan2(-dz) + brain.aim_error.0;
            let rise = (target.y + PLAYER_HEIGHT * 0.55) - (me.y + EYE_HEIGHT);
            let want_pitch = (rise / distance).atan() + brain.aim_error.1;
            // Swing toward the mark rather than snapping onto it.
            let track = self.skill(seat).tracking();
            input.yaw = me.yaw + wrap_angle(want_yaw - me.yaw) * (track * dt).min(1.0);
            input.pitch = me.pitch + (want_pitch - me.pitch) * (track * dt).min(1.0);
            let (fx, fz) = ((-input.yaw.sin()), (-input.yaw.cos()));
            let facing = (fx * dx + fz * dz) / distance;
            brain.saw_target_for = if visible {
                brain.saw_target_for + dt
            } else {
                0.0
            };
            let skill = self.skill(seat);
            if visible && facing > 0.985 && brain.saw_target_for > skill.reaction() {
                // A clumsy bot keeps missing its moment.
                input.fire = !self.rng.random_bool(skill.hesitation());
            }
            // Close the gap, then circle while shooting.
            let toward = if distance > 11.0 {
                1.0
            } else if distance < 4.0 {
                -0.6
            } else {
                0.0
            };
            input.stick = crate::bout::Stick {
                x: brain.strafe * if visible { 0.9 } else { 0.3 },
                z: -toward,
            }
            .clamped();
        } else {
            input.stick = crate::bout::Stick { x: 0.0, z: -0.6 };
            input.yaw = me.yaw + 0.6 * dt;
        }
        self.brains.insert(seat, brain);
        input
    }

    fn snapshot(&self) -> Value {
        json!({
            "game": "shooter",
            "time": round(self.time),
            "limit": self.round_seconds,
            "players": self.fighters.iter().map(|f| json!({
                "seat": f.seat,
                "x": round(f.x),
                "y": round(f.y),
                "z": round(f.z),
                "vx": round(f.vx),
                "vz": round(f.vz),
                "yaw": round(f.yaw),
                "pitch": round(f.pitch),
                "hp": f.hp,
                "kills": f.kills,
                "deaths": f.deaths,
                "alive": f.alive(),
                "respawn": round(f.respawn_in.max(0.0)),
                "ground": f.on_ground,
                "crouch": f.crouching,
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

/// The shortest way round the circle, so turns never take the long path.
fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut angle = a % tau;
    if angle > std::f32::consts::PI {
        angle -= tau;
    }
    if angle < -std::f32::consts::PI {
        angle += tau;
    }
    angle
}
