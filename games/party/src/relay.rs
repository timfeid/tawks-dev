//! Bomb Relay: a lit bomb is carried down a lane through sliding gates and
//! handed on by touching the next runner, who waits in a pen at the far end.
//! They carry it back the other way to whoever is waiting at the start.
//! Whoever holds it when the fuse runs out is out, and each new bomb burns
//! shorter than the last until one runner is left.

use std::cmp::Ordering;
use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, RUNNER_RADIUS, Runner};

pub const KEY: &str = "relay";
pub const HALF_X: f32 = 5.0;
pub const END_Z: f32 = 19.0;
/// How far from the middle each end's pen, and each leg's start, sits.
pub const PEN_Z: f32 = END_Z - 1.6;
pub const PEN_RADIUS: f32 = 1.0;
pub const PASS_RANGE: f32 = 1.5;
/// Each gate as its z and the phase of its sliding gap.
pub const GATES: [(f32, f32); 3] = [(-8.5, 0.0), (0.0, 2.2), (8.5, 4.4)];
pub const GATE_GAP: f32 = 3.4;
pub const GATE_THICK: f32 = 0.6;
const GATE_SWAY: f32 = 2.9;
/// Slow enough that a gap never outruns someone standing in it.
const GATE_RATE: f32 = 1.1;
pub const FIRST_FUSE: f32 = 15.0;
pub const FUSE_SHRINK: f32 = 0.78;
pub const MIN_FUSE: f32 = 6.0;
pub const FUSE_JITTER: f32 = 0.12;
pub const RELIGHT_SECONDS: f32 = 2.0;
const BENCH_X: f32 = HALF_X + 2.5;

/// The fuse a round's bomb is cut to, before a little jitter so nobody can
/// count it down from the start.
pub fn round_fuse(round: u32) -> f32 {
    (FIRST_FUSE * FUSE_SHRINK.powi(round.saturating_sub(1) as i32)).max(MIN_FUSE)
}

/// Where a gate's gap is centred at a moment in the round.
pub fn gate_x(gate: usize, time: f32) -> f32 {
    GATE_SWAY * (time * GATE_RATE + GATES[gate].1).sin()
}

#[derive(Clone, Debug)]
pub struct Relayer {
    pub seat: u8,
    pub run: Runner,
    pub out_at: Option<f32>,
    pub passes: u32,
}

impl Relayer {
    pub fn playing(&self) -> bool {
        self.out_at.is_none()
    }
}

