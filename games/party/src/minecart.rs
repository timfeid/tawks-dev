//! Minecart Rush: ride a cart down one shared line of rails that splits at
//! every junction. Lean to pick a branch: the short one is quick, the long one
//! winds out over loose rails. Ram a rival's cart and you both get knocked
//! back. Carts leave the gate one at a time and each rider is timed from their
//! own release, so the fastest ride wins.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};

pub const KEY: &str = "minecart";
pub const JUNCTIONS: usize = 5;
pub const LIMIT: f32 = 50.0;
pub const MAX_SPEED: f32 = 12.0;
/// The long branch's loose rails cap a cart at this share of its top speed.
pub const LOOSE: f32 = 0.6;
pub const CART_LENGTH: f32 = 2.4;
/// Closing faster than this is a crash; anything gentler just nudges along.
pub const CRASH_SPEED: f32 = 2.5;
pub const KNOCK: f32 = 4.0;
pub const STUN: f32 = 1.2;
pub const GATE_GAP: f32 = 1.2;
/// How many carts the siding behind the gate holds when four ride.
pub const SIDING: usize = 5;
const ACCEL: f32 = 6.0;
const BRAKE: f32 = 20.0;
const STRAIGHT: f32 = 14.0;
const SPAN: f32 = 20.0;
const FAST_BULGE: f32 = 3.0;
const SLOW_BULGE: f32 = 9.5;
const START_Z: f32 = 80.0;
const SAMPLES: usize = 24;
/// After a crash the pair cannot crash again until they are rolling apart.
const GRACE: f32 = STUN + 0.6;

#[derive(Clone, Debug)]
struct Leg {
    points: Vec<(f32, f32)>,
    reach: Vec<f32>,
}

impl Leg {
    fn new(points: Vec<(f32, f32)>) -> Self {
        let mut reach = vec![0.0];
        for pair in points.windows(2) {
            let last = *reach.last().unwrap();
            reach.push(last + (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1));
        }
        Leg { points, reach }
    }

    fn length(&self) -> f32 {
        *self.reach.last().unwrap_or(&0.0)
    }

    /// Where on the leg a cart `along` units in sits, and which way it faces.
    fn at(&self, along: f32) -> (f32, f32, f32) {
        let along = along.clamp(0.0, self.length());
        let index = self
            .reach
            .windows(2)
            .position(|pair| along <= pair[1])
            .unwrap_or(self.points.len().saturating_sub(2));
        let (a, b) = (self.points[index], self.points[index + 1]);
        let span = (self.reach[index + 1] - self.reach[index]).max(0.0001);
        let t = (along - self.reach[index]) / span;
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        (a.0 + dx * t, a.1 + dz * t, dx.atan2(dz))
    }

    fn json(&self) -> Value {
        json!(self.points.iter().map(|p| [p.0, p.1]).collect::<Vec<_>>())
    }
}

/// The line: a straight, then a junction splitting into two branches that
/// meet again, and so on, ending on a straight run to the finish. Legs are
/// numbered in riding order, so even legs are straights and odd legs are the
/// branch pair after junction `leg / 2`.
#[derive(Clone, Debug)]
pub struct Course {
    straights: Vec<Leg>,
    branches: Vec<[Leg; 2]>,
    /// Which branch, 0 left or 1 right, is the short one at each junction.
    pub fast: Vec<usize>,
}

impl Course {
    pub fn new(rng: &mut StdRng) -> Self {
        let mut z = START_Z;
        let mut straights = Vec::new();
        let mut branches = Vec::new();
        let mut fast = Vec::new();
        for junction in 0..=JUNCTIONS {
            straights.push(Leg::new(vec![(0.0, z), (0.0, z - STRAIGHT)]));
            z -= STRAIGHT;
            if junction == JUNCTIONS {
                break;
            }
            let quick = rng.random_range(0..2usize);
            let branch = |side: usize| {
                let bulge = if side == quick {
                    FAST_BULGE
                } else {
                    SLOW_BULGE
                };
                let sign = if side == 0 { -1.0 } else { 1.0 };
                Leg::new(
                    (0..=SAMPLES)
                        .map(|i| {
                            let t = i as f32 / SAMPLES as f32;
                            let out = (1.0 - (t * std::f32::consts::TAU).cos()) * 0.5;
                            (sign * bulge * out, z - t * SPAN)
                        })
                        .collect(),
                )
            };
            branches.push([branch(0), branch(1)]);
            fast.push(quick);
            z -= SPAN;
        }
        Course {
            straights,
            branches,
            fast,
        }
    }

