//! Stencil Rush: a glowing outline appears on everyone's metal sheet and they
//! trace it with a cutting torch. Each round brings a new, stranger shape.
//! The torch chases the player's aim point at a capped speed; every bit of
//! the cut is measured against the line, so drifting off it costs accuracy,
//! and a quicker finish is worth more. The best cut takes each round and the
//! most points over the rounds take the bout.
//!
//! The aim travels in `Input::yaw` (sheet x) and `Input::pitch` (sheet y), so
//! the room's pitch clamp of 1.4 is well clear of the sheet's edge.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};
use std::f32::consts::{PI, TAU};

use crate::bout::{Bout, Contestant, Input, Skill, round_form};

pub const KEY: &str = "stencil";
pub const HALF_W: f32 = 1.0;
pub const HALF_H: f32 = 0.7;
/// World units per sheet unit, for the client's benches and the characters.
pub const SHEET_SCALE: f32 = 2.0;
pub const BENCH_SPACING: f32 = 5.6;
pub const ROW_SPACING: f32 = 7.4;
pub const PER_ROW: usize = 4;
pub const ROUNDS: usize = 4;
pub const PREVIEW_SECONDS: f32 = 3.0;
pub const CUT_SECONDS: f32 = 14.0;
pub const TALLY_SECONDS: f32 = 3.0;
pub const TORCH_SPEED: f32 = 1.15;
/// Within this of the line a cut is perfect.
pub const LINE_TOLERANCE: f32 = 0.015;
/// Past this the torch throws sparks.
pub const SPARK_DISTANCE: f32 = 0.04;
/// At and past this a cut is worth nothing.
pub const MISS_DISTANCE: f32 = 0.12;
/// The torch has to be this close to the line for the cut to advance.
pub const CAPTURE: f32 = 0.1;
const SEARCH_AHEAD: f32 = 0.3;
const SEARCH_BEHIND: f32 = 0.08;
/// How far along either way a cut goes before its direction is settled.
const COMMIT: f32 = 0.1;
const FINISH_SLACK: f32 = 0.035;
pub const ROUND_BONUS: u32 = 15;
/// A player whose aim has not moved this long no longer holds the round open
/// once somebody has finished.
const IDLE_SECONDS: f32 = 4.0;
const SAMPLES: usize = 200;
pub const SHAPES: [&str; ROUNDS] = ["blob", "star", "bloom", "gear"];

/// Where a seat's bench stands: up to four to a row, rows stepping back.
pub fn bench(index: usize, count: usize) -> (f32, f32) {
    let row = index / PER_ROW;
    let col = index % PER_ROW;
    let in_row = (count - row * PER_ROW).min(PER_ROW).max(1);
    let x = (col as f32 - (in_row as f32 - 1.0) / 2.0) * BENCH_SPACING;
    (x, -(row as f32) * ROW_SPACING)
}

