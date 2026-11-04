//! Tower Teeter: everyone gets a raft on the same swell and the same queue of
//! furniture. Hover each piece over your raft, turn it, drop it. The sea grows
//! rougher as the clock runs; when the horn blows the tallest tower still
//! standing wins, and every piece that went into the water costs height.

mod physics;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Skill};
use physics::{Body, V2, World};

pub const KEY: &str = "teeter";
/// Building time, until the horn.
pub const PLAY_SECONDS: f32 = 68.0;
/// After the horn the sea gets one last say before the towers are measured.
pub const SETTLE_SECONDS: f32 = 4.0;
pub const ROUND_SECONDS: f32 = PLAY_SECONDS + SETTLE_SECONDS;
pub const RAFT_HALF_WIDTH: f32 = 3.2;
pub const RAFT_HALF_HEIGHT: f32 = 0.25;
/// How far either side of the raft's middle a piece can hover.
pub const HOVER_RANGE: f32 = 4.2;
pub const DROP_COOLDOWN: f32 = 0.9;
/// Height lost for every piece that ends up in the water.
pub const LOST_PENALTY: f32 = 0.5;
pub const MAX_PIECES: usize = 40;
/// Rafts sit side by side this far apart; the client lays them out.
pub const LANE_SPACING: f32 = 11.0;
/// A piece whose centre of mass sinks below this is in the sea.
pub const WATER_LINE: f32 = -0.6;
/// How far above the tallest point a new piece hangs before it is let go.
pub const DROP_GAP: f32 = 0.35;
fn hand_gap(width: f32) -> f32 {
    DROP_GAP / BodyProfile::new(width).reach.powi(10)
}
fn drop_interval(width: f32) -> f32 {
    DROP_COOLDOWN / BodyProfile::new(width).speed.powi(3)
}
/// A piece counts toward the tower for a moment after its last contact, so a
/// small bounce at the bell does not erase it.
const STANDING_GRACE: f32 = 0.35;
const SUBSTEPS: u32 = 2;
const ITERATIONS: u32 = 10;
const SWELL_SECONDS: f32 = 3.2;
const SWELL_WARNING: f32 = 2.0;
/// A sudden loss this large is announced as a topple.
const TOPPLE_DROP: f32 = 1.2;
/// How far ahead a bot imagines a drop, and at what step.
const REHEARSAL_SECONDS: f32 = 1.2;
const TICK_DT_SIM: f32 = 1.0 / 60.0;
const IMAGINED_PER_TICK: u32 = 1;

pub struct Kind {
    pub name: &'static str,
    pub half_width: f32,
    pub half_height: f32,
    pub mass: f32,
    /// The centre of mass, from the middle of the box.
    pub com: (f32, f32),
    pub friction: f32,
    /// How often it turns up in the queue.
    pub weight: u32,
}

pub const KINDS: &[Kind] = &[
    Kind {
        name: "chair",
        half_width: 0.42,
        half_height: 0.5,
        mass: 1.0,
        com: (0.0, -0.1),
        friction: 0.7,
        weight: 5,
    },
    Kind {
        name: "lamp",
        half_width: 0.2,
        half_height: 0.8,
        mass: 0.7,
        com: (0.0, -0.3),
        friction: 0.6,
        weight: 4,
    },
    Kind {
        name: "piano",
        half_width: 1.05,
        half_height: 0.62,
        mass: 5.0,
        com: (0.18, -0.05),
        friction: 0.7,
        weight: 2,
    },
    Kind {
        name: "fridge",
        half_width: 0.45,
        half_height: 0.95,
        mass: 3.5,
        com: (0.0, -0.1),
        friction: 0.5,
        weight: 3,
    },
    Kind {
        name: "armchair",
        half_width: 0.6,
        half_height: 0.45,
        mass: 1.6,
        com: (0.0, -0.06),
        friction: 0.8,
        weight: 4,
    },
    Kind {
        name: "crate",
        half_width: 0.5,
        half_height: 0.5,
        mass: 1.2,
        com: (0.0, 0.0),
        friction: 0.75,
        weight: 4,
    },
    Kind {
        name: "table",
        half_width: 0.85,
        half_height: 0.38,
        mass: 1.3,
        com: (0.0, 0.12),
        friction: 0.65,
        weight: 3,
    },
    Kind {
        name: "tv",
        half_width: 0.62,
        half_height: 0.42,
        mass: 1.0,
        com: (0.0, 0.0),
        friction: 0.6,
        weight: 3,
    },
    Kind {
        name: "bookcase",
        half_width: 0.55,
        half_height: 1.0,
        mass: 2.5,
        com: (0.0, 0.05),
        friction: 0.7,
        weight: 2,
    },
];

