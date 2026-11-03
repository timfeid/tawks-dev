//! One Bridge Crew course: rope bridges over a foggy drop, laid out along +z
//! in a lane's own coordinates. Every crew races a copy of the same course on
//! its own lane, so a round is fair however the pieces fall.

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::Serialize;
use serde_json::{Value, json};

pub const START_HALF: f32 = 2.2;
pub const DECK_HALF: f32 = 1.25;
pub const GUST_HALF: f32 = 1.15;
/// How far either side of its lane a course may wander. Lanes sit further
/// apart than twice this so no bridge reaches a neighbour's.
pub const REACH: f32 = 4.6;
/// How much past a plank's edge the cart's centre can hang before it tips.
pub const EDGE: f32 = 0.2;
pub const LIFT_PERIOD: f32 = 5.0;
pub const LIFT_UP: f32 = 2.0;
pub const GUST_CYCLE: f32 = 3.8;
pub const GUST_WARN: f32 = 1.1;
pub const GUST_BLOW: f32 = 1.4;
const FORK_SPREAD: f32 = 3.2;
/// The broken branch of a fork gives out this far along.
const DEAD_SHARE: f32 = 0.68;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Deck,
    /// Gives way a moment after a cart first rolls onto it.
    Weak,
    /// Swings out over the drop and back on a fixed beat.
    Lift,
    /// The branch of a fork that breaks off short.
    Dead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Piece {
    Straight,
    Jog,
    Fork,
    Weak,
    Gust,
    Lift,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub a: (f32, f32),
    pub b: (f32, f32),
    pub half: f32,
    pub kind: Kind,
    pub piece: usize,
    pub next: Vec<usize>,
}

impl Span {
    pub fn length(&self) -> f32 {
        (self.b.0 - self.a.0).hypot(self.b.1 - self.a.1)
    }

    pub fn heading(&self) -> f32 {
        (self.b.0 - self.a.0).atan2(self.b.1 - self.a.1)
    }

    /// Where along the span the point falls (0 at a, 1 at b, clamped) and
    /// how far from the centre line it is.
    pub fn project(&self, x: f32, z: f32) -> (f32, f32) {
        let (dx, dz) = (self.b.0 - self.a.0, self.b.1 - self.a.1);
        let len2 = (dx * dx + dz * dz).max(1e-6);
        let t = (((x - self.a.0) * dx + (z - self.a.1) * dz) / len2).clamp(0.0, 1.0);
        let (px, pz) = (self.a.0 + dx * t, self.a.1 + dz * t);
        (t, (x - px).hypot(z - pz))
    }

    pub fn at(&self, t: f32) -> (f32, f32) {
        (
            self.a.0 + (self.b.0 - self.a.0) * t,
            self.a.1 + (self.b.1 - self.a.1) * t,
        )
    }

    /// Seconds a cart has once it rolls onto a weak span before it gives way:
    /// enough at a steady clip, not enough for a crawl.
    pub fn weak_delay(&self) -> f32 {
        self.length() / 4.2 + 0.25
    }
}

#[derive(Clone, Debug)]
pub struct Fork {
    pub piece: usize,
    pub split: (f32, f32),
    /// Whether the branch that holds is on the driver's left (+x).
    pub good_left: bool,
    pub good: usize,
    pub dead: usize,
}

#[derive(Clone, Debug)]
pub struct Gust {
    pub piece: usize,
    pub z0: f32,
    pub z1: f32,
    offset: f32,
    seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Blow {
    Calm,
    Warn,
    Blow,
}

impl Gust {
    pub fn cycle(&self, time: f32) -> u32 {
        ((time + self.offset) / GUST_CYCLE).floor() as u32
    }

    /// What the wind is doing at `time`, and which way it pushes (+1 is +x).
    pub fn state(&self, time: f32) -> (Blow, f32) {
        let clock = time + self.offset;
        let cycle = (clock / GUST_CYCLE).floor() as u64;
        let into = clock - cycle as f32 * GUST_CYCLE;
        let mix = (self.seed ^ cycle.wrapping_mul(0x9E37_79B9_7F4A_7C15))
            .wrapping_mul(0xBF58_476D_1CE4_E5B9);
        let dir = if (mix >> 33) & 1 == 0 { 1.0 } else { -1.0 };
        let blow = if into < GUST_WARN {
            Blow::Warn
        } else if into < GUST_WARN + GUST_BLOW {
            Blow::Blow
        } else {
            Blow::Calm
        };
        (blow, dir)
    }
}

#[derive(Clone, Debug)]
pub struct Lift {
    pub span: usize,
    pub offset: f32,
}

impl Lift {
    pub fn cycle(&self, time: f32) -> u32 {
        ((time + self.offset) / LIFT_PERIOD).floor() as u32
    }

