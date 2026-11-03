//! Bots for both sides. Guards only use what a guard could know: the ghost
//! where a beam lights it, the creaks they hear, swaying frames and the chill
//! when it is close. The ghost knows where the guards are, as a person playing
//! it would from their screen.

use rand::Rng;

use super::{
    BEAM_HALF_ANGLE, BEAM_RANGE, CATCH_RANGE, GhostGallery, RATTLE_RANGE, distance, heading, wrap,
};
use crate::bout::{Input, Stick};
use crate::ground;

#[derive(Clone, Debug, Default)]
pub struct Brain {
    think: f32,
    route: Vec<(f32, f32)>,
    goal: Option<(f32, f32)>,
    replan: f32,
    aim: Option<f32>,
    noticed: Option<f32>,
    seen: Option<((f32, f32), f32)>,
    errand: Option<(usize, f32)>,
    patrol: usize,
    dwell: f32,
    heard: usize,
    sweep: f32,
    target: Option<usize>,
    escape: Option<(f32, f32)>,
    flee_until: f32,
    bait: Option<usize>,
}

fn sweep(brain: &Brain, width: f32, rate: f32) -> f32 {
    (brain.sweep * rate).sin() * width
}

impl GhostGallery {
    /// Walk toward `goal`; `through` takes the ghost's straight line through the walls.
    fn follow(
        &self,
        brain: &mut Brain,
        at: (f32, f32),
        goal: (f32, f32),
        dt: f32,
        through: bool,
    ) -> Stick {
        brain.replan -= dt;
        let moved = brain.goal.is_none_or(|g| distance(g, goal) > 1.0);
        if moved || brain.route.is_empty() || brain.replan <= 0.0 {
            brain.route = if through {
                vec![goal]
            } else {
                self.floor.route(at, goal)
            };
            brain.goal = Some(goal);
            brain.replan = 0.8;
        }
        while brain.route.len() > 1 && distance(at, brain.route[0]) < 0.6 {
            brain.route.remove(0);
        }
        match brain.route.first() {
            Some(next) if distance(at, *next) > 0.15 => {
                let stick = ground::steer(at, *next);
                let near = distance(at, goal);
                let ease = if brain.route.len() == 1 {
                    (near / 1.2).clamp(0.25, 1.0)
                } else {
                    1.0
                };
                Stick {
                    x: stick.x * ease,
                    z: stick.z * ease,
                }
            }
            _ => Stick::default(),
        }
    }

    fn turn(brain: &mut Brain, look: f32, rate: f32, dt: f32) -> f32 {
        let aim = brain.aim.unwrap_or(look);
        let step = wrap(look - aim).clamp(-rate * dt, rate * dt);
        let aim = wrap(aim + step);
        brain.aim = Some(aim);
        aim
    }

    /// Whether this guard is one of the two nearest to a creaking frame, so
    /// the whole team does not run at every noise.
    fn answers(&self, seat: u8, frame: usize) -> bool {
        let hook = &self.paintings[frame].hook;
        let mut near: Vec<(u8, f32)> = self
            .guards
            .iter()
            .filter(|g| g.here())
            .map(|g| (g.seat, distance((g.run.x, g.run.z), (hook.x, hook.z))))
            .collect();
        near.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let answering = if near.len() > 3 { 2 } else { 1 };
        near.iter().take(answering).any(|(s, _)| *s == seat)
    }

    fn watch_point(&self, frame: usize, back: f32) -> (f32, f32) {
        let hook = self.paintings[frame].hook;
        for step in [back, back * 0.7, back * 0.45] {
            let p = (hook.x + hook.nx * step, hook.z + hook.nz * step);
            if self.floor.is_open(p.0, p.1) {
                return p;
            }
        }
        self.paintings[frame].spot()
    }

    pub(super) fn guard_bot(&mut self, seat: u8, dt: f32) -> Input {
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        let input = self.guard_think(seat, &mut brain, dt);
        self.brains.insert(seat, brain);
        input
    }

