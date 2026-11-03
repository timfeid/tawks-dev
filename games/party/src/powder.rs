//! Powder Pop: an open snow-bowl stunt contest. Time a lift-ramp hop, then
//! land with speed still in the sled to bank a growing clean-landing combo.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::f32::consts::TAU;

use crate::bout::{Bout, Contestant, Input, Stick};

pub const KEY: &str = "powder";
pub const LIMIT: f32 = 67.0;
pub const RADIUS: f32 = 17.0;
pub const RAMP_RING: f32 = 7.0;
pub const RAMP_RADIUS: f32 = 2.0;
const ACCEL: f32 = 12.0;
const DRAG: f32 = 0.9;
const MAX_SPEED: f32 = 8.4;
pub const GRAVITY: f32 = 9.5;
const RAMP_RATE: f32 = 0.22;
const LAUNCH_COOLDOWN: f32 = 1.0;

#[derive(Clone, Debug)]
pub struct Rider {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub height: f32,
    pub vy: f32,
    pub score: u32,
    pub combo: u8,
    pub landings: u32,
    pub crashes: u32,
    pub air_time: f32,
    takeoff_speed: f32,
    cooldown: f32,
    skill: f32,
    bot_ready: f32,
    fire_latched: bool,
    last_ramp: Option<usize>,
}
pub struct PowderPop {
    pub players: Vec<Rider>,
    pub time: f32,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
pub fn ramp(index: usize, time: f32) -> (f32, f32) {
    let a = index as f32 * TAU / 4.0 + time * RAMP_RATE;
    (a.cos() * RAMP_RING, a.sin() * RAMP_RING)
}
impl PowderPop {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let count = contestants.len().max(1);
        let phase = (seed as usize % count) as f32 * TAU / count as f32;
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let a = i as f32 * TAU / count as f32 + phase;
                Rider {
                    seat: c.seat,
                    x: a.cos() * 2.8,
                    z: a.sin() * 2.8,
                    vx: 0.0,
                    vz: 0.0,
                    height: 0.0,
                    vy: 0.0,
                    score: 0,
                    combo: 0,
                    landings: 0,
                    crashes: 0,
                    air_time: 0.0,
                    takeoff_speed: 0.0,
                    cooldown: 0.0,
                    skill: c.skill.clamp(0.0, 1.0),
                    bot_ready: rng.random_range(0.0..0.5),
                    fire_latched: false,
                    last_ramp: None,
                }
            })
            .collect();
        Self {
            players,
            time: 0.0,
            rng,
            result: None,
        }
    }
    fn finish(&mut self) {
        let mut ranked = self
            .players
            .iter()
            .map(|p| (p.seat, p.score, p.landings))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score, landings) in ranked {
            if prior != Some((score, landings)) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some((score, landings));
        }
        self.result = Some(places);
    }
}
impl Bout for PowderPop {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"seconds":LIMIT,"radius":RADIUS})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        for p in &mut self.players {
            let input = inputs
                .iter()
                .find(|(s, _)| *s == p.seat)
                .map(|(_, v)| *v)
                .unwrap_or_default();
            let stick = input.stick.clamped();
            let tap = input.fire && !p.fire_latched;
            p.fire_latched = input.fire;
            let air = p.height > 0.0 || p.vy > 0.0;
            let control = if air { 2.2 } else { ACCEL };
            let drag = if air { 0.12 } else { DRAG };
            p.vx = (p.vx + stick.x * control * dt) * (1.0 - drag * dt);
            p.vz = (p.vz + stick.z * control * dt) * (1.0 - drag * dt);
            let speed = p.vx.hypot(p.vz);
            if speed > MAX_SPEED {
                p.vx *= MAX_SPEED / speed;
                p.vz *= MAX_SPEED / speed;
            }
            p.x += p.vx * dt;
            p.z += p.vz * dt;
            p.cooldown = (p.cooldown - dt).max(0.0);
            if air {
                p.air_time += dt;
                p.vy -= GRAVITY * dt;
                p.height += p.vy * dt;
                if p.height <= 0.0 {
                    p.height = 0.0;
                    p.vy = 0.0;
                    let clean = p.x.hypot(p.z) < RADIUS - 0.5 && p.vx.hypot(p.vz) > 1.0;
                    if clean {
                        p.combo = (p.combo + 1).min(2);
                        p.landings += 1;
                        let base = (p.air_time * 2.8 + p.takeoff_speed * 0.45)
                            .round()
                            .clamp(2.0, 8.0) as u32;
                        let gained = base * p.combo as u32;
                        p.score += gained;
                        events.push(json!({"kind":"powder_land","seat":p.seat,"points":gained,"combo":p.combo}));
                    } else {
                        p.combo = 0;
                        p.crashes += 1;
                        events.push(json!({"kind":"powder_crash","seat":p.seat}));
                    }
                    p.air_time = 0.0;
                }
            } else if tap && p.cooldown <= 0.0 && speed >= 3.1 {
                if let Some((ramp_index, (rx, rz))) = (0..4)
                    .filter(|i| Some(*i) != p.last_ramp)
                    .map(|i| (i, ramp(i, self.time)))
                    .find(|(_, (x, z))| {
                        p.x.hypot(p.z) > 4.0 && (p.x - *x).hypot(p.z - *z) < RAMP_RADIUS
                    })
                {
                    let radial = p.x.hypot(p.z).max(0.1);
                    p.vx -= p.x / radial * 1.7;
                    p.vz -= p.z / radial * 1.7;
                    p.height = 0.08;
                    p.vy = 4.8 + speed * 0.1;
                    p.air_time = 0.0;
                    p.takeoff_speed = speed;
                    p.cooldown = LAUNCH_COOLDOWN;
                    p.last_ramp = Some(ramp_index);
                    events.push(json!({"kind":"powder_launch","seat":p.seat,"x":rx,"z":rz}));
                }
            }
            let radius = p.x.hypot(p.z);
            if radius > RADIUS {
                let scale = RADIUS / radius;
                p.x *= scale;
                p.z *= scale;
                let nx = p.x / RADIUS;
                let nz = p.z / RADIUS;
                let outward = p.vx * nx + p.vz * nz;
                if outward > 0.0 {
                    p.vx -= nx * outward * 1.4;
                    p.vz -= nz * outward * 1.4;
                }
            }
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
        let p = &self.players[i];
        let air = p.height > 0.0 || p.vy > 0.0;
        if air {
            let length = p.x.hypot(p.z).max(0.1);
            return Input::from_stick(Stick {
                x: -p.x / length * 0.25,
                z: -p.z / length * 0.25,
            });
        }
        let (tx, tz) = (0..4)
            .filter(|ramp_index| Some(*ramp_index) != p.last_ramp)
            .map(|ramp_index| ramp(ramp_index, self.time + 0.35))
            .min_by(|a, b| {
                (p.x - a.0)
                    .hypot(p.z - a.1)
                    .partial_cmp(&(p.x - b.0).hypot(p.z - b.1))
                    .unwrap()
            })
            .unwrap();
        let lead = 0.06 + p.skill * 0.25;
        let dx = tx - p.x - p.vx * lead;
        let dz = tz - p.z - p.vz * lead;
        let length = dx.hypot(dz).max(0.001);
        let near = (p.x - tx).hypot(p.z - tz) < RAMP_RADIUS * 0.82 && p.vx.hypot(p.vz) > 3.1;
        let fire = near && p.cooldown <= 0.0 && self.time >= p.bot_ready;
        if fire {
            let success = self
                .rng
                .random_bool((0.38 + (p.skill / 0.95).powi(2).min(1.0) * 0.62) as f64);
            self.players[i].bot_ready = self.time + if success { 0.9 } else { 0.7 };
            return Input {
                stick: Stick {
                    x: dx / length,
                    z: dz / length,
                },
                fire: success,
                ..Default::default()
            };
        }
        let pace = 0.58 + p.skill * 0.42;
        Input::from_stick(Stick {
            x: dx / length * pace,
            z: dz / length * pace,
        })
    }
    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,
            "ramps":(0..4).map(|i|{let (x,z)=ramp(i,self.time);json!({"x":x,"z":z})}).collect::<Vec<_>>(),
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"y":0.25,"vx":p.vx,"vz":p.vz,"vy":p.vy,"yaw":p.vx.atan2(p.vz),"height":p.height,"score":p.score,"combo":p.combo,"landings":p.landings,"crashes":p.crashes})).collect::<Vec<_>>()})
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
    fn a_launch_needs_a_ramp_speed_and_tap() {
        let mut game = PowderPop::new(&people(), 7);
        let (x, z) = ramp(0, game.time);
        game.players[0].x = x;
        game.players[0].z = z;
        let fire = Input {
            fire: true,
            ..Default::default()
        };
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].height, 0.0);
        game.players[0].vx = 4.2;
        game.step(&[(0, Input::default())], 1.0 / 60.0, &mut vec![]);
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        assert!(game.players[0].height > 0.0);
        assert!(game.snapshot()["players"][0]["vy"].as_f64().unwrap() > 0.0);
    }
    #[test]
    fn a_clean_landing_banks_a_combo() {
        let mut game = PowderPop::new(&people(), 7);
        let (x, z) = ramp(0, game.time);
        game.players[0].x = x;
        game.players[0].z = z;
        game.players[0].vx = 4.2;
        let fire = Input {
            fire: true,
            ..Default::default()
        };
        game.step(&[(0, fire)], 1.0 / 60.0, &mut vec![]);
        for _ in 0..150 {
            game.step(&[(0, Input::default())], 1.0 / 60.0, &mut vec![]);
            if game.players[0].height == 0.0 {
                break;
            }
        }
        assert!(game.players[0].score > 0);
        assert_eq!(game.players[0].combo, 1);
    }
    #[test]
    fn the_same_ramp_cannot_be_farmed_twice() {
        let mut game = PowderPop::new(&people(), 7);
        let (x, z) = ramp(0, game.time);
        let rider = &mut game.players[0];
        rider.x = x;
        rider.z = z;
        rider.vx = 4.2;
        rider.last_ramp = Some(0);
        game.step(
            &[(
                0,
                Input {
                    fire: true,
                    ..Default::default()
                },
            )],
            1.0 / 60.0,
            &mut vec![],
        );
        assert_eq!(game.players[0].height, 0.0);
    }
    #[test]
    fn bots_and_missing_seat_finish() {
        let people = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = PowderPop::new(&people, 9);
        for _ in 0..(LIMIT as usize * 60 + 2) {
            let inputs = (0..7)
                .map(|s| (s, game.bot_input(s, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.landings > 0));
    }
}