    pub fn last_leg(&self) -> usize {
        JUNCTIONS * 2
    }

    fn leg(&self, leg: usize, side: usize) -> &Leg {
        if leg % 2 == 0 {
            &self.straights[leg / 2]
        } else {
            &self.branches[leg / 2][side]
        }
    }

    pub fn leg_length(&self, leg: usize, side: usize) -> f32 {
        self.leg(leg, side).length()
    }

    fn is_slow(&self, leg: usize, side: usize) -> bool {
        leg % 2 == 1 && self.fast[leg / 2] != side
    }
}

#[derive(Clone, Debug)]
pub struct Cart {
    pub seat: u8,
    pub leg: usize,
    /// The branch taken at the last junction, 0 left or 1 right.
    pub side: usize,
    pub along: f32,
    pub speed: f32,
    /// The way the rider is leaning for the next junction: -1, 0 or 1.
    pub lean: i8,
    pub stun: f32,
    pub grace: f32,
    pub released_at: Option<f32>,
    /// Seconds from release to the finish.
    pub ride: Option<f32>,
    pub wrong_turns: u8,
    pub crashes: u8,
    skill: f32,
    /// The junction a bot has made up its mind about, and which way.
    plan: Option<(usize, usize)>,
}

impl Cart {
    fn rolling(&self) -> bool {
        self.released_at.is_some() && self.ride.is_none()
    }
}

pub struct MinecartRush {
    pub course: Course,
    pub carts: Vec<Cart>,
    pub time: f32,
    gate: Vec<u8>,
    /// How many carts the siding behind the gate holds, more on a bigger field.
    pub siding: usize,
    next_release: f32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

fn sign(v: f32) -> i8 {
    if v < -0.35 {
        -1
    } else if v > 0.35 {
        1
    } else {
        0
    }
}

impl MinecartRush {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let course = Course::new(&mut rng);
        let mut gate: Vec<u8> = contestants.iter().map(|c| c.seat).collect();
        gate.shuffle(&mut rng);
        MinecartRush {
            course,
            carts: contestants
                .iter()
                .map(|one| Cart {
                    seat: one.seat,
                    leg: 0,
                    side: 0,
                    along: 0.0,
                    speed: 0.0,
                    lean: 0,
                    stun: 0.0,
                    grace: 0.0,
                    released_at: None,
                    ride: None,
                    wrong_turns: 0,
                    crashes: 0,
                    skill: one.skill,
                    plan: None,
                })
                .collect(),
            time: 0.0,
            gate,
            siding: crate::bout::crowd_count(SIDING, contestants.len()),
            next_release: 0.0,
            rng,
            placed: None,
        }
    }

    pub fn cart(&self, seat: u8) -> &Cart {
        self.carts.iter().find(|cart| cart.seat == seat).unwrap()
    }

    pub fn cart_mut(&mut self, seat: u8) -> &mut Cart {
        self.carts
            .iter_mut()
            .find(|cart| cart.seat == seat)
            .unwrap()
    }

    /// How far down the line a cart is, from 0 at the gate to 1 at the finish.
    pub fn progress(&self, cart: &Cart) -> f32 {
        if cart.ride.is_some() {
            return 1.0;
        }
        let length = self.course.leg_length(cart.leg, cart.side).max(0.001);
        (cart.leg as f32 + (cart.along / length).min(1.0)) / (self.course.last_leg() + 1) as f32
    }

    fn release(&mut self, events: &mut Vec<Value>) {
        if self.gate.is_empty() || self.time < self.next_release {
            return;
        }
        // Hold the next cart while the one before is still blocking the gate,
        // but not for long, so nobody can park there and keep the rest in.
        let blocked = self.time < self.next_release + GATE_GAP
            && self
                .carts
                .iter()
                .any(|cart| cart.rolling() && cart.leg == 0 && cart.along < CART_LENGTH * 1.5);
        if blocked {
            return;
        }
        let seat = self.gate.remove(0);
        let time = self.time;
        self.cart_mut(seat).released_at = Some(time);
        self.next_release = time + GATE_GAP;
        events.push(json!({ "kind": "release", "seat": seat }));
    }