fn kind_body(kind: usize) -> Body {
    let k = &KINDS[kind];
    Body::dynamic(
        V2::new(k.half_width, k.half_height),
        k.mass,
        V2::new(k.com.0, k.com.1),
        k.friction,
    )
}

/// The whole queue, the same for every raft. The opening pieces are steady
/// ones so nobody's first drop is a lamp on its end.
fn deal(rng: &mut StdRng, count: usize) -> Vec<u8> {
    let total: u32 = KINDS.iter().map(|k| k.weight).sum();
    let mut queue: Vec<u8> = vec![5, 2];
    while queue.len() < count {
        let mut roll = rng.random_range(0..total);
        let mut pick = 0;
        for (i, k) in KINDS.iter().enumerate() {
            if roll < k.weight {
                pick = i;
                break;
            }
            roll -= k.weight;
        }
        if queue.last() == Some(&(pick as u8)) && rng.random_bool(0.7) {
            continue;
        }
        queue.push(pick as u8);
    }
    queue
}

/// The sea's pose for the raft at a moment: heave, roll and sway, with the
/// swells laid on top. Every raft rides the same sea.
#[derive(Clone, Debug)]
pub struct Sea {
    pub swells: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

impl Sea {
    fn new(rng: &mut StdRng) -> Self {
        let mut swells = Vec::new();
        let mut at = rng.random_range(16.0..21.0);
        while at < PLAY_SECONDS - SWELL_SECONDS - 3.0 {
            swells.push(at);
            at += rng.random_range(11.0..16.0);
        }
        // The horn's own swell, the last word before the measuring.
        swells.push(PLAY_SECONDS);
        Sea { swells }
    }

    /// How rough it is, from 0 at the start to 1 at the horn.
    pub fn level(time: f32) -> f32 {
        (time / PLAY_SECONDS).clamp(0.0, 1.0)
    }

    pub fn pose(&self, time: f32) -> Pose {
        use std::f32::consts::TAU;
        let level = Self::level(time);
        let heave = (0.04 + 0.1 * level) * (TAU * time / 3.3).sin();
        let mut roll = (0.008 + 0.034 * level) * (TAU * time / 4.6 + 0.7).sin();
        let sway = 0.1 * level * (TAU * time / 5.9 + 1.9).sin();
        for &start in &self.swells {
            let u = (time - start) / SWELL_SECONDS;
            if (0.0..1.0).contains(&u) {
                let strength = if start >= PLAY_SECONDS {
                    0.036
                } else {
                    0.02 + 0.015 * level
                };
                roll += strength * (std::f32::consts::PI * u).sin() * (TAU * u).sin();
            }
        }
        Pose {
            x: sway,
            y: heave,
            angle: roll,
        }
    }

    /// Seconds until the next swell, while one is coming soon.
    pub fn warning(&self, time: f32) -> Option<f32> {
        self.swells
            .iter()
            .map(|s| s - time)
            .find(|d| *d > 0.0 && *d <= SWELL_WARNING)
    }

