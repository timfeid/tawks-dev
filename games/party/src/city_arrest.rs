//! Cuffing and escort rules for City Heist. Every value the clients draw
//! (trace position, struggle rhythm, slow motion) is derived from this state.
use serde::Serialize;
use std::f32::consts::TAU;

pub const GRAB_SECONDS: f32 = 0.3;
pub const GRAB_RANGE: f32 = 3.;
pub const SLOW_SCALE: f32 = 0.35;
pub const SLOW_SECONDS: f32 = 2.4;
pub const SLOW_COOLDOWN: f32 = 14.;
pub const OFFICER_COOLDOWN: f32 = 5.;
pub const CUFF_LIMIT: f32 = 10.;
pub const ESCORT_LIMIT: f32 = 25.;
pub const ESCORT_SPEED: f32 = 4.;
pub const DELIVER_RANGE: f32 = 3.4;
pub const STRUGGLE_PERIOD: f32 = 1.1;
pub const STRUGGLE_WINDOW: f32 = 0.12;
pub const STRUGGLE_LOCKOUT: f32 = 0.9;
pub const GROUND_ESCAPE_SECONDS: f32 = 1.4;
/// The escape band occupies one fifth of the ground check's sweep.
pub const GROUND_ESCAPE_WINDOW: f32 = 0.1;
// Presses are judged against the last 150 ms of the rhythm so a press made
// on time by a player with typical latency still lands inside the band.
const LATENCY_GRACE: f32 = 0.15;
pub const TRACE_TOLERANCE: f32 = 0.24;
pub const TRACE_MISS: f32 = 0.42;
const TRACE_PENALTY: f32 = 0.9;
const TRACE_MAX_RATE: f32 = TAU / 1.5;
const STRUGGLE_PUSHBACK: f32 = 0.7;
const STRUGGLE_ESCAPE: f32 = 0.4;
const ESCAPE_DECAY: f32 = 0.06;

