//! Taking a parked car: first getting in, then getting it started. Both are
//! abstract skill games refereed here; the client only reports when the
//! player pressed or let go, and this decides what that was worth.

use std::f64::consts::TAU;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::rules::Tier;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Entry {
    /// Instant and silent, when the car happens to be open.
    Handle,
    /// The ring: three good presses, slow but clean.
    Lock,
    /// Fast and loud, with an alarm chance and a dent in the value.
    Window,
}

/// The lock game: a marker sweeps a ring; press while it is over the mark.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Ring {
    /// Radians a second.
    pub speed: f64,
    pub target: f64,
    pub window: f64,
}

impl Ring {
    pub fn angle(&self, t: f64) -> f64 {
        (self.speed * t).rem_euclid(TAU)
    }

    pub fn on_mark(&self, t: f64) -> bool {
        let d = (self.angle(t) - self.target).rem_euclid(TAU);
        d.min(TAU - d) <= self.window
    }
}

/// The ignition game: hold to raise a needle, let go to drop it, and keep it
/// inside a band that drifts. Time inside the band fills the start.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Meter {
    pub phase: f64,
    pub freq: f64,
    pub band: f64,
    /// Seconds inside the band to start, before any help.
    pub need: f64,
    pub needle: f64,
    pub progress: f64,
    #[serde(skip)]
    at: f64,
    #[serde(skip)]
    holding: bool,
}

pub const RISE: f64 = 0.9;
pub const FALL: f64 = 0.7;
const STEP: f64 = 0.02;

impl Meter {
    pub fn centre(&self, t: f64) -> f64 {
        0.5 + 0.3 * (self.freq * t + self.phase).sin()
    }

    /// Runs the needle forward to `t` under the current hold, at `assist` speed.
    pub fn run_to(&mut self, t: f64, assist: f64) {
        while self.at + 1e-9 < t && self.progress < 1.0 {
            let dt = STEP.min(t - self.at);
            self.at += dt;
            let dir = if self.holding { RISE } else { -FALL };
            self.needle = (self.needle + dir * dt).clamp(0.0, 1.0);
            if (self.needle - self.centre(self.at)).abs() <= self.band {
                self.progress += dt * assist / self.need;
            } else {
                self.progress = (self.progress - 0.08 * dt).max(0.0);
            }
        }
        self.progress = self.progress.min(1.0);
    }

    pub fn set_hold(&mut self, t: f64, hold: bool, assist: f64) {
        self.run_to(t, assist);
        self.holding = hold;
    }

    pub fn holding(&self) -> bool {
        self.holding
    }

    pub fn time(&self) -> f64 {
        self.at
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Door {
        tried_handle: bool,
    },
    Lock {
        started: f64,
        ring: Ring,
        hits: u8,
        misses: u8,
        #[serde(skip)]
        last: f64,
    },
    Ignition {
        started: f64,
        meter: Meter,
    },
    Started,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// The handle gave; on to starting it.
    Opened,
    /// Locked after all; choose again.
    StillLocked,
    LockStarted,
    Hit,
    /// A miss makes a sound.
    Miss,
    /// The glass went; `alarm` says whether the car started screaming.
    Smashed {
        alarm: bool,
    },
    /// A spare key was in the car.
    SpareKey,
    Started,
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Alarm {
    pub start: f64,
    pub until: f64,
}

/// Seconds an alarm screams before it gives up.
pub const ALARM_FOR: f64 = 30.0;
/// Seconds of alarm before a neighbour calls it in.
pub const ALARM_WITNESS_AFTER: f64 = 8.0;
/// A lookout standing this close speeds the start.
pub const LOOKOUT_RANGE: f64 = 7.0;
pub const LOOKOUT_SPEED: f64 = 1.5;
/// Press times are taken on trust only this far ahead of the server's clock.
const AHEAD: f64 = 0.6;
const STALE: f64 = 3.0;
/// How far behind its own clock the server runs the start, for presses in flight.
const LAG: f64 = 1.0;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Take {
    pub car: usize,
    pub thief: String,
    pub tier: Tier,
    pub stage: Stage,
    pub alarm: Option<Alarm>,
    pub window_broken: bool,
    /// Sounds made, for witnesses.
    pub noise: u32,
}

/// What a car's surroundings add to its odds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Odds {
    pub unlocked_bonus: f64,
}

impl Take {
    pub fn new(car: usize, thief: &str, tier: Tier) -> Take {
        Take {
            car,
            thief: thief.to_string(),
            tier,
            stage: Stage::Door { tried_handle: false },
            alarm: None,
            window_broken: false,
            noise: 0,
        }
    }

