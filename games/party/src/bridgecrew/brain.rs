//! How a bot sits in either seat. The driver only knows what the fog lets it
//! see and what its navigator pings, heard a beat late; the navigator sees
//! everything and calls it out a beat late, now and then wrongly.

use std::collections::{HashMap, HashSet};

use rand::{Rng, rngs::StdRng};

use super::course::{Blow, Course, Kind, Span};
use super::{Cart, Crew, MAX_SPEED, Ping, SIGHT, wrap};
use crate::bout::{Input, Skill, Stick};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Topic {
    Fork(usize, u32),
    DeadEnd(usize, u32),
    Hold(usize, u32),
    Release(usize, u32),
    Weak(usize, u32),
    Wind(usize, u32),
    Drift(u32),
    Stalled(u32),
}

#[derive(Default)]
pub struct Brain {
    heard: u32,
    queue: Vec<(f32, Ping)>,
    arrow: Option<(bool, f32)>,
    guesses: HashMap<usize, bool>,
    stop_until: f32,
    careful_until: f32,
    boost_until: f32,
    nudge: (f32, f32),
    backing: bool,
    steer: f32,
    phase: Option<f32>,
    weak_boost: HashMap<usize, bool>,
    think_in: f32,
    pending: Option<(f32, Ping)>,
    said: HashSet<Topic>,
    slow_for: f32,
}

fn sight(skill: Skill) -> f32 {
    SIGHT * (0.55 + 0.45 * skill.level())
}

fn distance(cart: &Cart, point: (f32, f32)) -> f32 {
    (cart.x - point.0).hypot(cart.z - point.1)
}

/// How far left of the span's centre line the cart sits, in the span's frame.
fn left_of(span: &Span, cart: &Cart) -> f32 {
    let (t, _) = span.project(cart.x, cart.z);
    let (px, pz) = span.at(t);
    let h = span.heading();
    (cart.x - px) * h.cos() - (cart.z - pz) * h.sin()
}

impl Brain {
    fn hear(&mut self, crew: &Crew, time: f32, skill: Skill, rng: &mut StdRng) {
        for ping in &crew.pings {
            if ping.id <= self.heard {
                continue;
            }
            self.heard = ping.id;
            if rng.random_bool(skill.hesitation() * 0.2) {
                continue;
            }
            let delay = skill.reaction() * rng.random_range(0.6..1.1);
            self.queue.push((time + delay, ping.kind));
        }
        let mut index = 0;
        while index < self.queue.len() {
            if self.queue[index].0 <= time {
                let (_, kind) = self.queue.remove(index);
                self.obey(kind, time);
            } else {
                index += 1;
            }
        }
    }

    fn obey(&mut self, kind: Ping, time: f32) {
        match kind {
            Ping::Go => {
                self.stop_until = 0.0;
                self.careful_until = 0.0;
            }
            Ping::Stop => {
                self.stop_until = time + 3.0;
                self.boost_until = 0.0;
            }
            Ping::Careful => self.careful_until = time + 2.5,
            Ping::Boost => {
                self.boost_until = time + 1.5;
                self.stop_until = 0.0;
            }
            Ping::Left | Ping::Right => {
                let left = kind == Ping::Left;
                self.arrow = Some((left, time));
                self.nudge = (time + 0.5, if left { 1.0 } else { -1.0 });
            }
        }
    }

    /// The branch to take where a span splits: the one the navigator called,
    /// unless the break in the other is already in sight, else a guess.
    fn choose(
        &mut self,
        course: &Course,
        cart: &Cart,
        from: usize,
        time: f32,
        skill: Skill,
        rng: &mut StdRng,
    ) -> usize {
        let Some(index) = course
            .forks
            .iter()
            .position(|f| course.spans[from].next.contains(&f.good))
        else {
            return course.spans[from].next[0];
        };
        let fork = &course.forks[index];
        if distance(cart, course.spans[fork.dead].b) < sight(skill) {
            return fork.good;
        }
        let left = match self.arrow.filter(|(_, at)| time - at < 6.0) {
            Some((left, _)) => left,
            None => *self
                .guesses
                .entry(index)
                .or_insert_with(|| rng.random_bool(0.5)),
        };
        if left == fork.good_left {
            fork.good
        } else {
            fork.dead
        }
    }