    fn guard_think(&mut self, seat: u8, brain: &mut Brain, dt: f32) -> Input {
        let skill = self.skill(seat);
        let Some(me) = self.guards.iter().find(|g| g.seat == seat).cloned() else {
            return Input::default();
        };
        let at = (me.run.x, me.run.z);
        let now = self.time;
        brain.sweep += dt;
        if brain.aim.is_none() {
            brain.aim = Some(me.yaw);
        }
        let lit = self
            .ghost
            .as_ref()
            .filter(|g| g.lit)
            .map(|g| (g.run.x, g.run.z));
        match lit {
            Some(p) => {
                let first = *brain.noticed.get_or_insert(now);
                if now - first >= 0.1 + skill.reaction() * 0.7 {
                    brain.seen = Some((p, now));
                }
            }
            None => brain.noticed = None,
        }
        let fresh: Vec<usize> = self.creaks[brain.heard.min(self.creaks.len())..]
            .iter()
            .map(|c| c.0)
            .collect();
        brain.heard = self.creaks.len();
        for frame in fresh {
            let busy = brain
                .errand
                .is_some_and(|(f, until)| f != frame && until > now);
            if !busy
                && self.answers(seat, frame)
                && self.rng.random::<f64>() > skill.hesitation() * 0.6
            {
                brain.errand = Some((frame, now + 7.0));
            }
        }
        let wobble = (brain.sweep * 2.3 + seat as f32).sin() * skill.aim_error() * 0.6;
        let (goal, look): (Option<(f32, f32)>, f32) = match brain.seen {
            Some((p, when)) if now - when < 0.25 => {
                let d = distance(at, p);
                let goal = if d > BEAM_RANGE * 0.55 { Some(p) } else { None };
                (goal, heading(at, p) + wobble)
            }
            Some((p, when)) if now - when < 4.0 => {
                let arrived = distance(at, p) < 1.2;
                let look = if arrived {
                    brain.aim.unwrap_or(0.0) + 1.0
                } else {
                    heading(at, p) + sweep(brain, 0.8, 2.6)
                };
                (if arrived { None } else { Some(p) }, look)
            }
            _ if me.chill > 0.15 => (None, brain.aim.unwrap_or(0.0) + 0.9),
            _ => {
                let errand = brain
                    .errand
                    .filter(|(frame, until)| *until > now && !self.paintings[*frame].stolen);
                brain.errand = errand;
                if let Some((frame, until)) = errand {
                    let watch = self.watch_point(frame, 3.5);
                    let hook = self.paintings[frame].hook;
                    let arrived = distance(at, watch) < 1.5;
                    if arrived && self.paintings[frame].sway <= 0.0 {
                        brain.errand = Some((frame, until.min(now + 2.5)));
                    }
                    let look = heading(at, (hook.x + hook.nx * 0.9, hook.z + hook.nz * 0.9))
                        + sweep(brain, 0.35, 3.0);
                    (if arrived { None } else { Some(watch) }, look)
                } else {
                    self.patrol(seat, brain, at, dt)
                }
            }
        };
        let rate = 1.6 + skill.tracking() * 0.55;
        let aim = Self::turn(brain, look, rate, dt);
        let stick = match goal {
            Some(goal) => self.follow(brain, at, goal, dt, false),
            None => {
                brain.route.clear();
                Stick::default()
            }
        };
        let pace = 0.8 + skill.level() * 0.15;
        Input {
            stick: Stick {
                x: stick.x * pace,
                z: stick.z * pace,
            }
            .clamped(),
            yaw: aim,
            ..Default::default()
        }
    }

