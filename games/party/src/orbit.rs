//! Orbit Oddjobs: three short maintenance jobs on one low gravity platform.
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::f32::consts::TAU;

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};

pub const KEY: &str = "orbit";
pub const JOB_SECONDS: f32 = 22.0;
pub const LIMIT: f32 = JOB_SECONDS * 3.0;
pub const HALF: f32 = 10.0;
const SPEED: f32 = 6.4;
const ACCEL: f32 = 12.0;
const DRAG: f32 = 1.5;
const PANEL_RADIUS: f32 = 1.45;
const SCRAP_RADIUS: f32 = 1.4;

#[derive(Clone, Debug)]
pub struct Worker {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub score: u32,
    pub grabs: u32,
    pub patches: u32,
    pub recycled: u32,
    pub body_width: f32,
    charge: f32,
    cooldown: f32,
    skill: f32,
    bot_delay: f32,
}
#[derive(Clone, Debug)]
pub struct Prop {
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub owner: Option<u8>,
}
pub struct OrbitOddjobs {
    pub players: Vec<Worker>,
    pub props: Vec<Prop>,
    pub time: f32,
    pub job: usize,
    rng: StdRng,
    result: Option<Vec<Vec<u8>>>,
}
fn distance(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    (ax - bx).hypot(az - bz)
}
fn panel(index: usize, time: f32) -> (f32, f32) {
    let angle = index as f32 * TAU / 3.0 + time * 0.35;
    (angle.cos() * 5.2, angle.sin() * 5.2)
}
impl OrbitOddjobs {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let count = contestants.len().max(1);
                let slot = (i + seed as usize % count) % count;
                let a = slot as f32 * TAU / count as f32;
                Worker {
                    seat: c.seat,
                    x: a.cos() * 3.0,
                    z: a.sin() * 3.0,
                    vx: 0.0,
                    vz: 0.0,
                    score: 0,
                    grabs: 0,
                    patches: 0,
                    recycled: 0,
                    body_width: BodyProfile::new(c.body_width).width,
                    charge: 0.0,
                    cooldown: 0.0,
                    skill: c.skill.clamp(0.0, 1.0),
                    bot_delay: rng.random_range(0.0..0.4),
                }
            })
            .collect();
        let mut game = Self {
            players,
            props: Vec::new(),
            time: 0.0,
            job: 0,
            rng,
            result: None,
        };
        game.reset_props();
        game
    }
    fn reset_props(&mut self) {
        let offset = self.rng.random_range(0.0..TAU);
        self.props = (0..4)
            .map(|i| {
                let a = offset + i as f32 * TAU / 4.0 + self.rng.random_range(-0.25..0.25);
                let r = if self.job == 2 { 7.0 } else { 4.8 };
                Prop {
                    x: a.cos() * r,
                    z: a.sin() * r,
                    vx: -a.sin() * 0.7,
                    vz: a.cos() * 0.7,
                    owner: None,
                }
            })
            .collect();
    }
    fn respawn(&mut self, index: usize) {
        let a = self.rng.random_range(0.0..TAU);
        let r = if self.job == 2 { 7.3 } else { 5.5 };
        self.props[index] = Prop {
            x: a.cos() * r,
            z: a.sin() * r,
            vx: -a.sin() * 0.6,
            vz: a.cos() * 0.6,
            owner: None,
        };
    }
    fn finish(&mut self) {
        let mut ranked = self
            .players
            .iter()
            .map(|p| (p.seat, p.score))
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let mut places: Vec<Vec<u8>> = Vec::new();
        let mut prior = None;
        for (seat, score) in ranked {
            if prior != Some(score) {
                places.push(Vec::new());
            }
            places.last_mut().unwrap().push(seat);
            prior = Some(score);
        }
        self.result = Some(places);
    }
}
impl Bout for OrbitOddjobs {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"seconds":LIMIT,"half":HALF})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.result.is_some() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        self.time += dt;
        let next = (self.time / JOB_SECONDS).floor() as usize;
        if next > self.job && next < 3 {
            self.job = next;
            self.reset_props();
            for p in &mut self.players {
                p.charge = 0.0;
                p.cooldown = 0.0;
            }
            events.push(json!({"kind":"orbit_job","job":self.job}));
        }
        for p in &mut self.players {
            let body = BodyProfile::new(p.body_width);
            let input = inputs
                .iter()
                .find(|(s, _)| *s == p.seat)
                .map(|(_, v)| *v)
                .unwrap_or_default();
            let stick = input.stick.clamped();
            let drag = (1.0 - DRAG * dt / body.mass).max(0.0);
            p.vx = (p.vx + stick.x * ACCEL * body.acceleration * dt) * drag;
            p.vz = (p.vz + stick.z * ACCEL * body.acceleration * dt) * drag;
            let speed = p.vx.hypot(p.vz);
            let cap = SPEED * body.speed;
            if speed > cap {
                p.vx *= cap / speed;
                p.vz *= cap / speed;
            }
            let edge = HALF - 0.35 * body.radius;
            p.x = (p.x + p.vx * dt).clamp(-edge, edge);
            p.z = (p.z + p.vz * dt).clamp(-edge, edge);
            if p.x.abs() >= edge {
                p.vx *= -0.2;
            }
            if p.z.abs() >= edge {
                p.vz *= -0.2;
            }
            p.cooldown = (p.cooldown - dt).max(0.0);
            if self.job == 1 {
                let close = (0..3).any(|i| {
                    let (x, z) = panel(i, self.time);
                    distance(p.x, p.z, x, z) < PANEL_RADIUS * body.reach
                });
                if input.jump && close {
                    p.charge += dt;
                    let steady = 1.65 * body.stun;
                    if p.charge >= steady {
                        p.charge -= steady;
                        p.patches += 1;
                        p.score += 1;
                        events.push(json!({"kind":"orbit_patch","seat":p.seat}));
                    }
                } else {
                    p.charge = (p.charge - dt * 1.3).max(0.0);
                }
            }
        }
        if self.job != 1 {
            for prop in &mut self.props {
                prop.x += prop.vx * dt;
                prop.z += prop.vz * dt;
                let drag = if self.job == 2 { 0.7 } else { 0.2 };
                prop.vx *= 1.0 - drag * dt;
                prop.vz *= 1.0 - drag * dt;
                if prop.x.abs() > HALF - 0.6 {
                    prop.x = prop.x.clamp(-HALF + 0.6, HALF - 0.6);
                    prop.vx *= -0.6;
                }
                if prop.z.abs() > HALF - 0.6 {
                    prop.z = prop.z.clamp(-HALF + 0.6, HALF - 0.6);
                    prop.vz *= -0.6;
                }
            }
            for index in 0..self.props.len() {
                for player in &mut self.players {
                    let body = BodyProfile::new(player.body_width);
                    let input = inputs
                        .iter()
                        .find(|(s, _)| *s == player.seat)
                        .map(|(_, v)| *v)
                        .unwrap_or_default();
                    let prop = &mut self.props[index];
                    if !input.jump
                        || player.cooldown > 0.0
                        || distance(player.x, player.z, prop.x, prop.z)
                            > (if self.job == 0 { 1.1 } else { SCRAP_RADIUS }) * body.reach
                    {
                        continue;
                    }
                    player.cooldown = (if self.job == 0 { 0.45 } else { 0.62 }) * body.mass;
                    if self.job == 0 {
                        player.score += 1;
                        player.grabs += 1;
                        events.push(json!({"kind":"orbit_grab","seat":player.seat}));
                        self.respawn(index);
                        break;
                    }
                    let r = prop.x.hypot(prop.z).max(0.1);
                    prop.vx += -prop.x / r * 7.5 * body.mass + player.vx * 0.4;
                    prop.vz += -prop.z / r * 7.5 * body.mass + player.vz * 0.4;
                    prop.owner = Some(player.seat);
                    events.push(json!({"kind":"orbit_shove","seat":player.seat}));
                    break;
                }
                if self.job == 2 && self.props[index].x.hypot(self.props[index].z) < 1.55 {
                    if let Some(seat) = self.props[index].owner {
                        if let Some(p) = self.players.iter_mut().find(|p| p.seat == seat) {
                            p.score += 1;
                            p.recycled += 1;
                            events.push(json!({"kind":"orbit_recycle","seat":seat}));
                        }
                    }
                    self.respawn(index);
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
        let Some(p) = self.players.iter().find(|p| p.seat == seat) else {
            return Input::default();
        };
        if self.time < p.bot_delay {
            return Input::default();
        }
        let (x, z) = if self.job == 1 {
            (0..3)
                .map(|i| panel(i, self.time + 0.3))
                .min_by(|a, b| {
                    distance(p.x, p.z, a.0, a.1)
                        .partial_cmp(&distance(p.x, p.z, b.0, b.1))
                        .unwrap()
                })
                .unwrap()
        } else {
            self.props
                .iter()
                .min_by(|a, b| {
                    distance(p.x, p.z, a.x, a.z)
                        .partial_cmp(&distance(p.x, p.z, b.x, b.z))
                        .unwrap()
                })
                .map(|q| (q.x, q.z))
                .unwrap_or((0.0, 0.0))
        };
        let lead = 0.04 + p.skill * p.skill * 0.23;
        let dx = x - p.x - p.vx * lead;
        let dz = z - p.z - p.vz * lead;
        let length = dx.hypot(dz).max(0.001);
        let pace = 0.6 + p.skill * p.skill * 0.4;
        Input {
            stick: Stick {
                x: dx / length * pace,
                z: dz / length * pace,
            },
            jump: true,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        let stage = ["salvage", "stabilize", "recycle"][self.job];
        json!({"game":KEY,"time":self.time.min(LIMIT),"limit":LIMIT,"job":self.job,"stage":stage,"stage_time":self.time-self.job as f32*JOB_SECONDS,
            "props":if self.job==1{(0..3).map(|i|{let (x,z)=panel(i,self.time);json!({"x":x,"z":z})}).collect::<Vec<_>>()}else{self.props.iter().map(|q|json!({"x":q.x,"z":q.z,"owner":q.owner})).collect()},
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"yaw":p.vx.atan2(p.vz),"score":p.score,"grabs":p.grabs,"patches":p.patches,"recycled":p.recycled,"charge":p.charge,"body_width":p.body_width})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.result.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn players() -> Vec<Contestant> {
        vec![Contestant::person(0), Contestant::person(1)]
    }
    #[test]
    fn size_changes_movement_and_scrap_shove() {
        let mut contestants = players();
        contestants[0].body_width = 0.7;
        contestants[1].body_width = 1.4;
        let mut game = OrbitOddjobs::new(&contestants, 19);
        for p in &mut game.players {
            p.x = 0.0;
            p.z = -5.0;
        }
        let move_right = Input::from_stick(Stick { x: 1.0, z: 0.0 });
        for _ in 0..30 {
            game.step(&[(0, move_right), (1, move_right)], 1.0 / 60.0, &mut vec![]);
        }
        assert!(
            game.players[0].x > game.players[1].x + 0.06,
            "light={}, broad={}",
            game.players[0].x,
            game.players[1].x
        );

        let shove = Input {
            jump: true,
            ..Default::default()
        };
        let mut impulse = [0.0; 2];
        for (index, contestant) in contestants.iter().enumerate() {
            let mut job = OrbitOddjobs::new(&[contestant.clone()], 19);
            job.job = 2;
            job.players[0].x = 4.0;
            job.players[0].z = 0.0;
            job.props[0] = Prop {
                x: 3.0,
                z: 0.0,
                vx: 0.0,
                vz: 0.0,
                owner: None,
            };
            for prop in &mut job.props[1..] {
                prop.x = -8.0;
                prop.z = 0.0;
                prop.vx = 0.0;
                prop.vz = 0.0;
            }
            job.step(&[(contestant.seat, shove)], 1.0 / 60.0, &mut vec![]);
            impulse[index] = job.props[0].vx.abs();
            assert_eq!(job.props[0].owner, Some(contestant.seat));
        }
        assert!(impulse[1] > impulse[0] * 1.2);
    }
    #[test]
    fn mixed_size_bots_all_find_scoring_opportunities() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        let contestants = widths
            .iter()
            .enumerate()
            .map(|(seat, width)| {
                let mut c = Contestant::bot(seat as u8, 0.6);
                c.body_width = *width;
                c
            })
            .collect::<Vec<_>>();
        let mut scores = [0u32; 4];
        for seed in 0..8 {
            let mut game = OrbitOddjobs::new(&contestants, seed);
            while game.placements().is_none() {
                let inputs = contestants
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1.0 / 60.0)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1.0 / 60.0, &mut vec![]);
            }
            for p in &game.players {
                scores[p.seat as usize] += p.score;
            }
        }
        assert!(scores.iter().all(|score| *score > 0), "scores={scores:?}");
    }
    #[test]
    fn salvage_needs_nearby_action_and_respects_cooldown() {
        let mut game = OrbitOddjobs::new(&players(), 9);
        let prop = game.props[0].clone();
        game.players[0].x = prop.x;
        game.players[0].z = prop.z;
        game.step(&[], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 0);
        let grab = Input {
            jump: true,
            ..Default::default()
        };
        game.step(&[(0, grab)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 1);
        game.step(&[(0, grab)], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 1);
    }
    #[test]
    fn stabilizing_rewards_a_sustained_hold() {
        let mut game = OrbitOddjobs::new(&players(), 9);
        game.job = 1;
        let (x, z) = panel(0, game.time);
        game.players[0].x = x;
        game.players[0].z = z;
        for _ in 0..110 {
            let (x, z) = panel(0, game.time);
            game.players[0].x = x;
            game.players[0].z = z;
            game.step(
                &[(
                    0,
                    Input {
                        jump: true,
                        ..Default::default()
                    },
                )],
                1.0 / 60.0,
                &mut vec![],
            );
        }
        assert!(game.players[0].score >= 1);
    }
    #[test]
    fn owned_scrap_scores_on_recycling_ring() {
        let mut game = OrbitOddjobs::new(&players(), 9);
        game.job = 2;
        game.props[0] = Prop {
            x: 1.4,
            z: 0.0,
            vx: 0.0,
            vz: 0.0,
            owner: Some(0),
        };
        game.step(&[], 1.0 / 60.0, &mut vec![]);
        assert_eq!(game.players[0].score, 1);
        assert_eq!(game.players[0].recycled, 1);
        assert!(game.props[0].x.hypot(game.props[0].z) > 5.0);
    }
    #[test]
    fn bots_and_leaver_reach_a_result() {
        let players = (0..8).map(|s| Contestant::bot(s, 0.4)).collect::<Vec<_>>();
        let mut game = OrbitOddjobs::new(&players, 9);
        for _ in 0..(LIMIT as usize * 60 + 2) {
            let inputs = (0..7)
                .map(|s| (s, game.bot_input(s, 1.0 / 60.0)))
                .collect::<Vec<_>>();
            game.step(&inputs, 1.0 / 60.0, &mut vec![]);
        }
        assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
        assert!(game.players.iter().any(|p| p.score > 0));
    }
}
