//! How a run ends badly: a car boxed in or disabled with cops beside it, or
//! a thief on foot caught and held.

/// Seconds a stopped car must stay boxed in.
pub const BOXED_FOR: f64 = 5.0;
pub const DISABLED_FOR: f64 = 3.0;
pub const CAUGHT_FOR: f64 = 3.0;
pub const BOX_RANGE: f64 = 7.0;
pub const DISABLED_RANGE: f64 = 15.0;
pub const CATCH_RANGE: f64 = 2.2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hold {
    pub secs: f64,
}

impl Hold {
    /// Builds while `on`, drains twice as fast otherwise; true once it reaches `need`.
    pub fn step(&mut self, on: bool, dt: f64, need: f64) -> bool {
        self.secs = if on {
            self.secs + dt
        } else {
            (self.secs - 2.0 * dt).max(0.0)
        };
        self.secs >= need
    }
}

/// Whether a car is pinned: stopped with a cop car close, or two cops near.
pub fn boxed(speed: f64, cop_distances: &[f64]) -> bool {
    let close = cop_distances.iter().filter(|d| **d <= BOX_RANGE).count();
    let near = cop_distances.iter().filter(|d| **d <= BOX_RANGE * 1.5).count();
    speed < 2.0 && (close >= 1 || near >= 2)
}

pub fn disabled_with_cop(disabled: bool, cop_distances: &[f64]) -> bool {
    disabled && cop_distances.iter().any(|d| *d <= DISABLED_RANGE)
}

pub fn caught(cop_on_foot_distances: &[f64]) -> bool {
    cop_on_foot_distances.iter().any(|d| *d <= CATCH_RANGE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(hold: &mut Hold, on: bool, secs: f64, need: f64) -> bool {
        let mut done = false;
        for _ in 0..(secs * 10.0).round() as usize {
            done = hold.step(on, 0.1, need);
        }
        done
    }

    #[test]
    fn a_stopped_car_with_a_cop_beside_it_for_five_seconds_is_arrested() {
        let mut h = Hold::default();
        assert!(boxed(0.5, &[5.0]));
        assert!(boxed(0.5, &[9.0, 10.0]));
        assert!(!boxed(0.5, &[9.0]));
        assert!(!boxed(6.0, &[2.0]), "still moving");
        assert!(!run(&mut h, true, 4.5, BOXED_FOR));
        assert!(!run(&mut h, false, 1.0, BOXED_FOR), "breaking free drains it");
        assert!(run(&mut h, true, 3.0, BOXED_FOR));
    }

    #[test]
    fn a_disabled_car_or_a_caught_thief_goes_faster() {
        assert!(disabled_with_cop(true, &[12.0]));
        assert!(!disabled_with_cop(false, &[1.0]));
        assert!(caught(&[1.5, 40.0]));
        assert!(!caught(&[3.0]));
        let mut h = Hold::default();
        assert!(run(&mut h, true, 3.0, CAUGHT_FOR));
    }
}
