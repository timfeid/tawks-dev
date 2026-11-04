//! Paint Blasters: first-person speedball. One paintball eliminates a rival;
//! each team plays up to seven rounds, with the first four wins taking it.
//! Paint variants reuse the movement and shot physics with their own rules.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill};
use crate::fpmove::{self, Block, Body, EYE_HEIGHT, PLAYER_HEIGHT, PLAYER_RADIUS};

pub const ARENA_HALF: f32 = 26.0;
pub const MAX_HP: i16 = 150;
pub const SHOT_DAMAGE: i16 = 25;
pub const SHOT_RANGE: f32 = 60.0;
pub const FIRE_COOLDOWN: f32 = 0.22;
/// Paintballs travel quickly, but far shots need a little elevation.
pub const PAINTBALL_SPEED: f32 = 48.0;
pub const PAINTBALL_GRAVITY: f32 = 9.8;
pub const RESPAWN_SECONDS: f32 = 2.5;
pub const ROUND_SECONDS: f32 = 90.0;
pub const SERIES_ROUND_SECONDS: f32 = 25.0;
pub const SERIES_BREAK_SECONDS: f32 = 2.5;
pub const DIVE_SECONDS: f32 = 0.38;
pub const DIVE_COOLDOWN: f32 = 3.0;

/// Mirrored speedball lanes with inflatable bunkers and open flanks.
fn arena_layout() -> (Vec<Block>, Vec<&'static str>) {
    let mut blocks = Vec::new();
    let mut kinds = Vec::new();
    let mut bunker = |kind, x, z, hx, hz, top| {
        blocks.push(Block {
            x,
            z,
            hx,
            hz,
            top,
            bottom: 0.0,
        });
        kinds.push(kind);
    };
    // Square midfield cover and round wing cans make the speedball lanes readable.
    bunker("square", 0.0, 0.0, 1.8, 1.8, 2.2);
    for sign in [-1.0f32, 1.0] {
        bunker("brick", 0.0, sign * 9.5, 2.8, 0.9, 1.5);
        for wing in [-1.0f32, 1.0] {
            bunker("brick", wing * 9.0, sign * 6.0, 1.1, 2.5, 2.0);
            bunker("round", wing * 16.5, sign * 13.0, 1.6, 1.6, 1.65);
            bunker("brick", wing * 9.5, sign * 19.5, 2.1, 0.85, 1.4);
            bunker("brick", wing * 20.5, sign * 2.0, 0.8, 2.6, 1.8);
        }
    }
    (blocks, kinds)
}

pub fn arena() -> Vec<Block> {
    arena_layout().0
}

/// A map laid out for four, spread out for a bigger field: every block moves
/// out from the middle and grows by `size` across the floor, but keeps its
/// height so the jumps and crouches still work the same.
pub fn stretch(blocks: Vec<Block>, size: f32) -> Vec<Block> {
    blocks
        .into_iter()
        .map(|b| Block {
            x: b.x * size,
            z: b.z * size,
            hx: b.hx * size,
            hz: b.hz * size,
            ..b
        })
        .collect()
}

/// Spawn spots moved out from the middle by `size`, to match a stretched map.
pub fn spread(spots: &[(f32, f32)], size: f32) -> Vec<(f32, f32)> {
    spots.iter().map(|&(x, z)| (x * size, z * size)).collect()
}

