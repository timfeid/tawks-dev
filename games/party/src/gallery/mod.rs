//! Ghost in the Gallery: one player is an invisible ghost lifting paintings
//! off the walls of a dark museum, everyone else is a guard with a flashlight.
//! Lifting a painting takes a few seconds of holding still at its frame, and
//! the frame sways and creaks the whole time. The ghost drifts through the
//! inside walls, only shows where a beam falls on it, and a beam held on it
//! long enough catches it. Three
//! paintings before dawn wins it for the ghost; a catch or dawn wins it for
//! the guards. Guards are never sent the ghost's position unless it is lit.

mod bots;
mod map;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick, crowd_count, crowd_scale};
use crate::ground::{self, Runner};
pub use map::{Floor, Hook, Wall};

pub const KEY: &str = "ghost";
pub const ROUND_SECONDS: f32 = 75.0;
pub const GOAL: u32 = 3;
/// How long a painting takes to lift against three guards; quicker hands against more.
pub const STEAL_SECONDS: f32 = 2.5;
/// How far from a frame the ghost can reach it.
pub const STEAL_REACH: f32 = 1.6;
/// Progress a half-lifted painting loses each second nobody holds it.
pub const STEAL_DECAY: f32 = 0.12;
pub const CATCH_SECONDS: f32 = 1.5;
/// How fast the catch meter drains once the ghost is out of the light, per second.
pub const CATCH_DECAY: f32 = 1.2;
/// Each extra beam on the ghost adds this much to the fill rate.
pub const EXTRA_BEAM: f32 = 0.2;
pub const BEAM_RANGE: f32 = 8.5;
pub const BEAM_HALF_ANGLE: f32 = 0.3;
/// A beam shows the ghost all the way down, but only catches it this close.
pub const CATCH_RANGE: f32 = 6.0;
pub const GHOST_RADIUS: f32 = 0.4;
pub const GHOST_SPEED: f32 = 6.0;
/// A startled ghost bolts: it moves this much faster while lit.
pub const LIT_BOLT: f32 = 1.2;
/// How fast a flashlight can swing, in radians a second. A heavy lamp is
/// what lets a ghost slip round a guard's shoulder up close.
pub const BEAM_TURN: f32 = 2.8;
/// The ghost drifts through the inside walls, slowly.
pub const WALL_DRAG: f32 = 0.5;
pub const GUARD_SPEED: f32 = 4.6;
pub const RATTLE_COOLDOWN: f32 = 8.0;
pub const RATTLE_RANGE: f32 = 16.0;
pub const RATTLE_CONE: f32 = 0.6;
pub const SWAY_SECONDS: f32 = 1.4;
pub const CREAK_EVERY: f32 = 0.8;
/// Guards feel a chill this close to the ghost, without learning which way it is.
pub const CHILL_RANGE: f32 = 3.5;
/// A seat with no input this long has left the bout.
const GONE_AFTER: f32 = 0.5;
const PAINTINGS_FOR_FOUR: usize = 8;

const TITLES: [&str; 18] = [
    "Pear in Moonlight",
    "The Patient Heron",
    "Lighthouse, Thursday",
    "Portrait of a Nervous Duke",
    "Three Teacups",
    "Fog Over the Mill",
    "Lady With a Ferret",
    "Storm at Breakfast",
    "The Blue Accordion",
    "Orchard After Rain",
    "Self Portrait as a Turnip",
    "Night Ferry",
    "Mountain With Opinions",
    "The Last Sardine",
    "Sunflowers, Mostly",
    "Bridge of Small Regrets",
    "The Card Players' Cat",
    "Still Life With Spoon",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ghost,
    Guards,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    pub winner: Side,
    pub reason: &'static str,
}

#[derive(Clone, Debug)]
pub struct Painting {
    pub hook: Hook,
    pub art: u8,
    pub title: &'static str,
    pub stolen: bool,
    pub progress: f32,
    pub sway: f32,
    creak: f32,
}

