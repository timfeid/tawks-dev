//! Slipping the cuffs: a ten-second test of nerve once a robber is led away.
//! A grip marker drifts and jerks around a cuff ring; the robber keeps their
//! cursor on it and answers flick cues, and escapes only after four clean
//! seconds in a row. Any slip cracks the streak back to zero. The officer can
//! tighten the cuffs (faster, jumpier marker) but must stand still to do it.
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

pub const SLIP_SECONDS: f32 = 10.;
pub const CLEAN_SECONDS: f32 = 4.;
/// Half the width of the grip marker, in radians around the ring.
pub const GRIP: f32 = 0.34;
/// How long a cue stays up for its answer.
pub const CUE_SECONDS: f32 = 0.75;
/// Inputs are judged against the last 150 ms so typical latency is not a slip.
const GRACE: f32 = 0.15;
const DRIFT: f32 = 1.5;
const TIGHT_DRIFT: f32 = 2.3;
const JITTER: f32 = 0.05;
const TIGHT_JITTER: f32 = 0.13;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum SlipInput {
    /// Where the cursor is around the ring, in radians.
    Aim { angle: f32 },
    /// A quick flick: -1 left, 1 right.
    Flick { dir: i8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Escaped,
    Held,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Cue {
    /// The arrow's direction; the answer is a flick the other way.
    dir: i8,
    fake: bool,
    shown: f32,
    answered: bool,
}

/// What an automated robber will do: a moment after which it plays clean,
/// or none if it keeps fumbling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotPlan {
    pub clean_from: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct SlipCheck {
    seed: u64,
    pub clock: f32,
    target: f32,
    speed: f32,
    next_flip: f32,
    history: [f32; 10],
    cursor: f32,
    pub streak: f32,
    pub best: f32,
    pub mistakes: u8,
    pub crack: f32,
    outside: bool,
    cue: Option<Cue>,
    next_cue: f32,
    cues: u32,
    pub tightening: bool,
    pub outcome: Option<Outcome>,
    pub bot: Option<BotPlan>,
    /// Half the marker's width; scripted reviews widen it for slow frames.
    pub grip: f32,
    bot_slip_at: f32,
}

fn hash(seed: u64, a: u32, b: u32) -> f32 {
    let mut h = seed ^ (u64::from(a) << 32) ^ u64::from(b);
    h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h % 10_000) as f32 / 10_000.
}

fn gap(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

/// An automated robber's odds of four clean seconds: a casual bot almost
/// never manages it, a good one now and then.
pub fn bot_escape_chance(skill: f32) -> f32 {
    (0.01 + skill.clamp(0., 1.).powi(3) * 0.16).min(0.2)
}

impl SlipCheck {
    pub fn new(seed: u64, bot_skill: Option<f32>) -> Self {
        let start = hash(seed, 1, 0) * TAU;
        let bot = bot_skill.map(|skill| BotPlan {
            clean_from: (hash(seed, 2, 0) < bot_escape_chance(skill))
                .then(|| 0.5 + hash(seed, 3, 0) * (SLIP_SECONDS - CLEAN_SECONDS - 1.)),
        });
        SlipCheck {
            seed,
            clock: 0.,
            target: start,
            speed: DRIFT * if hash(seed, 4, 0) < 0.5 { 1. } else { -1. },
            next_flip: 1.2 + hash(seed, 5, 0) * 1.3,
            history: [start; 10],
            cursor: start,
            streak: 0.,
            best: 0.,
            mistakes: 0,
            crack: 0.,
            outside: false,
            cue: None,
            next_cue: 1.5 + hash(seed, 6, 0) * 1.2,
            cues: 0,
            tightening: false,
            outcome: None,
            bot,
            grip: GRIP,
            bot_slip_at: 0.8 + hash(seed, 7, 0) * 1.5,
        }
    }
    /// The marker as drawn: its drift plus the cuffs' jitter.
    pub fn target(&self) -> f32 {
        let jitter = if self.tightening {
            TIGHT_JITTER
        } else {
            JITTER
        };
        (self.target + (self.clock * 19.).sin() * jitter + (self.clock * 31.).sin() * jitter * 0.5)
            .rem_euclid(TAU)
    }
    pub fn input(&mut self, input: SlipInput) {
        if self.outcome.is_some() || self.bot.is_some() {
            return;
        }
        match input {
            SlipInput::Aim { angle } if angle.is_finite() => {
                self.cursor = angle.rem_euclid(TAU);
            }
            SlipInput::Flick { dir } if dir == 1 || dir == -1 => self.flick(dir),
            _ => {}
        }
    }
    fn flick(&mut self, dir: i8) {
        let clock = self.clock;
        let live = self
            .cue
            .filter(|c| !c.answered && clock <= c.shown + CUE_SECONDS + GRACE);
        match live {
            Some(c) if !c.fake && dir == -c.dir => {
                if let Some(cue) = self.cue.as_mut() {
                    cue.answered = true;
                }
            }
            _ => self.mistake(),
        }
    }
    fn mistake(&mut self) {
        self.streak = 0.;
        self.mistakes = self.mistakes.saturating_add(1);
        self.crack = 0.5;
    }
    pub fn advance(&mut self, dt: f32) -> Option<Outcome> {
        if self.outcome.is_some() {
            return self.outcome;
        }
        self.clock += dt;
        self.crack = (self.crack - dt).max(0.);
        let tick = (self.clock * 60.) as u32;
        // Drift with the odd sudden reversal.
        let top = if self.tightening { TIGHT_DRIFT } else { DRIFT };
        self.speed = self.speed.signum() * top * (0.75 + 0.5 * hash(self.seed, 8, tick / 20));
        if self.clock >= self.next_flip {
            self.speed = -self.speed;
            self.next_flip = self.clock + 0.9 + hash(self.seed, 9, tick) * 1.6;
        }
        self.target = (self.target + self.speed * dt).rem_euclid(TAU);
        self.history.rotate_right(1);
        self.history[0] = self.target();
        // Cues: a real arrow wants the opposite flick; a red one wants nothing.
        if let Some(c) = self.cue {
            if self.clock > c.shown + CUE_SECONDS + GRACE {
                if !c.fake && !c.answered {
                    self.mistake();
                }
                self.cue = None;
            }
        } else if self.clock >= self.next_cue {
            self.cues += 1;
            self.cue = Some(Cue {
                dir: if hash(self.seed, 10, self.cues) < 0.5 {
                    -1
                } else {
                    1
                },
                fake: hash(self.seed, 11, self.cues) < 0.3,
                shown: self.clock,
                answered: false,
            });
            self.next_cue = self.clock + CUE_SECONDS + 0.9 + hash(self.seed, 12, self.cues) * 1.3;
        }
        if let Some(plan) = self.bot {
            self.play_bot(plan);
        }
        // On the marker as it is, or as it was a moment ago.
        let window = ((GRACE / dt.max(0.001)) as usize).clamp(1, self.history.len());
        let on = self.history[..window]
            .iter()
            .any(|t| gap(*t, self.cursor) <= self.grip);
        if !on && !self.outside {
            self.mistake();
        }
        self.outside = !on;
        if on {
            self.streak += dt;
            self.best = self.best.max(self.streak);
        }
        if self.streak >= CLEAN_SECONDS {
            self.outcome = Some(Outcome::Escaped);
        } else if self.clock >= SLIP_SECONDS {
            self.outcome = Some(Outcome::Held);
        }
        self.outcome
    }
    fn play_bot(&mut self, plan: BotPlan) {
        let clean = plan.clean_from.is_some_and(|from| self.clock >= from);
        self.cursor = self.target();
        if let Some(c) = self.cue {
            if !c.fake && !c.answered && self.clock > c.shown + 0.3 {
                if let Some(cue) = self.cue.as_mut() {
                    cue.answered = true;
                }
            }
        }
        if !clean && self.clock >= self.bot_slip_at {
            // A fumble: the cursor slides off the marker for a beat.
            self.cursor = (self.target() + PI).rem_euclid(TAU);
            self.bot_slip_at =
                self.clock + 1.2 + hash(self.seed, 13, (self.clock * 10.) as u32) * 2.;
        }
    }
    pub fn view(&self) -> SlipView {
        SlipView {
            clock: self.clock,
            left: (SLIP_SECONDS - self.clock).max(0.),
            target: self.target(),
            grip: self.grip,
            cursor: self.cursor,
            streak: self.streak,
            need: CLEAN_SECONDS,
            crack: self.crack,
            cue: self.cue.filter(|c| !c.answered).map(|c| CueView {
                dir: c.dir,
                fake: c.fake,
                left: (c.shown + CUE_SECONDS - self.clock).max(0.),
            }),
            tightening: self.tightening,
            mistakes: self.mistakes,
            outcome: self.outcome,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CueView {
    pub dir: i8,
    pub fake: bool,
    pub left: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct SlipView {
    pub clock: f32,
    pub left: f32,
    pub target: f32,
    pub grip: f32,
    pub cursor: f32,
    pub streak: f32,
    pub need: f32,
    pub crack: f32,
    pub cue: Option<CueView>,
    pub tightening: bool,
    pub mistakes: u8,
    pub outcome: Option<Outcome>,
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f32 = 1. / 60.;

    /// A flawless player: on the marker every tick, answering each real cue.
    fn perfect(s: &mut SlipCheck) {
        s.input(SlipInput::Aim { angle: s.target() });
        if let Some(c) = s
            .cue
            .filter(|c| !c.fake && !c.answered && s.clock > c.shown + 0.2)
        {
            s.input(SlipInput::Flick { dir: -c.dir });
        }
    }

    #[test]
    fn four_clean_seconds_slip_the_cuffs() {
        let mut s = SlipCheck::new(7, None);
        let mut out = None;
        while out.is_none() {
            perfect(&mut s);
            out = s.advance(DT);
        }
        assert_eq!(out, Some(Outcome::Escaped));
        assert!((s.clock - CLEAN_SECONDS).abs() < 0.1, "{}", s.clock);
    }

    #[test]
    fn standing_still_holds_the_cuffs() {
        let mut s = SlipCheck::new(7, None);
        let mut out = None;
        while out.is_none() {
            out = s.advance(DT);
        }
        assert_eq!(out, Some(Outcome::Held));
        assert!(s.best < CLEAN_SECONDS);
    }

    #[test]
    fn a_slip_or_a_wrong_flick_cracks_the_streak() {
        let mut s = SlipCheck::new(3, None);
        for _ in 0..60 {
            perfect(&mut s);
            s.advance(DT);
        }
        assert!(s.streak > 0.9);
        s.input(SlipInput::Aim {
            angle: s.target() + PI,
        });
        s.advance(DT);
        assert_eq!(s.streak, 0.);
        let mut s = SlipCheck::new(3, None);
        for _ in 0..30 {
            perfect(&mut s);
            s.advance(DT);
        }
        // No cue is up: any flick is a mistake.
        s.cue = None;
        s.input(SlipInput::Flick { dir: 1 });
        assert_eq!(s.streak, 0.);
        assert_eq!(s.mistakes, 1);
    }

    #[test]
    fn flicking_at_a_fake_or_missing_a_real_cue_is_a_mistake() {
        let mut s = SlipCheck::new(5, None);
        s.cue = Some(Cue {
            dir: 1,
            fake: true,
            shown: 0.,
            answered: false,
        });
        s.input(SlipInput::Flick { dir: -1 });
        assert_eq!(s.mistakes, 1);
        let mut s = SlipCheck::new(5, None);
        s.next_cue = 99.;
        s.cue = Some(Cue {
            dir: 1,
            fake: false,
            shown: 0.,
            answered: false,
        });
        for _ in 0..60 {
            s.input(SlipInput::Aim { angle: s.target() });
            s.advance(DT);
        }
        assert_eq!(s.mistakes, 1, "the unanswered cue");
        let mut s = SlipCheck::new(5, None);
        s.cue = Some(Cue {
            dir: 1,
            fake: false,
            shown: 0.,
            answered: false,
        });
        s.input(SlipInput::Flick { dir: 1 });
        assert_eq!(s.mistakes, 1, "the arrow's own way is wrong");
    }

    #[test]
    fn bots_rarely_slip_free_and_casual_ones_almost_never() {
        let rate = |skill: f32| {
            let escapes = (0..2000u64)
                .filter(|seed| {
                    let mut s = SlipCheck::new(*seed * 31 + 1, Some(skill));
                    loop {
                        if let Some(out) = s.advance(DT) {
                            return out == Outcome::Escaped;
                        }
                    }
                })
                .count();
            escapes as f32 / 2000.
        };
        let (casual, good) = (rate(0.3), rate(0.8));
        assert!(casual < 0.04, "casual {casual}");
        assert!((0.04..0.15).contains(&good), "good {good}");
    }

    #[test]
    fn tightening_speeds_and_shakes_the_marker() {
        let mut calm = SlipCheck::new(9, None);
        let mut tight = SlipCheck::new(9, None);
        tight.tightening = true;
        let (mut a, mut b) = (0., 0.);
        for _ in 0..120 {
            let (ca, cb) = (calm.target(), tight.target());
            calm.advance(DT);
            tight.advance(DT);
            a += gap(ca, calm.target());
            b += gap(cb, tight.target());
        }
        assert!(b > a * 1.3, "{a} {b}");
    }

    #[test]
    fn a_client_cannot_steer_a_bot_or_finish_a_closed_check() {
        let mut s = SlipCheck::new(1, Some(0.5));
        let before = s.cursor;
        s.input(SlipInput::Aim { angle: before + 1. });
        assert_eq!(s.cursor, before);
        let mut s = SlipCheck::new(1, None);
        s.outcome = Some(Outcome::Held);
        s.input(SlipInput::Flick { dir: 1 });
        assert_eq!(s.mistakes, 0);
    }
}
