//! Crumble Run: the floor is a grid of tiles and every one you stand on starts
//! to give way. Keep moving onto fresh ground; fall through and you are out.
//! Last one still on the floor wins.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::ground::{self, Runner};
use crate::bout::{Contestant, Input, Bout, Skill};

pub const TILE: f32 = 2.6;
pub const GRID: i32 = 7;
/// How long a tile lasts once someone stands on it.
pub const CRACK_SECONDS: f32 = 2.4;
/// Running across a tile is fine; standing on it is what starts the crack.
pub const DWELL_SECONDS: f32 = 0.35;
/// A tile that has fallen leaves a hole for the rest of the round.
pub const FALL_SECONDS: f32 = 0.45;
pub const ROUND_SECONDS: f32 = 75.0;
/// Once the floor stops shrinking on its own, tiles start going by themselves.
const SPONTANEOUS_FROM: f32 = 25.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileState {
    Solid,
    Cracking,
    Falling,
    Gone,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub col: i32,
    pub row: i32,
    pub x: f32,
    pub z: f32,
    pub state: TileState,
    pub timer: f32,
}

impl Tile {
    pub fn standable(&self) -> bool {
        matches!(self.state, TileState::Solid | TileState::Cracking)
    }
}

#[derive(Clone, Debug)]
pub struct Faller {
    pub seat: u8,
    pub run: Runner,
    pub y: f32,
    pub fell_at: Option<f32>,
    /// How long they have been on the tile they are standing on.
    pub dwell: f32,
    pub on: Option<usize>,
}

impl Faller {
    pub fn up(&self) -> bool {
        self.fell_at.is_none()
    }
}

pub struct CrumbleRun {
    pub players: Vec<Faller>,
    pub tiles: Vec<Tile>,
    pub time: f32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    react: std::collections::HashMap<u8, f32>,
    placements: Option<Vec<Vec<u8>>>,
    next_collapse: f32,
}

fn floor_tiles() -> Vec<Tile> {
    let mut tiles = Vec::new();
    for col in -GRID..=GRID {
        for row in -GRID..=GRID {
            // Cut the corners so the floor reads as a rounded pad.
            if (col * col + row * row) as f32 > (GRID as f32 + 0.6).powi(2) {
                continue;
            }
            tiles.push(Tile {
                col,
                row,
                x: col as f32 * TILE,
                z: row as f32 * TILE,
                state: TileState::Solid,
                timer: 0.0,
            });
        }
    }
    tiles
}