pub struct BombRelay {
    pub players: Vec<Relayer>,
    pub holder: Option<u8>,
    pub receiver: Option<u8>,
    /// Which way the bomb is travelling along z: +1 or -1.
    pub heading: f32,
    pub fuse: f32,
    /// How long this round's fuse was when it was lit.
    pub lit: f32,
    pub round: u32,
    pub time: f32,
    relight: f32,
    /// Who takes the next bomb: whoever was due to take the last one.
    relight_after: Option<u8>,
    limit: f32,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl BombRelay {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let players: Vec<Relayer> = contestants
            .iter()
            .map(|c| Relayer {
                seat: c.seat,
                run: Runner::at(0.0, 0.0).with_width(c.body_width),
                out_at: None,
                passes: 0,
            })
            .collect();
        let rounds = players.len().saturating_sub(1) as u32;
        let limit = (1..=rounds)
            .map(|round| round_fuse(round) * (1.0 + FUSE_JITTER) + RELIGHT_SECONDS)
            .sum::<f32>()
            .ceil()
            .max(RELIGHT_SECONDS);
        let mut rng = StdRng::seed_from_u64(seed);
        let first = players
            .get(rng.random_range(0..players.len().max(1)))
            .map(|p| p.seat);
        let mut game = BombRelay {
            players,
            holder: None,
            receiver: None,
            heading: 1.0,
            fuse: 0.0,
            lit: 0.0,
            round: 0,
            time: 0.0,
            relight: 0.0,
            relight_after: None,
            limit,
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        };
        for index in 0..game.players.len() {
            game.bench(index);
        }
        if let Some(first) = first {
            game.light(first, &mut Vec::new());
        }
        game
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    fn index(&self, seat: u8) -> Option<usize> {
        self.players.iter().position(|p| p.seat == seat)
    }

    pub fn player(&self, seat: u8) -> Option<&Relayer> {
        self.players.iter().find(|p| p.seat == seat)
    }

    fn still_in(&self) -> Vec<u8> {
        self.players
            .iter()
            .filter(|p| p.playing())
            .map(|p| p.seat)
            .collect()
    }

    /// The runner after `seat` in seat order, among those still in.
    fn next_after(&self, seat: u8) -> Option<u8> {
        let left = self.still_in();
        let at = self.players.iter().position(|p| p.seat == seat)?;
        (1..self.players.len())
            .map(|step| self.players[(at + step) % self.players.len()].seat)
            .find(|other| left.contains(other))
    }

    /// Off the lane at the side: waiting runners on one side, the knocked out
    /// on the other.
    fn bench(&mut self, index: usize) {
        let count = self.players.len() as f32;
        let player = &mut self.players[index];
        let side = if player.playing() { 1.0 } else { -1.0 };
        player.run = Runner::at(side * BENCH_X, (index as f32 - (count - 1.0) / 2.0) * 3.0)
            .with_width(player.run.width);
    }

    fn pen(&self) -> (f32, f32) {
        (0.0, self.heading * PEN_Z)
    }

    fn call_up(&mut self, from: u8) {
        self.receiver = self.next_after(from);
        let pen = self.pen();
        if let Some(index) = self.receiver.and_then(|seat| self.index(seat)) {
            self.players[index].run = Runner::at(pen.0, pen.1).with_width(self.players[index].run.width);
        }
    }

    fn light(&mut self, seat: u8, events: &mut Vec<Value>) {
        if self.still_in().len() < 2 {
            return;
        }
        self.round += 1;
        self.heading = if self.round % 2 == 1 { 1.0 } else { -1.0 };
        let jitter = self.rng.random_range(-FUSE_JITTER..=FUSE_JITTER);
        self.fuse = round_fuse(self.round) * (1.0 + jitter);
        self.lit = self.fuse;
        for index in 0..self.players.len() {
            self.bench(index);
        }
        if let Some(index) = self.index(seat) {
            self.players[index].run = Runner::at(0.0, -self.heading * PEN_Z)
                .with_width(self.players[index].run.width);
        }
        self.holder = Some(seat);
        self.call_up(seat);
        events.push(json!({
            "kind": "light",
            "seat": seat,
            "to": self.receiver,
            "fuse": self.fuse,
            "round": self.round,
        }));
    }

    fn try_pass(&mut self, events: &mut Vec<Value>) {
        let (Some(holder), Some(receiver)) = (self.holder, self.receiver) else {
            return;
        };
        let (Some(from), Some(to)) = (self.player(holder), self.player(receiver)) else {
            return;
        };
        if from.run.distance_to(to.run.x, to.run.z) > PASS_RANGE + RUNNER_RADIUS {
            return;
        }
        if let Some(index) = self.index(holder) {
            self.players[index].passes += 1;
            self.bench(index);
        }
        self.holder = Some(receiver);
        self.heading = -self.heading;
        self.call_up(receiver);
        events.push(json!({
            "kind": "pass",
            "from": holder,
            "to": receiver,
            "next": self.receiver,
            "fuse": self.fuse,
        }));
    }

    fn blow_up(&mut self, events: &mut Vec<Value>) {
        let Some(holder) = self.holder.take() else {
            return;
        };
        self.receiver = None;
        let Some(index) = self.index(holder) else {
            return;
        };
        let at = (self.players[index].run.x, self.players[index].run.z);
        self.players[index].out_at = Some(self.time);
        events.push(json!({ "kind": "boom", "seat": holder, "x": at.0, "z": at.1 }));
        for index in 0..self.players.len() {
            self.bench(index);
        }
        self.relight = RELIGHT_SECONDS;
    }

    fn move_runner(&mut self, index: usize, stick: Stick, dt: f32) {
        let time = self.time;
        let carrying = self.holder == Some(self.players[index].seat);
        let pen = self.pen();
        let run = &mut self.players[index].run;
        let before = *run;
        ground::step_runner(run, stick, dt, ground::RUN_SPEED);
        if carrying {
            pass_gates(run, &before, time);
            let (edge_x, edge_z) = (HALF_X - RUNNER_RADIUS, END_Z - RUNNER_RADIUS);
            if run.x.abs() > edge_x {
                run.x = run.x.clamp(-edge_x, edge_x);
                run.vx = 0.0;
            }
            if run.z.abs() > edge_z {
                run.z = run.z.clamp(-edge_z, edge_z);
                run.vz = 0.0;
            }
        } else {
            let (dx, dz) = (run.x - pen.0, run.z - pen.1);
            let away = (dx * dx + dz * dz).sqrt();
            if away > PEN_RADIUS {
                run.x = pen.0 + dx / away * PEN_RADIUS;
                run.z = pen.1 + dz / away * PEN_RADIUS;
                run.vx *= 0.2;
                run.vz *= 0.2;
            }
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() {
            return;
        }
        if self.still_in().len() > 1 && self.time < self.limit {
            return;
        }
        let mut order: Vec<(f32, u8)> = self
            .players
            .iter()
            .map(|p| (p.out_at.unwrap_or(f32::INFINITY), p.seat))
            .collect();
        order.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<f32> = None;
        for (at, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == at || (prev - at).abs() < 1e-3 => {
                    group.push(seat)
                }
                _ => groups.push(vec![seat]),
            }
            last = Some(at);
        }
        self.placements = Some(groups);
    }

    /// Where a carrier should run next: the gap of the next gate ahead, read
    /// ahead of time by a sharp bot, or the pen once the gates are behind.
    fn carry_line(&self, me: &Runner, skill: Skill) -> (f32, f32) {
        let pen = self.pen();
        let band = GATE_THICK / 2.0 + RUNNER_RADIUS;
        let ahead = GATES
            .iter()
            .enumerate()
            .filter(|(_, (z, _))| (z - me.z) * self.heading > -band)
            .min_by(|a, b| {
                ((a.1.0 - me.z) * self.heading)
                    .partial_cmp(&((b.1.0 - me.z) * self.heading))
                    .unwrap_or(Ordering::Equal)
            });
        let Some((gate, (gate_z, _))) = ahead else {
            return pen;
        };
        let to_go = (gate_z - me.z) * self.heading;
        let open = GATE_GAP / 2.0 - RUNNER_RADIUS;
        let now = gate_x(gate, self.time);
        if to_go < 2.5 {
            if (me.x - now).abs() < open - 0.2 || to_go < 0.0 {
                return (now, gate_z + self.heading * 3.0);
            }
            return (now, me.z);
        }
        let lead = to_go / ground::RUN_SPEED * skill.level();
        (gate_x(gate, self.time + lead), *gate_z)
    }
}

/// Gates block a runner who meets the wall and carry along one already in the
/// gap, so a sliding gap never pins anyone.
fn pass_gates(run: &mut Runner, before: &Runner, time: f32) {
    let band = GATE_THICK / 2.0 + RUNNER_RADIUS;
    let open = GATE_GAP / 2.0 - RUNNER_RADIUS;
    for (gate, (gate_z, _)) in GATES.iter().enumerate() {
        if (run.z - gate_z).abs() >= band {
            continue;
        }
        let gap = gate_x(gate, time);
        if (run.x - gap).abs() <= open {
            continue;
        }
        if (before.z - gate_z).abs() < band {
            run.x = run.x.clamp(gap - open, gap + open);
            run.vx = 0.0;
        } else {
            let side = if before.z < *gate_z { -1.0 } else { 1.0 };
            run.z = gate_z + side * band;
            run.vz = 0.0;
        }
    }
}

impl Bout for BombRelay {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rule": "Carry the bomb through the gates and touch the runner waiting at the end. Holding it when the fuse ends puts you out.",
            "half_x": HALF_X,
            "end_z": END_Z,
            "pen_z": PEN_Z,
            "pen_radius": PEN_RADIUS,
            "gate_gap": GATE_GAP,
            "gate_thick": GATE_THICK,
            "gates": GATES.iter().map(|(z, _)| *z).collect::<Vec<_>>(),
            "bench_x": BENCH_X,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        let moving = [self.holder, self.receiver];
        for seat in moving.into_iter().flatten() {
            let Some(index) = self.index(seat) else {
                continue;
            };
            let stick = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| input.stick)
                .unwrap_or_default();
            self.move_runner(index, stick, dt);
        }

