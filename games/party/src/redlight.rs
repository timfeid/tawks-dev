//! Red Light Rush: move on green, freeze on red, and race through a checkpoint.
use crate::bout::{Contestant, Input, Bout};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
const START: f32 = -38.0;
const FINISH: f32 = 38.0;
const LIMIT: f32 = 55.0;
/// How far either side of the middle the course runs for four players.
pub const HALF: f32 = 11.0;
#[derive(Clone)]
struct Runner {
    seat: u8,
    x: f32,
    z: f32,
    checkpoint: f32,
    finished: Option<f32>,
    skill: crate::bout::Skill,
}
pub struct RedLightRush {
    players: Vec<Runner>,
    time: f32,
    /// How much wider than the course for four, for the size of the field.
    size: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}
impl RedLightRush {
    pub fn new(c: &[Contestant], seed: u64) -> Self {
        let n = c.len().max(1);
        // A bigger field runs on a wider course; it is no longer.
        let size = crate::bout::crowd_scale(c.len());
        Self {
            players: c
                .iter()
                .enumerate()
                .map(|(i, p)| Runner {
                    seat: p.seat,
                    x: (i as f32 / (n - 1).max(1) as f32 - 0.5) * 14.0 * size,
                    z: START,
                    checkpoint: START,
                    finished: None,
                    skill: crate::bout::Skill(p.skill),
                })
                .collect(),
            time: 0.0,
            size,
            rng: StdRng::seed_from_u64(seed),
            placed: None,
        }
    }
}
impl RedLightRush {
    /// How far either side of the middle the course runs.
    pub fn half(&self) -> f32 {
        HALF * self.size
    }
}
impl Bout for RedLightRush {
    fn key(&self) -> &'static str {
        "redlight"
    }
    fn setup(&self) -> Value {
        json!({"game":"redlight","half":self.half(),"start":START,"finish":FINISH,"boosts":[],"mud":[]})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.time += dt;
        let red = self.time % 7.0 > 4.32;
        let edge = self.half() - 1.0;
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
                p.z += -input.stick.clamped().z * 9.0 * dt;
                p.x = (p.x + input.stick.x * 6.0 * dt).clamp(-edge, edge);
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
            let mut groups:Vec<Vec<u8>>=Vec::new();let mut previous=None;
            for (score,seat) in r {if previous.is_some_and(|last:f32|(last-score).abs()<0.001){groups.last_mut().unwrap().push(seat);}else{groups.push(vec![seat]);}previous=Some(score);}
            self.placed=Some(groups);
        }
    }
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let skill=self.players.iter().find(|p|p.seat==seat).map(|p|p.skill).unwrap_or_default();
        let phase=(self.time+dt)%7.;
        // Leave a readable reaction delay at green, and stop before the red
        // boundary. Sampling last tick's lamp used to catch every bot forever.
        let red=phase>4.2-(1.-skill.level())*0.12||phase<skill.reaction()*0.3;
        if red {
            Input::default()
        } else {
            Input {
                stick: crate::bout::Stick {
                    x: self.rng.random_range(-0.3..=0.3),
                    z: -skill.pace(),
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
#[cfg(test)]
mod tests {
    use super::*;

    fn field(n: u8) -> Vec<Contestant> {
        (0..n)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: crate::bout::DEFAULT_SKILL,
                team: seat % 2,
                ally: None,
                body_width: 1.0,
                pet: 4,
            })
            .collect()
    }

    #[test]
    fn a_bigger_field_runs_on_a_wider_course() {
        let four = RedLightRush::new(&field(4), 1);
        assert_eq!(four.half(), HALF, "four run it as built");
        assert_eq!(four.players[0].x, -7.0, "lined up as before");
        let eight = RedLightRush::new(&field(8), 1);
        assert!((eight.half() / HALF - 2f32.sqrt()).abs() < 1e-4, "eight get the room of two fours");
        assert!((eight.players[0].x / -7.0 - 2f32.sqrt()).abs() < 1e-4, "and line up wider");
        assert_eq!(eight.setup()["half"], json!(eight.half()), "and the client is told");
    }
    #[test]
    fn neutral_input_stays_put_and_bot_runners_can_finish_across_light_changes(){
        let mut idle=RedLightRush::new(&field(2),5);let mut events=Vec::new();
        for _ in 0..60{idle.step(&[],1./60.,&mut events);}
        assert_eq!(idle.players[0].z,START,"neutral input must not walk forward");
        let mut game=RedLightRush::new(&field(4),9);
        for _ in 0..60*30{let inputs:Vec<_>=(0..4).map(|seat|(seat,game.bot_input(seat,1./60.))).collect();game.step(&inputs,1./60.,&mut events);if game.placements().is_some(){break;}}
        assert!(game.players.iter().all(|p|p.finished.is_some()),"bots cross multiple lamp cycles without resetting forever");
        assert_eq!(game.placements().unwrap()[0].len(),4,"equal finish times share a placement rather than favouring seat zero");
    }

}