/// A sheet point in world space on a bench centred at `(bx, bz)`.
pub fn sheet_to_world(bx: f32, bz: f32, x: f32, y: f32) -> (f32, f32) {
    (bx + x * SHEET_SCALE, bz - y * SHEET_SCALE)
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub kind: &'static str,
    pub points: Vec<(f32, f32)>,
    cum: Vec<f32>,
    pub length: f32,
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

impl Shape {
    /// Fit a closed outline into the sheet and resample it evenly.
    fn from_outline(kind: &'static str, raw: Vec<(f32, f32)>, start: f32) -> Shape {
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for p in &raw {
            lo = (lo.0.min(p.0), lo.1.min(p.1));
            hi = (hi.0.max(p.0), hi.1.max(p.1));
        }
        let centre = ((lo.0 + hi.0) / 2.0, (lo.1 + hi.1) / 2.0);
        let scale = (0.82 * HALF_W / ((hi.0 - lo.0) / 2.0).max(1e-3))
            .min(0.8 * HALF_H / ((hi.1 - lo.1) / 2.0).max(1e-3));
        let fitted: Vec<(f32, f32)> = raw
            .iter()
            .map(|p| ((p.0 - centre.0) * scale, (p.1 - centre.1) * scale))
            .collect();
        let mut cum = vec![0.0];
        for i in 0..fitted.len() {
            let next = cum[i] + dist(fitted[i], fitted[(i + 1) % fitted.len()]);
            cum.push(next);
        }
        let total = *cum.last().unwrap();
        let rough = Shape {
            kind,
            points: fitted,
            cum,
            length: total,
        };
        let offset = start.rem_euclid(1.0) * total;
        let points: Vec<(f32, f32)> = (0..SAMPLES)
            .map(|i| rough.at(offset + total * i as f32 / SAMPLES as f32))
            .collect();
        let mut cum = vec![0.0];
        for i in 0..SAMPLES {
            let next = cum[i] + dist(points[i], points[(i + 1) % SAMPLES]);
            cum.push(next);
        }
        let length = *cum.last().unwrap();
        Shape {
            kind,
            points,
            cum,
            length,
        }
    }

    fn segment(&self, s: f32) -> (usize, f32) {
        let s = s.rem_euclid(self.length);
        let i = self.cum.partition_point(|c| *c <= s).saturating_sub(1);
        let i = i.min(self.points.len() - 1);
        let span = (self.cum[i + 1] - self.cum[i]).max(1e-6);
        (i, ((s - self.cum[i]) / span).clamp(0.0, 1.0))
    }

    /// The point `s` along the outline, wrapping round.
    pub fn at(&self, s: f32) -> (f32, f32) {
        let (i, t) = self.segment(s);
        let a = self.points[i];
        let b = self.points[(i + 1) % self.points.len()];
        (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
    }

    pub fn start(&self) -> (f32, f32) {
        self.points[0]
    }

    pub fn tangent(&self, s: f32) -> (f32, f32) {
        let (i, _) = self.segment(s);
        let a = self.points[i];
        let b = self.points[(i + 1) % self.points.len()];
        let len = dist(a, b).max(1e-6);
        ((b.0 - a.0) / len, (b.1 - a.1) / len)
    }

    /// How sharply the outline turns around `s`, in radians.
    pub fn turn(&self, s: f32, span: f32) -> f32 {
        let a = self.tangent(s - span);
        let b = self.tangent(s + span);
        (a.0 * b.0 + a.1 * b.1).clamp(-1.0, 1.0).acos()
    }

    /// The closest point of the outline to `p` whose distance along it lies in
    /// `lo..=hi` (unwrapped, so the window may run past either end), as that
    /// distance along and the gap.
    pub fn nearest(&self, p: (f32, f32), lo: f32, hi: f32) -> (f32, f32) {
        let n = self.points.len();
        let mut best = (lo, f32::MAX);
        let first = (lo / self.length).floor() as i32;
        let last = (hi / self.length).floor() as i32;
        for lap in first..=last {
            let base = lap as f32 * self.length;
            for i in 0..n {
                let (s0, s1) = (base + self.cum[i], base + self.cum[i + 1]);
                if s1 < lo || s0 > hi {
                    continue;
                }
                let a = self.points[i];
                let b = self.points[(i + 1) % n];
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len2 = (dx * dx + dy * dy).max(1e-9);
                let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
                let s = (s0 + t * (s1 - s0)).clamp(lo.max(s0), hi.min(s1));
                let t = ((s - s0) / (s1 - s0).max(1e-9)).clamp(0.0, 1.0);
                let gap = dist(p, (a.0 + dx * t, a.1 + dy * t));
                if gap < best.1 {
                    best = (s, gap);
                }
            }
        }
        best
    }

    pub fn generate(round: usize, rng: &mut StdRng) -> Shape {
        let kind = SHAPES[round % ROUNDS];
        let phase = |rng: &mut StdRng| rng.random_range(0.0..TAU);
        let radial = |f: &dyn Fn(f32) -> f32, stretch: (f32, f32)| -> Vec<(f32, f32)> {
            (0..720)
                .map(|i| {
                    let a = i as f32 / 720.0 * TAU;
                    let r = f(a).max(0.25);
                    (r * a.cos() * stretch.0, r * a.sin() * stretch.1)
                })
                .collect()
        };
        let raw = match kind {
            "blob" => {
                let (a2, p2) = (rng.random_range(0.1..0.2), phase(rng));
                let (a3, p3) = (rng.random_range(0.06..0.14), phase(rng));
                radial(
                    &|a| 1.0 + a2 * (2.0 * a + p2).cos() + a3 * (3.0 * a + p3).cos(),
                    (1.25, 1.0),
                )
            }
            "star" => {
                let points = rng.random_range(5..=7);
                let spin = phase(rng);
                let corners: Vec<(f32, f32)> = (0..points * 2)
                    .map(|j| {
                        let a =
                            spin + j as f32 * PI / points as f32 + rng.random_range(-0.08..0.08);
                        let r = if j % 2 == 0 {
                            rng.random_range(0.9..1.05)
                        } else {
                            rng.random_range(0.42..0.58)
                        };
                        (r * a.cos() * 1.3, r * a.sin())
                    })
                    .collect();
                let mut raw = Vec::new();
                for j in 0..corners.len() {
                    let (a, b) = (corners[j], corners[(j + 1) % corners.len()]);
                    for k in 0..40 {
                        let t = k as f32 / 40.0;
                        raw.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
                    }
                }
                raw
            }
            "bloom" => {
                let (a2, p2) = (rng.random_range(0.08..0.16), phase(rng));
                let k = rng.random_range(5..=8) as f32;
                let (ak, pk) = (rng.random_range(0.1..0.15), phase(rng));
                radial(
                    &|a| 1.0 + a2 * (2.0 * a + p2).cos() + ak * (k * a + pk).cos(),
                    (1.3, 1.0),
                )
            }
            _ => {
                let teeth = rng.random_range(6..=9) as f32;
                let (pt, p2, p3) = (phase(rng), phase(rng), phase(rng));
                let a3 = rng.random_range(0.05..0.12);
                radial(
                    &|a| {
                        1.0 + 0.17 * (5.0 * (teeth * a + pt).sin()).tanh()
                            + 0.12 * (2.0 * a + p2).cos()
                            + a3 * (3.0 * a + p3).cos()
                    },
                    (rng.random_range(1.2..1.45), 1.0),
                )
            }
        };
        Shape::from_outline(kind, raw, rng.random_range(0.0..1.0))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Preview,
    Cut,
    Tally,
    Done,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Stage::Preview => "preview",
            Stage::Cut => "cut",
            Stage::Tally => "tally",
            Stage::Done => "done",
        }
    }

    fn seconds(self) -> f32 {
        match self {
            Stage::Preview => PREVIEW_SECONDS,
            Stage::Cut => CUT_SECONDS,
            Stage::Tally => TALLY_SECONDS,
            Stage::Done => 0.0,
        }
    }
}

/// How a bot's hand is doing on the current outline.
#[derive(Clone, Debug, Default)]
struct Hand {
    /// How far along the outline the bot's eye is.
    lead: f32,
    react: f32,
    pace: f32,
    /// Some rounds a hand is steadier than others.
    shake: f32,
    sway: [(f32, f32); 3],
    /// Running straight on past a corner: from where, which way, how far is left.
    overshoot: Option<((f32, f32), (f32, f32), f32, f32)>,
    checked_to: f32,
    aim: (f32, f32),
    wobble: (f32, f32),
}

#[derive(Clone, Debug)]
pub struct Cutter {
    pub seat: u8,
    pub torch: (f32, f32),
    pub aim: (f32, f32),
    /// Distance cut along the outline in the settled direction.
    pub progress: f32,
    /// 1 along the outline's order, -1 against it, 0 not yet settled.
    pub dir: i8,
    back: f32,
    pub gap: f32,
    pub spark: bool,
    good: f32,
    cut: f32,
    pub finished: Option<f32>,
    moved_at: f32,
    pub points: u32,
    pub total: u32,
    pub wins: u32,
    pub history: Vec<u32>,
    hand: Hand,
}

impl Cutter {
    pub fn accuracy(&self) -> f32 {
        if self.cut <= 1e-4 {
            1.0
        } else {
            self.good / self.cut
        }
    }

    pub fn completion(&self, length: f32) -> f32 {
        if self.finished.is_some() {
            1.0
        } else {
            (self.progress / length).clamp(0.0, 1.0)
        }
    }
}

/// How much a stretch of cut `gap` from the line is worth, from 1 to 0.
pub fn quality(gap: f32) -> f32 {
    if gap <= LINE_TOLERANCE {
        1.0
    } else {
        (1.0 - (gap - LINE_TOLERANCE) / (MISS_DISTANCE - LINE_TOLERANCE)).clamp(0.0, 1.0)
    }
}

/// A round's points: accuracy counts twice over, then how much was cut, then
/// how quickly it was finished.
pub fn round_points(accuracy: f32, completion: f32, finished: Option<f32>, length: f32) -> u32 {
    let fastest = length / TORCH_SPEED;
    let speed = finished
        .map(|t| ((CUT_SECONDS - t) / (CUT_SECONDS - fastest).max(0.5)).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    (100.0 * accuracy * accuracy * completion * (0.6 + 0.4 * speed)).round() as u32
}

pub struct StencilRush {
    pub shapes: Vec<Shape>,
    pub players: Vec<Cutter>,
    pub round: usize,
    pub stage: Stage,
    pub stage_time: f32,
    pub time: f32,
    pub winners: Vec<u8>,
    skills: Vec<(u8, Skill)>,
    seed: u64,
    rng: StdRng,
    placements: Option<Vec<Vec<u8>>>,
}

impl StencilRush {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed ^ 0x57E4_C11);
        let shapes: Vec<Shape> = (0..ROUNDS).map(|r| Shape::generate(r, &mut rng)).collect();
        let start = shapes[0].start();
        let players = contestants
            .iter()
            .map(|c| Cutter {
                seat: c.seat,
                torch: start,
                aim: start,
                progress: 0.0,
                dir: 0,
                back: 0.0,
                gap: 0.0,
                spark: false,
                good: 0.0,
                cut: 0.0,
                finished: None,
                moved_at: 0.0,
                points: 0,
                total: 0,
                wins: 0,
                history: Vec::new(),
                hand: Hand::default(),
            })
            .collect();
        let mut game = StencilRush {
            shapes,
            players,
            round: 0,
            stage: Stage::Preview,
            stage_time: 0.0,
            time: 0.0,
            winners: Vec::new(),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            seed,
            rng,
            placements: None,
        };
        game.deal_round();
        game
    }

    pub fn shape(&self) -> &Shape {
        &self.shapes[self.round.min(ROUNDS - 1)]
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills
            .iter()
            .find(|(s, _)| *s == seat)
            .map(|(_, k)| *k)
            .unwrap_or_default()
    }

    pub fn player(&self, seat: u8) -> Option<&Cutter> {
        self.players.iter().find(|p| p.seat == seat)
    }

    fn deal_round(&mut self) {
        let start = self.shape().start();
        let skills: Vec<Skill> = self
            .players
            .iter()
            .map(|p| {
                let usual = self.skill(p.seat).level();
                Skill(round_form(usual, self.seed, p.seat, self.round as u32))
            })
            .collect();
        for (player, skill) in self.players.iter_mut().zip(skills) {
            player.torch = start;
            player.aim = start;
            player.progress = 0.0;
            player.dir = 0;
            player.back = 0.0;
            player.gap = 0.0;
            player.spark = false;
            player.good = 0.0;
            player.cut = 0.0;
            player.finished = None;
            player.moved_at = 0.0;
            player.points = 0;
            let mut sway = [(0.0, 0.0); 3];
            for (i, s) in sway.iter_mut().enumerate() {
                *s = (
                    self.rng.random_range(3.5..7.0) * [1.0, 1.9, 3.3][i],
                    self.rng.random_range(0.0..TAU),
                );
            }
            player.hand = Hand {
                lead: 0.0,
                react: skill.reaction() * self.rng.random_range(0.7..1.3),
                pace: (0.5 + 0.38 * skill.level()) * self.rng.random_range(0.92..1.06),
                shake: self.rng.random_range(0.6..1.6),
                sway,
                overshoot: None,
                checked_to: 0.0,
                aim: start,
                wobble: (0.0, 0.0),
            };
        }
    }

    fn cut(&mut self, index: usize, input: Option<&Input>, dt: f32, events: &mut Vec<Value>) {
        let stage_time = self.stage_time;
        let shape = &self.shapes[self.round];
        let player = &mut self.players[index];
        if let Some(input) = input {
            let fresh =
                *input != Input::default() && input.yaw.is_finite() && input.pitch.is_finite();
            if fresh {
                let aim = (
                    input.yaw.clamp(-HALF_W, HALF_W),
                    input.pitch.clamp(-HALF_H, HALF_H),
                );
                if dist(aim, player.aim) > 1e-4 {
                    player.moved_at = stage_time;
                }
                player.aim = aim;
            }
        }
        if player.finished.is_some() {
            player.spark = false;
            return;
        }
        let to = (player.aim.0 - player.torch.0, player.aim.1 - player.torch.1);
        let far = to.0.hypot(to.1);
        let step = far.min(TORCH_SPEED * dt);
        if step <= 1e-6 {
            player.spark = player.gap > SPARK_DISTANCE;
            return;
        }
        player.torch = (
            player.torch.0 + to.0 / far * step,
            player.torch.1 + to.1 / far * step,
        );
        let length = shape.length;
        let p = player.torch;
        let forward = shape.nearest(
            p,
            player.progress - SEARCH_BEHIND,
            player.progress + SEARCH_AHEAD,
        );
        let backward = {
            let (s, gap) = shape.nearest(
                p,
                length - (player.back + SEARCH_AHEAD),
                length - (player.back - SEARCH_BEHIND),
            );
            (length - s, gap)
        };
        let gap = match player.dir {
            1 => forward.1,
            -1 => backward.1,
            _ => forward.1.min(backward.1),
        };
        if player.dir >= 0 && forward.1 <= CAPTURE && forward.0 > player.progress {
            player.progress = forward.0.min(length);
        }
        if player.dir <= 0 && backward.1 <= CAPTURE && backward.0 > player.back {
            player.back = backward.0.min(length);
        }
        if player.dir == 0 {
            if player.progress >= COMMIT && player.progress >= player.back {
                player.dir = 1;
            } else if player.back >= COMMIT {
                player.dir = -1;
                player.progress = player.back;
            }
        } else if player.dir == -1 {
            player.progress = player.back;
        }
        player.gap = gap;
        player.spark = gap > SPARK_DISTANCE;
        player.good += quality(gap) * step;
        player.cut += step;
        if player.dir != 0 && player.progress >= length - FINISH_SLACK {
            player.progress = length;
            player.finished = Some(stage_time);
            events.push(json!({
                "kind": "cut_done",
                "seat": player.seat,
                "accuracy": round2(player.accuracy()),
                "time": round2(stage_time),
            }));
        }
    }

    fn round_over(&self) -> bool {
        if self.stage_time >= CUT_SECONDS {
            return true;
        }
        let any_done = self.players.iter().any(|p| p.finished.is_some());
        any_done
            && self
                .players
                .iter()
                .all(|p| p.finished.is_some() || self.stage_time - p.moved_at >= IDLE_SECONDS)
    }

    fn score_round(&mut self, events: &mut Vec<Value>) {
        let length = self.shape().length;
        for p in self.players.iter_mut() {
            p.points = round_points(p.accuracy(), p.completion(length), p.finished, length);
        }
        let best = self.players.iter().map(|p| p.points).max().unwrap_or(0);
        self.winners = if best > 0 {
            self.players
                .iter()
                .filter(|p| p.points == best)
                .map(|p| p.seat)
                .collect()
        } else {
            Vec::new()
        };
        for p in self.players.iter_mut() {
            if self.winners.contains(&p.seat) {
                p.wins += 1;
                p.points += ROUND_BONUS;
            }
            p.total += p.points;
            p.history.push(p.points);
        }
        events.push(json!({
            "kind": "round_over",
            "round": self.round + 1,
            "winners": self.winners,
            "scores": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "points": p.points,
                "accuracy": round2(p.accuracy()),
            })).collect::<Vec<_>>(),
        }));
    }

    fn settle(&mut self) {
        let mut order: Vec<(u32, u32, u8)> = self
            .players
            .iter()
            .map(|p| (p.total, p.wins, p.seat))
            .collect();
        order.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last = None;
        for (total, wins, seat) in order {
            match groups.last_mut() {
                Some(group) if last == Some((total, wins)) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((total, wins));
        }
        self.placements = Some(groups);
    }

    fn advance(&mut self, events: &mut Vec<Value>) {
        match self.stage {
            Stage::Preview if self.stage_time >= PREVIEW_SECONDS => {
                self.stage = Stage::Cut;
                self.stage_time = 0.0;
                events.push(json!({ "kind": "cut_start", "round": self.round + 1 }));
            }
            Stage::Cut if self.round_over() => {
                self.score_round(events);
                self.stage = Stage::Tally;
                self.stage_time = 0.0;
            }
            Stage::Tally if self.stage_time >= TALLY_SECONDS => {
                if self.round + 1 < ROUNDS {
                    self.round += 1;
                    self.deal_round();
                    self.stage = Stage::Preview;
                    self.stage_time = 0.0;
                    self.winners.clear();
                    events.push(json!({ "kind": "new_shape", "round": self.round + 1, "shape": self.shape().kind }));
                } else {
                    self.stage = Stage::Done;
                    self.stage_time = 0.0;
                    self.settle();
                }
            }
            _ => {}
        }
    }

    fn bot_aim(&mut self, index: usize, dt: f32) -> (f32, f32) {
        let skill = self.skill(self.players[index].seat);
        let (stage, stage_time) = (self.stage, self.stage_time);
        let level = skill.level();
        let StencilRush {
            shapes,
            players,
            rng,
            round,
            ..
        } = self;
        let shape = &shapes[*round];
        let p = &players[index];
        let (progress, finished) = (p.progress, p.finished.is_some());
        let mut hand = p.hand.clone();
        if stage != Stage::Cut || finished || stage_time < hand.react {
            return hand.aim;
        }
        let length = shape.length;
        let pace = TORCH_SPEED * hand.pace;
        if let Some((from, way, left, resume)) = hand.overshoot {
            let left = left - pace * 0.8 * dt;
            let run = dist(from, hand.aim) + pace * 0.8 * dt;
            hand.aim = (from.0 + way.0 * run, from.1 + way.1 * run);
            if left <= 0.0 {
                hand.overshoot = None;
                hand.lead = resume;
            } else {
                hand.overshoot = Some((from, way, left, resume));
            }
        } else {
            let corner = shape.turn(hand.lead + 0.03, 0.035);
            let slow = 1.0 - 0.6 * (corner / 1.4).min(1.0) * (1.0 - 0.4 * level);
            if hand.lead < progress + 0.12 {
                hand.lead = (hand.lead + pace * slow * dt).min(length);
            }
            if corner > 0.85 && hand.lead > hand.checked_to {
                hand.checked_to = hand.lead + 0.12;
                let chance = 0.12 + 0.5 * (1.0 - level);
                if rng.random_bool(chance as f64) {
                    let way = shape.tangent(hand.lead);
                    let reach = rng.random_range(0.04..0.1) * (1.25 - level);
                    hand.overshoot = Some((shape.at(hand.lead), way, reach, hand.lead + 0.02));
                }
            }
            let t = stage_time;
            let amp = (0.036 + (1.0 - level) * 0.04) * hand.shake * (0.5 + 0.6 * hand.pace);
            let sway: f32 = hand
                .sway
                .iter()
                .zip([0.6, 0.3, 0.1])
                .map(|((f, ph), w)| (f * t + ph).sin() * w)
                .sum();
            let tremor = rng.random_range(-1.0..1.0) * 0.012 * (1.2 - level);
            let tangent = shape.tangent(hand.lead);
            let normal = (-tangent.1, tangent.0);
            let on = shape.at(hand.lead);
            let off = amp * sway + tremor;
            let ease = (skill.tracking() * 2.5 * dt).min(1.0);
            hand.aim = (
                hand.aim.0 + (on.0 - hand.aim.0) * ease,
                hand.aim.1 + (on.1 - hand.aim.1) * ease,
            );
            if hand.lead >= length {
                hand.aim = on;
            }
            hand.wobble = (normal.0 * off, normal.1 * off);
        }
        let aim = (hand.aim.0 + hand.wobble.0, hand.aim.1 + hand.wobble.1);
        players[index].hand = hand;
        aim
    }
}