    fn patrol(
        &mut self,
        seat: u8,
        brain: &mut Brain,
        at: (f32, f32),
        dt: f32,
    ) -> (Option<(f32, f32)>, f32) {
        let mut team: Vec<u8> = self
            .guards
            .iter()
            .filter(|g| g.here())
            .map(|g| g.seat)
            .collect();
        team.sort();
        let rank = team.iter().position(|s| *s == seat).unwrap_or(0);
        let hanging: Vec<usize> = (0..self.paintings.len())
            .filter(|i| !self.paintings[*i].stolen)
            .collect();
        if hanging.is_empty() {
            return (None, brain.aim.unwrap_or(0.0) + sweep(brain, 1.0, 1.2));
        }
        // Each guard walks its own share of the frames, the way a real team splits a floor.
        let mut mine: Vec<usize> = hanging
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| i % team.len().max(1) == rank)
            .map(|(_, f)| f)
            .collect();
        if mine.is_empty() {
            mine = hanging.clone();
        }
        let frame = mine[brain.patrol % mine.len()];
        let watch = self.watch_point(frame, 4.0);
        if distance(at, watch) < 1.5 {
            brain.dwell += dt;
            let hook = self.paintings[frame].hook;
            let look = heading(at, (hook.x, hook.z)) + sweep(brain, 1.3, 1.1);
            if brain.dwell > 1.6 {
                brain.dwell = 0.0;
                brain.patrol += 1 + self.rng.random_range(0..2);
            }
            return (None, look);
        }
        brain.dwell = 0.0;
        let moving = brain
            .route
            .first()
            .map_or(heading(at, watch), |next| heading(at, *next));
        (Some(watch), moving + sweep(brain, 0.7, 1.7))
    }

    pub(super) fn ghost_bot(&mut self, seat: u8, dt: f32) -> Input {
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        let input = self.ghost_think(seat, &mut brain, dt);
        self.brains.insert(seat, brain);
        input
    }

    /// How exposed a point is: guards that could put a beam on it soon.
    fn exposure(&self, p: (f32, f32), slack: f32) -> f32 {
        self.guards
            .iter()
            .filter(|g| g.here())
            .map(|g| {
                let from = (g.run.x, g.run.z);
                let d = distance(from, p);
                if d < 2.5 {
                    return 1.0;
                }
                if d > BEAM_RANGE + slack || !self.floor.sees(from, p) {
                    return 0.0;
                }
                let off = wrap(heading(from, p) - g.yaw).abs();
                if off < BEAM_HALF_ANGLE + slack * 0.25 {
                    1.0
                } else {
                    0.35
                }
            })
            .sum()
    }

    fn nearest_guard(&self, p: (f32, f32)) -> f32 {
        self.guards
            .iter()
            .filter(|g| g.here())
            .map(|g| distance((g.run.x, g.run.z), p))
            .fold(30.0, f32::min)
    }

    /// How likely a beam is to land on a point soon: guards close enough to
    /// catch with a clear view, weighted by how far their lamp must swing.
    fn risk(&self, p: (f32, f32)) -> f32 {
        let mut risk = 0.0;
        for g in self.guards.iter().filter(|g| g.here()) {
            let lamp = (g.run.x, g.run.z);
            let d = distance(lamp, p);
            if d > CATCH_RANGE + 2.0 || !self.floor.sees(lamp, p) {
                continue;
            }
            let off = wrap(heading(lamp, p) - g.yaw).abs();
            risk += if d < 1.5 || off < BEAM_HALF_ANGLE + 0.5 {
                1.0
            } else if off < BEAM_HALF_ANGLE + 1.4 {
                0.6
            } else {
                0.3
            };
        }
        risk
    }

    /// Somewhere a few strides off, out of every guard's sight if it can be
    /// had, away from the lights and never into a corner: what a person does
    /// when a beam finds them. The ghost goes straight through inside walls.
    fn escape_point(&mut self, at: (f32, f32)) -> (f32, f32) {
        let (hx, hz) = (self.floor.half_x - 0.8, self.floor.half_z - 0.8);
        let mut best = at;
        let mut best_score = f32::NEG_INFINITY;
        for step in [3.0f32, 5.0, 7.5] {
            for k in 0..20 {
                let angle = k as f32 / 20.0 * std::f32::consts::TAU;
                let p = (at.0 + angle.sin() * step, at.1 + angle.cos() * step);
                if p.0.abs() > hx || p.1.abs() > hz || self.floor.in_wall(p.0, p.1) {
                    continue;
                }
                // Out of a cone is as good as out of sight: slipping past a
                // guard's shoulder beats running for a corner.
                let risk = self.risk(p);
                let cornered = (hx - p.0.abs()).min(hz - p.1.abs()) < 2.0;
                let score = 10.0 - risk * 12.0 - if cornered { 6.0 } else { 0.0 } - step * 0.8
                    + self.nearest_guard(p).min(6.0) * 0.5
                    + self.rng.random_range(0.0..1.5);
                if score > best_score {
                    best_score = score;
                    best = p;
                }
            }
        }
        best
    }

    fn pick_target(&mut self, at: (f32, f32), sloppiness: f32) -> Option<usize> {
        let mut best = None;
        let mut best_score = f32::NEG_INFINITY;
        for i in 0..self.paintings.len() {
            let p = &self.paintings[i];
            if p.stolen {
                continue;
            }
            let spot = p.spot();
            let score = self.nearest_guard(spot).min(16.0) - self.exposure(spot, 3.0) * 7.0
                + p.progress * 9.0
                - distance(at, spot) * 0.35
                + self.rng.random_range(0.0..1.0) * sloppiness;
            if score > best_score {
                best_score = score;
                best = Some(i);
            }
        }
        best
    }

    fn ghost_think(&mut self, seat: u8, brain: &mut Brain, dt: f32) -> Input {
        let skill = self.skill(seat);
        let Some(me) = self.ghost.clone() else {
            return Input::default();
        };
        let at = (me.run.x, me.run.z);
        let now = self.time;
        brain.think -= dt;
        brain.sweep += dt;
        let thinking = brain.think <= 0.0;
        if thinking {
            brain.think = 0.15 + skill.reaction() * 0.35;
        }
        if me.lit {
            brain.noticed.get_or_insert(now);
        } else {
            brain.noticed = None;
        }
        let burned = brain
            .noticed
            .is_some_and(|first| now - first >= skill.reaction() * 0.5);
        let pace = skill.pace();
        let mut input = Input::default();

        if burned {
            let stale = brain
                .escape
                .is_none_or(|e| distance(at, e) < 0.8 || now >= brain.flee_until);
            if stale {
                brain.escape = Some(self.escape_point(at));
                brain.flee_until = now + 0.9 + self.rng.random_range(0.0..0.5);
            }
            brain.target = None;
            let route = self.follow(brain, at, brain.escape.unwrap_or(at), dt, true);
            input.stick = Stick {
                x: route.x * 2.0,
                z: route.z * 2.0,
            }
            .clamped();
            input.yaw = heading((0.0, 0.0), (input.stick.x, input.stick.z));
            return input;
        }
        if now < brain.flee_until {
            let goal = brain.escape.unwrap_or(at);
            let stick = self.follow(brain, at, goal, dt, true);
            input.stick = Stick {
                x: stick.x * pace,
                z: stick.z * pace,
            }
            .clamped();
            input.yaw = heading((0.0, 0.0), (stick.x, stick.z));
            if distance(at, goal) > 0.8 || self.exposure(at, 1.5) > 0.0 {
                return input;
            }
        }
        brain.escape = None;

        let stale = brain.target.is_none_or(|t| self.paintings[t].stolen);
        if stale || (thinking && self.rng.random::<f32>() < 0.08) {
            brain.target = self.pick_target(at, 2.0 + (1.0 - skill.level()) * 8.0);
        }
        let Some(target) = brain.target else {
            return input;
        };
        let spot = self.paintings[target].spot();

        if thinking && me.rattle <= 0.0 && self.rng.random::<f32>() < 0.15 + skill.level() * 0.4 {
            let crowded = self.nearest_guard(spot) < 9.0;
            if crowded {
                let mut best = None;
                let mut far = 9.0;
                for (i, p) in self.paintings.iter().enumerate() {
                    if p.stolen || i == target {
                        continue;
                    }
                    let hook = (p.hook.x, p.hook.z);
                    let from_target = distance(hook, spot);
                    if distance(at, hook) <= RATTLE_RANGE - 1.0 && from_target > far {
                        far = from_target;
                        best = Some(i);
                    }
                }
                brain.bait = best;
            }
        }
        if let Some(bait) = brain.bait.take() {
            let hook = self.paintings[bait].hook;
            input.yaw = heading(at, (hook.x, hook.z));
            input.fire = true;
            return input;
        }

        let here = distance(at, spot) < 0.45 && self.reachable(at) == Some(target);
        let danger = self.exposure(at, 1.5);
        if here {
            let left = 1.0 - self.paintings[target].progress;
            let greedy = left < 0.2 && self.rng.random::<f64>() < 0.5 + skill.level() as f64 * 0.3;
            if thinking
                && danger > 0.3
                && !greedy
                && self.rng.random::<f64>() > skill.hesitation() * 0.7
            {
                brain.escape = Some(self.escape_point(at));
                brain.flee_until = now + 1.0;
                brain.target = None;
                return input;
            }
            input.crouch = true;
            input.yaw = heading(
                at,
                (self.paintings[target].hook.x, self.paintings[target].hook.z),
            );
            return input;
        }
        if thinking && danger > 0.9 && self.rng.random::<f64>() > skill.hesitation() {
            brain.escape = Some(self.escape_point(at));
            brain.flee_until = now + 0.9;
            brain.target = None;
            return input;
        }
        let stick = self.follow(brain, at, spot, dt, true);
        let ahead = (at.0 + stick.x * 1.5, at.1 + stick.z * 1.5);
        if thinking
            && self.risk(ahead) >= 1.0
            && self.rng.random::<f32>() < 0.3 + skill.level() * 0.6
        {
            brain.escape = Some(self.escape_point(at));
            brain.flee_until = now + 0.7;
            return input;
        }
        input.stick = Stick {
            x: stick.x * pace,
            z: stick.z * pace,
        }
        .clamped();
        input.yaw = if stick.x.abs() + stick.z.abs() > 0.05 {
            heading((0.0, 0.0), (stick.x, stick.z))
        } else {
            me.yaw
        };
        input
    }
}
