//! King of the Hill, played on the island's own peak: the ground falls away
//! from the middle, so holding the top means fighting the slope as well as
//! everyone else. Most seconds on the summit when the clock runs out wins.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input};

pub const KEY: &str = "hill";
/// How far the hill reaches before the ground gives out.
const RADIUS: f32 = 20.0;
/// How high the peak stands over the rim.
const PEAK: f32 = 7.5;
/// Standing inside this of the middle is holding the summit.
const SUMMIT: f32 = 3.6;
const LIMIT: f32 = 60.0;
const WALK: f32 = 16.0;
const DRAG: f32 = 3.4;
/// How hard the slope pulls downhill, per unit of steepness. Kept well under
/// `WALK` so the hill can be climbed, slowly, and standing still loses ground.
const SLIDE: f32 = 13.0;
const SHOVE: f32 = 9.0;
const BUMP: f32 = 1.5;
/// How long someone who slides off the bottom takes to climb back up.
const CLIMB_BACK: f32 = 3.0;

#[derive(Clone)]
struct Climber {
    seat: u8,
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    /// Seconds held on the summit, which is the score.
    held: f32,
    /// Counts down while they walk back from the bottom.
    out: f32,
    skill: f32,
}

pub struct KingOfTheHill {
    climbers: Vec<Climber>,
    time: f32,
    /// How much wider and taller than the hill for four, for the size of the
    /// field. Height grows with width so the slope is just as steep.
    size: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

/// The hill's height at a point, and how steeply it falls there. One curve,
/// shared by the sim and the renderer through `setup`. `size` stretches the
/// whole hill, wide and high alike.
fn ground(x: f32, z: f32, size: f32) -> f32 {
    let reach = (x * x + z * z).sqrt() / (RADIUS * size);
    if reach >= 1.0 {
        return 0.0;
    }
    let falloff = 1.0 - reach;
    PEAK * size * falloff * falloff * (3.0 - 2.0 * falloff)
}

impl KingOfTheHill {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1) as f32;
        let size = crate::bout::crowd_scale(contestants.len());
        let rim = RADIUS * size;
        KingOfTheHill {
            climbers: contestants
                .iter()
                .enumerate()
                .map(|(index, one)| {
                    // Spread around the rim, so nobody starts closer to the top.
                    let angle = index as f32 / count * std::f32::consts::TAU;
                    Climber {
                        seat: one.seat,
                        x: angle.cos() * (rim - 2.0),
                        z: angle.sin() * (rim - 2.0),
                        vx: 0.0,
                        vz: 0.0,
                        held: 0.0,
                        out: 0.0,
                        skill: one.skill,
                    }
                })
                .collect(),
            time: 0.0,
            size,
            rng: StdRng::seed_from_u64(seed),
            placed: None,
        }
    }

    fn radius(&self) -> f32 {
        RADIUS * self.size
    }

    fn summit(&self) -> f32 {
        SUMMIT * self.size
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, f32)> = self
            .climbers
            .iter()
            .map(|climber| (climber.seat, climber.held))
            .collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (seat, held) in order {
            // A tenth of a second apart is a tie as far as anyone can tell.
            match (last, groups.last_mut()) {
                (Some(previous), Some(group)) if (previous - held).abs() < 0.1 => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(held);
        }
        self.placed = Some(groups);
    }
}

