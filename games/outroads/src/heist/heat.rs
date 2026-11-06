//! Heat: how much a car's driving gives it away. Driving like everyone else
//! lets it fade; speeding, running junctions, dark headlights, crashes and
//! noise build it.

pub const SUSPICIOUS: f64 = 45.0;
pub const MAX: f64 = 100.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub speed: f64,
    pub limit: f64,
    pub lights: bool,
    /// Crossed a junction this step without slowing.
    pub ran_junction: bool,
    /// Hardest hit this step, meters per second.
    pub impact: f64,
    pub horn: bool,
    pub alarm: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Heat {
    pub value: f64,
    /// The highest it reached, for the payout.
    pub peak: f64,
}

impl Heat {
    pub fn add(&mut self, amount: f64) {
        self.value = (self.value + amount).clamp(0.0, MAX);
        self.peak = self.peak.max(self.value);
    }

    pub fn step(&mut self, s: Sample, dt: f64) {
        let mut gain = 0.0;
        let over = if s.limit > 0.0 { s.speed / s.limit } else { 0.0 };
        if over > 1.6 {
            gain += 14.0 * dt;
        } else if over > 1.2 {
            gain += 6.0 * (over - 1.2) / 0.4 * dt + 2.0 * dt;
        }
        if !s.lights && s.speed > 3.0 {
            gain += 3.0 * dt;
        }
        if s.ran_junction {
            gain += 7.0;
        }
        if s.impact > 4.0 {
            gain += s.impact * 1.4;
        }
        if s.horn {
            gain += 2.5;
        }
        if s.alarm {
            gain += 6.0 * dt;
        }
        let calm = gain == 0.0;
        self.add(gain);
        if calm {
            let fade = if s.speed < 1.0 { 3.0 } else { 2.0 };
            self.add(-fade * dt);
        }
    }

    pub fn suspicious(&self) -> bool {
        self.value >= SUSPICIOUS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(h: &mut Heat, s: Sample, secs: f64) {
        for _ in 0..(secs * 10.0) as usize {
            h.step(s, 0.1);
        }
    }

    #[test]
    fn driving_normally_stays_cold() {
        let mut h = Heat::default();
        drive(
            &mut h,
            Sample {
                speed: 11.0,
                limit: 11.2,
                lights: true,
                ..Sample::default()
            },
            60.0,
        );
        assert_eq!(h.value, 0.0);
    }

    #[test]
    fn speeding_and_dark_headlights_build_heat_and_it_fades_after() {
        let mut h = Heat::default();
        drive(
            &mut h,
            Sample {
                speed: 25.0,
                limit: 11.2,
                lights: false,
                ..Sample::default()
            },
            4.0,
        );
        assert!(h.suspicious(), "{}", h.value);
        let hot = h.value;
        drive(
            &mut h,
            Sample {
                speed: 10.0,
                limit: 11.2,
                lights: true,
                ..Sample::default()
            },
            10.0,
        );
        assert!(h.value < hot - 15.0);
        assert!(h.peak >= hot);
    }

    #[test]
    fn crashes_horns_and_junctions_add_at_once() {
        let mut h = Heat::default();
        h.step(
            Sample {
                speed: 15.0,
                limit: 11.2,
                lights: true,
                ran_junction: true,
                impact: 12.0,
                horn: true,
                alarm: false,
            },
            0.1,
        );
        assert!(h.value > 25.0);
        h.add(500.0);
        assert_eq!(h.value, MAX);
    }
}
