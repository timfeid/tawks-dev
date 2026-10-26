//! Red Light Rush: move on green, freeze on red, and race through a checkpoint.
use crate::bout::{Contestant, Input, Bout};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
const START: f32 = -38.0;
const FINISH: f32 = 38.0;
const LIMIT: f32 = 55.0;
#[derive(Clone)]
struct Runner {
    seat: u8,
    x: f32,
    z: f32,
    checkpoint: f32,
    finished: Option<f32>,
}
pub struct RedLightRush {
    players: Vec<Runner>,
    time: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}
impl RedLightRush {
    pub fn new(c: &[Contestant], seed: u64) -> Self {
        let n = c.len().max(1);
        Self {
            players: c
                .iter()
                .enumerate()
                .map(|(i, p)| Runner {
                    seat: p.seat,
                    x: (i as f32 / (n - 1).max(1) as f32 - 0.5) * 14.0,
                    z: START,
                    checkpoint: START,
                    finished: None,
                })
                .collect(),
            time: 0.0,
            rng: StdRng::seed_from_u64(seed),
            placed: None,
        }
    }
}
impl Bout for RedLightRush {
    fn key(&self) -> &'static str {
        "redlight"
    }
    fn setup(&self) -> Value {
        json!({"game":"redlight","half":11,"start":START,"finish":FINISH,"boosts":[],"mud":[]})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.time += dt;
        let red = self.time % 7.0 > 4.2;
        for p in &mut self.players {
            if p.finished.is_some() {
                continue;
            }
            let input = inputs
                .iter()
                .find(|(s, _)| *s == p.seat)
                .map(|x| x.1)
                .unwrap_or_default();
            let moving = input.stick.x.abs() + input.stick.z.abs() > 0.12;
            if red && moving {
                p.z = p.checkpoint;
                events.push(json!({"kind":"caught","seat":p.seat}));
                continue;
            }
            if !red {
                p.z += (1.0 - input.stick.z).max(0.0) * 4.5 * dt;
                p.x = (p.x + input.stick.x * 6.0 * dt).clamp(-10.0, 10.0);
                if p.z > 0.0 {
                    p.checkpoint = 0.0
                }
                if p.z >= FINISH {
                    p.z = FINISH;
                    p.finished = Some(self.time);
                    events.push(json!({"kind":"finish","seat":p.seat}));
                }
            }
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.finished.is_some()) {
            let mut r: Vec<_> = self
                .players
                .iter()
                .map(|p| (p.finished.map(|t| 1000.0 - t).unwrap_or(p.z), p.seat))
                .collect();
            r.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            self.placed = Some(r.into_iter().map(|x| vec![x.1]).collect())
        }
    }
    fn bot_input(&mut self, _: u8, _: f32) -> Input {
        let red = self.time % 7.0 > 4.2;
        if red {
            Input::default()
        } else {
            Input {
                stick: crate::bout::Stick {
                    x: self.rng.random_range(-0.3..=0.3),
                    z: -1.,
                },
                ..Default::default()
            }
        }
    }
    fn snapshot(&self) -> Value {
        json!({"game":"redlight","time":self.time,"limit":LIMIT,"players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"speed":0,"bounce":0,"finished":p.finished.is_some()})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}
