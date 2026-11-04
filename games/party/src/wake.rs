//! Wake Mosaic: steer skiffs through shifting currents to color water tiles.
//! A wake fades within seconds, and another skiff can claim it sooner.
use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

pub const KEY: &str = "wake";
pub const LIMIT: f32 = 62.0;
pub const HALF: f32 = 10.0;
pub const GRID: usize = 16;
pub const CELL: f32 = 2.0 * HALF / GRID as f32;
pub const WAKE_SECONDS: f32 = 5.2;
const ACCEL: f32 = 11.0;
const DRAG: f32 = 1.2;
const MAX_SPEED: f32 = 7.2;
const CURRENT: f32 = 1.35;
fn wake_life(width: f32) -> f32 {
    WAKE_SECONDS * (0.90 + (width.clamp(0.7, 1.4) - 0.7) / 0.7 * 0.20)
}

#[derive(Clone, Copy)]
struct Tile {
    owner: Option<u8>,
    ttl: f32,
    lock: f32,
}
#[derive(Clone, Debug)]
pub struct Skiff {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub held: u32,
    pub points: u32,
    pub body_width: f32,
    skill: f32,
    goal: usize,
    plan_at: f32,
}
pub struct WakeMosaic {
    pub players: Vec<Skiff>,
    tiles: Vec<Tile>,
    pub time: f32,
    tick: u64,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
fn index(x: f32, z: f32) -> usize {
    let col = ((x + HALF) / CELL).floor().clamp(0.0, (GRID - 1) as f32) as usize;
    let row = ((z + HALF) / CELL).floor().clamp(0.0, (GRID - 1) as f32) as usize;
    row * GRID + col
}
fn center(i: usize) -> (f32, f32) {
    (
        ((i % GRID) as f32 + 0.5) * CELL - HALF,
        ((i / GRID) as f32 + 0.5) * CELL - HALF,
    )
}
fn current(x: f32, z: f32, t: f32) -> (f32, f32) {
    (
        (z * 0.36 + t * 0.9).sin() * CURRENT,
        (x * 0.32 - t * 0.7).cos() * CURRENT,
    )
}
impl WakeMosaic {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let count = contestants.len().max(1);
        let offset = (seed as usize % count) as f32 * std::f32::consts::TAU / count as f32;
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let a = i as f32 * std::f32::consts::TAU / count as f32 + offset;
                Skiff {
                    seat: c.seat,
                    x: a.cos() * 3.5,
                    z: a.sin() * 3.5,
                    vx: 0.0,
                    vz: 0.0,
                    held: 0,
                    points: 0,
                    body_width: BodyProfile::new(c.body_width).width,
                    skill: c.skill.clamp(0.0, 1.0),
                    goal: rng.random_range(0..GRID * GRID),
                    plan_at: rng.random_range(0.0..0.8),
                }
            })
            .collect();
        Self {
            players,
            tiles: vec![
                Tile {
                    owner: None,
                    ttl: 0.0,
                    lock: 0.0,
                };
                GRID * GRID
            ],
            time: 0.0,
            tick: 0,
            rng,
            result: None,
        }
    }
    fn score(&mut self) {
        for p in &mut self.players {
            p.held = self
                .tiles
                .iter()
                .filter(|t| t.owner == Some(p.seat))
                .count() as u32;
        }
    }
    fn finish(&mut self) {
        self.score();
        let mut scores = self
            .players
            .iter()
            .map(|p| (p.seat, p.points, p.held))
            .collect::<Vec<_>>();
        scores.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, points, held) in scores {
            if prior != Some((points, held)) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some((points, held));
        }
        self.result = Some(places);
    }
}
impl Bout for WakeMosaic {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"grid":GRID,"cell":CELL,"seconds":LIMIT})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        self.tick += 1;
        for tile in &mut self.tiles {
            tile.lock = (tile.lock - dt).max(0.0);
            if tile.ttl > 0.0 {
                tile.ttl = (tile.ttl - dt).max(0.0);
                if tile.ttl == 0.0 {
                    tile.owner = None;
                }
            }
        }
        for p in &mut self.players {
            let body = BodyProfile::new(p.body_width);
            let input = inputs
                .iter()
                .find(|(s, _)| *s == p.seat)
                .map(|(_, i)| i.stick.clamped())
                .unwrap_or_default();
            let (cx, cz) = current(p.x, p.z, self.time);
            let drag = (1.0 - DRAG * dt / body.mass).max(0.0);
            p.vx = (p.vx + (input.x * ACCEL * body.acceleration + cx / body.mass) * dt) * drag;
            p.vz = (p.vz + (input.z * ACCEL * body.acceleration + cz / body.mass) * dt) * drag;
            let speed = p.vx.hypot(p.vz);
            let cap = MAX_SPEED * body.speed;
            if speed > cap {
                p.vx *= cap / speed;
                p.vz *= cap / speed;
            }
            let edge = HALF - 0.25 * body.radius;
            p.x = (p.x + p.vx * dt).clamp(-edge, edge);
            p.z = (p.z + p.vz * dt).clamp(-edge, edge);
            if p.x.abs() >= edge {
                p.vx *= -0.35;
            }
            if p.z.abs() >= edge {
                p.vz *= -0.35;
            }
        }
        let count = self.players.len();
        if count > 0 {
            let start = self.tick as usize % count;
            for n in 0..count {
                let p = &mut self.players[(start + n) % count];
                let tile = &mut self.tiles[index(p.x, p.z)];
                let life = wake_life(p.body_width);
                if tile.owner != Some(p.seat) && tile.lock <= 0.0 {
                    tile.owner = Some(p.seat);
                    tile.lock = 0.75;
                    tile.ttl = life;
                    p.points += 1;
                    events.push(json!({"kind":"wake_claim","seat":p.seat}));
                } else if tile.owner == Some(p.seat) {
                    tile.ttl = life;
                }
            }
        }
        if self.tick % 3 == 0 {
            self.score();
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        if self.result.is_some() {
            return Input::default();
        }
        let Some(i) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let (gx, gz) = center(self.players[i].goal);
        let reached = (gx - self.players[i].x).hypot(gz - self.players[i].z) < 1.0;
        if self.time >= self.players[i].plan_at || reached {
            let p = &self.players[i];
            let mut best = p.goal;
            let mut value = f32::NEG_INFINITY;
            for (cell, tile) in self.tiles.iter().enumerate() {
                if tile.owner == Some(seat) && tile.ttl > 1.5 {
                    continue;
                }
                let (x, z) = center(cell);
                let d = (x - p.x).hypot(z - p.z);
                if !(3.0..=9.0).contains(&d) {
                    continue;
                }
                let target = if tile.owner.is_none() {
                    2.0
                } else if tile.owner == Some(seat) {
                    0.8
                } else {
                    2.4
                };
                let noise = self.rng.random_range(-1.0..1.0) * (1.0 - p.skill) * 1.7;
                let sweep = 3.0 + p.skill * 4.0;
                let clear = (1..=4)
                    .filter(|step| {
                        let fraction = *step as f32 / 5.0;
                        let path = index(p.x + (x - p.x) * fraction, p.z + (z - p.z) * fraction);
                        self.tiles[path].owner != Some(seat)
                    })
                    .count() as f32;
                let route_skill = ((p.skill - 0.8) / 0.2).clamp(0.0, 1.0);
                let score = target - (d - sweep).abs() * 0.25 + noise + clear * route_skill * 0.35;
                if score > value {
                    value = score;
                    best = cell;
                }
            }
            let skill = self.players[i].skill;
            if self
                .rng
                .random_bool(((1.0 - skill).powf(0.25) * 0.6) as f64)
            {
                if let Some(own) = self
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, tile)| tile.owner == Some(seat))
                    .min_by(|(a, _), (b, _)| {
                        let (ax, az) = center(*a);
                        let (bx, bz) = center(*b);
                        let da = (ax - self.players[i].x).hypot(az - self.players[i].z);
                        let db = (bx - self.players[i].x).hypot(bz - self.players[i].z);
                        da.partial_cmp(&db).unwrap()
                    })
                    .map(|(index, _)| index)
                {
                    best = own;
                }
            }
            self.players[i].goal = best;
            self.players[i].plan_at =
                self.time + 2.0 + self.players[i].skill * 0.75 + self.rng.random_range(0.0..0.45);
        }
        let p = &self.players[i];
        let (tx, tz) = center(p.goal);
        let lead = 0.05 + p.skill * 0.22;
        let dx = tx - p.x - p.vx * lead;
        let dz = tz - p.z - p.vz * lead;
        let length = dx.hypot(dz).max(0.001);
        let pace = 0.45 + p.skill * 0.4;
        Input::from_stick(Stick {
            x: dx / length * pace,
            z: dz / length * pace,
        })
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,"grid":GRID,
            "tiles":self.tiles.iter().map(|t|json!({"owner":t.owner,"ttl":t.ttl})).collect::<Vec<_>>(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"y":0.43,"yaw":p.vx.atan2(p.vz),"held":p.held,"points":p.points,"body_width":p.body_width})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn people() -> Vec<Contestant> {
        vec![Contestant::person(0), Contestant::person(1)]
    }
    #[test]
    fn wake_claims_can_be_stolen_and_expire() {
        let mut game = WakeMosaic::new(&people(), 4);
        game.players[0].x = 0.0;
        game.players[0].z = 0.0;
        game.players[1].x = 6.0;
        game.players[1].z = 6.0;
        game.step(&[], 1.0 / 60.0, &mut vec![]);
        let cell = index(0.0, 0.0);
        assert_eq!(game.tiles[cell].owner, Some(0));
        game.players[0].x = 6.0;
        game.players[0].z = 6.0;
        game.players[1].x = 0.0;
        game.players[1].z = 0.0;
        for _ in 0..55 {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.tiles[cell].owner, Some(1));
        game.players[0].x = 6.0;
        game.players[0].z = 6.0;
        game.players[1].x = -6.0;
        game.players[1].z = -6.0;
        for _ in 0..(WAKE_SECONDS as usize * 60 + 30) {
            game.step(&[], 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.tiles[cell].owner, None);
    }
    #[test]
    fn invalid_input_cannot_escape_pool() {
        let mut game = WakeMosaic::new(&people(), 4);
        for _ in 0..1000 {
            game.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::NAN,
                        z: 100.0,
                    }),
                )],
                1.0 / 60.0,
                &mut vec![],
            );
        }
        assert!(game.players[0].x.abs() <= HALF && game.players[0].z.abs() <= HALF);
    }
    #[test]
    fn bots_and_missing_seat_finish() {
        let players = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = WakeMosaic::new(&players, 7);
        for _ in 0..(LIMIT as usize * 60 + 2) {
            let inputs = (0..7)
                .map(|s| (s, game.bot_input(s, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.held > 0));
    }
    #[test]
    fn light_skiff_turns_faster_but_broad_wake_lasts_longer() {
        let mut light = Contestant::person(0);
        light.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = WakeMosaic::new(&[light, broad], 4);
        game.players[0].x = -5.0;
        game.players[0].z = -5.0;
        game.players[1].x = 5.0;
        game.players[1].z = 5.0;
        game.step(&[], 0.0, &mut Vec::new());
        assert!(game.tiles[index(-5.0, -5.0)].ttl < game.tiles[index(5.0, 5.0)].ttl);

        game.players[0].x = 0.0;
        game.players[1].x = 0.0;
        game.players[0].z = 0.0;
        game.players[1].z = 0.0;
        for _ in 0..45 {
            let steer = Input::from_stick(Stick { x: 1.0, z: 0.0 });
            game.step(&[(0, steer), (1, steer)], 1.0 / 60.0, &mut Vec::new());
        }
        assert!(game.players[0].vx > game.players[1].vx);
    }
    #[test]
    fn mixed_build_skiffs_each_claim_tiles_across_seeds() {
        let mut points = [0u32; 4];
        for seed in 0..8 {
            let players = (0..4).map(|seat| {
                let mut c = Contestant::bot(seat, 0.7);
                c.body_width = [0.7, 0.9, 1.2, 1.4][seat as usize];
                c
            }).collect::<Vec<_>>();
            let mut game = WakeMosaic::new(&players, seed);
            for _ in 0..(LIMIT as usize * 60 + 2) {
                let inputs = players.iter().map(|c|
                    (c.seat, game.bot_input(c.seat, 1.0 / 60.0))).collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut Vec::new());
            }
            for (total, skiff) in points.iter_mut().zip(&game.players) {
                *total += skiff.points;
            }
        }
        assert!(points.iter().all(|&score| score > 0), "mixed builds: {points:?}");
    }
}