    fn to_ignition(&mut self, now: f64, rng: &mut impl Rng) -> Outcome {
        if rng.random_bool(self.tier.spare_key_chance()) {
            self.stage = Stage::Started;
            return Outcome::SpareKey;
        }
        self.stage = Stage::Ignition {
            started: now,
            meter: Meter {
                phase: rng.random_range(0.0..TAU),
                freq: rng.random_range(0.6..1.1),
                band: self.tier.band(),
                need: self.tier.ignition_time(),
                needle: 0.2,
                progress: 0.0,
                at: 0.0,
                holding: false,
            },
        };
        Outcome::Opened
    }

    pub fn choose(&mut self, entry: Entry, now: f64, odds: Odds, rng: &mut impl Rng) -> Outcome {
        let Stage::Door { tried_handle } = self.stage else {
            return Outcome::Ignored;
        };
        match entry {
            Entry::Handle if tried_handle => Outcome::Ignored,
            Entry::Handle => {
                let p = (self.tier.unlocked_chance() + odds.unlocked_bonus).clamp(0.0, 1.0);
                if rng.random_bool(p) {
                    self.to_ignition(now, rng)
                } else {
                    self.stage = Stage::Door { tried_handle: true };
                    Outcome::StillLocked
                }
            }
            Entry::Lock => {
                self.stage = Stage::Lock {
                    started: now,
                    ring: Ring {
                        speed: rng.random_range(2.6..3.8) * if rng.random_bool(0.5) { 1.0 } else { -1.0 },
                        target: rng.random_range(0.0..TAU),
                        window: self.tier.lock_window(),
                    },
                    hits: 0,
                    misses: 0,
                    last: -1.0,
                };
                Outcome::LockStarted
            }
            Entry::Window => {
                self.window_broken = true;
                self.noise += 3;
                let alarm = rng.random_bool(self.tier.alarm_chance());
                if alarm {
                    self.alarm = Some(Alarm {
                        start: now,
                        until: now + ALARM_FOR,
                    });
                }
                match self.to_ignition(now, rng) {
                    Outcome::SpareKey => Outcome::SpareKey,
                    _ => Outcome::Smashed { alarm },
                }
            }
        }
    }

    /// A press in the lock game, `t` seconds after it began by the player's clock.
    pub fn press(&mut self, t: f64, now: f64, rng: &mut impl Rng) -> Outcome {
        let Stage::Lock {
            started,
            ring,
            hits,
            misses,
            last,
        } = &mut self.stage
        else {
            return Outcome::Ignored;
        };
        let elapsed = now - *started;
        if t <= *last + 0.15 || t > elapsed + AHEAD || t < elapsed - STALE {
            return Outcome::Ignored;
        }
        *last = t;
        if ring.on_mark(t) {
            *hits += 1;
            if *hits >= 3 {
                return self.to_ignition(now, rng);
            }
            Outcome::Hit
        } else {
            *misses += 1;
            *last = t + 0.85;
            self.noise += 1;
            Outcome::Miss
        }
    }

