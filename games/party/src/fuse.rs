//! Overcooked Fuse: everyone sits round a table while a lit cannonball goes
//! from hand to hand. Holding it pays, and pays faster the longer the fuse has
//! burned, but nobody knows how long the fuse is. When it blows, whoever owns
//! the ball loses everything they earned on that fuse. Throws have a visible
//! wind-up that can be faked, and a target can slap an incoming ball back if
//! they time it, so the table is a game of reading and bluffing each other.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Press, Skill};

pub const KEY: &str = "fuse";
pub const FUSES: u32 = 5;
pub const FUSE_MIN: f32 = 5.0;
pub const FUSE_MAX: f32 = 14.0;
pub const OPENING: f32 = 1.5;
pub const RELIGHT: f32 = 2.5;
pub const WINDUP: f32 = 0.3;
pub const FLIGHT: f32 = 0.5;
/// Only the last moment of a flight can be slapped back, so it takes timing.
pub const SWAT_WINDOW: f32 = 0.18;
pub const WHIFF_FLINCH: f32 = 1.2;
/// A caught ball has to be held this long before it can go again.
pub const CATCH_LOCK: f32 = 0.5;
/// Nobody can sit on it forever: past this the holder fumbles it to a neighbour.
pub const HOT_HANDS: f32 = 6.0;
pub const PAY_RATE: f32 = 10.0;
/// Seconds of burn that double the pay rate.
pub const PAY_RAMP: f32 = 5.0;
pub const LIMIT: f32 = 110.0;
pub const TABLE_RADIUS: f32 = 2.6;
pub const SEAT_RADIUS: f32 = 3.7;

pub fn swat_window(width: f32) -> f32 {
    SWAT_WINDOW * BodyProfile::new(width).reach.powi(4)
}