    pub fn swelling(&self, time: f32) -> bool {
        self.swells
            .iter()
            .any(|s| (0.0..SWELL_SECONDS).contains(&(time - s)))
    }
}

#[derive(Clone, Debug)]
pub struct Piece {
    pub id: u16,
    pub kind: u8,
    /// The last time it rested on the raft or on something that did.
    pub standing_at: f32,
}

#[derive(Clone, Debug)]
pub struct Raft {
    pub seat: u8,
    pub lane: usize,
    pub body_width: f32,
    pub world: World,
    /// One per body after the raft itself, in the same order.
    pub pieces: Vec<Piece>,
    pub next_id: u16,
    /// Where in the queue this raft has got to.
    pub dealt: usize,
    pub placed: u32,
    pub lost: u32,
    /// Aim: the hover position from the raft's middle, and the turn.
    pub cursor: f32,
    pub angle: f32,
    pub cooldown: f32,
    pub height: f32,
    pub best: f32,
    peak: f32,
}

impl Raft {
    fn new(seat: u8, lane: usize, body_width: f32) -> Self {
        let mut world = World::new(ITERATIONS);
        let mut raft = Body::kinematic(V2::new(RAFT_HALF_WIDTH, RAFT_HALF_HEIGHT), 0.9);
        raft.pos = V2::new(0.0, 0.0);
        world.bodies.push(raft);
        Raft {
            seat,
            lane,
            body_width: BodyProfile::new(body_width).width,
            world,
            pieces: Vec::new(),
            next_id: 0,
            dealt: 0,
            placed: 0,
            lost: 0,
            cursor: 0.0,
            angle: 0.0,
            cooldown: 0.0,
            height: 0.0,
            best: 0.0,
            peak: 0.0,
        }
    }

    pub fn score(&self) -> f32 {
        self.height - LOST_PENALTY * self.lost as f32
    }

    pub fn raft(&self) -> &Body {
        &self.world.bodies[0]
    }

    pub fn piece_body(&self, index: usize) -> &Body {
        &self.world.bodies[index + 1]
    }

    /// The highest point of anything on this raft, falling pieces included.
    fn top(&self) -> f32 {
        self.world
            .bodies
            .iter()
            .map(|b| b.bounds().1.y)
            .fold(f32::MIN, f32::max)
    }

    /// Where a piece would be let go with this aim.
    pub fn hold(&self, kind: usize, cursor: f32, angle: f32) -> Body {
        let mut body = kind_body(kind);
        body.angle = angle;
        let k = &KINDS[kind];
        let reach = angle.cos().abs() * k.half_height + angle.sin().abs() * k.half_width;
        let center = V2::new(cursor, self.top() + hand_gap(self.body_width) + reach);
        body.pos = V2::new(center.x, center.y) - rotate(body.offset, angle);
        body
    }

