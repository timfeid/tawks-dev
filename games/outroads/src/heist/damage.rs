//! A car's condition: four body panels, the engine, two headlights, four
//! tyres and the side glass. Hits dent panels, spikes flatten tyres, a flat
//! driven on too long is down to the rim, and the condition sets the pay.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Tyre {
    Ok,
    Flat,
    Rim,
}

/// Panel order: front, rear, left, right. Tyre order: front left, front right, rear left, rear right.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Damage {
    pub panels: [f64; 4],
    pub engine: f64,
    pub lights: [bool; 2],
    pub tyres: [Tyre; 4],
    pub window_broken: bool,
    #[serde(skip)]
    flat_for: [f64; 4],
}

impl Default for Damage {
    fn default() -> Self {
        Damage {
            panels: [1.0; 4],
            engine: 1.0,
            lights: [true; 2],
            tyres: [Tyre::Ok; 4],
            window_broken: false,
            flat_for: [0.0; 4],
        }
    }
}

/// How the car drives with its damage.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Handling {
    pub grip: f64,
    /// Steering pull toward the damaged side, -1 left to 1 right.
    pub pull: f64,
    pub top_speed: f64,
    pub sparks: bool,
    pub smoke: bool,
}

/// Seconds on a flat before it is down to the rim.
pub const RIM_AFTER: f64 = 25.0;

impl Damage {
    /// A hit at `speed` meters per second from a direction in the car's own
    /// frame: x to the right, z forward.
    pub fn hit(&mut self, dir_x: f64, dir_z: f64, speed: f64) {
        if speed < 2.0 {
            return;
        }
        let amount = ((speed - 2.0) / 30.0).powf(1.3).min(0.6);
        let len = dir_x.hypot(dir_z).max(1e-6);
        let (x, z) = (dir_x / len, dir_z / len);
        let shares = [z.max(0.0), (-z).max(0.0), (-x).max(0.0), x.max(0.0)];
        for (panel, share) in self.panels.iter_mut().zip(shares) {
            *panel = (*panel - amount * share).max(0.0);
        }
        self.engine = (self.engine - amount * shares[0] * 0.7).max(0.0);
        if self.panels[0] < 0.45 {
            if x <= 0.2 {
                self.lights[0] = false;
            }
            if x >= -0.2 {
                self.lights[1] = false;
            }
        }
    }

    /// A spike strip takes out the tyres that cross it.
    pub fn spike(&mut self, front: bool, rear: bool) {
        for (i, t) in self.tyres.iter_mut().enumerate() {
            if *t == Tyre::Ok && ((i < 2 && front) || (i >= 2 && rear)) {
                *t = Tyre::Flat;
            }
        }
    }

    pub fn break_window(&mut self) {
        self.window_broken = true;
    }

    pub fn tick(&mut self, dt: f64, speed: f64) {
        for i in 0..4 {
            if self.tyres[i] == Tyre::Flat && speed > 3.0 {
                self.flat_for[i] += dt;
                if self.flat_for[i] >= RIM_AFTER {
                    self.tyres[i] = Tyre::Rim;
                }
            }
        }
    }

    pub fn handling(&self) -> Handling {
        let mut grip: f64 = 1.0;
        let mut pull: f64 = 0.0;
        let mut top: f64 = 44.0;
        let mut sparks = false;
        for (i, t) in self.tyres.iter().enumerate() {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            match t {
                Tyre::Ok => {}
                Tyre::Flat => {
                    grip -= 0.12;
                    pull += 0.12 * side;
                    top = top.min(30.0);
                }
                Tyre::Rim => {
                    grip -= 0.2;
                    pull += 0.2 * side;
                    top = top.min(18.0);
                    sparks = true;
                }
            }
        }
        top = top.min(12.0 + 32.0 * self.engine);
        Handling {
            grip: grip.max(0.3),
            pull: pull.clamp(-0.5, 0.5),
            top_speed: if self.disabled() { 0.0 } else { top },
            sparks,
            smoke: self.engine < 0.45,
        }
    }

    pub fn disabled(&self) -> bool {
        self.engine <= 0.0 || self.tyres.iter().filter(|t| **t == Tyre::Rim).count() >= 3
    }

    /// 0 to 1: what the buyer thinks of it.
    pub fn condition(&self) -> f64 {
        let body = self.panels.iter().sum::<f64>() / 4.0;
        let tyres = self
            .tyres
            .iter()
            .map(|t| match t {
                Tyre::Ok => 1.0,
                Tyre::Flat => 0.6,
                Tyre::Rim => 0.2,
            })
            .sum::<f64>()
            / 4.0;
        let glass = if self.window_broken { 0.85 } else { 1.0 };
        ((0.35 * body + 0.4 * self.engine + 0.25 * tyres) * glass).clamp(0.0, 1.0)
    }

    /// Damage a cop can tell from the street.
    pub fn visibly_damaged(&self) -> bool {
        self.window_broken || self.panels.iter().any(|p| *p < 0.6) || self.lights.iter().any(|l| !l)
    }
}

/// In-game cash for a delivered car.
pub fn payout(value: i64, condition: f64, time_left: f64, peak_heat: f64) -> i64 {
    let time = 0.85 + 0.3 * time_left.clamp(0.0, 1.0);
    let heat = 1.0 - (peak_heat / 250.0).clamp(0.0, 0.4);
    (value as f64 * condition.clamp(0.0, 1.0) * time * heat).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_front_hit_dents_the_front_hurts_the_engine_and_can_kill_a_headlight() {
        let mut d = Damage::default();
        d.hit(0.0, 1.0, 1.5);
        assert_eq!(d, Damage::default(), "a nudge does nothing");
        d.hit(-0.3, 1.0, 28.0);
        assert!(d.panels[0] < 0.6 && d.panels[1] == 1.0);
        assert!(d.engine < 1.0);
        d.hit(-0.3, 1.0, 28.0);
        assert!(!d.lights[0]);
        assert!(d.visibly_damaged());
        assert!(d.condition() < 0.8);
    }

    #[test]
    fn flats_pull_lose_grip_cap_speed_and_wear_to_sparking_rims() {
        let mut d = Damage::default();
        d.spike(true, false);
        let h = d.handling();
        assert!(h.grip < 1.0 && h.top_speed <= 30.0 && !h.sparks);
        assert!(h.pull.abs() < 1e-9, "both fronts flat pull evenly");
        let mut one = Damage::default();
        one.tyres[1] = Tyre::Flat;
        assert!(one.handling().pull > 0.0, "a flat on the right pulls right");
        for _ in 0..300 {
            d.tick(0.1, 15.0);
        }
        assert_eq!(d.tyres[0], Tyre::Rim);
        let h = d.handling();
        assert!(h.sparks && h.top_speed <= 18.0);
        d.spike(false, true);
        for _ in 0..300 {
            d.tick(0.1, 15.0);
        }
        assert!(d.disabled());
        assert_eq!(d.handling().top_speed, 0.0);
    }

    #[test]
    fn condition_scales_the_payout() {
        let clean = payout(14_000, 1.0, 1.0, 0.0);
        let rough = payout(14_000, 0.5, 1.0, 0.0);
        assert_eq!(clean, 16_100);
        assert!(rough * 2 == clean || (rough * 2 - clean).abs() <= 1);
        assert!(payout(14_000, 1.0, 0.0, 100.0) < clean);
    }
}