    fn ride(&mut self, index: usize, input: Input, dt: f32, events: &mut Vec<Value>) {
        let time = self.time;
        let course = &self.course;
        let cart = &mut self.carts[index];
        if !cart.rolling() {
            return;
        }
        let lean = sign(input.stick.x);
        if lean != 0 {
            cart.lean = lean;
        }
        cart.grace = (cart.grace - dt).max(0.0);
        if cart.stun > 0.0 {
            cart.stun -= dt;
            cart.speed = 0.0;
            return;
        }
        let top = if course.is_slow(cart.leg, cart.side) {
            MAX_SPEED * LOOSE
        } else {
            MAX_SPEED
        };
        if input.stick.z > 0.5 {
            cart.speed = (cart.speed - BRAKE * dt).max(0.0);
        } else if cart.speed > top {
            cart.speed = (cart.speed - BRAKE * 0.5 * dt).max(top);
        } else {
            cart.speed = (cart.speed + ACCEL * dt).min(top);
        }
        cart.along += cart.speed * dt;
        loop {
            let length = course.leg_length(cart.leg, cart.side);
            if cart.along < length {
                break;
            }
            if cart.leg == course.last_leg() {
                let started = cart.released_at.unwrap_or(0.0);
                cart.ride = Some(time - started);
                cart.along = length;
                events.push(json!({ "kind": "finish", "seat": cart.seat, "ride": time - started }));
                break;
            }
            cart.along -= length;
            cart.leg += 1;
            if cart.leg % 2 == 1 {
                let junction = cart.leg / 2;
                let fast = course.fast[junction];
                // Nobody leaning rides the switch as it is set, which is
                // always against you.
                cart.side = match cart.lean {
                    -1 => 0,
                    1 => 1,
                    _ => 1 - fast,
                };
                cart.lean = 0;
                let right = cart.side == fast;
                if !right {
                    cart.wrong_turns += 1;
                }
                events.push(json!({ "kind": "junction", "seat": cart.seat, "fast": right }));
            }
        }
    }

    /// How far ahead `front` is of `rear` along rails they share, if they do.
    fn gap(&self, front: &Cart, rear: &Cart) -> Option<f32> {
        let same_branch = front.leg % 2 == 0 || front.side == rear.side;
        if front.leg == rear.leg && same_branch {
            return Some(front.along - rear.along);
        }
        // Coming off either branch onto the straight they meet on.
        if front.leg % 2 == 0 && front.leg == rear.leg + 1 {
            let rest = self.course.leg_length(rear.leg, rear.side) - rear.along;
            return Some(front.along + rest.max(0.0));
        }
        None
    }

    fn collide(&mut self, events: &mut Vec<Value>) {
        for i in 0..self.carts.len() {
            for j in 0..self.carts.len() {
                if i == j || !self.carts[i].rolling() || !self.carts[j].rolling() {
                    continue;
                }
                let (front, rear) = (&self.carts[i], &self.carts[j]);
                let Some(gap) = self.gap(front, rear) else {
                    continue;
                };
                if gap < 0.0 || gap >= CART_LENGTH {
                    continue;
                }
                let closing = rear.speed - front.speed;
                let crash = closing > CRASH_SPEED && front.grace <= 0.0 && rear.grace <= 0.0;
                let shared_leg = front.leg == rear.leg;
                let front_speed = front.speed;
                let (front_seat, rear_seat) = (front.seat, rear.seat);
                if crash {
                    for index in [i, j] {
                        let cart = &mut self.carts[index];
                        cart.along = (cart.along - KNOCK).max(0.0);
                        cart.speed = 0.0;
                        cart.stun = STUN;
                        cart.grace = GRACE;
                        cart.crashes += 1;
                    }
                    if shared_leg {
                        let ahead = self.carts[i].along;
                        let cart = &mut self.carts[j];
                        cart.along = cart.along.min(ahead - CART_LENGTH).max(0.0);
                    }
                    events.push(json!({ "kind": "crash", "a": rear_seat, "b": front_seat }));
                } else {
                    // Too gentle to crash: the rear cart buffers up behind.
                    let cart = &mut self.carts[j];
                    cart.speed = cart.speed.min(front_speed);
                    if shared_leg {
                        let ahead = self.carts[i].along;
                        let cart = &mut self.carts[j];
                        cart.along = cart.along.min(ahead - CART_LENGTH).max(0.0);
                    }
                }
            }
        }
    }