    /// How tall the standing tower is, measured square to the deck so the
    /// raft's own bobbing does not count.
    fn measure(&mut self, time: f32) {
        let n = self.world.bodies.len();
        let mut linked = vec![false; n];
        linked[0] = true;
        let mut grew = true;
        while grew {
            grew = false;
            for arb in self.world.arbiters.iter().filter(|a| a.touching()) {
                if linked[arb.a] != linked[arb.b] {
                    linked[arb.a] = true;
                    linked[arb.b] = true;
                    grew = true;
                }
            }
        }
        for (i, piece) in self.pieces.iter_mut().enumerate() {
            if linked[i + 1] {
                piece.standing_at = time;
            }
        }
        let deck = self.raft();
        let up = rotate(V2::new(0.0, 1.0), deck.angle);
        let base = deck.pos + up * RAFT_HALF_HEIGHT;
        let mut height: f32 = 0.0;
        for (i, piece) in self.pieces.iter().enumerate() {
            if time - piece.standing_at > STANDING_GRACE {
                continue;
            }
            for corner in self.piece_body(i).corners() {
                height = height.max((corner - base).dot(up));
            }
        }
        self.height = height;
        self.best = self.best.max(height);
    }
}

/// How close a raft's load is to tipping as one: the height of its centre
/// of mass over the distance from there to the nearer edge of whatever rests
/// on the deck. A sea that tilts the load more than one part in this many
/// tips it over.
pub fn slenderness(world: &World) -> f32 {
    let deck = &world.bodies[0];
    let up = rotate(V2::new(0.0, 1.0), deck.angle);
    let side = V2::new(up.y, -up.x);
    let base = deck.pos + up * RAFT_HALF_HEIGHT;
    let (mut mass, mut x, mut y) = (0.0, 0.0, 0.0);
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for b in world.bodies.iter().skip(1) {
        let rel = b.pos - base;
        let h = rel.dot(up);
        if h < -0.3 || rel.dot(side).abs() > RAFT_HALF_WIDTH + 0.5 {
            continue;
        }
        let m = 1.0 / b.inv_mass;
        mass += m;
        x += rel.dot(side) * m;
        y += h * m;
        for c in b.corners() {
            let rc = c - base;
            if rc.dot(up) < 0.12 {
                lo = lo.min(rc.dot(side));
                hi = hi.max(rc.dot(side));
            }
        }
    }
    if mass == 0.0 || lo > hi {
        return 0.0;
    }
    let (x, y) = (x / mass, y / mass);
    y.max(0.0) / (x - lo).min(hi - x).max(0.05)
}

fn rotate(v: V2, angle: f32) -> V2 {
    let (s, c) = angle.sin_cos();
    V2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

#[derive(Clone, Debug, Default)]
struct Plan {
    x: f32,
    angle: f32,
    /// Let go only while the deck is within this much of level.
    calm: f32,
    /// And only once nothing on the raft is moving faster than this.
    settled: f32,
    patience: f32,
    fumble: bool,
    greedy: bool,
}

#[derive(Clone, Debug, Default)]
struct Brain {
    skill: f32,
    /// A personal streak of recklessness, fixed for the bout.
    greed: f32,
    think: f32,
    plan: Option<Plan>,
    hand: f32,
    turn: f32,
    waited: f32,
    greedy: bool,
    pending: Vec<(f32, f32, f32)>,
    rehearsed: Vec<(f32, f32, f32)>,
}

pub struct TowerTeeter {
    pub rafts: Vec<Raft>,
    pub time: f32,
    pub queue: Vec<u8>,
    pub sea: Sea,
    rng: StdRng,
    brains: Vec<(u8, Brain)>,
    horn: bool,
    /// Imagined drops this tick, shared by every bot so a room of them
    /// thinking at once never makes one tick slow.
    imagined: u32,
    warned: Vec<f32>,
    placements: Option<Vec<Vec<u8>>>,
}

fn gauss(rng: &mut StdRng) -> f32 {
    let u: f32 = rng.random_range(1e-6..1.0);
    let v: f32 = rng.random_range(0.0..1.0);
    (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
}

fn wrap(angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (angle + PI).rem_euclid(TAU) - PI
}

impl TowerTeeter {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let queue = deal(&mut rng, 200);
        let sea = Sea::new(&mut rng);
        let mut seats: Vec<&Contestant> = contestants.iter().collect();
        seats.sort_by_key(|c| c.seat);
        let rafts = seats
            .iter()
            .enumerate()
            .map(|(lane, c)| Raft::new(c.seat, lane, c.body_width))
            .collect();
        let brains = seats
            .iter()
            .map(|c| {
                (
                    c.seat,
                    Brain {
                        skill: c.skill,
                        greed: rng.random_range(0.03..0.2),
                        think: 0.8 + rng.random_range(0.0..0.8),
                        ..Default::default()
                    },
                )
            })
            .collect();
        TowerTeeter {
            rafts,
            time: 0.0,
            queue,
            sea,
            rng,
            brains,
            horn: false,
            imagined: 0,
            warned: Vec::new(),
            placements: None,
        }
    }

    pub fn lane_x(&self, lane: usize) -> f32 {
        (lane as f32 - (self.rafts.len() as f32 - 1.0) / 2.0) * LANE_SPACING
    }

    pub fn raft(&self, seat: u8) -> Option<&Raft> {
        self.rafts.iter().find(|r| r.seat == seat)
    }

    pub fn held(&self, raft: &Raft) -> usize {
        self.queue[raft.dealt % self.queue.len()] as usize
    }

    pub fn building(&self) -> bool {
        self.time < PLAY_SECONDS && self.placements.is_none()
    }

    /// Takes one player's aim and, if they let go and may, drops their piece.
    fn handle(&mut self, index: usize, input: &Input, events: &mut Vec<Value>) {
        let building = self.building();
        let kind = self.held(&self.rafts[index]);
        let raft = &mut self.rafts[index];
        if input.pitch.is_finite() {
            raft.cursor = input.pitch.clamp(-1.0, 1.0) * HOVER_RANGE;
        }
        if input.yaw.is_finite() {
            raft.angle = wrap(input.yaw);
        }
        if !input.fire || !building || raft.cooldown > 0.0 || raft.pieces.len() >= MAX_PIECES {
            return;
        }
        let body = raft.hold(kind, raft.cursor, raft.angle);
        let at = body.center();
        raft.world.bodies.push(body);
        raft.pieces.push(Piece {
            id: raft.next_id,
            kind: kind as u8,
            standing_at: f32::MIN,
        });
        raft.next_id = raft.next_id.wrapping_add(1);
        raft.dealt += 1;
        raft.placed += 1;
        raft.cooldown = drop_interval(raft.body_width);
        events.push(json!({
            "kind": "drop",
            "seat": raft.seat,
            "id": raft.next_id.wrapping_sub(1),
            "piece": KINDS[kind].name,
            "x": round(at.x),
            "y": round(at.y),
        }));
    }

    fn settle(&mut self, events: &mut Vec<Value>) {
        let mut order: Vec<(f32, u8)> = self.rafts.iter().map(|r| (r.score(), r.seat)).collect();
        order.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (score, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if (prev - score).abs() < 0.005 => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(score);
        }
        events.push(json!({
            "kind": "final",
            "standings": groups,
            "scores": self.rafts.iter().map(|r| json!({"seat": r.seat, "height": round(r.height), "lost": r.lost, "score": round(r.score())})).collect::<Vec<_>>(),
        }));
        self.placements = Some(groups);
    }

    /// Whether the bot is happy to leave its tower alone for now: tall
    /// enough for its nerve, or a swell is about to hit. Near the horn the
    /// nerve grows, so most bots risk one more piece at the end.
    fn content(&self, index: usize, bot: usize) -> bool {
        let brain = &self.brains[bot].1;
        let level = Skill(brain.skill).level();
        let raft = &self.rafts[index];
        let mut nerve = 5.5 + brain.greed * 8.0;
        let left = PLAY_SECONDS - self.time;
        if left < 8.0 {
            nerve += 2.0 + brain.greed * 6.0;
        }
        let bracing =
            level > 0.3 && (self.sea.warning(self.time).is_some() || self.sea.swelling(self.time));
        let wobbly = slenderness(&raft.world) > 5.0 + brain.greed * 10.0;
        raft.height >= nerve || bracing && raft.height > 2.0 || wobbly && left > 8.0
    }

    /// The bot's first impression: a handful of spots that look promising
    /// from the shape of the tower, and whether it is feeling greedy.
    fn shortlist(&mut self, index: usize, bot: usize) -> (bool, Vec<(f32, f32, f32)>) {
        let raft = &self.rafts[index];
        let kind = self.held(raft);
        let k = &KINDS[kind];
        let level = Skill(self.brains[bot].1.skill).level();
        let leader = self.rafts.iter().map(Raft::score).fold(f32::MIN, f32::max);
        let behind = leader - raft.score();
        let mut greed = self.brains[bot].1.greed;
        if behind > 1.5 && self.time > 30.0 {
            greed += 0.15;
        }
        if self.time > PLAY_SECONDS - 14.0 {
            greed += 0.15;
        }
        let greedy = self.rng.random_bool(greed.clamp(0.0, 0.9) as f64);
        let support: Vec<&Body> = std::iter::once(raft.raft())
            .chain(raft.pieces.iter().enumerate().filter_map(|(i, p)| {
                let b = raft.piece_body(i);
                (self.time - p.standing_at < STANDING_GRACE && b.speed() < 0.6).then_some(b)
            }))
            .collect();
        let skyline = |x: f32| {
            support
                .iter()
                .filter_map(|b| b.top_at(x))
                .fold(f32::MIN, f32::max)
        };
        let deck = raft.raft();
        let mut spots: Vec<(f32, f32, f32)> = Vec::new();
        for (angle, hw, hh) in [
            (0.0, k.half_width, k.half_height),
            (std::f32::consts::FRAC_PI_2, k.half_height, k.half_width),
        ] {
            let mut x = -2.8;
            while x <= 2.81 {
                let samples: Vec<f32> = [-0.9f32, -0.45, 0.0, 0.45, 0.9]
                    .iter()
                    .map(|f| skyline(deck.pos.x + x + f * hw))
                    .collect();
                let top = samples.iter().copied().fold(f32::MIN, f32::max);
                if top > f32::MIN / 2.0 {
                    let low = samples.iter().copied().fold(f32::MAX, f32::min);
                    let covered = samples.iter().filter(|s| **s > top - 0.12).count() as f32 / 5.0;
                    let tall = top + 2.0 * hh;
                    let lean = (top - low).min(3.0);
                    let value = if greedy {
                        tall * 1.6 - lean * 0.4 - (1.0 - covered) * 0.7 - x.abs() * 0.1
                    } else {
                        tall - lean * 1.4
                            - (1.0 - covered) * 2.2 * hh
                            - x.abs() * 0.35
                            - (2.0 * hh - 2.0 * hw).max(0.0) * 1.6
                    } + gauss(&mut self.rng) * (1.0 - level) * 0.6;
                    spots.push((value, x, angle));
                }
                x += 0.2;
            }
        }
        spots.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        // A sharp player weighs more options before committing.
        let keep = 1 + (level * 6.0).round() as usize;
        let mut chosen: Vec<(f32, f32, f32)> = Vec::new();
        for spot in spots {
            if chosen.len() >= keep {
                break;
            }
            if chosen
                .iter()
                .all(|c| c.2 != spot.2 || (c.1 - spot.1).abs() > 0.3)
            {
                chosen.push(spot);
            }
        }
        // Doing nothing is an option too; it is weighed like any drop.
        chosen.push((0.0, 0.0, f32::NAN));
        (greedy, chosen)
    }

    /// Imagines one drop: copies the raft, lets the piece fall onto it for a
    /// second of the coming sea, and judges what happened.
    fn rehearse(&self, index: usize, x: f32, angle: f32, greedy: bool) -> f32 {
        let raft = &self.rafts[index];
        let mut world = raft.world.clone();
        let before: Vec<V2> = world.bodies.iter().map(|b| b.pos).collect();
        if angle.is_finite() {
            world.bodies.push(raft.hold(self.held(raft), x, angle));
        }
        let h = TICK_DT_SIM / SUBSTEPS as f32;
        let mut t = self.time;
        for _ in 0..(REHEARSAL_SECONDS / h) as usize {
            let next = self.sea.pose(t + h);
            let deck = &mut world.bodies[0];
            deck.vel = V2::new((next.x - deck.pos.x) / h, (next.y - deck.pos.y) / h);
            deck.spin = (next.angle - deck.angle) / h;
            world.step(h);
            let deck = &mut world.bodies[0];
            deck.pos = V2::new(next.x, next.y);
            deck.angle = next.angle;
            t += h;
        }
        let deck = &world.bodies[0];
        let up = rotate(V2::new(0.0, 1.0), deck.angle);
        let base = deck.pos + up * RAFT_HALF_HEIGHT;
        let mut lost = 0.0;
        let mut shaken = 0.0;
        let mut height: f32 = 0.0;
        for (i, b) in world.bodies.iter().enumerate().skip(1) {
            if b.pos.y < WATER_LINE + 0.5 || b.pos.x.abs() > RAFT_HALF_WIDTH + 0.6 {
                lost += 1.0;
                continue;
            }
            if let Some(was) = before.get(i) {
                shaken += (b.pos - *was).len().min(2.0);
            }
            for c in b.corners() {
                height = height.max((c - base).dot(up));
            }
        }
        let new = world.bodies.last().unwrap();
        let restless = if angle.is_finite() {
            new.speed().min(3.0)
        } else {
            0.0
        };
        let risk = slenderness(&world).min(30.0);
        if greedy {
            height * 1.5 - lost * 1.5 - shaken * 0.5 - restless * 0.3 - risk * 0.05
        } else {
            height - lost * 3.0 - shaken * 2.0 - restless * 0.8 - risk * 0.6
        }
    }

    /// Settles on the best rehearsed spot, then adds the wobble of a hand.
    fn commit(&mut self, bot: usize) -> Option<Plan> {
        let level = Skill(self.brains[bot].1.skill).level();
        let brain = &self.brains[bot].1;
        let greedy = brain.greedy;
        let best = brain
            .rehearsed
            .iter()
            .copied()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))?;
        if !best.2.is_finite() && !greedy {
            return None;
        }
        let best = if best.2.is_finite() {
            best
        } else {
            brain
                .rehearsed
                .iter()
                .copied()
                .filter(|r| r.2.is_finite())
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))?
        };
        let aim = 0.03 + (1.0 - level) * 0.32;
        let twist = 0.01 + (1.0 - level) * 0.14;
        Some(Plan {
            x: (best.1 + gauss(&mut self.rng) * aim).clamp(-HOVER_RANGE, HOVER_RANGE),
            angle: best.2 + gauss(&mut self.rng) * twist,
            calm: 0.012 + (1.0 - level) * 0.04 + if greedy { 0.03 } else { 0.0 },
            settled: if greedy {
                2.0
            } else {
                0.3 + (1.0 - level) * 0.6
            },
            patience: (0.8 + level * 2.2) * if greedy { 0.4 } else { 1.0 },
            fumble: self.rng.random_bool((0.07 * (1.0 - level)) as f64),
            greedy,
        })
    }
}