    /// The player pressed or let go in the ignition game.
    pub fn hold(&mut self, t: f64, hold: bool, now: f64, assist: f64) -> Outcome {
        let Stage::Ignition { started, meter } = &mut self.stage else {
            return Outcome::Ignored;
        };
        let elapsed = now - *started;
        if t > elapsed + AHEAD {
            return Outcome::Ignored;
        }
        // A client whose clock lags the server's still steers from now on.
        meter.set_hold(t.max(meter.time()), hold, assist);
        if meter.progress >= 1.0 {
            self.stage = Stage::Started;
            return Outcome::Started;
        }
        Outcome::Ignored
    }

    /// Keeps the ignition running between presses, a little behind the
    /// server's clock so presses still on their way can land in order.
    pub fn tick(&mut self, now: f64, assist: f64) -> Outcome {
        if let Stage::Ignition { started, meter } = &mut self.stage {
            meter.run_to(now - *started - LAG, assist);
            if meter.progress >= 1.0 {
                self.stage = Stage::Started;
                return Outcome::Started;
            }
        }
        Outcome::Ignored
    }

    /// Seconds since the lock or start game began, for clients to line their clocks up.
    pub fn stage_elapsed(&self, now: f64) -> Option<f64> {
        match &self.stage {
            Stage::Lock { started, .. } | Stage::Ignition { started, .. } => Some(now - started),
            _ => None,
        }
    }

    pub fn started(&self) -> bool {
        self.stage == Stage::Started
    }

    pub fn alarm_on(&self, now: f64) -> bool {
        self.alarm.is_some_and(|a| now < a.until)
    }

    /// When an alarm makes a neighbour call it in.
    pub fn witness_due(&self) -> Option<f64> {
        self.alarm.map(|a| a.start + ALARM_WITNESS_AFTER)
    }
}

/// How fast a lookout makes the start go: a crewmate on foot near the car.
pub fn assist(lookouts_in_range: usize) -> f64 {
    if lookouts_in_range > 0 { LOOKOUT_SPEED } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(11)
    }

    /// Plays the ignition well: holds below the band's middle, lets go above.
    fn play_ignition(take: &mut Take, now0: f64, assist: f64) -> Option<f64> {
        let mut t = 0.0;
        while t < 60.0 {
            let Stage::Ignition { meter, .. } = &take.stage else {
                return Some(t);
            };
            let hold = meter.needle < meter.centre(t);
            if take.hold(t, hold, now0 + t, assist) == Outcome::Started {
                return Some(t);
            }
            t += 0.05;
        }
        None
    }

    #[test]
    fn the_handle_opens_only_unlocked_cars_and_only_once() {
        let mut open = Take::new(0, "a", Tier(1));
        assert_eq!(
            open.choose(Entry::Handle, 0.0, Odds { unlocked_bonus: 1.0 }, &mut rng()),
            Outcome::Opened
        );
        assert!(matches!(open.stage, Stage::Ignition { .. }));
        let mut shut = Take::new(0, "a", Tier(3));
        assert_eq!(
            shut.choose(Entry::Handle, 0.0, Odds { unlocked_bonus: -1.0 }, &mut rng()),
            Outcome::StillLocked
        );
        assert_eq!(
            shut.choose(Entry::Handle, 1.0, Odds { unlocked_bonus: 1.0 }, &mut rng()),
            Outcome::Ignored
        );
        assert_eq!(
            shut.choose(Entry::Lock, 1.0, Odds::default(), &mut rng()),
            Outcome::LockStarted
        );
    }

    #[test]
    fn three_good_presses_open_the_lock_and_misses_make_noise() {
        let mut take = Take::new(0, "a", Tier(2));
        let mut r = rng();
        take.choose(Entry::Lock, 10.0, Odds::default(), &mut r);
        let Stage::Lock { ring, .. } = take.stage.clone() else {
            panic!()
        };
        let off = (ring.target + std::f64::consts::PI).rem_euclid(TAU) / ring.speed.abs();
        let first_miss = if ring.speed > 0.0 {
            off
        } else {
            (TAU - (ring.target + std::f64::consts::PI).rem_euclid(TAU)) / ring.speed.abs()
        };
        assert_eq!(
            take.press(first_miss + 0.01, 10.0 + first_miss + 0.01, &mut r),
            Outcome::Miss
        );
        assert_eq!(take.noise, 1);
        let mut t = first_miss + 1.2;
        let mut hits = 0;
        let mut opened = false;
        while t < 30.0 && !opened {
            if ring.on_mark(t) {
                match take.press(t, 10.0 + t, &mut r) {
                    Outcome::Hit => hits += 1,
                    Outcome::Opened | Outcome::SpareKey => opened = true,
                    _ => {}
                }
                t += 0.4;
            }
            t += 0.01;
        }
        assert!(opened && hits == 2, "{hits}");
        assert_eq!(take.press(t + 1.0, 10.0 + t + 1.0, &mut r), Outcome::Ignored);
    }