    fn settle(&mut self) {
        let mut order: Vec<(u8, f32, f32)> = self
            .carts
            .iter()
            .map(|cart| {
                (
                    cart.seat,
                    cart.ride.unwrap_or(f32::MAX),
                    self.progress(cart),
                )
            })
            .collect();
        order.sort_by(|a, b| a.1.total_cmp(&b.1).then(b.2.total_cmp(&a.2)));
        self.placed = Some(order.into_iter().map(|(seat, _, _)| vec![seat]).collect());
    }
}

impl Bout for MinecartRush {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "straights": self.course.straights.iter().map(Leg::json).collect::<Vec<_>>(),
            "branches": self.course.branches.iter().map(|pair| [pair[0].json(), pair[1].json()]).collect::<Vec<_>>(),
            "fast": self.course.fast,
            "gauge": 0.9,
            "siding": self.siding,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.time += dt;
        self.release(events);
        for index in 0..self.carts.len() {
            let seat = self.carts[index].seat;
            let input = inputs
                .iter()
                .find(|(one, _)| *one == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            self.ride(index, input, dt, events);
        }
        self.collide(events);
        if self.time >= LIMIT || self.carts.iter().all(|cart| cart.ride.is_some()) {
            self.settle();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.carts.iter().find(|cart| cart.seat == seat).cloned() else {
            return Input::default();
        };
        if !me.rolling() {
            return Input::default();
        }
        let skill = Skill(me.skill);
        let mut stick = Stick { x: 0.0, z: -1.0 };
        if me.leg % 2 == 0 && me.leg < self.course.last_leg() {
            let junction = me.leg / 2;
            let side = match me.plan {
                Some((planned, side)) if planned == junction => side,
                _ => {
                    // A sharp bot nearly always reads the short branch; a
                    // clumsy one guesses about as often as it reads it.
                    let reads = self.rng.random_bool((0.5 + skill.level() * 0.48) as f64);
                    let fast = self.course.fast[junction];
                    let side = if reads { fast } else { 1 - fast };
                    self.cart_mut(seat).plan = Some((junction, side));
                    side
                }
            };
            stick.x = if side == 0 { -1.0 } else { 1.0 };
        }
        // Ease off when a stalled cart sits close ahead, if quick enough to
        // notice it.
        let blocked = self.carts.iter().any(|other| {
            other.seat != seat
                && other.rolling()
                && me.speed - other.speed > CRASH_SPEED * 0.6
                && self
                    .gap(other, &me)
                    .is_some_and(|gap| gap >= 0.0 && gap < CART_LENGTH + me.speed * 0.5)
        });
        if blocked && self.rng.random_bool(skill.level().max(0.05) as f64) {
            stick.z = 1.0;
        }
        Input::from_stick(stick)
    }

    fn snapshot(&self) -> Value {
        json!({
            "game": KEY,
            "time": self.time,
            "limit": LIMIT,
            "players": self.carts.iter().map(|cart| {
                let (x, z, heading) = self.course.leg(cart.leg, cart.side).at(cart.along);
                json!({
                    "seat": cart.seat,
                    "x": x,
                    "z": z,
                    "heading": heading,
                    "speed": cart.speed,
                    "lean": cart.lean,
                    "progress": self.progress(cart),
                    "waiting": cart.released_at.is_none(),
                    "stunned": cart.stun > 0.0,
                    "finished": cart.ride.is_some(),
                    "ride": cart.ride.or(cart.released_at.map(|at| self.time - at)).unwrap_or(0.0),
                    "wrong": cart.wrong_turns,
                    "crashes": cart.crashes,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn riders(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_a_longer_siding() {
        let four = MinecartRush::new(&riders(4), 1);
        assert_eq!(four.siding, SIDING, "four play it as built");
        let eight = MinecartRush::new(&riders(8), 1);
        assert_eq!(eight.siding, SIDING * 2, "eight get the siding of two fours");
        assert!(eight.siding >= 8 - 1, "room for everyone still waiting");
        assert_eq!(eight.setup()["siding"], json!(eight.siding), "and the client is told");
    }
}