fn round(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

impl Bout for TowerTeeter {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "lane_spacing": LANE_SPACING,
            "raft": [RAFT_HALF_WIDTH, RAFT_HALF_HEIGHT],
            "hover": HOVER_RANGE,
            "horn": PLAY_SECONDS,
            "lanes": self.rafts.iter().map(|r| json!({"seat": r.seat, "x": self.lane_x(r.lane)})).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        self.imagined = 0;
        if self.placements.is_some() {
            return;
        }
        for i in 0..self.rafts.len() {
            let seat = self.rafts[i].seat;
            if let Some((_, input)) = inputs.iter().find(|(s, _)| *s == seat) {
                self.handle(i, input, events);
            }
        }
        if let Some(due) = self.sea.warning(self.time) {
            let start = self.time + due;
            if !self.warned.iter().any(|w| (w - start).abs() < 0.01) {
                self.warned.push(start);
                events.push(
                    json!({"kind": "swell", "in": round(due), "final": start >= PLAY_SECONDS}),
                );
            }
        }
        let h = dt / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            let next = self.sea.pose(self.time + h);
            for raft in &mut self.rafts {
                let deck = &mut raft.world.bodies[0];
                deck.vel = V2::new((next.x - deck.pos.x) / h, (next.y - deck.pos.y) / h);
                deck.spin = (next.angle - deck.angle) / h;
                raft.world.step(h);
                let deck = &mut raft.world.bodies[0];
                deck.pos = V2::new(next.x, next.y);
                deck.angle = next.angle;
            }
            self.time += h;
        }
        let time = self.time;
        for raft in &mut self.rafts {
            raft.cooldown = (raft.cooldown - dt).max(0.0);
            let mut i = 0;
            while i < raft.pieces.len() {
                let body = raft.piece_body(i);
                if body.pos.y < WATER_LINE || body.pos.x.abs() > LANE_SPACING / 2.0 - 0.3 {
                    let at = body.pos;
                    let piece = raft.pieces.remove(i);
                    raft.world.remove(i + 1);
                    raft.lost += 1;
                    events.push(json!({
                        "kind": "splash",
                        "seat": raft.seat,
                        "id": piece.id,
                        "piece": KINDS[piece.kind as usize].name,
                        "x": round(at.x),
                    }));
                } else {
                    i += 1;
                }
            }
            raft.measure(time);
            raft.peak = raft.peak.max(raft.height);
            if raft.height < raft.peak - TOPPLE_DROP {
                events.push(json!({"kind": "topple", "seat": raft.seat, "from": round(raft.peak), "to": round(raft.height)}));
                raft.peak = raft.height;
            }
            raft.peak = (raft.peak - dt * 0.4).max(raft.height);
        }
        if !self.horn && self.time >= PLAY_SECONDS {
            self.horn = true;
            events.push(json!({"kind": "horn"}));
        }
        if self.time >= ROUND_SECONDS {
            self.settle(events);
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(index) = self.rafts.iter().position(|r| r.seat == seat) else {
            return Input::default();
        };
        let Some(bot) = self.brains.iter().position(|(s, _)| *s == seat) else {
            return Input::default();
        };
        let ready = self.building() && self.rafts[index].cooldown <= 0.0;
        if ready && self.brains[bot].1.plan.is_none() && self.content(index, bot) {
            self.brains[bot].1.think = self.brains[bot].1.think.max(0.4);
        } else if ready && self.brains[bot].1.plan.is_none() {
            self.brains[bot].1.think -= dt;
            if self.brains[bot].1.think <= 0.0 {
                // One imagined drop a tick, so thinking never stalls the room.
                if self.brains[bot].1.pending.is_empty() && self.brains[bot].1.rehearsed.is_empty()
                {
                    let (greedy, spots) = self.shortlist(index, bot);
                    let brain = &mut self.brains[bot].1;
                    brain.greedy = greedy;
                    brain.pending = spots;
                } else if self.imagined >= IMAGINED_PER_TICK {
                } else if let Some((_, x, angle)) = self.brains[bot].1.pending.pop() {
                    self.imagined += 1;
                    let value = self.rehearse(index, x, angle, self.brains[bot].1.greedy);
                    self.brains[bot].1.rehearsed.push((value, x, angle));
                } else {
                    let plan = self.commit(bot);
                    let brain = &mut self.brains[bot].1;
                    brain.rehearsed.clear();
                    if plan.is_none() {
                        brain.think = 0.8;
                    }
                    brain.plan = plan;
                    brain.waited = 0.0;
                }
            }
        }
        let roll = self.rafts[index].raft().angle;
        let restless = {
            let raft = &self.rafts[index];
            (0..raft.pieces.len())
                .map(|i| raft.piece_body(i).speed())
                .fold(0.0f32, f32::max)
        };
        let reaction = Skill(self.brains[bot].1.skill).reaction();
        let jitter = self.rng.random_range(0.6..1.4);
        let brain = &mut self.brains[bot].1;
        let pace = 2.5 + 3.5 * brain.skill;
        let mut fire = false;
        if let Some(plan) = brain.plan.clone() {
            let gap = plan.x - brain.hand;
            brain.hand += gap.clamp(-pace * dt, pace * dt);
            let turn = wrap(plan.angle - brain.turn);
            brain.turn += turn.clamp(-5.0 * dt, 5.0 * dt);
            let there =
                (plan.x - brain.hand).abs() < 0.03 && wrap(plan.angle - brain.turn).abs() < 0.02;
            if there {
                brain.waited += dt;
            }
            let steady =
                roll.abs() < plan.calm && restless < plan.settled || brain.waited > plan.patience;
            if ready && (there && steady || plan.fumble && gap.abs() < 0.6) {
                fire = true;
                brain.plan = None;
                brain.think = (0.3 + reaction) * jitter * if plan.greedy { 0.5 } else { 1.0 };
            }
        }
        Input {
            pitch: brain.hand / HOVER_RANGE,
            yaw: brain.turn,
            fire,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let r2 = |v: f32| (v * 100.0).round() / 100.0;
        let pose = self.sea.pose(self.time);
        json!({
            "game": KEY,
            "time": r2(self.time),
            "limit": ROUND_SECONDS,
            "horn": PLAY_SECONDS,
            "building": self.building(),
            "sea": {
                "level": r2(Sea::level(self.time)),
                "swell": self.sea.swelling(self.time),
                "warning": self.sea.warning(self.time).map(r2),
                "roll": round(pose.angle),
            },
            "players": self.rafts.iter().map(|r| {
                let deck = r.raft();
                let lane = self.lane_x(r.lane);
                let held = self.held(r);
                let hold = r.hold(held, r.cursor, r.angle);
                json!({
                    "seat": r.seat,
                    "body_width": r.body_width,
                    "hand_gap": r2(hand_gap(r.body_width)),
                    "drop_interval": r2(drop_interval(r.body_width)),
                    // The player stands on a jetty beside their raft.
                    "x": r2(lane - RAFT_HALF_WIDTH - 1.4),
                    "z": 0.0,
                    "y": r2(0.2 + pose.y * 0.4),
                    "yaw": 0.0,
                    "lane": lane,
                    "tower": r2(r.height),
                    "best": r2(r.best),
                    "lost": r.lost,
                    "placed": r.placed,
                    "score": r2(r.score()),
                    "cursor": r2(r.cursor),
                    "angle": round(r.angle),
                    "held": held,
                    "hold": [round(hold.center().x), round(hold.center().y)],
                    "next": (1..=3).map(|n| self.queue[(r.dealt + n) % self.queue.len()]).collect::<Vec<_>>(),
                    "cooldown": r2(r.cooldown),
                    "raft": [round(deck.pos.x), round(deck.pos.y), round(deck.angle)],
                    "pieces": r.pieces.iter().enumerate().map(|(i, p)| {
                        let b = r.piece_body(i);
                        let c = b.center();
                        json!([p.id, p.kind, round(c.x), round(c.y), round(b.angle), u8::from(self.time - p.standing_at <= STANDING_GRACE)])
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "standings": self.placements,
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests;