pub fn spawn_points() -> Vec<(f32, f32)> {
    vec![
        (-22.0, -22.0),
        (22.0, 22.0),
        (22.0, -22.0),
        (-22.0, 22.0),
        (0.0, -23.0),
        (0.0, 23.0),
        (-14.0, -23.0),
        (14.0, 23.0),
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
        self.blocks.push(Block {
            x,
            z,
            hx,
            hz,
            top,
            bottom: 0.0,
        });
        self.kinds.push(kind);
    }

    fn floating(
        &mut self,
        x: f32,
        z: f32,
        hx: f32,
        hz: f32,
        bottom: f32,
        top: f32,
        kind: &'static str,
    ) {
        self.blocks.push(Block::floating(x, z, hx, hz, bottom, top));
        self.kinds.push(kind);
    }

    /// Stairs climbing to `top`, ending against an edge at `edge` on one axis.
    /// `outward` is the direction the stairs run away from that edge; risers
    /// are low enough to walk up without jumping.
    fn stairs(
        &mut self,
        along_x: bool,
        edge: f32,
        outward: f32,
        across: f32,
        half_width: f32,
        top: f32,
    ) {
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
        Self::build_for(1.0)
    }

    /// The map spread out by `size` for a bigger field. The ridges, lookout,
    /// rocks and logs grow across the floor; stairs keep their steps and the
    /// trees keep their size, they just stand further apart.
    pub fn build_for(size: f32) -> Self {
        let mut map = Timberline {
            blocks: Vec::new(),
            kinds: Vec::new(),
            spawns: Vec::new(),
        };
        // Everything below is placed once for the north half and mirrored
        // through the centre for the south.
        for sign in [-1.0f32, 1.0] {
            let m = |x: f32, z: f32| (x * sign * size, z * sign * size);

            // The ridge, reached by two flights of stairs from the meadow.
            let (x, z) = m(0.0, -21.5);
            map.solid(x, z, 14.0 * size, 4.5 * size, RIDGE_TOP, "earth");
            for across in [-9.5f32, 9.5] {
                map.stairs(
                    false,
                    -17.0 * sign * size,
                    sign,
                    across * sign * size,
                    1.2,
                    RIDGE_TOP,
                );
            }
            // Rocks on the ridge to duck behind.
            for (rx, rz, hx, hz) in [
                (-5.5, -19.6, 1.3, 0.7),
                (4.0, -23.5, 1.0, 1.0),
                (12.0, -19.5, 0.8, 1.2),
            ] {
                let (x, z) = m(rx, rz);
                map.solid(x, z, hx * size, hz * size, RIDGE_TOP + 1.05, "rock");
            }

            // The bridge from the ridge to the lookout, high enough to walk under.
            let (x, z) = m(0.0, -9.75);
            map.floating(x, z, 1.1, 7.25 * size, 1.95, RIDGE_TOP, "wood");

            // Meadow cover: low rocks and a fallen log, good for crouching.
            for (rx, rz, hx, hz, top, kind) in [
                (-6.0, -6.5, 1.1, 0.8, 1.0, "rock"),
                (-15.0, 2.5, 0.9, 1.2, 1.1, "rock"),
                (8.5, -3.5, 2.3, 0.35, 0.6, "log"),
                (4.5, -13.0, 0.9, 0.9, 0.95, "rock"),
            ] {
                let (x, z) = m(rx, rz);
                map.solid(x, z, hx * size, hz * size, top, kind);
            }

            // The woods on this side's flank.
            for (tx, tz) in [
                (-20.5, -9.0),
                (-23.0, -1.5),
                (-18.5, 5.5),
                (-22.5, 11.5),
                (-15.5, -4.0),
                (-7.0, 12.5),
            ] {
                let (x, z) = m(tx, tz);
                map.tree(x, z);
            }

            for (sx, sz) in [(-11.5, -23.5), (11.0, -24.0), (-24.0, 4.0), (-4.0, -11.0)] {
                map.spawns.push(m(sx, sz));
            }
        }

        // The first four seats should use both halves of the mirrored map.
        let spawns = std::mem::take(&mut map.spawns);
        let half = spawns.len() / 2;
        map.spawns = (0..half)
            .flat_map(|index| [spawns[index], spawns[index + half]])
            .collect();

        // The lookout in the middle, with stairs up from east and west.
        map.solid(0.0, 0.0, 2.5 * size, 2.5 * size, RIDGE_TOP, "wood");
        map.stairs(true, 2.5 * size, 1.0, 0.0, 1.1, RIDGE_TOP);
        map.stairs(true, -2.5 * size, -1.0, 0.0, 1.1, RIDGE_TOP);
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
        blocks.push(Block {
            x,
            z,
            hx,
            hz,
            top,
            bottom: 0.0,
        });
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
    .map(|(x, z, hx, hz, top)| Block {
        x,
        z,
        hx,
        hz,
        top,
        bottom: 0.0,
    })
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
    pub dive_for: f32,
    pub dive_cooldown: f32,
    pub body_width: f32,
    dive_held: bool,
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

#[derive(Clone, Debug)]
struct Series {
    round: u8,
    wins: [u8; 2],
    left: f32,
    break_left: f32,
    last_winner: Option<u8>,
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
    /// Partners, when played in pairs: shots pass through them.
    allies: HashMap<u8, u8>,
    skills: HashMap<u8, Skill>,
    /// How much wider than the map for four, for the size of the field.
    size: f32,
    shot_range: f32,
    series: Option<Series>,
}

impl PaintBlasters {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let size = crate::bout::crowd_scale(contestants.len());
        let (blocks, block_kinds) = arena_layout();
        let spawns = spread(&spawn_points(), size);
        let fighters = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (x, z) = spawns[i % spawns.len()];
                let mut fighter = blank(c.seat);
                fighter.body_width = c.body_width;
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
        let series = (contestants.len() >= 2).then_some(Series {
            round: 1,
            wins: [0, 0],
            left: SERIES_ROUND_SECONDS,
            break_left: 0.0,
            last_winner: None,
        });
        let assigned_sides =
            contestants.iter().any(|c| c.team == 0) && contestants.iter().any(|c| c.team == 1);
        let teams = contestants
            .iter()
            .enumerate()
            .map(|(index, c)| {
                (
                    c.seat,
                    if assigned_sides {
                        c.team.min(1)
                    } else {
                        (index % 2) as u8
                    },
                )
            })
            .collect();
        PaintBlasters {
            fighters,
            time: 0.0,
            blocks: stretch(blocks, size),
            block_kinds,
            spawns,
            rng: StdRng::seed_from_u64(seed),
            brains,
            placements: None,
            damage: if series.is_some() {
                MAX_HP
            } else {
                SHOT_DAMAGE
            },
            respawn_seconds: if series.is_some() {
                f32::INFINITY
            } else {
                RESPAWN_SECONDS
            },
            round_seconds: ROUND_SECONDS,
            half_x: ARENA_HALF * size,
            half_z: ARENA_HALF * size,
            player_damage: true,
            allow_movement: true,
            teams,
            allies: contestants
                .iter()
                .filter_map(|c| c.ally.map(|ally| (c.seat, ally)))
                .collect(),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            size,
            // A paintball still carries across the yard when the yard is bigger.
            shot_range: SHOT_RANGE * size,
            series,
        }
    }

    /// How much wider than the map for four this field plays.
    pub fn size(&self) -> f32 {
        self.size
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
        self.series = None;
        self.teams.clear();
        let (blocks, spawns, half_x) = target_range(self.fighters.len());
        // The range already gets a lane per player, so it is never stretched.
        self.size = 1.0;
        self.shot_range = SHOT_RANGE;
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
        self.series = None;
        self.teams.clear();
        let map = Timberline::build_for(self.size);
        self.blocks = map.blocks;
        self.block_kinds = map.kinds;
        self.spawns = map.spawns;
        self.half_x = 26.0 * self.size;
        self.half_z = 26.0 * self.size;
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
        self.series = None;
        self.teams.clear();
        self.damage = SHOT_DAMAGE;
        self.respawn_seconds = RESPAWN_SECONDS;
        let (blocks, mut spawns) = ricochet_table();
        // Six spots seat six; any more get the middle of each short end
        // rather than doubling up on a corner.
        if self.fighters.len() > spawns.len() {
            spawns.extend([(-30.0, 0.0), (30.0, 0.0)]);
        }
        self.blocks = stretch(blocks, self.size);
        self.spawns = spread(&spawns, self.size);
        self.half_x = 34.0 * self.size;
        self.half_z = 24.0 * self.size;
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
        self.series = None;
        let size = self.size;
        self.blocks = stretch(retake_site(), size);
        self.spawns = spread(
            &[
                (-10.0, -26.0),
                (-26.0, 26.0),
                (10.0, -26.0),
                (26.0, 26.0),
                (0.0, -28.0),
                (0.0, 27.0),
            ],
            size,
        );
        self.half_x = 34.0 * size;
        self.half_z = 32.0 * size;
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
            let (x, z) = (x * size, z * size);
            fighter.x = x;
            fighter.z = z;
            fighter.y = 0.0;
            fighter.vx = 0.0;
            fighter.vy = 0.0;
            fighter.vz = 0.0;
            fighter.yaw = facing_center(x, z);
        }
    }

    /// A team game on its own map: one hit puts a player out until the mode
    /// brings them back with `place`, and the mode decides when it is over.
    pub fn use_team_field(
        &mut self,
        blocks: Vec<Block>,
        kinds: Vec<&'static str>,
        half_x: f32,
        half_z: f32,
    ) {
        self.series = None;
        self.blocks = blocks;
        self.block_kinds = kinds;
        self.half_x = half_x;
        self.half_z = half_z;
        self.damage = MAX_HP;
        self.respawn_seconds = f32::INFINITY;
        self.round_seconds = f32::INFINITY;
        self.teams.clear();
    }

    pub fn set_team(&mut self, seat: u8, team: u8) {
        self.teams.insert(seat, team);
    }

    /// Puts a player back on their feet at a spot, facing across the field.
    pub fn place(&mut self, seat: u8, x: f32, z: f32) {
        let y = ground_height(&self.blocks, x, z);
        if let Some(fighter) = self.fighters.iter_mut().find(|f| f.seat == seat) {
            *fighter = Fighter {
                x,
                y,
                z,
                yaw: facing_center(x, z),
                kills: fighter.kills,
                deaths: fighter.deaths,
                body_width: fighter.body_width,
                ..blank(seat)
            };
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
        let diving = fighter.dive_for > 0.0;
        fighter.crouching = (input.crouch || diving) && fighter.on_ground;
        let mut body = Body {
            x: fighter.x,
            y: fighter.y,
            z: fighter.z,
            vx: fighter.vx,
            vy: fighter.vy,
            vz: fighter.vz,
            on_ground: fighter.on_ground,
        };
        let mut movement = input;
        if diving {
            let (dx, _, dz) = fpmove::look(fighter.yaw, 0.0);
            body.vx = dx * 12.5;
            body.vz = dz * 12.5;
            movement.stick = Default::default();
            movement.jump = false;
            movement.crouch = true;
        }
        fpmove::step_body_profile(
            &mut body,
            movement,
            dt,
            &blocks,
            self.half_x,
            self.half_z,
            crate::bout::BodyProfile::new(fighter.body_width),
        );
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
        let (hit, endpoint, flight, gravity) = if self.player_damage {
            let (hit, endpoint, flight) = self.paintball_path(seat, (ox, oy, oz), (dx, dy, dz));
            (hit, endpoint, flight, PAINTBALL_GRAVITY)
        } else {
            // Target Frenzy's wall targets score against a straight shot ray.
            let distance = self.wall_distance((ox, oy, oz), (dx, dy, dz));
            (
                None,
                (ox + dx * distance, oy + dy * distance, oz + dz * distance),
                distance / PAINTBALL_SPEED,
                0.0,
            )
        };
        events.push(json!({
            "kind": "shot",
            "seat": seat,
            "from": [round(ox), round(oy), round(oz)],
            "to": [round(endpoint.0), round(endpoint.1), round(endpoint.2)],
            "flight": round(flight),
            "gravity": gravity,
            "hit": hit.is_some(),
        }));
        let Some(target_seat) = hit else {
            return;
        };
        let (hx, hy, hz) = endpoint;
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

    /// Sweep short straight segments along the ballistic path. The same
    /// collision point is sent to clients for the visible flying paintball.
    fn paintball_path(
        &self,
        seat: u8,
        origin: (f32, f32, f32),
        direction: (f32, f32, f32),
    ) -> (Option<u8>, (f32, f32, f32), f32) {
        let at = |travel: f32| {
            let seconds = travel / PAINTBALL_SPEED;
            (
                origin.0 + direction.0 * travel,
                origin.1 + direction.1 * travel - 0.5 * PAINTBALL_GRAVITY * seconds * seconds,
                origin.2 + direction.2 * travel,
            )
        };
        let mut travel = 0.0;
        while travel < self.shot_range {
            let next = (travel + 0.5).min(self.shot_range);
            let start = at(travel);
            let end = at(next);
            let delta = (end.0 - start.0, end.1 - start.1, end.2 - start.2);
            let length = (delta.0 * delta.0 + delta.1 * delta.1 + delta.2 * delta.2).sqrt();
            let ray = (delta.0 / length, delta.1 / length, delta.2 / length);
            let wall = self.wall_distance(start, ray).min(length);
            // Target Frenzy shares the blaster, but contestants cannot be hit.
            if self.player_damage {
                if let Some((target, distance)) = self.first_target(seat, start, ray, wall) {
                    let fraction = distance / length;
                    let hit = (
                        start.0 + delta.0 * fraction,
                        start.1 + delta.1 * fraction,
                        start.2 + delta.2 * fraction,
                    );
                    return (
                        Some(target),
                        hit,
                        (travel + (next - travel) * fraction) / PAINTBALL_SPEED,
                    );
                }
            }
            if wall < length - 0.0001 {
                let fraction = wall / length;
                return (
                    None,
                    (
                        start.0 + delta.0 * fraction,
                        start.1 + delta.1 * fraction,
                        start.2 + delta.2 * fraction,
                    ),
                    (travel + (next - travel) * fraction) / PAINTBALL_SPEED,
                );
            }
            travel = next;
        }
        (None, at(self.shot_range), self.shot_range / PAINTBALL_SPEED)
    }

    /// How far a shot travels before a block or a wall stops it.
    fn wall_distance(&self, origin: (f32, f32, f32), dir: (f32, f32, f32)) -> f32 {
        fpmove::clear_distance(
            origin,
            dir,
            &self.blocks,
            self.half_x,
            self.half_z,
            self.shot_range,
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
            .filter(|f| {
                !crate::bout::allied(
                    self.allies.get(&seat).copied(),
                    self.allies.get(&f.seat).copied(),
                )
            })
            .filter_map(|f| {
                fpmove::ray_player(
                    origin,
                    dir,
                    (f.x, f.y, f.z),
                    PLAYER_RADIUS * crate::bout::BodyProfile::new(f.body_width).radius,
                    f.height(),
                )
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

    fn restart_series_round(&mut self, events: &mut Vec<Value>) {
        let Some(series) = &mut self.series else {
            return;
        };
        series.round += 1;
        series.left = SERIES_ROUND_SECONDS;
        series.break_left = 0.0;
        for (index, fighter) in self.fighters.iter_mut().enumerate() {
            let (x, z) = self.spawns[index % self.spawns.len()];
            let (seat, kills, deaths, body_width) = (
                fighter.seat,
                fighter.kills,
                fighter.deaths,
                fighter.body_width,
            );
            *fighter = Fighter {
                x,
                z,
                yaw: facing_center(x, z),
                kills,
                deaths,
                body_width,
                ..blank(seat)
            };
        }
        for brain in self.brains.values_mut() {
            *brain = BotBrain::default();
        }
        events.push(json!({"kind":"paint_round_start","round":series.round}));
    }

    fn resolve_series(&mut self, dt: f32, events: &mut Vec<Value>) {
        let Some(series) = &mut self.series else {
            return;
        };
        series.left = (series.left - dt).max(0.0);
        let tally = |team: u8| {
            self.fighters
                .iter()
                .filter(|f| self.teams.get(&f.seat) == Some(&team))
                .fold((0usize, 0i32), |(alive, hp), f| {
                    (alive + usize::from(f.hp > 0), hp + i32::from(f.hp.max(0)))
                })
        };
        let a = tally(0);
        let b = tally(1);
        if a.0 > 0 && b.0 > 0 && series.left > 0.0 {
            return;
        }
        let winner = if a > b {
            0
        } else if b > a {
            1
        } else if series.wins[0] < series.wins[1] {
            0
        } else if series.wins[1] < series.wins[0] {
            1
        } else {
            (series.round - 1) % 2
        };
        series.wins[winner as usize] += 1;
        series.last_winner = Some(winner);
        events.push(json!({
            "kind":"paint_round_end", "round":series.round,
            "winner":winner, "wins":series.wins,
        }));
        if series.wins[winner as usize] >= 4 {
            let winners = self
                .fighters
                .iter()
                .filter(|f| self.teams[&f.seat] == winner)
                .map(|f| f.seat)
                .collect();
            let losers = self
                .fighters
                .iter()
                .filter(|f| self.teams[&f.seat] != winner)
                .map(|f| f.seat)
                .collect();
            self.placements = Some(vec![winners, losers]);
        } else {
            series.break_left = SERIES_BREAK_SECONDS;
        }
    }

    fn settle(&mut self) {
        if self.series.is_some() {
            return;
        }
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
        dive_for: 0.0,
        dive_cooldown: 0.0,
        body_width: 1.0,
        dive_held: false,
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
            "rule": "Eliminate the other team. First to four rounds wins; one paintball takes a player out.",
            "half": self.half_x,
            "half_z": self.half_z,
            "scale": self.size,
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
        if let Some(series) = &mut self.series {
            if series.break_left > 0.0 {
                series.break_left -= dt;
                if series.break_left <= 0.0 {
                    self.restart_series_round(events);
                }
                return;
            }
        }

        for index in 0..self.fighters.len() {
            let seat = self.fighters[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, i)| *i)
                .unwrap_or_default();
            let fighter = &mut self.fighters[index];
            fighter.cooldown = (fighter.cooldown - dt).max(0.0);
            fighter.dive_cooldown = (fighter.dive_cooldown - dt).max(0.0);
            fighter.dive_for = (fighter.dive_for - dt).max(0.0);
            let dive_pressed = input.jump && input.crouch;
            if dive_pressed
                && !fighter.dive_held
                && fighter.dive_cooldown <= 0.0
                && fighter.alive()
                && fighter.on_ground
            {
                fighter.dive_for = DIVE_SECONDS;
                fighter.dive_cooldown = DIVE_COOLDOWN;
                events.push(json!({"kind":"dive","seat":seat}));
            }
            fighter.dive_held = dive_pressed;
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

        if self.series.is_some() {
            self.resolve_series(dt, events);
        } else {
            self.settle();
        }
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
            let flight = distance / PAINTBALL_SPEED;
            let rise = (target.y + PLAYER_HEIGHT * 0.55) - (me.y + EYE_HEIGHT)
                + 0.5 * PAINTBALL_GRAVITY * flight * flight;
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
            "time": round(self.series.as_ref().map_or(self.time, |series| SERIES_ROUND_SECONDS - series.left)),
            "limit": self.series.as_ref().map_or(self.round_seconds, |_| SERIES_ROUND_SECONDS),
            "players": self.fighters.iter().map(|f| json!({
                "seat": f.seat,
                "team": self.team_of(f.seat),
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
                "respawn": round(if f.respawn_in.is_finite() { f.respawn_in.max(0.0) } else { 0.0 }),
                "ground": f.on_ground,
                "crouch": f.crouching,
                "diving": f.dive_for > 0.0,
                "dive_cd": round(f.dive_cooldown),
            })).collect::<Vec<_>>(),
            "series": self.series.as_ref().map(|series| json!({
                "round": series.round,
                "wins": series.wins,
                "left": round(series.left),
                "break_left": round(series.break_left.max(0.0)),
                "last_winner": series.last_winner,
            })),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn players(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    #[test]
    fn speedball_is_one_hit_elimination_first_to_four() {
        let mut game = PaintBlasters::new(&players(2), 17);
        game.blocks.clear();
        let mut events = Vec::new();
        for round in 1..=4 {
            game.fighters[0].x = 0.0;
            game.fighters[0].z = 0.0;
            game.fighters[0].yaw = 0.0;
            game.fighters[1].x = 0.0;
            game.fighters[1].z = -3.0;
            game.step(
                &[(
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                )],
                0.016,
                &mut events,
            );
            assert_eq!(
                game.fighters[1].hp, 0,
                "one hit eliminates in round {round}"
            );
            assert_eq!(game.series.as_ref().unwrap().wins, [round, 0]);
            if round < 4 {
                assert!(game.placements().is_none());
                game.step(&[], SERIES_BREAK_SECONDS + 0.01, &mut events);
                assert_eq!(game.fighters[1].hp, MAX_HP);
            }
            events.clear();
        }
        assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
    }

    #[test]
    fn speedball_dive_has_a_cooldown_and_requires_a_fresh_press() {
        let mut game = PaintBlasters::new(&players(2), 18);
        game.blocks.clear();
        game.fighters[0].x = 0.0;
        game.fighters[0].z = 0.0;
        game.fighters[0].yaw = 0.0;
        let input = Input {
            jump: true,
            crouch: true,
            ..Default::default()
        };
        let mut events = Vec::new();
        game.step(&[(0, input)], 0.016, &mut events);
        assert!(events.iter().any(|e| e["kind"] == "dive" && e["seat"] == 0));
        assert!(game.fighters[0].z < 0.0, "dive moves toward the aim ray");
        game.fighters[0].dive_cooldown = 0.0;
        events.clear();
        game.step(&[(0, input)], 0.016, &mut events);
        assert!(!events.iter().any(|e| e["kind"] == "dive"));
        game.step(&[], 0.016, &mut events);
        game.step(&[(0, input)], 0.016, &mut events);
        assert_eq!(events.iter().filter(|e| e["kind"] == "dive").count(), 1);
    }

    #[test]
    fn a_bigger_field_gets_a_bigger_yard() {
        let four = PaintBlasters::new(&players(4), 1);
        assert_eq!(four.half_x, ARENA_HALF, "four play it as built");
        assert_eq!(four.blocks.len(), arena().len());
        assert!(
            four.blocks
                .iter()
                .zip(arena())
                .all(|(a, b)| a.x == b.x && a.hx == b.hx)
        );
        assert_eq!((four.fighters[0].x, four.fighters[0].z), spawn_points()[0]);

        let eight = PaintBlasters::new(&players(8), 1);
        let grown = eight.half_x / ARENA_HALF;
        assert!(
            (grown - 2f32.sqrt()).abs() < 1e-4,
            "eight get the room of two fours"
        );
        assert!(
            (eight.blocks[0].hx / arena()[0].hx - grown).abs() < 1e-4,
            "the cover grows with it"
        );
        assert_eq!(
            eight.blocks[0].top,
            arena()[0].top,
            "but stays the same height"
        );
        assert!(
            eight
                .fighters
                .iter()
                .all(|f| f.x.abs() < eight.half_x && f.z.abs() < eight.half_z)
        );
        assert_eq!(
            eight.setup()["half"],
            json!(eight.half_x),
            "and the client is told"
        );
        assert_eq!(eight.setup()["scale"], json!(eight.size));
    }

    #[test]
    fn paintball_drops_and_reports_the_same_point_as_the_splat() {
        let mut game = PaintBlasters::new(&players(2), 7);
        game.blocks.clear();
        game.fighters[0].x = 0.0;
        game.fighters[0].z = 0.0;
        game.fighters[0].yaw = 0.0;
        game.fighters[0].pitch = 0.0;
        game.fighters[1].x = 0.0;
        game.fighters[1].z = -18.0;
        let mut events = Vec::new();
        game.fire(0, &mut events);
        let shot = events.iter().find(|event| event["kind"] == "shot").unwrap();
        let splat = events
            .iter()
            .find(|event| event["kind"] == "splat")
            .unwrap();
        assert_eq!(shot["hit"], true);
        assert!(shot["flight"].as_f64().unwrap() > 0.3);
        assert!(shot["to"][1].as_f64().unwrap() < EYE_HEIGHT as f64);
        assert_eq!(shot["to"][0], splat["x"]);
        assert_eq!(shot["to"][1], splat["y"]);
        assert_eq!(shot["to"][2], splat["z"]);
    }

    #[test]
    fn timberline_seats_both_halves_of_the_map() {
        let map = Timberline::build();
        assert_eq!(map.spawns.len(), 8);
        for pair in map.spawns.chunks_exact(2) {
            assert!((pair[0].0 + pair[1].0).abs() < 1e-4);
            assert!((pair[0].1 + pair[1].1).abs() < 1e-4);
        }
        let mut four = PaintBlasters::new(&players(4), 1);
        four.use_popgun_knockout();
        assert_eq!(
            four.fighters
                .iter()
                .filter(|fighter| fighter.z < 0.0)
                .count(),
            2
        );
        assert_eq!(
            four.fighters
                .iter()
                .filter(|fighter| fighter.z > 0.0)
                .count(),
            2
        );
    }
}