    fn target(
        &mut self,
        course: &Course,
        cart: &Cart,
        mut index: usize,
        mut t: f32,
        mut ahead: f32,
        time: f32,
        skill: Skill,
        rng: &mut StdRng,
    ) -> (f32, f32) {
        for _ in 0..4 {
            let span = &course.spans[index];
            let length = span.length().max(0.01);
            let rest = (1.0 - t) * length;
            if ahead <= rest {
                return span.at(t + ahead / length);
            }
            ahead -= rest;
            index = match span.next.len() {
                0 => return span.b,
                1 => span.next[0],
                _ => self.choose(course, cart, index, time, skill, rng),
            };
            t = 0.0;
        }
        course.spans[index].b
    }

    /// How far ahead a swing plank stands open, if the fog shows one.
    fn open_lift(course: &Course, cart: &Cart, time: f32, skill: Skill) -> Option<f32> {
        course.lifts.iter().find_map(|lift| {
            let span = &course.spans[lift.span];
            let ahead = span.a.1 - cart.z;
            (ahead > -0.2
                && ahead < sight(skill)
                && (cart.x - span.a.0).abs() < 2.5
                && lift.up(time))
            .then_some(ahead)
        })
    }

    pub fn drive(
        &mut self,
        course: &Course,
        crew: &Crew,
        time: f32,
        skill: Skill,
        rng: &mut StdRng,
        dt: f32,
    ) -> Input {
        self.hear(crew, time, skill, rng);
        let cart = &crew.cart;
        if cart.falling.is_some() || cart.finished_at.is_some() {
            self.backing = false;
            self.steer = 0.0;
            return Input::default();
        }
        let level = skill.level();
        let phase = *self
            .phase
            .get_or_insert_with(|| rng.random_range(0.0..std::f32::consts::TAU));
        let current = cart.span.unwrap_or(0).min(course.spans.len() - 1);
        let span = &course.spans[current];
        let (t, _) = span.project(cart.x, cart.z);
        if span.kind == Kind::Dead && distance(cart, span.b) < sight(skill) {
            self.backing = true;
            if let Some(index) = course.forks.iter().position(|f| f.dead == current) {
                self.guesses.insert(index, course.forks[index].good_left);
                self.arrow = None;
            }
        }
        if self.backing {
            if span.kind != Kind::Dead || t < 0.08 {
                self.backing = false;
            } else {
                let err = wrap(span.heading() - cart.heading);
                return Input {
                    stick: Stick {
                        x: (err * 2.0).clamp(-1.0, 1.0),
                        z: -1.0,
                    },
                    ..Default::default()
                };
            }
        }
        let ahead = 1.8 + cart.speed.abs() * 0.35;
        let (mut tx, mut tz) = self.target(course, cart, current, t, ahead, time, skill, rng);
        let h = span.heading();
        let mut shift = (time * 1.3 + phase).sin() * (1.0 - level) * 0.45;
        if time < self.nudge.0 {
            shift += self.nudge.1 * 0.5;
        }
        tx += h.cos() * shift;
        tz -= h.sin() * shift;
        let desired = (tx - cart.x).atan2(tz - cart.z);
        let err = wrap(desired - cart.heading);
        let want = (-err * 2.6).clamp(-1.0, 1.0);
        self.steer += (want - self.steer) * (skill.tracking() * dt).min(1.0);

        let mut pace = MAX_SPEED * skill.pace();
        if err.abs() > 0.45 {
            pace *= 0.6;
        }
        if time < self.careful_until {
            pace *= 0.55;
        }
        if let Some(gap) = Self::open_lift(course, cart, time, skill) {
            pace = if gap < 2.5 + cart.speed * 0.6 {
                0.0
            } else {
                pace * 0.5
            };
        }
        if time < self.stop_until {
            pace = 0.0;
        }
        let creaking = span.kind == Kind::Weak && cart.weak_since[current].is_some();
        let panic = creaking
            && *self
                .weak_boost
                .entry(current)
                .or_insert_with(|| rng.random_bool(level as f64 * 0.8));
        let boost = pace > 0.0 && (time < self.boost_until || panic);
        let throttle = if pace < 0.1 {
            if cart.speed > 0.15 { -1.0 } else { 0.0 }
        } else if cart.speed > pace + 0.6 && !boost {
            -0.5
        } else {
            (pace / MAX_SPEED).clamp(0.2, 1.0)
        };
        Input {
            stick: Stick {
                x: self.steer,
                z: throttle,
            },
            jump: boost,
            ..Default::default()
        }
    }