impl Bout for KingOfTheHill {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "radius": self.radius(),
            "peak": PEAK * self.size,
            "summit": self.summit(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.time += dt;
        let (size, radius, summit) = (self.size, self.radius(), self.summit());

        for climber in self.climbers.iter_mut() {
            if climber.out > 0.0 {
                climber.out -= dt;
                if climber.out <= 0.0 {
                    // Back on at the rim, wherever they went off.
                    let angle = climber.z.atan2(climber.x);
                    climber.x = angle.cos() * (radius - 1.5);
                    climber.z = angle.sin() * (radius - 1.5);
                    climber.vx = 0.0;
                    climber.vz = 0.0;
                }
                continue;
            }

            let stick = inputs
                .iter()
                .find(|(seat, _)| *seat == climber.seat)
                .map(|(_, input)| (input.stick.x, input.stick.z))
                .unwrap_or((0.0, 0.0));
            climber.vx += stick.0 * WALK * dt;
            climber.vz += stick.1 * WALK * dt;

            // The ground pushes you off the way it falls, so the steeper it is
            // the harder it is to stand still.
            let step = 0.6;
            let slope_x = (ground(climber.x + step, climber.z, size)
                - ground(climber.x - step, climber.z, size))
                / (2.0 * step);
            let slope_z = (ground(climber.x, climber.z + step, size)
                - ground(climber.x, climber.z - step, size))
                / (2.0 * step);
            climber.vx -= slope_x * SLIDE * dt;
            climber.vz -= slope_z * SLIDE * dt;

            let drag = (1.0 - DRAG * dt).max(0.0);
            climber.vx *= drag;
            climber.vz *= drag;
            climber.x += climber.vx * dt;
            climber.z += climber.vz * dt;

            if (climber.x * climber.x + climber.z * climber.z).sqrt() > radius {
                climber.out = CLIMB_BACK;
                events.push(json!({ "kind": "off", "seat": climber.seat }));
            }
        }

        // Running into someone shoves them, harder the faster you were going.
        for i in 0..self.climbers.len() {
            for j in (i + 1)..self.climbers.len() {
                let (a, b) = (self.climbers[i].clone(), self.climbers[j].clone());
                if a.out > 0.0 || b.out > 0.0 {
                    continue;
                }
                let dx = b.x - a.x;
                let dz = b.z - a.z;
                let apart = (dx * dx + dz * dz).sqrt();
                if apart >= BUMP || apart <= 0.001 {
                    continue;
                }
                let nx = dx / apart;
                let nz = dz / apart;
                let closing = (a.vx - b.vx) * nx + (a.vz - b.vz) * nz;
                let push = SHOVE + closing.max(0.0);
                self.climbers[i].vx -= nx * push * 0.5;
                self.climbers[i].vz -= nz * push * 0.5;
                self.climbers[j].vx += nx * push * 0.5;
                self.climbers[j].vz += nz * push * 0.5;
                // Pull them apart so they do not sit inside one another.
                let gap = (BUMP - apart) * 0.5;
                self.climbers[i].x -= nx * gap;
                self.climbers[i].z -= nz * gap;
                self.climbers[j].x += nx * gap;
                self.climbers[j].z += nz * gap;
                events.push(json!({ "kind": "shove", "by": a.seat, "at": b.seat }));
            }
        }

        for climber in self.climbers.iter_mut() {
            if climber.out <= 0.0
                && (climber.x * climber.x + climber.z * climber.z).sqrt() <= summit
            {
                climber.held += dt;
            }
        }

        if self.time >= LIMIT {
            self.settle();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self
            .climbers
            .iter()
            .find(|climber| climber.seat == seat)
            .cloned()
        else {
            return Input::default();
        };
        // Head for the middle, or for whoever is holding it when close enough
        // to shove them off instead.
        let summit = self.summit();
        let holder = self
            .climbers
            .iter()
            .filter(|other| {
                other.seat != seat
                    && other.out <= 0.0
                    && (other.x * other.x + other.z * other.z).sqrt() <= summit
            })
            .min_by(|a, b| {
                let da = (a.x - me.x).hypot(a.z - me.z);
                let db = (b.x - me.x).hypot(b.z - me.z);
                da.total_cmp(&db)
            })
            .cloned();
        let (tx, tz) = match holder {
            Some(other) => (other.x, other.z),
            None => (0.0, 0.0),
        };
        let dx = tx - me.x;
        let dz = tz - me.z;
        let far = (dx * dx + dz * dz).sqrt().max(0.001);
        // A clumsy bot wanders; a sharp one walks the line.
        let wobble = (1.0 - me.skill) * 0.7;
        Input::from_stick(crate::bout::Stick {
            x: (dx / far + self.rng.random_range(-wobble..=wobble)).clamp(-1.0, 1.0),
            z: (dz / far + self.rng.random_range(-wobble..=wobble)).clamp(-1.0, 1.0),
        })
    }

    fn snapshot(&self) -> Value {
        let summit = self.summit();
        json!({
            "game": KEY,
            "time": self.time,
            "limit": LIMIT,
            "players": self
                .climbers
                .iter()
                .map(|climber| json!({
                    "seat": climber.seat,
                    "x": climber.x,
                    "z": climber.z,
                    "y": ground(climber.x, climber.z, self.size),
                    "held": climber.held,
                    "out": climber.out > 0.0,
                    "king": climber.out <= 0.0
                        && (climber.x * climber.x + climber.z * climber.z).sqrt() <= summit,
                }))
                .collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_a_bigger_hill() {
        let four = KingOfTheHill::new(&field(4), 1);
        assert_eq!(four.setup(), json!({ "game": KEY, "radius": RADIUS, "peak": PEAK, "summit": SUMMIT }), "four play it as built");
        assert!((four.climbers[0].x - (RADIUS - 2.0)).abs() < 1e-4);
        let eight = KingOfTheHill::new(&field(8), 1);
        let grown = 2f32.sqrt();
        assert!((eight.radius() / RADIUS - grown).abs() < 1e-4, "eight get the room of two fours");
        assert!((eight.summit() / SUMMIT - grown).abs() < 1e-4, "and a wider summit to fight over");
        assert!((eight.climbers[0].x - (RADIUS * grown - 2.0)).abs() < 1e-3, "starting at the new rim");
        // The slope is just as steep, so climbing it feels the same.
        let halfway = |game: &KingOfTheHill| ground(game.radius() / 2.0, 0.0, game.size) / game.radius();
        assert!((halfway(&eight) - halfway(&four)).abs() < 1e-4);
        let setup = eight.setup();
        assert_eq!(setup["radius"], json!(eight.radius()), "and the client is told");
        assert_eq!(setup["summit"], json!(eight.summit()));
        assert_eq!(setup["peak"], json!(PEAK * eight.size));
    }
}