fn round2(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

fn round3(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

impl Bout for StencilRush {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        let count = self.players.len();
        json!({
            "game": KEY,
            "rule": "Trace the glowing outline with your torch. Stay on the line and finish fast.",
            "sheet": { "half_w": HALF_W, "half_h": HALF_H, "scale": SHEET_SCALE },
            "tolerance": LINE_TOLERANCE,
            "spark": SPARK_DISTANCE,
            "torch_speed": TORCH_SPEED,
            "rounds": ROUNDS,
            "shapes": self.shapes.iter().map(|s| json!({
                "kind": s.kind,
                "length": round3(s.length),
                "points": s.points.iter().map(|p| [round3(p.0), round3(p.1)]).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "benches": self.players.iter().enumerate().map(|(i, p)| {
                let (x, z) = bench(i, count);
                json!({ "seat": p.seat, "x": x, "z": z })
            }).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.stage_time += dt;
        if self.stage == Stage::Cut {
            for index in 0..self.players.len() {
                let seat = self.players[index].seat;
                let input = inputs.iter().find(|(s, _)| *s == seat).map(|(_, i)| *i);
                self.cut(index, input.as_ref(), dt, events);
            }
        }
        self.advance(events);
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat) else {
            return Input::default();
        };
        let aim = self.bot_aim(index, dt);
        Input {
            yaw: aim.0,
            pitch: aim.1,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let count = self.players.len();
        let length = self.shape().length;
        json!({
            "game": KEY,
            "time": round2(self.stage_time),
            "limit": self.stage.seconds(),
            "round": self.round + 1,
            "rounds": ROUNDS,
            "stage": self.stage.name(),
            "shape": self.shape().kind,
            "winners": self.winners,
            "players": self.players.iter().enumerate().map(|(i, p)| {
                let (bx, bz) = bench(i, count);
                let (x, _) = sheet_to_world(bx, bz, p.torch.0 * 0.6, 0.0);
                json!({
                    "seat": p.seat,
                    "tx": round3(p.torch.0),
                    "ty": round3(p.torch.1),
                    "ax": round3(p.aim.0),
                    "ay": round3(p.aim.1),
                    "progress": round3(p.completion(length)),
                    "dir": p.dir,
                    "accuracy": round3(p.accuracy()),
                    "spark": p.spark,
                    "finished": p.finished.is_some(),
                    "finish_time": p.finished.map(round2),
                    "points": p.points,
                    "total": p.total,
                    "wins": p.wins,
                    "history": p.history,
                    "x": round2(x),
                    "z": round2(bz - HALF_H * SHEET_SCALE - 0.8),
                    "yaw": 0.0,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
#[path = "stencil_tests.rs"]
mod tests;