fn whiff_recovery(width: f32) -> f32 {
    WHIFF_FLINCH / BodyProfile::new(width).speed.powi(3)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ball {
    /// Between fuses.
    Out,
    Held {
        seat: u8,
        held: f32,
        lock: f32,
    },
    Windup {
        seat: u8,
        to: u8,
        t: f32,
        real: bool,
        held: f32,
    },
    Flight {
        from: u8,
        to: u8,
        t: f32,
        swatted: bool,
    },
}

impl Ball {
    /// Who loses if it blows now. A ball in the air already belongs to its target.
    pub fn owner(&self) -> Option<u8> {
        match *self {
            Ball::Out => None,
            Ball::Held { seat, .. } | Ball::Windup { seat, .. } => Some(seat),
            Ball::Flight { to, .. } => Some(to),
        }
    }

    fn holder(&self) -> Option<u8> {
        match *self {
            Ball::Held { seat, .. } | Ball::Windup { seat, .. } => Some(seat),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Seat {
    pub seat: u8,
    pub body_width: f32,
    pub angle: f32,
    pub x: f32,
    pub z: f32,
    pub score: f32,
    pub pending: f32,
    pub booms: u32,
    pub swats: u32,
    pub flinch: f32,
    pub aim: Option<u8>,
    ally: Option<u8>,
    swat_held: bool,
}

#[derive(Clone, Debug, Default)]
struct Mind {
    nerve: f32,
    bluff: f32,
    /// Perceived spark level, lagging the real one by the bot's reaction.
    spark: f32,
    target: Option<u8>,
    decoy: Option<u8>,
    feints: u8,
    jitter: f32,
    /// Weighs pay against risk this possession rather than going on nerve.
    smart: bool,
    planned_for: Option<u32>,
    swat_in: Option<f32>,
    swat_for: Option<u32>,
    pause: f32,
}

pub struct OvercookedFuse {
    pub seats: Vec<Seat>,
    pub ball: Ball,
    pub time: f32,
    pub fuse: u32,
    pub burn: f32,
    length: f32,
    wait: f32,
    /// Bumped on every wind-up, throw and catch so bots notice a fresh one.
    pub throw_id: u32,
    spark_noise: f32,
    spark_goal: f32,
    spark_shift: f32,
    flare: f32,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    minds: HashMap<u8, Mind>,
    placements: Option<Vec<Vec<u8>>>,
}

fn angle_gap(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d.min(std::f32::consts::TAU - d)
}

impl OvercookedFuse {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let scale = crate::bout::crowd_scale(contestants.len());
        let radius = SEAT_RADIUS * scale;
        let count = contestants.len().max(1);
        let mut rng = StdRng::seed_from_u64(seed);
        let seats = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle =
                    std::f32::consts::TAU * i as f32 / count as f32 + std::f32::consts::FRAC_PI_2;
                Seat {
                    seat: c.seat,
                    body_width: BodyProfile::new(c.body_width).width,
                    angle,
                    x: angle.cos() * radius,
                    z: angle.sin() * radius,
                    score: 0.0,
                    pending: 0.0,
                    booms: 0,
                    swats: 0,
                    flinch: 0.0,
                    aim: None,
                    ally: c.ally,
                    swat_held: false,
                }
            })
            .collect();
        let minds = contestants
            .iter()
            .filter(|c| c.is_bot)
            .map(|c| {
                (
                    c.seat,
                    Mind {
                        nerve: rng.random_range(0.15..0.95),
                        bluff: rng.random_range(0.0..1.0),
                        ..Default::default()
                    },
                )
            })
            .collect();
        OvercookedFuse {
            seats,
            ball: Ball::Out,
            time: 0.0,
            fuse: 0,
            burn: 0.0,
            length: 0.0,
            wait: OPENING,
            throw_id: 0,
            spark_noise: 0.0,
            spark_goal: 0.0,
            spark_shift: 0.0,
            flare: 0.0,
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            minds,
            placements: None,
        }
    }

    pub fn table_scale(&self) -> f32 {
        crate::bout::crowd_scale(self.seats.len())
    }

    fn index(&self, seat: u8) -> Option<usize> {
        self.seats.iter().position(|s| s.seat == seat)
    }

    pub fn seat(&self, seat: u8) -> Option<&Seat> {
        self.seats.iter().find(|s| s.seat == seat)
    }

    /// The hidden fuse length, for tests and the end-of-fuse reveal only.
    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn set_length(&mut self, length: f32) {
        self.length = length;
    }

    pub fn live(&self) -> bool {
        self.ball != Ball::Out
    }

    fn partners(&self, a: u8, b: u8) -> bool {
        let ally = |s: u8| self.seat(s).and_then(|s| s.ally);
        crate::bout::allied(ally(a), ally(b))
    }

    fn can_target(&self, from: u8, to: u8) -> bool {
        from != to && self.index(to).is_some() && !self.partners(from, to)
    }

    /// The next seat round the table to your left or right as you face the
    /// middle, skipping your partner.
    pub fn neighbour(&self, seat: u8, hand: Press) -> Option<u8> {
        let i = self.index(seat)?;
        let n = self.seats.len();
        // Seats go anticlockwise seen from above, which is leftward for
        // someone facing the middle.
        let step = match hand {
            Press::Left => 1,
            Press::Right => n - 1,
            Press::Both => return None,
        };
        (1..n)
            .map(|k| self.seats[(i + step * k) % n].seat)
            .find(|&to| self.can_target(seat, to))
    }

    /// The seat round the table nearest the given angle, never yourself.
    pub fn seat_at_angle(&self, from: u8, angle: f32) -> Option<u8> {
        if !angle.is_finite() {
            return None;
        }
        self.seats
            .iter()
            .filter(|s| self.can_target(from, s.seat))
            .min_by(|a, b| {
                angle_gap(a.angle, angle)
                    .partial_cmp(&angle_gap(b.angle, angle))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.seat)
    }

    pub fn angle_of(&self, seat: u8) -> f32 {
        self.seat(seat).map(|s| s.angle).unwrap_or(0.0)
    }

    /// What the sparks show: the true burn fraction smeared with drift and the
    /// odd false flare, so they hint without telling.
    pub fn spark(&self) -> f32 {
        if !self.live() || self.length <= 0.0 {
            return 0.0;
        }
        let truth = (self.burn / self.length).clamp(0.0, 1.0);
        (truth * 0.8 + 0.1 + self.spark_noise + self.flare).clamp(0.05, 1.0)
    }

    pub fn pay_rate(&self) -> f32 {
        PAY_RATE * (1.0 + self.burn / PAY_RAMP)
    }

    fn light(&mut self, events: &mut Vec<Value>) {
        let low = self
            .seats
            .iter()
            .map(|s| s.score.round())
            .fold(f32::INFINITY, f32::min);
        let trailing: Vec<u8> = self
            .seats
            .iter()
            .filter(|s| s.score.round() <= low)
            .map(|s| s.seat)
            .collect();
        let Some(&seat) = trailing.get(self.rng.random_range(0..trailing.len().max(1))) else {
            return;
        };
        self.fuse += 1;
        self.burn = 0.0;
        self.length = self.rng.random_range(FUSE_MIN..FUSE_MAX);
        self.spark_noise = 0.0;
        self.spark_goal = 0.0;
        self.flare = 0.0;
        self.ball = Ball::Held {
            seat,
            held: 0.0,
            lock: CATCH_LOCK,
        };
        self.throw_id += 1;
        for s in &mut self.seats {
            s.pending = 0.0;
        }
        events.push(json!({ "kind": "light", "seat": seat, "fuse": self.fuse }));
    }

    fn blow(&mut self, events: &mut Vec<Value>) {
        let Some(owner) = self.ball.owner() else {
            return;
        };
        let mut lost = 0.0;
        for s in &mut self.seats {
            if s.seat == owner {
                lost = s.pending;
                s.booms += 1;
            } else {
                s.score += s.pending;
            }
            s.pending = 0.0;
        }
        events.push(json!({
            "kind": "boom",
            "seat": owner,
            "lost": lost.round(),
            "fuse": self.fuse,
            "length": (self.length * 10.0).round() / 10.0,
        }));
        self.ball = Ball::Out;
        self.throw_id += 1;
        if self.fuse >= FUSES {
            self.finish();
        } else {
            self.wait = RELIGHT;
        }
    }

    fn finish(&mut self) {
        if self.placements.is_some() {
            return;
        }
        for s in &mut self.seats {
            s.score += s.pending;
            s.pending = 0.0;
        }
        let mut order: Vec<(i64, u8)> = self
            .seats
            .iter()
            .map(|s| (s.score.round() as i64, s.seat))
            .collect();
        order.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (score, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == score => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(score);
        }
        self.placements = Some(groups);
    }

    fn step_sparks(&mut self, dt: f32) {
        self.spark_shift -= dt;
        if self.spark_shift <= 0.0 {
            self.spark_shift = self.rng.random_range(0.3..0.7);
            self.spark_goal = self.rng.random_range(-0.22..0.22);
            if self.rng.random_bool(0.12) {
                self.flare = self.rng.random_range(0.25..0.5);
            }
        }
        self.spark_noise += (self.spark_goal - self.spark_noise) * (dt * 4.0).min(1.0);
        self.flare = (self.flare - dt * 0.7).max(0.0);
    }

    /// The holder's choices this tick: a neighbour tap, an aimed throw, or a feint.
    fn holder_act(&mut self, seat: u8, input: &Input, events: &mut Vec<Value>) {
        let Ball::Held { held, lock, .. } = self.ball else {
            return;
        };
        if lock > 0.0 {
            return;
        }
        let (to, real) = match input.press {
            Some(hand @ (Press::Left | Press::Right)) => (self.neighbour(seat, hand), true),
            Some(Press::Both) => (self.seat_at_angle(seat, input.yaw), false),
            None if input.fire => (self.seat_at_angle(seat, input.yaw), true),
            None => return,
        };
        let Some(to) = to else { return };
        self.ball = Ball::Windup {
            seat,
            to,
            t: 0.0,
            real,
            held,
        };
        self.throw_id += 1;
        if let Some(i) = self.index(seat) {
            self.seats[i].aim = Some(to);
        }
        events.push(json!({ "kind": "windup", "seat": seat, "to": to }));
    }

    fn try_swat(&mut self, seat: u8, events: &mut Vec<Value>) {
        let Some(i) = self.index(seat) else { return };
        if self.seats[i].flinch > 0.0 || self.ball.holder() == Some(seat) {
            return;
        }
        match self.ball {
            Ball::Flight {
                from,
                to,
                t,
                swatted: false,
            } if to == seat && t >= FLIGHT - swat_window(self.seats[i].body_width) => {
                self.ball = Ball::Flight {
                    from: seat,
                    to: from,
                    t: 0.0,
                    swatted: true,
                };
                self.throw_id += 1;
                self.seats[i].swats += 1;
                events.push(json!({ "kind": "swat", "seat": seat, "to": from }));
            }
            _ => {
                self.seats[i].flinch = whiff_recovery(self.seats[i].body_width);
                events.push(json!({ "kind": "whiff", "seat": seat }));
            }
        }
    }

    fn step_ball(&mut self, dt: f32, events: &mut Vec<Value>) {
        match self.ball {
            Ball::Out => {}
            Ball::Held { seat, held, lock } => {
                let held = held + dt;
                self.ball = Ball::Held {
                    seat,
                    held,
                    lock: (lock - dt).max(0.0),
                };
                if held >= HOT_HANDS {
                    let hand = if self.rng.random_bool(0.5) {
                        Press::Left
                    } else {
                        Press::Right
                    };
                    if let Some(to) = self.neighbour(seat, hand) {
                        self.ball = Ball::Flight {
                            from: seat,
                            to,
                            t: 0.0,
                            swatted: false,
                        };
                        self.throw_id += 1;
                        events.push(json!({ "kind": "fumble", "seat": seat, "to": to }));
                    }
                }
            }
            Ball::Windup {
                seat,
                to,
                t,
                real,
                held,
            } => {
                let t = t + dt;
                if t < WINDUP {
                    self.ball = Ball::Windup {
                        seat,
                        to,
                        t,
                        real,
                        held: held + dt,
                    };
                } else if real {
                    self.ball = Ball::Flight {
                        from: seat,
                        to,
                        t: 0.0,
                        swatted: false,
                    };
                    self.throw_id += 1;
                    events.push(json!({ "kind": "throw", "seat": seat, "to": to }));
                } else {
                    self.ball = Ball::Held {
                        seat,
                        held: held + dt,
                        lock: 0.0,
                    };
                    self.throw_id += 1;
                    events.push(json!({ "kind": "feint", "seat": seat, "to": to }));
                }
            }
            Ball::Flight {
                from,
                to,
                t,
                swatted,
            } => {
                let t = t + dt;
                if t < FLIGHT {
                    self.ball = Ball::Flight {
                        from,
                        to,
                        t,
                        swatted,
                    };
                } else {
                    self.ball = Ball::Held {
                        seat: to,
                        held: 0.0,
                        lock: CATCH_LOCK,
                    };
                    self.throw_id += 1;
                    events.push(json!({ "kind": "catch", "seat": to, "from": from }));
                }
            }
        }
    }

    fn mind_skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    fn leader_except(&self, seat: u8) -> Option<u8> {
        self.seats
            .iter()
            .filter(|s| self.can_target(seat, s.seat))
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.seat)
    }

    /// A bot that just got the ball picks who it means to hit, who it might
    /// pretend to throw at, how long it dares to keep it, and how many fakes.
    fn plan_hold(&mut self, seat: u8) {
        let skill = self.mind_skill(seat).level();
        let targets: Vec<u8> = self
            .seats
            .iter()
            .filter(|s| self.can_target(seat, s.seat))
            .map(|s| s.seat)
            .collect();
        if targets.is_empty() {
            return;
        }
        let flinched: Vec<u8> = targets
            .iter()
            .copied()
            .filter(|t| self.seat(*t).is_some_and(|s| s.flinch > 0.3))
            .collect();
        let leader = self.leader_except(seat);
        let roll: f32 = self.rng.random();
        let target = if !flinched.is_empty() && roll < skill * 0.8 {
            flinched[self.rng.random_range(0..flinched.len())]
        } else if roll < skill * 0.6 {
            leader.unwrap_or(targets[0])
        } else {
            targets[self.rng.random_range(0..targets.len())]
        };
        let decoy = targets[self.rng.random_range(0..targets.len())];
        let jitter = self.rng.random_range(0.75..1.3);
        let my_score = self.seat(seat).map(|s| s.score).unwrap_or(0.0);
        let best = self.seats.iter().map(|s| s.score).fold(0.0, f32::max);
        let feint_roll: f32 = self.rng.random();
        let smart = self.rng.random::<f32>() < skill * 0.9 + 0.05;
        let Some(mind) = self.minds.get_mut(&seat) else {
            return;
        };
        mind.target = Some(target);
        mind.smart = smart;
        mind.decoy = Some(decoy);
        // Trailing bots get braver.
        mind.jitter = jitter * if best - my_score > 60.0 { 1.25 } else { 1.0 };
        mind.feints = if feint_roll < mind.bluff * 0.7 {
            1 + u8::from(feint_roll < mind.bluff * 0.2)
        } else {
            0
        };
        mind.pause = 0.0;
    }

    /// How long this bot is willing to keep the ball right now, given what the
    /// clock and the sparks seem to say.
    /// How worried this bot is that the fuse is about to go, from the
    /// clock and the sparks as it reads them.
    fn danger(&self, seat: u8) -> f32 {
        let Some(mind) = self.minds.get(&seat) else {
            return 0.0;
        };
        let skill = self.mind_skill(seat).level();
        let overdue =
            ((self.burn - FUSE_MIN * 0.75) / (FUSE_MAX - FUSE_MIN * 0.75)).clamp(0.0, 1.0);
        let sparky = ((mind.spark - 0.45) * 1.6).clamp(0.0, 1.0);
        // Sharp bots trust the clock more than the sparks.
        (overdue * (0.5 + skill * 0.4) + sparky * (0.6 - skill * 0.3)).clamp(0.0, 1.0)
    }

    fn wants_to_pass(&self, seat: u8, held: f32) -> bool {
        let Some(mind) = self.minds.get(&seat) else {
            return true;
        };
        if mind.smart {
            let pending = self.seat(seat).map(|s| s.pending).unwrap_or(0.0);
            let hazard = if self.burn < FUSE_MIN * 0.9 {
                0.01
            } else {
                1.0 / (FUSE_MAX - self.burn).max(0.4)
            };
            let fumbling = held > HOT_HANDS - 0.7 * mind.jitter;
            fumbling || pending * hazard > self.pay_rate() * (0.4 + mind.nerve) * mind.jitter
        } else {
            held >= (0.35 + mind.nerve * 3.0) * mind.jitter * (1.0 - self.danger(seat)).max(0.06)
        }
    }

    /// Early on a catch is income, so a thinking bot only slaps back once the
    /// fuse is getting old.
    fn bothers_to_swat(&mut self, seat: u8, skill: Skill) -> bool {
        let smart = self.rng.random::<f32>() < skill.level();
        let worth = !smart || self.danger(seat) > 0.25 || self.burn > FUSE_MIN;
        worth && self.rng.random::<f32>() < 0.25 + 0.5 * skill.level()
    }

    fn swat_noise(&mut self, skill: Skill) -> f32 {
        let sigma = 0.07 + 0.2 * (1.0 - skill.level());
        self.rng.random_range(-1.0..1.0) * sigma
    }

    fn bot_hold(&mut self, seat: u8, held: f32, lock: f32, dt: f32, input: &mut Input) {
        let throw_id = self.throw_id;
        if let Some(mind) = self.minds.get_mut(&seat) {
            mind.swat_in = None;
        }
        if self.minds.get(&seat).and_then(|m| m.planned_for) != Some(throw_id) {
            if let Some(mind) = self.minds.get_mut(&seat) {
                mind.planned_for = Some(throw_id);
            }
            // A feint ends in a fresh hold, which keeps the plan made at the catch.
            if held < 0.2 {
                self.plan_hold(seat);
            }
        }
        let skill = self.mind_skill(seat);
        let angle = |game: &Self, s: u8| game.angle_of(s);
        let Some(mind) = self.minds.get(&seat).cloned() else {
            return;
        };
        let Some(target) = mind.target else { return };
        let decoy = mind.decoy.unwrap_or(target);
        input.yaw = angle(self, if mind.bluff > 0.5 { decoy } else { target });
        let pause = mind.pause - dt;
        if let Some(m) = self.minds.get_mut(&seat) {
            m.pause = pause;
        }
        if lock > 0.0 || pause > 0.0 || !self.wants_to_pass(seat, held) {
            return;
        }
        if mind.feints > 0 {
            let wait = WINDUP + self.rng.random_range(0.2..0.7);
            if let Some(m) = self.minds.get_mut(&seat) {
                m.feints -= 1;
                m.pause = wait;
            }
            input.press = Some(Press::Both);
            input.yaw = angle(self, decoy);
            return;
        }
        let open = self
            .seats
            .iter()
            .filter(|s| s.flinch > 0.25 && self.can_target(seat, s.seat))
            .map(|s| s.seat)
            .next();
        let target = match open {
            Some(open) if mind.smart => open,
            _ => target,
        };
        if self.neighbour(seat, Press::Left) == Some(target) && self.rng.random_bool(0.6) {
            input.press = Some(Press::Left);
        } else if self.neighbour(seat, Press::Right) == Some(target) && self.rng.random_bool(0.6) {
            input.press = Some(Press::Right);
        } else {
            // A sloppy throw can land a seat over.
            let wobble = self.rng.random_range(-1.0..1.0) * skill.aim_error() * 0.6;
            input.fire = true;
            input.yaw = angle(self, target) + wobble;
        }
    }
}

impl Bout for OvercookedFuse {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        let scale = self.table_scale();
        json!({
            "game": KEY,
            "rule": "Hold the lit cannonball to earn. Pass before it blows. Broad builds have a wider swat window; small builds recover faster from a miss.",
            "radius": SEAT_RADIUS * scale + 3.0,
            "table_radius": TABLE_RADIUS * scale,
            "seat_radius": SEAT_RADIUS * scale,
            "fuses": FUSES,
            "windup": WINDUP,
            "flight": FLIGHT,
            "swat_window": SWAT_WINDOW,
            "seats": self.seats.iter().map(|s| json!({ "seat": s.seat, "angle": s.angle, "x": s.x, "z": s.z })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        for s in &mut self.seats {
            s.flinch = (s.flinch - dt).max(0.0);
        }

        for (seat, input) in inputs {
            let Some(i) = self.index(*seat) else { continue };
            let pressed = input.jump && !self.seats[i].swat_held;
            self.seats[i].swat_held = input.jump;
            if self.ball.holder() == Some(*seat) {
                if matches!(self.ball, Ball::Held { .. }) {
                    self.seats[i].aim = self.seat_at_angle(*seat, input.yaw);
                }
            } else {
                self.seats[i].aim = None;
            }
            if pressed && self.live() {
                self.try_swat(*seat, events);
            }
        }
        if let Some(holder) = self.ball.holder() {
            if let Some((_, input)) = inputs.iter().find(|(s, _)| *s == holder) {
                let input = *input;
                self.holder_act(holder, &input, events);
            }
        }

        if self.live() {
            self.burn += dt;
            self.step_sparks(dt);
            if let Some(holder) = self.ball.holder() {
                let pay = self.pay_rate() * dt;
                if let Some(i) = self.index(holder) {
                    self.seats[i].pending += pay;
                }
            }
            self.step_ball(dt, events);
            if self.burn >= self.length {
                self.blow(events);
            }
        } else {
            self.wait -= dt;
            if self.wait <= 0.0 {
                self.light(events);
            }
        }
        if self.time >= LIMIT {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let skill = self.mind_skill(seat);
        let spark = self.spark();
        let throw_id = self.throw_id;
        let seat_swat_window = self
            .seat(seat)
            .map_or(SWAT_WINDOW, |s| swat_window(s.body_width));
        let mut input = Input::default();
        {
            let Some(mind) = self.minds.get_mut(&seat) else {
                return input;
            };
            mind.spark += (spark - mind.spark) * (dt / skill.reaction()).min(1.0);
            if let Some(left) = mind.swat_in.as_mut() {
                *left -= dt;
                if *left <= 0.0 {
                    mind.swat_in = None;
                    input.jump = true;
                }
            }
        }
        match self.ball {
            Ball::Held {
                seat: holder,
                held,
                lock,
            } if holder == seat => {
                self.bot_hold(seat, held, lock, dt, &mut input);
            }
            Ball::Windup {
                seat: from, to, t, ..
            } if to == seat && from != seat => {
                let seen = self.minds.get(&seat).and_then(|m| m.swat_for) == Some(throw_id);
                if !seen && t >= skill.reaction() * 0.3 {
                    let fooled = 0.15 + (1.0 - skill.level()) * 0.5;
                    let commit =
                        self.rng.random::<f32>() < fooled && self.bothers_to_swat(seat, skill);
                    let noise = self.swat_noise(skill);
                    if let Some(mind) = self.minds.get_mut(&seat) {
                        mind.swat_for = Some(throw_id);
                        if commit {
                            mind.swat_in =
                                Some((WINDUP - t) + FLIGHT - seat_swat_window * 0.5 + noise);
                        }
                    }
                }
            }
            Ball::Flight {
                to,
                t,
                swatted: false,
                ..
            } if to == seat => {
                let seen = self.minds.get(&seat).and_then(|m| m.swat_for) == Some(throw_id);
                if !seen && t >= skill.reaction() * 0.3 {
                    let commit = self.bothers_to_swat(seat, skill);
                    let noise = self.swat_noise(skill);
                    if let Some(mind) = self.minds.get_mut(&seat) {
                        mind.swat_for = Some(throw_id);
                        if commit && mind.swat_in.is_none() {
                            mind.swat_in =
                                Some((FLIGHT - seat_swat_window * 0.5 - t + noise).max(0.0));
                        }
                    }
                }
            }
            _ => {}
        }
        input
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        let holder = self.ball.holder();
        let ball = match self.ball {
            Ball::Out => json!({ "state": "out" }),
            Ball::Held { seat, held, lock } => json!({
                "state": "held", "seat": seat, "held": round(held), "locked": lock > 0.0,
                "heat": round((held / HOT_HANDS).min(1.0)),
            }),
            Ball::Windup {
                seat, to, t, held, ..
            } => json!({
                "state": "windup", "seat": seat, "to": to, "t": round(t / WINDUP),
                "heat": round((held / HOT_HANDS).min(1.0)),
            }),
            Ball::Flight {
                from,
                to,
                t,
                swatted,
            } => json!({
                "state": "flight", "from": from, "to": to, "t": round(t / FLIGHT), "swatted": swatted,
            }),
        };
        let heat = match self.ball {
            Ball::Held { held, .. } | Ball::Windup { held, .. } => (held / HOT_HANDS).min(1.0),
            _ => 0.0,
        };
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": LIMIT,
            "fuse": self.fuse,
            "fuses": FUSES,
            "lit": self.live(),
            "burn": round(if self.live() { self.burn } else { 0.0 }),
            "spark": round(self.spark()),
            "rate": round(if self.live() { self.pay_rate() } else { 0.0 }),
            "next": round(if self.live() { 0.0 } else { self.wait.max(0.0) }),
            "ball": ball,
            "players": self.seats.iter().map(|s| {
                let facing = match s.aim.filter(|_| holder == Some(s.seat)).and_then(|a| self.seat(a)) {
                    Some(t) => (t.x - s.x).atan2(t.z - s.z),
                    None => (-s.x).atan2(-s.z),
                };
                json!({
                    "seat": s.seat,
                    "body_width": s.body_width,
                    "x": round(s.x),
                    "z": round(s.z),
                    "yaw": round(facing),
                    "angle": round(s.angle),
                    "score": s.score.round(),
                    "pending": s.pending.round(),
                    "booms": s.booms,
                    "swats": s.swats,
                    "flinch": s.flinch > 0.0,
                    "holding": holder == Some(s.seat),
                    "heat": round(if holder == Some(s.seat) { heat } else { 0.0 }),
                    "aim": s.aim.filter(|_| holder == Some(s.seat)),
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broad_seat_swats_earlier_and_small_seat_recovers_from_a_whiff_sooner() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut game = OvercookedFuse::new(&[small, broad], 3);
        let mut events = Vec::new();
        game.ball = Ball::Flight {
            from: 0,
            to: 1,
            t: FLIGHT - 0.19,
            swatted: false,
        };
        game.try_swat(1, &mut events);
        assert_eq!(game.seat(1).unwrap().swats, 1);
        assert!(events.iter().any(|e| e["kind"] == "swat"));

        game.ball = Ball::Flight {
            from: 1,
            to: 0,
            t: FLIGHT - 0.19,
            swatted: false,
        };
        game.try_swat(0, &mut events);
        assert_eq!(game.seat(0).unwrap().swats, 0);
        let small_recovery = game.seat(0).unwrap().flinch;
        game.ball = Ball::Out;
        game.try_swat(1, &mut events);
        assert!(small_recovery < game.seat(1).unwrap().flinch);
        let width = game.snapshot()["players"][0]["body_width"].as_f64().unwrap();
        assert!((width - 0.7).abs() < 1e-5);
    }
    use crate::bout::{Contestant, TICK_DT};

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn idle(game: &OvercookedFuse) -> Vec<(u8, Input)> {
        game.seats
            .iter()
            .map(|s| (s.seat, Input::default()))
            .collect()
    }

    fn run(game: &mut OvercookedFuse, seconds: f32, events: &mut Vec<Value>) {
        for _ in 0..(seconds / TICK_DT).round() as usize {
            let inputs = idle(game);
            game.step(&inputs, TICK_DT, events);
        }
    }

    fn lit(count: u8) -> (OvercookedFuse, u8) {
        let mut game = OvercookedFuse::new(&people(count), 7);
        let mut events = Vec::new();
        run(&mut game, OPENING + 0.05, &mut events);
        game.set_length(60.0);
        let holder = game.ball.holder().expect("first fuse lit");
        (game, holder)
    }

    fn act(game: &mut OvercookedFuse, seat: u8, input: Input, events: &mut Vec<Value>) {
        let mut inputs = idle(game);
        for (s, i) in inputs.iter_mut() {
            if *s == seat {
                *i = input;
            }
        }
        game.step(&inputs, TICK_DT, events);
    }

    fn press(hand: Press) -> Input {
        Input {
            press: Some(hand),
            ..Default::default()
        }
    }

    #[test]
    fn left_and_right_are_the_neighbours_as_you_face_the_middle() {
        let game = OvercookedFuse::new(&people(4), 1);
        for s in &game.seats {
            let facing = (-s.x, -s.z);
            // Right of someone facing (fx, fz) in a y-up world is (-fz, fx).
            let right = (-facing.1, facing.0);
            let side = |to: u8| {
                let t = game.seat(to).unwrap();
                (t.x - s.x) * right.0 + (t.z - s.z) * right.1
            };
            assert!(side(game.neighbour(s.seat, Press::Right).unwrap()) > 0.0);
            assert!(side(game.neighbour(s.seat, Press::Left).unwrap()) < 0.0);
        }
    }

    #[test]
    fn the_first_fuse_lights_after_the_opening_and_pays_the_holder() {
        let (mut game, holder) = lit(4);
        let mut events = Vec::new();
        run(&mut game, 2.0, &mut events);
        let pay = game.seat(holder).unwrap().pending;
        assert!(
            pay > 2.0 * PAY_RATE,
            "holding pays at least the base rate, got {pay}"
        );
        assert!(
            game.seats
                .iter()
                .filter(|s| s.seat != holder)
                .all(|s| s.pending == 0.0)
        );
    }

    #[test]
    fn a_fresh_catch_cannot_be_thrown_straight_back() {
        let (mut game, holder) = lit(4);
        let mut events = Vec::new();
        act(&mut game, holder, press(Press::Left), &mut events);
        assert!(
            matches!(game.ball, Ball::Held { .. }),
            "the opening lock holds"
        );
        run(&mut game, CATCH_LOCK, &mut events);
        act(&mut game, holder, press(Press::Left), &mut events);
        assert!(matches!(game.ball, Ball::Windup { real: true, .. }));
        let to = game.neighbour(holder, Press::Left).unwrap();
        run(&mut game, WINDUP + FLIGHT + 0.05, &mut events);
        assert_eq!(game.ball.holder(), Some(to));
        act(&mut game, to, press(Press::Right), &mut events);
        assert_eq!(game.ball.holder(), Some(to), "locked after the catch");
        assert!(
            events
                .iter()
                .any(|e| e["kind"] == "catch" && e["seat"] == to)
        );
    }

    #[test]
    fn an_aimed_throw_goes_to_the_seat_nearest_the_aim() {
        let (mut game, holder) = lit(6);
        run(&mut game, CATCH_LOCK, &mut Vec::new());
        let across = game
            .seats
            .iter()
            .max_by(|a, b| {
                angle_gap(a.angle, game.angle_of(holder))
                    .partial_cmp(&angle_gap(b.angle, game.angle_of(holder)))
                    .unwrap()
            })
            .unwrap()
            .seat;
        let mut events = Vec::new();
        let aim = game.angle_of(across) + 0.2;
        act(
            &mut game,
            holder,
            Input {
                fire: true,
                yaw: aim,
                ..Default::default()
            },
            &mut events,
        );
        assert!(matches!(game.ball, Ball::Windup { to, real: true, .. } if to == across));
        assert_eq!(
            game.seat_at_angle(holder, game.angle_of(holder))
                .map(|s| s == holder),
            Some(false),
            "never yourself"
        );
        assert_eq!(
            game.seat_at_angle(holder, f32::NAN),
            None,
            "garbage aim is ignored"
        );
    }

    #[test]
    fn a_feint_looks_like_a_throw_and_keeps_the_ball() {
        let (mut game, holder) = lit(4);
        run(&mut game, CATCH_LOCK, &mut Vec::new());
        let mut events = Vec::new();
        let to = game.neighbour(holder, Press::Right).unwrap();
        let yaw = game.angle_of(to);
        act(
            &mut game,
            holder,
            Input {
                press: Some(Press::Both),
                yaw,
                ..Default::default()
            },
            &mut events,
        );
        let shown = game.snapshot()["ball"].clone();
        assert_eq!(shown["state"], "windup");
        assert!(
            shown.get("real").is_none(),
            "nobody can tell a feint from the snapshot"
        );
        run(&mut game, WINDUP + 0.05, &mut events);
        assert_eq!(game.ball.holder(), Some(holder));
        assert!(events.iter().any(|e| e["kind"] == "feint"));
        assert!(!events.iter().any(|e| e["kind"] == "throw"));
    }

    fn throw_at_neighbour(game: &mut OvercookedFuse, holder: u8) -> u8 {
        run(game, CATCH_LOCK, &mut Vec::new());
        let to = game.neighbour(holder, Press::Left).unwrap();
        act(game, holder, press(Press::Left), &mut Vec::new());
        run(game, WINDUP + 0.02, &mut Vec::new());
        assert!(matches!(game.ball, Ball::Flight { .. }));
        to
    }

    #[test]
    fn a_well_timed_slap_sends_it_back_and_cannot_be_slapped_again() {
        let (mut game, holder) = lit(4);
        let to = throw_at_neighbour(&mut game, holder);
        run(&mut game, FLIGHT - SWAT_WINDOW * 0.5, &mut Vec::new());
        let mut events = Vec::new();
        act(
            &mut game,
            to,
            Input {
                jump: true,
                ..Default::default()
            },
            &mut events,
        );
        assert!(
            matches!(game.ball, Ball::Flight { from, to: back, swatted: true, .. } if from == to && back == holder)
        );
        assert_eq!(game.seat(to).unwrap().swats, 1);
        run(&mut game, FLIGHT - SWAT_WINDOW * 0.5, &mut events);
        act(
            &mut game,
            holder,
            Input {
                jump: true,
                ..Default::default()
            },
            &mut events,
        );
        run(&mut game, 0.2, &mut events);
        assert_eq!(
            game.ball.holder(),
            Some(holder),
            "a slapped ball comes home"
        );
        assert!(
            events
                .iter()
                .any(|e| e["kind"] == "whiff" && e["seat"] == holder)
        );
    }

    #[test]
    fn slapping_early_whiffs_and_flinches() {
        let (mut game, holder) = lit(4);
        let to = throw_at_neighbour(&mut game, holder);
        let mut events = Vec::new();
        act(
            &mut game,
            to,
            Input {
                jump: true,
                ..Default::default()
            },
            &mut events,
        );
        assert!(events.iter().any(|e| e["kind"] == "whiff"));
        assert!(game.seat(to).unwrap().flinch > 0.0);
        // Still holding the key is not a fresh press, and flinching blocks one anyway.
        run(&mut game, FLIGHT - SWAT_WINDOW * 0.5, &mut Vec::new());
        act(
            &mut game,
            to,
            Input {
                jump: true,
                ..Default::default()
            },
            &mut Vec::new(),
        );
        run(&mut game, 0.2, &mut Vec::new());
        assert_eq!(game.ball.holder(), Some(to));
    }

    #[test]
    fn the_blast_takes_the_owners_fuse_points_and_banks_everyone_elses() {
        let (mut game, holder) = lit(4);
        run(&mut game, 2.0, &mut Vec::new());
        let to = throw_at_neighbour(&mut game, holder);
        run(&mut game, FLIGHT + 1.0, &mut Vec::new());
        let earned = game.seat(holder).unwrap().pending;
        assert!(earned > 0.0);
        game.set_length(game.burn + 0.01);
        let mut events = Vec::new();
        run(&mut game, 0.05, &mut events);
        let boom = events.iter().find(|e| e["kind"] == "boom").expect("boom");
        assert_eq!(boom["seat"], to);
        assert_eq!(game.seat(holder).unwrap().score, earned);
        assert_eq!(game.seat(to).unwrap().score, 0.0);
        assert_eq!(game.seat(to).unwrap().booms, 1);
        assert!(!game.live());
    }

    #[test]
    fn a_ball_in_the_air_blows_up_on_its_target() {
        let (mut game, holder) = lit(3);
        let to = throw_at_neighbour(&mut game, holder);
        game.set_length(game.burn + 0.05);
        let mut events = Vec::new();
        run(&mut game, 0.1, &mut events);
        assert_eq!(
            events.iter().find(|e| e["kind"] == "boom").unwrap()["seat"],
            to
        );
    }

    #[test]
    fn hot_hands_fumble_it_away_so_a_leaver_cannot_stall() {
        let (mut game, holder) = lit(5);
        let mut events = Vec::new();
        run(&mut game, HOT_HANDS + 0.1, &mut events);
        let fumble = events
            .iter()
            .find(|e| e["kind"] == "fumble")
            .expect("fumbled");
        assert_eq!(fumble["seat"], holder);
        let to = fumble["to"].as_u64().unwrap() as u8;
        assert!(
            game.neighbour(holder, Press::Left) == Some(to)
                || game.neighbour(holder, Press::Right) == Some(to)
        );
    }

    #[test]
    fn five_fuses_end_the_bout_with_ties_grouped() {
        let mut game = OvercookedFuse::new(&people(4), 3);
        let mut events = Vec::new();
        for _ in 0..(LIMIT / TICK_DT) as usize {
            let inputs = idle(&game);
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        let placements = game.placements().expect("finished");
        assert_eq!(
            events.iter().filter(|e| e["kind"] == "boom").count() as u32,
            FUSES
        );
        assert!(game.time < 95.0, "done in {}", game.time);
        let seated: usize = placements.iter().map(Vec::len).sum();
        assert_eq!(seated, 4);
        for pair in placements.windows(2) {
            assert!(game.seat(pair[0][0]).unwrap().score > game.seat(pair[1][0]).unwrap().score);
        }
        for group in &placements {
            let first = game.seat(group[0]).unwrap().score.round();
            assert!(
                group
                    .iter()
                    .all(|s| game.seat(*s).unwrap().score.round() == first)
            );
        }
    }

    #[test]
    fn the_trailing_player_gets_the_next_fuse() {
        let (mut game, holder) = lit(3);
        for s in &mut game.seats {
            s.score = if s.seat == holder { 10.0 } else { 50.0 };
        }
        game.seats
            .iter_mut()
            .find(|s| s.seat == holder)
            .unwrap()
            .score = 50.0;
        let trailing = game.seats.iter().find(|s| s.seat != holder).unwrap().seat;
        game.seats
            .iter_mut()
            .find(|s| s.seat == trailing)
            .unwrap()
            .score = 5.0;
        game.set_length(game.burn + 0.01);
        let mut events = Vec::new();
        run(&mut game, RELIGHT + 0.2, &mut events);
        assert_eq!(game.ball.holder(), Some(trailing));
    }

    #[test]
    fn partners_never_get_thrown_to() {
        let mut field = people(4);
        field[0].ally = Some(1);
        field[1].ally = Some(1);
        let game = OvercookedFuse::new(&field, 2);
        let partner = 1;
        assert_ne!(game.neighbour(0, Press::Left), Some(partner));
        assert_ne!(game.neighbour(0, Press::Right), Some(partner));
        assert_ne!(game.seat_at_angle(0, game.angle_of(partner)), Some(partner));
    }

    #[test]
    fn the_snapshot_never_gives_away_the_fuse_length() {
        let (mut game, _) = lit(4);
        game.set_length(9.876);
        let text = game.snapshot().to_string();
        assert!(!text.contains("9.87") && !text.contains("9.88"));
        assert!(game.snapshot()["spark"].as_f64().unwrap() > 0.0);
    }

    fn bots(count: u8, skill: f32) -> Vec<Contestant> {
        (0..count).map(|s| Contestant::bot(s, skill)).collect()
    }

    fn play(field: &[Contestant], seed: u64) -> (OvercookedFuse, Vec<Value>) {
        let mut game = OvercookedFuse::new(field, seed);
        let mut events = Vec::new();
        for _ in 0..(LIMIT / TICK_DT) as usize + 10 {
            let inputs: Vec<_> = field
                .iter()
                .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        (game, events)
    }

    #[test]
    fn bots_pass_feint_slap_and_finish() {
        let mut kinds: HashMap<String, u32> = HashMap::new();
        for seed in 0..12 {
            let (game, events) = play(&bots(5, 0.5), seed);
            assert!(game.placements().is_some(), "seed {seed} finished");
            assert!(game.time < 100.0);
            for e in events {
                *kinds
                    .entry(e["kind"].as_str().unwrap().to_owned())
                    .or_default() += 1;
            }
        }
        for kind in ["throw", "catch", "feint", "swat", "whiff", "boom"] {
            assert!(
                kinds.get(kind).copied().unwrap_or(0) > 0,
                "bots never did {kind}: {kinds:?}"
            );
        }
        let throws = kinds["throw"] + kinds.get("fumble").copied().unwrap_or(0);
        assert!(
            kinds["swat"] < throws / 2,
            "slaps stay the exception: {kinds:?}"
        );
    }

    #[test]
    fn sharp_bots_beat_clumsy_ones_more_often_than_not() {
        let mut sharp_wins = 0.0;
        let games = 40;
        for seed in 0..games {
            let mut field = bots(4, 0.15);
            let sharp = (seed % 4) as usize;
            field[sharp] = Contestant::bot(sharp as u8, 1.0);
            let (game, _) = play(&field, seed);
            let first = &game.placements().unwrap()[0];
            if first.contains(&(sharp as u8)) {
                sharp_wins += 1.0 / first.len() as f64;
            }
        }
        let share = sharp_wins / games as f64;
        assert!(
            share > 0.25,
            "a sharp bot should beat the 25% baseline, got {share}"
        );
    }

    #[test]
    fn mixed_size_bots_both_win_over_rotated_seats() {
        let mut wins = [0.0f32; 2];
        for seed in 0..64 {
            let mut field = bots(2, 0.5);
            field[(seed as usize) % 2].body_width = 0.7;
            field[(seed as usize + 1) % 2].body_width = 1.4;
            let (game, _) = play(&field, seed);
            let first = &game.placements().unwrap()[0];
            for &seat in first {
                let size = usize::from(field[seat as usize].body_width > 1.0);
                wins[size] += 1.0 / first.len() as f32;
            }
        }
        assert!(wins[0] >= 20.0 && wins[1] >= 20.0, "small/broad wins: {wins:?}");
    }

    #[test]
    #[ignore = "balance report: cargo test -p party --lib fuse::tests::balance_report -- --ignored --nocapture"]
    fn balance_report() {
        for players in [2u8, 4, 8] {
            for sharp_level in [0.4f32, 1.0] {
                let games = 200u64;
                let mut seconds = 0.0;
                let mut sharp_wins = 0.0;
                let mut ties = 0;
                let mut kinds: HashMap<String, u32> = HashMap::new();
                let mut lost = 0.0;
                let mut top = 0.0;
                for seed in 0..games {
                    let mut field = bots(players, 0.4);
                    let sharp = (seed % players as u64) as u8;
                    field[sharp as usize] = Contestant::bot(sharp, sharp_level);
                    let (game, events) = play(&field, seed);
                    seconds += game.time;
                    let order = game.placements().expect("finished");
                    if order[0].len() > 1 {
                        ties += 1;
                    }
                    if order[0].contains(&sharp) {
                        sharp_wins += 1.0 / order[0].len() as f32;
                    }
                    top += game.seat(order[0][0]).unwrap().score;
                    for e in &events {
                        *kinds
                            .entry(e["kind"].as_str().unwrap().to_owned())
                            .or_default() += 1;
                        if e["kind"] == "boom" {
                            lost += e["lost"].as_f64().unwrap() as f32;
                        }
                    }
                }
                let per = |k: &str| kinds.get(k).copied().unwrap_or(0) as f32 / games as f32;
                println!(
                    "{players}p one bot at {sharp_level}: {:.1}s mean, wins {:.0}% (fair {:.0}%), ties {ties}, top score {:.0}, lost per boom {:.0}, per game: throws {:.1} feints {:.1} swats {:.1} whiffs {:.1} fumbles {:.1}",
                    seconds / games as f32,
                    100.0 * sharp_wins / games as f32,
                    100.0 / players as f32,
                    top / games as f32,
                    lost / (games as f32 * FUSES as f32),
                    per("throw"),
                    per("feint"),
                    per("swat"),
                    per("whiff"),
                    per("fumble"),
                );
            }
        }
    }

    #[test]
    fn two_players_still_have_a_game() {
        let (game, events) = play(&bots(2, 0.4), 9);
        assert!(game.placements().is_some());
        assert!(events.iter().any(|e| e["kind"] == "catch"));
    }
}