        if self.holder.is_some() {
            self.try_pass(events);
            self.fuse -= dt;
            if self.fuse <= 0.0 {
                let due = self.receiver;
                self.blow_up(events);
                self.relight_after = due;
            }
        } else if self.relight > 0.0 {
            self.relight -= dt;
            if self.relight <= 0.0 {
                let after = self.relight_after.take();
                let start = after
                    .filter(|seat| self.player(*seat).is_some_and(Relayer::playing))
                    .or_else(|| self.still_in().first().copied());
                if let Some(seat) = start {
                    self.light(seat, events);
                }
            }
        }

        self.settle();
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.player(seat).map(|p| p.run) else {
            return Input::default();
        };
        let skill = self.skill(seat);
        let stick = if self.holder == Some(seat) {
            let line = self.carry_line(&me, skill);
            let toward = ground::steer((me.x, me.z), line);
            let wobble = (1.0 - skill.level()) * 0.35;
            Stick {
                x: (toward.x + self.rng.random_range(-wobble..=wobble)) * skill.pace(),
                z: toward.z * skill.pace(),
            }
        } else if self.receiver == Some(seat) {
            let pen = self.pen();
            let toward = ground::steer((me.x, me.z), pen);
            if me.distance_to(pen.0, pen.1) > 0.3 {
                toward
            } else {
                Stick::default()
            }
        } else {
            Stick::default()
        };
        Input::from_stick(stick.clamped())
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": self.limit,
            "holder": self.holder,
            "receiver": self.receiver,
            "fuse": round(self.fuse.max(0.0)),
            "lit": round(self.lit),
            "round": self.round,
            "heading": self.heading,
            "gates": (0..GATES.len()).map(|gate| round(gate_x(gate, self.time))).collect::<Vec<_>>(),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "out": !p.playing(),
                "holding": self.holder == Some(p.seat),
                "passes": p.passes,
                "waiting": p.playing()
                    && self.holder != Some(p.seat)
                    && self.receiver != Some(p.seat),
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