impl CrumbleRun {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let spread = ground::ring(contestants.len().max(1), GRID as f32 * TILE * 0.55, 0.4);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Faller {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1),
                y: 0.0,
                fell_at: None,
                dwell: 0.0,
                on: None,
            })
            .collect();
        CrumbleRun {
            players,
            tiles: floor_tiles(),
            time: 0.0,
            rng: StdRng::seed_from_u64(seed),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            react: Default::default(),
            placements: None,
            next_collapse: SPONTANEOUS_FROM,
        }
    }

    fn tile_at(&self, x: f32, z: f32) -> Option<usize> {
        self.tiles
            .iter()
            .position(|tile| (x - tile.x).abs() <= TILE / 2.0 && (z - tile.z).abs() <= TILE / 2.0)
    }

    pub fn tile_state(&self, x: f32, z: f32) -> Option<TileState> {
        self.tile_at(x, z).map(|index| self.tiles[index].state)
    }

    fn standing_on(&self, seat: u8) -> Option<usize> {
        let player = self.players.iter().find(|p| p.seat == seat)?;
        self.tile_at(player.run.x, player.run.z)
    }

    /// The next tile to step onto: nearby, still solid, and with solid
    /// neighbours so you do not strand yourself.
    fn hop_to(&self, from: (f32, f32)) -> Option<(f32, f32)> {
        let current = self.tile_at(from.0, from.1);
        let score = |tile: &Tile| {
            let away = ((tile.x - from.0).powi(2) + (tile.z - from.1).powi(2)).sqrt();
            let neighbours = self
                .tiles
                .iter()
                .filter(|other| {
                    other.state == TileState::Solid
                        && (other.col - tile.col).abs() <= 1
                        && (other.row - tile.row).abs() <= 1
                })
                .count() as f32;
            away - neighbours * 2.2
        };
        let solid = || {
            self.tiles
                .iter()
                .enumerate()
                .filter(|(index, tile)| tile.state == TileState::Solid && Some(*index) != current)
        };
        let near = solid()
            .filter(|(_, tile)| {
                ((tile.x - from.0).powi(2) + (tile.z - from.1).powi(2)).sqrt() <= TILE * 1.8
            })
            .min_by(|a, b| {
                score(a.1)
                    .partial_cmp(&score(b.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        near.or_else(|| {
            solid().min_by(|a, b| {
                score(a.1)
                    .partial_cmp(&score(b.1))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
        .map(|(_, tile)| (tile.x, tile.z))
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        let up = self.players.iter().filter(|p| p.up()).count();
        if up > 1 && self.time < ROUND_SECONDS {
            return;
        }
        let mut order: Vec<(f32, u8)> = self
            .players
            .iter()
            .map(|p| (p.fell_at.unwrap_or(f32::INFINITY), p.seat))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if (prev - at).abs() < 1e-3 => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }
}

impl Bout for CrumbleRun {
    fn key(&self) -> &'static str {
        "crumble"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "crumble",
            "rule": "Every tile you stand on gives way. Keep moving.",
            "tile": TILE,
            "radius": (GRID as f32 + 0.5) * TILE,
            "tiles": self.tiles.iter().map(|tile| json!({ "x": tile.x, "z": tile.z })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let stick = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| input.stick)
                .unwrap_or_default();
            let player = &mut self.players[index];
            if !player.up() {
                player.y -= 14.0 * dt;
                continue;
            }
            ground::step_runner(&mut player.run, stick, dt, ground::RUN_SPEED);
            ground::hold_inside(&mut player.run, (GRID as f32 + 0.5) * TILE);
        }
        let mut runners: Vec<Runner> = self.players.iter().map(|p| p.run).collect();
        ground::separate(&mut runners);
        for (player, run) in self.players.iter_mut().zip(runners) {
            if player.up() {
                player.run = run;
            }
        }

        // Standing on a tile starts it cracking.
        for index in 0..self.players.len() {
            if !self.players[index].up() {
                continue;
            }
            let seat = self.players[index].seat;
            let tile_now = self.standing_on(seat);
            if self.players[index].on == tile_now {
                self.players[index].dwell += dt;
            } else {
                self.players[index].on = tile_now;
                self.players[index].dwell = 0.0;
            }
            if self.players[index].dwell < DWELL_SECONDS {
                continue;
            }
            if let Some(tile) = tile_now
                && self.tiles[tile].state == TileState::Solid
            {
                {
                    self.tiles[tile].state = TileState::Cracking;
                    self.tiles[tile].timer = CRACK_SECONDS;
                    events.push(json!({
                        "kind": "crack",
                        "seat": seat,
                        "x": self.tiles[tile].x,
                        "z": self.tiles[tile].z,
                    }));
                }
            }
        }

        // Once the round has warmed up, the floor starts going on its own.
        if self.time >= self.next_collapse {
            self.next_collapse = self.time + self.rng.random_range(1.4..2.8);
            let solid: Vec<usize> = self
                .tiles
                .iter()
                .enumerate()
                .filter(|(_, tile)| tile.state == TileState::Solid)
                .map(|(index, _)| index)
                .collect();
            if let Some(&pick) =
                solid.get(self.rng.random_range(0..solid.len().max(1)) % solid.len().max(1))
            {
                self.tiles[pick].state = TileState::Cracking;
                self.tiles[pick].timer = CRACK_SECONDS;
            }
        }

        for index in 0..self.tiles.len() {
            let tile = &mut self.tiles[index];
            match tile.state {
                TileState::Cracking => {
                    tile.timer -= dt;
                    if tile.timer <= 0.0 {
                        tile.state = TileState::Falling;
                        tile.timer = FALL_SECONDS;
                        let (x, z) = (tile.x, tile.z);
                        events.push(json!({ "kind": "tile_fall", "x": x, "z": z }));
                    }
                }
                TileState::Falling => {
                    tile.timer -= dt;
                    if tile.timer <= 0.0 {
                        tile.state = TileState::Gone;
                    }
                }
                _ => {}
            }
        }

        // Anyone over a hole drops through it.
        let time = self.time;
        let mut dropped: Vec<u8> = Vec::new();
        for index in 0..self.players.len() {
            if !self.players[index].up() {
                continue;
            }
            let seat = self.players[index].seat;
            let standing = self
                .standing_on(seat)
                .map(|tile| self.tiles[tile].standable())
                .unwrap_or(false);
            if !standing {
                self.players[index].fell_at = Some(time);
                self.players[index].run.vx = 0.0;
                self.players[index].run.vz = 0.0;
                dropped.push(seat);
            }
        }
        for seat in dropped {
            events.push(json!({ "kind": "drop", "seat": seat }));
        }

        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let skill = self.skills.get(&seat).copied().unwrap_or_default();
        let Some(me) = self.players.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if !me.up() {
            return Input::default();
        }
        let here = (me.run.x, me.run.z);
        let under_foot = self
            .tile_at(here.0, here.1)
            .map(|index| self.tiles[index].state)
            .unwrap_or(TileState::Gone);
        // Hold your ground until it starts to go, then find the next tile.
        if under_foot == TileState::Solid {
            self.react.insert(seat, skill.reaction());
            return Input::default();
        }
        // Noticing takes a moment, and a clumsy bot takes longer.
        let waited = self.react.entry(seat).or_insert(skill.reaction());
        *waited -= dt;
        if *waited > 0.0 {
            return Input::default();
        }
        let Some(target) = self.hop_to(here) else {
            return Input::default();
        };
        Input {
            stick: ground::steer(here, target),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "crumble",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "y": round(p.y),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "out": !p.up(),
            })).collect::<Vec<_>>(),
            "tiles": self.tiles.iter().map(|tile| json!({
                "x": tile.x,
                "z": tile.z,
                "state": match tile.state {
                    TileState::Solid => "solid",
                    TileState::Cracking => "cracking",
                    TileState::Falling => "falling",
                    TileState::Gone => "gone",
                },
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