/// The figure eight both clients draw, in pad units (x right, y up).
pub fn figure_eight(t: f32) -> (f32, f32) {
    (t.sin(), (2. * t).sin() * 0.6)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Ground,
    Cuffing,
    Escort,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Struggle {
    Escaped,
    Hit,
    Miss,
    Ignored,
}

#[derive(Clone, Debug)]
pub struct Arrest {
    pub officer: u8,
    pub robber: u8,
    pub stage: Stage,
    pub clock: f32,
    pub stage_clock: f32,
    pub t: f32,
    pub cursor: (f32, f32),
    pub mistakes: u8,
    pub slip: f32,
    pub flash: f32,
    pub escape: f32,
    pub lockout: f32,
    pub jolt: f32,
    pub hits: u8,
    /// The robber's one try at slipping the cuffs, once they are led away.
    pub cuffs: Option<crate::city_slip::SlipCheck>,
}

impl Arrest {
    pub fn new(officer: u8, robber: u8) -> Self {
        Self {
            officer,
            robber,
            stage: Stage::Cuffing,
            clock: 0.,
            stage_clock: 0.,
            t: 0.,
            cursor: (0., 0.),
            mistakes: 0,
            slip: 0.,
            flash: 0.,
            escape: 0.,
            lockout: 0.,
            jolt: 0.,
            hits: 0,
            cuffs: None,
        }
    }
    pub fn ground(officer: u8, robber: u8) -> Self {
        let mut arrest = Self::new(officer, robber);
        arrest.stage = Stage::Ground;
        arrest
    }
    pub fn progress(&self) -> f32 {
        (self.t / TAU).clamp(0., 1.)
    }
    pub fn phase_at(&self, clock: f32) -> f32 {
        (clock / if self.stage == Stage::Ground { GROUND_ESCAPE_SECONDS } else { STRUGGLE_PERIOD }).rem_euclid(1.)
    }
    fn in_window(&self, clock: f32) -> bool {
        (self.phase_at(clock) - 0.5).abs() <= STRUGGLE_WINDOW
    }
    pub fn set_cursor(&mut self, x: f32, y: f32) {
        if x.is_finite() && y.is_finite() {
            self.cursor = (x.clamp(-1.6, 1.6), y.clamp(-1.6, 1.6));
        }
    }
    /// Advance the shared timers and, while cuffing, judge the latest cursor.
    /// Returns true when the cursor strayed off the line this tick.
    pub fn advance(&mut self, dt: f32) -> bool {
        self.clock += dt;
        self.stage_clock += dt;
        self.lockout = (self.lockout - dt).max(0.);
        self.flash = (self.flash - dt).max(0.);
        self.jolt = (self.jolt - dt).max(0.);
        self.slip = (self.slip - dt).max(0.);
        if self.stage == Stage::Ground {
            return false;
        }
        if self.stage == Stage::Escort {
            self.escape = (self.escape - ESCAPE_DECAY * dt).max(0.);
            return false;
        }
        let (best, distance) = nearest_on_curve(self.cursor, self.t);
        if distance <= TRACE_TOLERANCE {
            self.t = self.t.max(best.min(self.t + TRACE_MAX_RATE * dt));
            false
        } else if distance > TRACE_MISS && self.slip <= 0. && self.t > 0.05 {
            self.t = (self.t - TRACE_PENALTY).max(0.);
            self.mistakes = self.mistakes.saturating_add(1);
            self.slip = 0.6;
            self.flash = 0.45;
            true
        } else {
            false
        }
    }
    /// The traced path an automated officer follows, with a slip now and then.
    pub fn automated_trace(&mut self, dt: f32, slip: bool) {
        if slip && self.slip <= 0. {
            self.cursor = (self.cursor.0 + 0.6, self.cursor.1 - 0.6);
        } else {
            self.cursor = figure_eight(self.t + TAU / 2.3 * dt);
        }
    }
    pub fn struggle(&mut self) -> Struggle {
        if self.stage == Stage::Ground {
            if (self.phase_at(self.clock) - 0.5).abs() <= GROUND_ESCAPE_WINDOW {
                return Struggle::Escaped;
            }
            self.stage = Stage::Cuffing;
            self.stage_clock = 0.;
            self.flash = 0.45;
            return Struggle::Miss;
        }
        // Once led away the robber's only way out is slipping the cuffs.
        if self.lockout > 0. || self.stage == Stage::Escort {
            return Struggle::Ignored;
        }
        let hit = (0..=4).any(|i| self.in_window(self.clock - LATENCY_GRACE * i as f32 / 4.));
        if hit {
            self.hits = self.hits.saturating_add(1);
            self.jolt = 0.4;
            // A press only counts once per beat, so mashing cannot farm hits.
            self.lockout = STRUGGLE_PERIOD * 0.5;
            match self.stage {
                Stage::Cuffing => self.t = (self.t - STRUGGLE_PUSHBACK).max(0.),
                Stage::Escort => self.escape = (self.escape + STRUGGLE_ESCAPE).min(1.),
                Stage::Ground => unreachable!(),
            }
            Struggle::Hit
        } else {
            self.lockout = STRUGGLE_LOCKOUT;
            self.escape = (self.escape - 0.1).max(0.);
            Struggle::Miss
        }
    }
    pub fn view(&self) -> ArrestView {
        ArrestView {
            officer: self.officer,
            robber: self.robber,
            stage: self.stage,
            progress: self.progress(),
            trace: self.t,
            cursor: [self.cursor.0, self.cursor.1],
            mistakes: self.mistakes,
            flash: self.flash,
            escape: self.escape,
            clock: self.clock,
            period: if self.stage == Stage::Ground { GROUND_ESCAPE_SECONDS } else { STRUGGLE_PERIOD },
            window: if self.stage == Stage::Ground { GROUND_ESCAPE_WINDOW } else { STRUGGLE_WINDOW },
            lockout: self.lockout,
            jolt: self.jolt,
            cuffs: self.cuffs.as_ref().map(|c| c.view()),
            remaining: match self.stage {
                Stage::Ground => GROUND_ESCAPE_SECONDS,
                Stage::Cuffing => CUFF_LIMIT,
                Stage::Escort => ESCORT_LIMIT,
            } - self.stage_clock,
        }
    }
}

fn nearest_on_curve(cursor: (f32, f32), t: f32) -> (f32, f32) {
    let mut best = (t, f32::INFINITY);
    let mut s = (t - 0.25).max(0.);
    while s <= t + 0.7 {
        let p = figure_eight(s);
        let d = (p.0 - cursor.0).hypot(p.1 - cursor.1);
        if d < best.1 {
            best = (s, d);
        }
        s += 0.02;
    }
    best
}

#[derive(Clone, Debug, Serialize)]
pub struct ArrestView {
    pub officer: u8,
    pub robber: u8,
    pub stage: Stage,
    pub progress: f32,
    pub trace: f32,
    pub cursor: [f32; 2],
    pub mistakes: u8,
    pub flash: f32,
    pub escape: f32,
    pub clock: f32,
    pub period: f32,
    pub window: f32,
    pub lockout: f32,
    pub jolt: f32,
    pub cuffs: Option<crate::city_slip::SlipView>,
    pub remaining: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracing_the_curve_completes_one_loop() {
        let mut a = Arrest::new(0, 1);
        let mut s = 0.;
        for _ in 0..60 * 4 {
            s += TAU / 2.5 / 60.;
            a.set_cursor(figure_eight(s).0, figure_eight(s).1);
            a.advance(1. / 60.);
        }
        assert!(a.progress() >= 1., "{}", a.progress());
        assert_eq!(a.mistakes, 0);
    }
    #[test]
    fn leaving_the_line_costs_progress_once_per_slip() {
        let mut a = Arrest::new(0, 1);
        a.t = 2.;
        a.set_cursor(1.5, -1.5);
        assert!(a.advance(1. / 60.));
        assert!((a.t - 1.1).abs() < 0.001);
        assert!(!a.advance(1. / 60.), "one excursion is one mistake");
        assert_eq!(a.mistakes, 1);
    }
    #[test]
    fn jumping_ahead_on_the_curve_is_rate_limited() {
        let mut a = Arrest::new(0, 1);
        let p = figure_eight(0.6);
        a.set_cursor(p.0, p.1);
        a.advance(1. / 60.);
        assert!(a.t <= TRACE_MAX_RATE / 60. + 0.001);
    }
    #[test]
    fn struggle_timing_rewards_the_band_and_locks_out_mashing() {
        let mut a = Arrest::new(0, 1);
        a.clock = STRUGGLE_PERIOD * 0.5;
        assert_eq!(a.struggle(), Struggle::Hit);
        assert!(a.t == 0.);
        assert_eq!(a.struggle(), Struggle::Ignored);
        let mut b = Arrest::new(0, 1);
        b.clock = STRUGGLE_PERIOD * 0.05;
        assert_eq!(b.struggle(), Struggle::Miss);
        assert!(b.lockout > 0.);
    }
    #[test]
    fn a_late_press_inside_the_latency_grace_still_counts() {
        let mut a = Arrest::new(0, 1);
        a.clock = STRUGGLE_PERIOD * (0.5 + STRUGGLE_WINDOW) + 0.1;
        assert_eq!(a.struggle(), Struggle::Hit);
    }
}