    pub fn up(&self, time: f32) -> bool {
        (time + self.offset).rem_euclid(LIFT_PERIOD) < LIFT_UP
    }

    /// Seconds until the lift next changes, and whether it is up now.
    pub fn until_change(&self, time: f32) -> (f32, bool) {
        let into = (time + self.offset).rem_euclid(LIFT_PERIOD);
        if into < LIFT_UP {
            (LIFT_UP - into, true)
        } else {
            (LIFT_PERIOD - into, false)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Checkpoint {
    pub x: f32,
    pub z: f32,
    pub heading: f32,
}

#[derive(Clone, Debug)]
pub struct Course {
    pub spans: Vec<Span>,
    pub pieces: Vec<Piece>,
    pub checkpoints: Vec<Checkpoint>,
    pub forks: Vec<Fork>,
    pub gusts: Vec<Gust>,
    pub lifts: Vec<Lift>,
    pub start_z: f32,
    pub finish_z: f32,
}

struct Builder {
    course: Course,
    x: f32,
    z: f32,
    tail: Option<usize>,
}

impl Builder {
    fn span(&mut self, to: (f32, f32), half: f32, kind: Kind) -> usize {
        let index = self.course.spans.len();
        let piece = self.course.pieces.len().saturating_sub(1);
        self.course.spans.push(Span {
            a: (self.x, self.z),
            b: to,
            half,
            kind,
            piece,
            next: Vec::new(),
        });
        if let Some(tail) = self.tail {
            self.course.spans[tail].next.push(index);
        }
        self.tail = Some(index);
        (self.x, self.z) = to;
        index
    }

    fn piece(&mut self, piece: Piece, heading: f32) {
        self.course.pieces.push(piece);
        self.course.checkpoints.push(Checkpoint {
            x: self.x,
            z: self.z,
            heading,
        });
    }

    fn straight(&mut self, length: f32, half: f32, kind: Kind) -> usize {
        let to = (self.x, self.z + length);
        self.span(to, half, kind)
    }
}

impl Course {
    pub fn generate(seed: u64) -> Course {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut b = Builder {
            course: Course {
                spans: Vec::new(),
                pieces: Vec::new(),
                checkpoints: Vec::new(),
                forks: Vec::new(),
                gusts: Vec::new(),
                lifts: Vec::new(),
                start_z: -4.0,
                finish_z: 0.0,
            },
            x: 0.0,
            z: -4.0,
            tail: None,
        };
        b.piece(Piece::Straight, 0.0);
        b.straight(8.0, START_HALF, Kind::Deck);
        let mut middle = vec![
            Piece::Fork,
            Piece::Weak,
            Piece::Gust,
            Piece::Lift,
            if rng.random_bool(0.5) {
                Piece::Fork
            } else {
                Piece::Jog
            },
            Piece::Jog,
        ];
        // Two forks back to back read as one long guess; space them out.
        middle.shuffle(&mut rng);
        while middle
            .windows(2)
            .any(|pair| pair == [Piece::Fork, Piece::Fork])
        {
            middle.shuffle(&mut rng);
        }
        let first = if rng.random_bool(0.5) {
            Piece::Jog
        } else {
            Piece::Straight
        };
        for piece in std::iter::once(first).chain(middle) {
            match piece {
                Piece::Straight => {
                    b.piece(piece, 0.0);
                    b.straight(rng.random_range(8.0..10.5), DECK_HALF, Kind::Deck);
                }
                Piece::Jog => {
                    let mut shift =
                        rng.random_range(2.4..3.4) * if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                    if (b.x + shift).abs() > REACH - DECK_HALF {
                        shift = -shift;
                    }
                    shift = (b.x + shift).clamp(DECK_HALF - REACH, REACH - DECK_HALF) - b.x;
                    let length = rng.random_range(8.5..10.5);
                    b.piece(piece, shift.atan2(length));
                    b.span((b.x + shift, b.z + length), DECK_HALF, Kind::Deck);
                }
                Piece::Weak => {
                    b.piece(piece, 0.0);
                    b.straight(2.0, DECK_HALF, Kind::Deck);
                    b.straight(rng.random_range(6.5..8.0), DECK_HALF, Kind::Weak);
                    b.straight(2.0, DECK_HALF, Kind::Deck);
                }
                Piece::Gust => {
                    b.piece(piece, 0.0);
                    let z0 = b.z;
                    b.straight(rng.random_range(10.0..12.0), GUST_HALF, Kind::Deck);
                    let index = b.course.pieces.len() - 1;
                    b.course.gusts.push(Gust {
                        piece: index,
                        z0,
                        z1: b.z,
                        offset: rng.random_range(0.0..GUST_CYCLE),
                        seed: rng.random(),
                    });
                }
                Piece::Lift => {
                    b.piece(piece, 0.0);
                    b.straight(3.5, DECK_HALF, Kind::Deck);
                    let span = b.straight(3.0, DECK_HALF, Kind::Lift);
                    b.straight(4.0, DECK_HALF, Kind::Deck);
                    b.course.lifts.push(Lift {
                        span,
                        offset: rng.random_range(0.0..LIFT_PERIOD),
                    });
                }
                Piece::Fork => {
                    // Bring the split near the middle so both branches fit.
                    let lead_x = b.x.clamp(-0.1, 0.1);
                    b.piece(piece, (lead_x - b.x).atan2(4.0));
                    b.span((lead_x, b.z + 4.0), DECK_HALF, Kind::Deck);
                    let split = (b.x, b.z);
                    let fork_from = b.tail;
                    let length = rng.random_range(9.5..11.0);
                    let good_left = rng.random_bool(0.5);
                    let side = if good_left { 1.0 } else { -1.0 };
                    let good_end = (split.0 + side * FORK_SPREAD, split.1 + length);
                    let dead_end = (
                        split.0 - side * FORK_SPREAD * DEAD_SHARE,
                        split.1 + length * DEAD_SHARE,
                    );
                    let dead = b.span(dead_end, DECK_HALF, Kind::Dead);
                    (b.x, b.z) = split;
                    b.tail = fork_from;
                    let good = b.span(good_end, DECK_HALF, Kind::Deck);
                    let index = b.course.pieces.len() - 1;
                    b.course.forks.push(Fork {
                        piece: index,
                        split,
                        good_left,
                        good,
                        dead,
                    });
                }
            }
        }
        b.piece(Piece::Straight, 0.0);
        let back = (-b.x).clamp(-2.0, 2.0);
        b.span((b.x + back, b.z + 4.0), DECK_HALF, Kind::Deck);
        let finish_from = b.z;
        b.straight(7.0, START_HALF, Kind::Deck);
        b.course.finish_z = finish_from + 3.0;
        b.course
    }

    pub fn length(&self) -> f32 {
        self.finish_z - self.start_z
    }

    pub fn json(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "start_z": self.start_z,
            "finish_z": round(self.finish_z),
            "spans": self.spans.iter().map(|s| json!({
                "ax": round(s.a.0), "az": round(s.a.1),
                "bx": round(s.b.0), "bz": round(s.b.1),
                "half": s.half, "kind": s.kind, "piece": s.piece,
            })).collect::<Vec<_>>(),
            "pieces": self.pieces,
            "forks": self.forks.iter().map(|f| json!({
                "x": round(f.split.0), "z": round(f.split.1),
                "good": f.good, "dead": f.dead, "good_left": f.good_left,
            })).collect::<Vec<_>>(),
            "gusts": self.gusts.iter().map(|g| json!({ "z0": round(g.z0), "z1": round(g.z1) })).collect::<Vec<_>>(),
            "lifts": self.lifts.iter().map(|l| l.span).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_course_holds_each_hazard_and_stays_in_its_lane() {
        for seed in 0..300 {
            let course = Course::generate(seed);
            for piece in [Piece::Fork, Piece::Weak, Piece::Gust, Piece::Lift] {
                assert!(
                    course.pieces.contains(&piece),
                    "seed {seed} lacks {piece:?}"
                );
            }
            for span in &course.spans {
                for x in [span.a.0, span.b.0] {
                    assert!(
                        x.abs() + span.half <= REACH + 0.01 || span.half == START_HALF,
                        "seed {seed} wanders {x}"
                    );
                }
            }
            assert!(
                (60.0..110.0).contains(&course.length()),
                "seed {seed} length {}",
                course.length()
            );
            for pair in course.pieces.windows(2) {
                assert!(
                    !(pair[0] == Piece::Fork && pair[1] == Piece::Fork),
                    "seed {seed} stacks forks"
                );
            }
        }
    }

    #[test]
    fn the_good_branch_of_a_fork_leads_on_and_the_dead_one_stops() {
        for seed in 0..100 {
            let course = Course::generate(seed);
            for fork in &course.forks {
                assert!(course.spans[fork.dead].next.is_empty());
                assert_eq!(course.spans[fork.good].next.len(), 1);
                let good_x = course.spans[fork.good].b.0 - fork.split.0;
                assert_eq!(good_x > 0.0, fork.good_left);
            }
        }
    }

    #[test]
    fn a_path_runs_from_the_start_to_the_finish() {
        for seed in 0..100 {
            let course = Course::generate(seed);
            let mut at = 0;
            let mut steps = 0;
            while let Some(next) = course.spans[at]
                .next
                .iter()
                .copied()
                .find(|&n| course.spans[n].kind != Kind::Dead)
            {
                at = next;
                steps += 1;
                assert!(steps < 100);
            }
            assert!(
                course.spans[at].b.1 > course.finish_z,
                "seed {seed} never reaches the line"
            );
        }
    }
}
