//! Whether the police are on the car: calm, in pursuit while a cop keeps it
//! in sight, searching once sight is lost for a while, then cooling down.

use serde::Serialize;

/// Seconds out of sight before a pursuit turns into a search.
pub const LOSE_AFTER: f64 = 8.0;
pub const SEARCH_FOR: f64 = 30.0;
pub const COOL_FOR: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Phase {
    Calm,
    Pursuit { last_seen: f64 },
    Search { until: f64 },
    Cooldown { until: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pursuit {
    pub phase: Phase,
    /// Where cops last had eyes on it.
    pub last_known: Option<(f64, f64)>,
}

impl Default for Pursuit {
    fn default() -> Self {
        Pursuit {
            phase: Phase::Calm,
            last_known: None,
        }
    }
}

impl Pursuit {
    /// `seen` is a cop having the car in view; `chase` is whether being seen
    /// is enough to give chase (the car is reported, or driving hot).
    /// Returns true when a pursuit just began.
    pub fn update(&mut self, now: f64, seen: Option<(f64, f64)>, chase: bool) -> bool {
        if let Some(p) = seen {
            self.last_known = Some(p);
        }
        let started = match self.phase {
            Phase::Calm | Phase::Cooldown { .. } | Phase::Search { .. } if seen.is_some() && chase => {
                self.phase = Phase::Pursuit { last_seen: now };
                true
            }
            Phase::Pursuit { .. } if seen.is_some() => {
                self.phase = Phase::Pursuit { last_seen: now };
                false
            }
            Phase::Pursuit { last_seen } if now - last_seen >= LOSE_AFTER => {
                self.phase = Phase::Search {
                    until: now + SEARCH_FOR,
                };
                false
            }
            Phase::Search { until } if now >= until => {
                self.phase = Phase::Cooldown { until: now + COOL_FOR };
                false
            }
            Phase::Cooldown { until } if now >= until => {
                self.phase = Phase::Calm;
                false
            }
            _ => false,
        };
        started
    }

    pub fn chasing(&self) -> bool {
        matches!(self.phase, Phase::Pursuit { .. })
    }

    pub fn active(&self) -> bool {
        matches!(self.phase, Phase::Pursuit { .. } | Phase::Search { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hot_car_in_view_is_chased_lost_searched_and_cooled() {
        let mut p = Pursuit::default();
        assert!(
            !p.update(0.0, Some((1.0, 1.0)), false),
            "a cold car in view is no chase"
        );
        assert!(p.update(1.0, Some((2.0, 2.0)), true));
        assert!(p.chasing());
        p.update(5.0, Some((3.0, 3.0)), true);
        p.update(5.0 + LOSE_AFTER - 0.1, None, true);
        assert!(p.chasing(), "not lost yet");
        p.update(5.0 + LOSE_AFTER, None, true);
        assert!(matches!(p.phase, Phase::Search { .. }));
        assert_eq!(p.last_known, Some((3.0, 3.0)));
        let t = 5.0 + LOSE_AFTER + SEARCH_FOR;
        p.update(t, None, true);
        assert!(matches!(p.phase, Phase::Cooldown { .. }));
        p.update(t + COOL_FOR, None, true);
        assert_eq!(p.phase, Phase::Calm);
    }

    #[test]
    fn being_seen_again_while_searched_restarts_the_chase() {
        let mut p = Pursuit::default();
        p.update(0.0, Some((0.0, 0.0)), true);
        p.update(LOSE_AFTER, None, true);
        assert!(p.active() && !p.chasing());
        assert!(p.update(LOSE_AFTER + 3.0, Some((5.0, 5.0)), true));
        assert!(p.chasing());
    }
}