impl Painting {
    /// Where the ghost stands to lift it.
    pub fn spot(&self) -> (f32, f32) {
        (
            self.hook.x + self.hook.nx * 0.9,
            self.hook.z + self.hook.nz * 0.9,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Guard {
    pub seat: u8,
    pub run: Runner,
    pub yaw: f32,
    pub chill: f32,
    pub on_ghost: bool,
    missing: f32,
}

impl Guard {
    pub fn here(&self) -> bool {
        self.missing < GONE_AFTER
    }
}

#[derive(Clone, Debug)]
pub struct Ghost {
    pub seat: u8,
    pub run: Runner,
    pub yaw: f32,
    pub lit: bool,
    pub meter: f32,
    pub stealing: Option<usize>,
    pub stolen: u32,
    pub rattle: f32,
    missing: f32,
}

/// Who a view is for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Viewer {
    Ghost,
    Guard(u8),
    Spectator,
}

pub struct GhostGallery {
    pub scale: f32,
    pub floor: Floor,
    pub paintings: Vec<Painting>,
    pub ghost: Option<Ghost>,
    pub guards: Vec<Guard>,
    pub time: f32,
    pub outcome: Option<Outcome>,
    seats: Vec<u8>,
    rng: StdRng,
    skills: HashMap<u8, Skill>,
    /// Creaks so far, frame and time, as any guard could hear them.
    creaks: Vec<(usize, f32)>,
    brains: HashMap<u8, bots::Brain>,
    placements: Option<Vec<Vec<u8>>>,
}

pub fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

pub fn heading(from: (f32, f32), to: (f32, f32)) -> f32 {
    (to.0 - from.0).atan2(to.1 - from.1)
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn round(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

impl GhostGallery {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let scale = crowd_scale(contestants.len());
        let floor = Floor::new(scale);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut hooks = map::hooks(scale);
        hooks.shuffle(&mut rng);
        let count = crowd_count(PAINTINGS_FOR_FOUR, contestants.len()).min(hooks.len());
        let mut arts: Vec<usize> = (0..TITLES.len()).collect();
        arts.shuffle(&mut rng);
        let paintings = hooks
            .into_iter()
            .take(count)
            .zip(arts)
            .map(|(hook, art)| Painting {
                hook,
                art: art as u8,
                title: TITLES[art],
                stolen: false,
                progress: 0.0,
                sway: 0.0,
                creak: 0.0,
            })
            .collect();
        let mut gallery = GhostGallery {
            scale,
            floor,
            paintings,
            ghost: None,
            guards: Vec::new(),
            time: 0.0,
            outcome: None,
            seats: contestants.iter().map(|c| c.seat).collect(),
            rng,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            creaks: Vec::new(),
            brains: HashMap::new(),
            placements: None,
        };
        let first = if gallery.seats.is_empty() {
            None
        } else {
            Some(gallery.seats[gallery.rng.random_range(0..gallery.seats.len())])
        };
        gallery.cast(first);
        gallery
    }

    /// Hand the ghost to `ghost` and put everyone on their starting spots.
    fn cast(&mut self, ghost: Option<u8>) {
        let s = self.scale;
        let corners = [(-15.0, 7.5), (15.0, 7.5)];
        let corner = corners[self.rng.random_range(0..corners.len())];
        self.ghost = ghost.map(|seat| Ghost {
            seat,
            run: Runner::at(corner.0 * s, corner.1 * s),
            yaw: heading(corner, (0.0, 0.0)),
            lit: false,
            meter: 0.0,
            stealing: None,
            stolen: 0,
            rattle: RATTLE_COOLDOWN * 0.5,
            missing: 0.0,
        });
        let guards: Vec<u8> = self
            .seats
            .iter()
            .copied()
            .filter(|seat| Some(*seat) != ghost)
            .collect();
        let count = guards.len().max(1) as f32;
        self.guards = guards
            .into_iter()
            .enumerate()
            .map(|(i, seat)| Guard {
                seat,
                run: Runner::at((i as f32 - (count - 1.0) / 2.0) * 1.6 * s, -8.0 * s),
                yaw: 0.0,
                chill: 0.0,
                on_ghost: false,
                missing: 0.0,
            })
            .collect();
        self.brains.clear();
    }

    pub fn ghost_seat(&self) -> Option<u8> {
        self.ghost.as_ref().map(|g| g.seat)
    }

    pub fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    pub fn steal_seconds(&self) -> f32 {
        let guards = self.guards.len().max(1) as f32;
        let ease = if guards < 3.0 { 0.6 } else { 0.35 };
        STEAL_SECONDS * (3.0 / guards).powf(ease)
    }

    pub fn goal(&self) -> u32 {
        GOAL
    }

    pub fn limit(&self) -> f32 {
        ROUND_SECONDS
    }

    /// Whether a guard's flashlight falls on a point.
    pub fn beam_covers(&self, guard: &Guard, at: (f32, f32)) -> bool {
        let from = (guard.run.x, guard.run.z);
        let away = distance(from, at);
        if away > BEAM_RANGE + GHOST_RADIUS {
            return false;
        }
        if away > 0.6 {
            let off = wrap(heading(from, at) - guard.yaw).abs();
            if off > BEAM_HALF_ANGLE + (GHOST_RADIUS / away).atan() {
                return false;
            }
        }
        self.floor.sees(from, at)
    }

    /// The frame a ghost at `at` could lift right now.
    pub fn reachable(&self, at: (f32, f32)) -> Option<usize> {
        self.paintings
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.stolen)
            .filter(|(_, p)| {
                let (dx, dz) = (at.0 - p.hook.x, at.1 - p.hook.z);
                dx * p.hook.nx + dz * p.hook.nz > 0.2
                    && distance(at, (p.hook.x, p.hook.z)) <= STEAL_REACH
            })
            .min_by(|a, b| {
                let da = distance(at, (a.1.hook.x, a.1.hook.z));
                let db = distance(at, (b.1.hook.x, b.1.hook.z));
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
    }

    /// The hanging frame a rattle aimed along `yaw` from `at` would shake.
    pub fn rattle_target(&self, at: (f32, f32), yaw: f32) -> Option<usize> {
        self.paintings
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.stolen)
            .map(|(i, p)| {
                let target = (p.hook.x, p.hook.z);
                (
                    i,
                    distance(at, target),
                    wrap(heading(at, target) - yaw).abs(),
                )
            })
            .filter(|(_, d, off)| *d <= RATTLE_RANGE && (*off <= RATTLE_CONE || *d < 2.0))
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _, _)| i)
    }

    fn creak(&mut self, frame: usize, events: &mut Vec<Value>) {
        self.creaks.push((frame, self.time));
        events.push(json!({ "kind": "creak", "frame": frame }));
    }

    fn step_ghost(&mut self, input: Option<&Input>, dt: f32, events: &mut Vec<Value>) {
        let Some(mut ghost) = self.ghost.take() else {
            return;
        };
        match input {
            Some(_) => ghost.missing = 0.0,
            None => ghost.missing += dt,
        }
        let input = input.copied().unwrap_or_default();
        if input.yaw.is_finite() {
            ghost.yaw = wrap(input.yaw);
        }
        ghost.rattle = (ghost.rattle - dt).max(0.0);
        let at = (ghost.run.x, ghost.run.z);
        if input.fire && ghost.rattle <= 0.0 {
            if let Some(frame) = self.rattle_target(at, ghost.yaw) {
                ghost.rattle = RATTLE_COOLDOWN;
                self.paintings[frame].sway = 1.0;
                self.creak(frame, events);
            }
        }
        ghost.stealing = if input.crouch {
            self.reachable(at)
        } else {
            None
        };
        if let Some(frame) = ghost.stealing {
            let lift_seconds = self.steal_seconds();
            ghost.run.vx = 0.0;
            ghost.run.vz = 0.0;
            let painting = &mut self.paintings[frame];
            painting.progress += dt / lift_seconds;
            painting.sway = 1.0;
            painting.creak -= dt;
            let creak = painting.creak <= 0.0;
            if creak {
                painting.creak = CREAK_EVERY;
            }
            if painting.progress >= 1.0 {
                painting.progress = 1.0;
                painting.stolen = true;
                painting.sway = 0.0;
                ghost.stolen += 1;
                ghost.stealing = None;
                events.push(json!({
                    "kind": "stolen",
                    "frame": frame,
                    "seat": ghost.seat,
                    "count": ghost.stolen,
                    "goal": GOAL,
                    "title": painting.title,
                }));
            } else if creak {
                self.creak(frame, events);
            }
        } else {
            let thick = if self.floor.in_wall(ghost.run.x, ghost.run.z) {
                WALL_DRAG
            } else {
                1.0
            };
            let speed = GHOST_SPEED * thick * if ghost.lit { LIT_BOLT } else { 1.0 };
            ground::step_runner(&mut ghost.run, input.stick, dt, speed);
            let run = &mut ghost.run;
            self.floor
                .hold_inside(&mut run.x, &mut run.z, &mut run.vx, &mut run.vz);
        }
        for (i, painting) in self.paintings.iter_mut().enumerate() {
            if ghost.stealing == Some(i) || painting.stolen {
                continue;
            }
            painting.progress = (painting.progress - STEAL_DECAY * dt).max(0.0);
            painting.sway = (painting.sway - dt / SWAY_SECONDS).max(0.0);
            painting.creak = 0.0;
        }
        self.ghost = Some(ghost);
    }

    fn step_guards(&mut self, inputs: &[(u8, Input)], dt: f32) {
        for guard in &mut self.guards {
            let input = inputs
                .iter()
                .find(|(seat, _)| *seat == guard.seat)
                .map(|(_, i)| *i);
            match input {
                Some(_) => guard.missing = 0.0,
                None => guard.missing += dt,
            }
            if !guard.here() {
                guard.run.vx = 0.0;
                guard.run.vz = 0.0;
                continue;
            }
            let input = input.unwrap_or_default();
            if input.yaw.is_finite() {
                let swing = wrap(input.yaw - guard.yaw).clamp(-BEAM_TURN * dt, BEAM_TURN * dt);
                guard.yaw = wrap(guard.yaw + swing);
            }
            ground::step_runner(&mut guard.run, input.stick, dt, GUARD_SPEED);
            let run = &mut guard.run;
            self.floor
                .collide(&mut run.x, &mut run.z, &mut run.vx, &mut run.vz);
        }
        let mut runners: Vec<Runner> = self.guards.iter().map(|g| g.run).collect();
        ground::separate(&mut runners);
        for (guard, mut run) in self.guards.iter_mut().zip(runners) {
            self.floor
                .collide(&mut run.x, &mut run.z, &mut run.vx, &mut run.vz);
            guard.run = run;
        }
    }

    fn light(&mut self, dt: f32, events: &mut Vec<Value>) {
        let Some(at) = self.ghost.as_ref().map(|g| (g.run.x, g.run.z)) else {
            return;
        };
        let covered: Vec<bool> = self
            .guards
            .iter()
            .map(|g| g.here() && self.beam_covers(g, at))
            .collect();
        let beams = self
            .guards
            .iter()
            .zip(&covered)
            .filter(|(g, on)| **on && distance((g.run.x, g.run.z), at) <= CATCH_RANGE)
            .count();
        let shown = covered.iter().any(|c| *c);
        for (guard, on) in self.guards.iter_mut().zip(&covered) {
            guard.on_ghost = *on;
            let away = distance((guard.run.x, guard.run.z), at);
            guard.chill = if guard.here() {
                (1.0 - away / CHILL_RANGE).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        let ghost = self.ghost.as_mut().unwrap();
        let was_lit = ghost.lit;
        ghost.lit = shown;
        if beams > 0 {
            ghost.meter += dt / CATCH_SECONDS * (1.0 + EXTRA_BEAM * (beams - 1) as f32);
        } else {
            ghost.meter = (ghost.meter - CATCH_DECAY * dt).max(0.0);
        }
        if ghost.lit && !was_lit {
            events.push(json!({ "kind": "spotted", "x": round(at.0), "z": round(at.1) }));
        }
    }

    fn settle(&mut self, events: &mut Vec<Value>) {
        if self.outcome.is_some() {
            return;
        }
        let Some(ghost) = self.ghost.as_ref() else {
            self.finish(Side::Guards, "empty");
            return;
        };
        let guards_here = self.guards.iter().any(Guard::here);
        let outcome = if ghost.stolen >= GOAL {
            Some((Side::Ghost, "stole"))
        } else if ghost.meter >= 1.0 {
            Some((Side::Guards, "caught"))
        } else if ghost.missing >= GONE_AFTER {
            Some((Side::Guards, "fled"))
        } else if !guards_here {
            Some((Side::Ghost, "unguarded"))
        } else if self.time >= ROUND_SECONDS {
            Some((Side::Guards, "dawn"))
        } else {
            None
        };
        let Some((winner, reason)) = outcome else {
            return;
        };
        let seat = ghost.seat;
        let by: Vec<u8> = self
            .guards
            .iter()
            .filter(|g| g.on_ghost)
            .map(|g| g.seat)
            .collect();
        let (x, z) = (round(ghost.run.x), round(ghost.run.z));
        events.push(match reason {
            "caught" => json!({ "kind": "caught", "seat": seat, "by": by, "x": x, "z": z }),
            "stole" => json!({ "kind": "escaped", "seat": seat, "x": x, "z": z }),
            _ => json!({ "kind": "over", "winner": side_name(winner), "reason": reason }),
        });
        self.finish(winner, reason);
    }

    fn finish(&mut self, winner: Side, reason: &'static str) {
        self.outcome = Some(Outcome { winner, reason });
        let ghost: Vec<u8> = self.ghost.iter().map(|g| g.seat).collect();
        let here: Vec<u8> = self
            .guards
            .iter()
            .filter(|g| g.here())
            .map(|g| g.seat)
            .collect();
        let left: Vec<u8> = self
            .guards
            .iter()
            .filter(|g| !g.here())
            .map(|g| g.seat)
            .collect();
        let mut tiers = match winner {
            Side::Ghost => vec![ghost, here],
            Side::Guards => vec![here, ghost],
        };
        tiers.push(left);
        tiers.retain(|tier| !tier.is_empty());
        self.placements = Some(tiers);
    }

    pub fn view(&self, viewer: Viewer) -> Value {
        let sees_ghost = |g: &Ghost| match viewer {
            Viewer::Ghost | Viewer::Spectator => true,
            Viewer::Guard(_) => g.lit,
        };
        let ghost = self.ghost.as_ref().map(|g| {
            let mut value = json!({
                "seat": g.seat,
                "lit": g.lit,
                "meter": round(g.meter.min(1.0)),
                "stolen": g.stolen,
                "visible": sees_ghost(g),
            });
            if sees_ghost(g) {
                value["x"] = json!(round(g.run.x));
                value["z"] = json!(round(g.run.z));
                value["vx"] = json!(round(g.run.vx));
                value["vz"] = json!(round(g.run.vz));
                value["yaw"] = json!(round(g.yaw));
            }
            if matches!(viewer, Viewer::Ghost | Viewer::Spectator) {
                value["stealing"] = json!(g.stealing);
                value["rattle"] = json!(round(g.rattle));
                value["reach"] = json!(self.reachable((g.run.x, g.run.z)));
            }
            value
        });
        let secret = matches!(viewer, Viewer::Guard(_));
        let you = match viewer {
            Viewer::Ghost => "ghost",
            Viewer::Guard(_) => "guard",
            Viewer::Spectator => "spectator",
        };
        let chill = match viewer {
            Viewer::Guard(seat) => self
                .guards
                .iter()
                .find(|g| g.seat == seat)
                .map_or(0.0, |g| g.chill),
            _ => 0.0,
        };
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "you": you,
            "goal": GOAL,
            "chill": round(chill),
            "ghost": ghost,
            "players": self.guards.iter().filter(|g| g.here()).map(|g| json!({
                "seat": g.seat,
                "x": round(g.run.x),
                "z": round(g.run.z),
                "vx": round(g.run.vx),
                "vz": round(g.run.vz),
                "yaw": round(g.yaw),
                "on_ghost": g.on_ghost,
            })).collect::<Vec<_>>(),
            "paintings": self.paintings.iter().map(|p| {
                let mut value = json!({
                    "stolen": p.stolen,
                    "sway": round(p.sway),
                });
                if !secret {
                    value["progress"] = json!(round(p.progress));
                }
                value
            }).collect::<Vec<_>>(),
            "result": self.outcome.as_ref().map(|o| json!({
                "winner": side_name(o.winner),
                "reason": o.reason,
            })),
        })
    }

    fn viewer(&self, seat: Option<u8>) -> Viewer {
        match seat {
            Some(seat) if Some(seat) == self.ghost_seat() => Viewer::Ghost,
            Some(seat) if self.seats.contains(&seat) => Viewer::Guard(seat),
            _ => Viewer::Spectator,
        }
    }
}

pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::Ghost => "ghost",
        Side::Guards => "guards",
    }
}