    fn advice(&self, course: &Course, cart: &Cart, time: f32) -> Option<(Topic, Ping)> {
        let attempt = cart.falls;
        for (index, fork) in course.forks.iter().enumerate() {
            if cart.span == Some(fork.dead) && cart.speed > 0.3 {
                return Some((Topic::DeadEnd(index, attempt), Ping::Stop));
            }
            if cart.checkpoint <= fork.piece
                && cart.z < fork.split.1 + 0.3
                && fork.split.1 - cart.z < 11.0
            {
                let arrow = if fork.good_left {
                    Ping::Left
                } else {
                    Ping::Right
                };
                return Some((Topic::Fork(index, attempt), arrow));
            }
        }
        for (index, lift) in course.lifts.iter().enumerate() {
            let span = &course.spans[lift.span];
            let ahead = span.a.1 - cart.z;
            if ahead > -0.3 && ahead < 9.0 {
                let pace = cart.speed.max(2.5);
                let eta = ahead.max(0.0) / pace;
                let out = eta + (span.length() + 1.2) / pace;
                let blocked = (0..=4).any(|i| lift.up(time + eta + (out - eta) * i as f32 / 4.0));
                let cycle = lift.cycle(time) * 8 + attempt;
                if blocked && cart.speed > 0.4 && ahead > 0.5 {
                    return Some((Topic::Hold(index, cycle), Ping::Stop));
                }
                let (left, up) = lift.until_change(time);
                let clear = if up { left < 0.4 } else { left > 1.8 };
                if cart.speed < 0.6 && clear {
                    return Some((Topic::Release(index, cycle), Ping::Go));
                }
            }
        }
        for (index, span) in course.spans.iter().enumerate() {
            if span.kind == Kind::Weak && !cart.collapsed[index] {
                let ahead = span.a.1 - cart.z;
                if ahead > -1.0 && ahead < 5.5 {
                    return Some((Topic::Weak(index, attempt), Ping::Boost));
                }
            }
        }
        if let Some(span) = cart.span.map(|s| &course.spans[s]) {
            let off = left_of(span, cart);
            if off.abs() > span.half * 0.5 {
                let ping = if off > 0.0 { Ping::Right } else { Ping::Left };
                return Some((Topic::Drift(time as u32), ping));
            }
        }
        for (index, gust) in course.gusts.iter().enumerate() {
            if cart.z > gust.z0 - 4.0 && cart.z < gust.z1 && gust.state(time).0 != Blow::Calm {
                return Some((Topic::Wind(index, gust.cycle(time)), Ping::Careful));
            }
        }
        if self.slow_for > 1.5 {
            return Some((Topic::Stalled((time / 3.0) as u32), Ping::Go));
        }
        None
    }

    pub fn navigate(
        &mut self,
        course: &Course,
        crew: &Crew,
        time: f32,
        skill: Skill,
        rng: &mut StdRng,
        dt: f32,
    ) -> Input {
        if let Some((at, ping)) = self.pending {
            if time >= at {
                self.pending = None;
                return Input {
                    fire: true,
                    yaw: ping.code(),
                    ..Default::default()
                };
            }
            return Input::default();
        }
        self.think_in -= dt;
        if self.think_in > 0.0 {
            return Input::default();
        }
        self.think_in = 0.15;
        let cart = &crew.cart;
        if cart.falling.is_some() || cart.finished_at.is_some() {
            return Input::default();
        }
        if cart.speed.abs() < 0.3 {
            self.slow_for += 0.15;
        } else {
            self.slow_for = 0.0;
        }
        let Some((topic, mut ping)) = self.advice(course, cart, time) else {
            return Input::default();
        };
        if self.said.contains(&topic) || rng.random_bool(skill.hesitation() * 0.25) {
            return Input::default();
        }
        if matches!(topic, Topic::Fork(..))
            && rng.random_bool(((1.0 - skill.level()) * 0.12) as f64)
        {
            ping = if ping == Ping::Left {
                Ping::Right
            } else {
                Ping::Left
            };
        }
        if matches!(topic, Topic::Stalled(_)) {
            self.slow_for = 0.0;
        }
        self.said.insert(topic);
        self.pending = Some((time + skill.reaction() * rng.random_range(0.5..1.0), ping));
        Input::default()
    }
}