    #[test]
    fn presses_from_the_future_or_long_ago_do_not_count() {
        let mut take = Take::new(0, "a", Tier(1));
        take.choose(Entry::Lock, 0.0, Odds::default(), &mut rng());
        assert_eq!(take.press(5.0, 1.0, &mut rng()), Outcome::Ignored);
        assert_eq!(take.press(1.0, 9.0, &mut rng()), Outcome::Ignored);
    }

    #[test]
    fn a_window_is_quick_but_loud_and_an_alarm_calls_a_witness() {
        let mut take = Take::new(0, "a", Tier(3));
        let mut r = StdRng::seed_from_u64(1);
        let out = take.choose(Entry::Window, 50.0, Odds::default(), &mut r);
        assert!(matches!(out, Outcome::Smashed { .. } | Outcome::SpareKey));
        assert!(take.window_broken && take.noise >= 3);
        let mut alarms = 0;
        for seed in 0..200 {
            let mut t = Take::new(0, "a", Tier(3));
            t.choose(Entry::Window, 0.0, Odds::default(), &mut StdRng::seed_from_u64(seed));
            if let Some(a) = t.alarm {
                alarms += 1;
                assert_eq!(t.witness_due(), Some(a.start + ALARM_WITNESS_AFTER));
                assert!(t.alarm_on(ALARM_FOR - 1.0) && !t.alarm_on(ALARM_FOR));
            }
        }
        assert!((130..190).contains(&alarms), "about four in five: {alarms}");
    }

    #[test]
    fn starting_takes_skill_and_a_lookout_makes_it_quicker() {
        let mut alone = Take::new(0, "a", Tier(2));
        let mut helped = alone.clone();
        let mut r = StdRng::seed_from_u64(5);
        alone.to_ignition(0.0, &mut r);
        let mut r = StdRng::seed_from_u64(5);
        helped.to_ignition(0.0, &mut r);
        if alone.started() {
            return;
        }
        let solo = play_ignition(&mut alone, 0.0, assist(0)).expect("good play starts it");
        let duo = play_ignition(&mut helped, 0.0, assist(1)).expect("good play starts it");
        assert!(solo >= Tier(2).ignition_time() * 0.95, "{solo}");
        assert!(duo < solo * 0.8, "{duo} {solo}");
        let mut late = Take::new(0, "a", Tier(1));
        late.to_ignition(0.0, &mut StdRng::seed_from_u64(9));
        if !late.started() {
            late.tick(5.0, 1.0);
            assert_ne!(
                late.hold(4.5, true, 5.0, 1.0),
                Outcome::Started,
                "a press half a second late still counts"
            );
            let Stage::Ignition { meter, .. } = &late.stage else {
                panic!()
            };
            assert!(meter.holding());
            assert_ne!(late.hold(1.0, false, 5.0, 1.0), Outcome::Started);
            let Stage::Ignition { meter, .. } = &late.stage else {
                panic!()
            };
            assert!(!meter.holding(), "a clock running behind still steers the needle");
        }
        let mut idle = Take::new(0, "a", Tier(1));
        idle.to_ignition(0.0, &mut StdRng::seed_from_u64(9));
        idle.tick(60.0, 1.0);
        assert!(!idle.started(), "doing nothing never starts it");
    }
}