/// Who should be the ghost: whoever has haunted least this party, and of
/// them whoever haunted longest ago, so the role goes round.
pub fn next_ghost(seats: &[u8], earlier: &[u8]) -> Vec<u8> {
    let turns = |seat: u8| earlier.iter().filter(|s| **s == seat).count();
    let last = |seat: u8| {
        earlier
            .iter()
            .rposition(|s| *s == seat)
            .map_or(-1, |i| i as i64)
    };
    let fewest = seats.iter().map(|s| turns(*s)).min().unwrap_or(0);
    let candidates: Vec<u8> = seats
        .iter()
        .copied()
        .filter(|s| turns(*s) == fewest)
        .collect();
    let oldest = candidates.iter().map(|s| last(*s)).min().unwrap_or(-1);
    candidates
        .into_iter()
        .filter(|s| last(*s) == oldest)
        .collect()
}

impl Bout for GhostGallery {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "half_x": self.floor.half_x,
            "half_z": self.floor.half_z,
            "thick": map::THICK,
            "walls": self.floor.walls.iter().map(|w| json!({
                "x": w.x, "z": w.z, "hx": w.hx, "hz": w.hz,
            })).collect::<Vec<_>>(),
            "paintings": self.paintings.iter().map(|p| json!({
                "x": p.hook.x, "z": p.hook.z, "nx": p.hook.nx, "nz": p.hook.nz,
                "art": p.art, "title": p.title,
            })).collect::<Vec<_>>(),
            "beam": { "range": BEAM_RANGE, "half_angle": BEAM_HALF_ANGLE },
            "steal_reach": STEAL_REACH,
            "steal_seconds": self.steal_seconds(),
            "rattle_cooldown": RATTLE_COOLDOWN,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        let ghost_input = self
            .ghost_seat()
            .and_then(|seat| inputs.iter().find(|(s, _)| *s == seat).map(|(_, i)| i));
        let ghost_input = ghost_input.copied();
        self.step_ghost(ghost_input.as_ref(), dt, events);
        self.step_guards(inputs, dt);
        self.light(dt, events);
        self.settle(events);
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        if self.placements.is_some() {
            return Input::default();
        }
        if Some(seat) == self.ghost_seat() {
            self.ghost_bot(seat, dt)
        } else if self.guards.iter().any(|g| g.seat == seat) {
            self.guard_bot(seat, dt)
        } else {
            Input {
                stick: Stick::default(),
                ..Default::default()
            }
        }
    }

    fn snapshot(&self) -> Value {
        self.view(Viewer::Spectator)
    }

    fn snapshot_for(&self, seat: Option<u8>) -> Option<Value> {
        Some(self.view(self.viewer(seat)))
    }

    fn lead(&self) -> Option<u8> {
        self.ghost_seat()
    }

    fn rotate_lead(&mut self, earlier: &[u8]) {
        let choices = next_ghost(&self.seats, earlier);
        if choices.is_empty()
            || self
                .ghost_seat()
                .is_some_and(|seat| choices.contains(&seat))
        {
            return;
        }
        let pick = choices[self.rng.random_range(0..choices.len())];
        self.cast(Some(pick));
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
