//! Glass Cannon: everyone stands on one firing line facing a big stained-glass
//! window whose panes are tinted in the players' colors. Marbles that break a
//! pane of your own color score; break someone else's and they get the point
//! while you lose one. Broken panes glaze over again in whoever is shortest of
//! glass, the window shrinks as the clock runs, and every few seconds a wave
//! shuffles or slides the panes to new spots. The last stretch counts double.

use std::collections::HashMap;

use rand::{Rng, SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill};

pub const KEY: &str = "glass";
pub const ROUND_SECONDS: f32 = 75.0;
pub const COLS: usize = 8;
pub const ROWS: usize = 5;
/// A crowd of more than four gets a bigger window, so nobody waits for glass.
pub const CROWD_COLS: usize = 10;
pub const CROWD_ROWS: usize = 6;
pub const CELL: f32 = 2.5;
pub const WINDOW_Z: f32 = -26.0;
pub const WINDOW_CENTER_Y: f32 = 6.6;
/// The bottom of the window never sinks below this, however tall it is.
const SILL: f32 = 1.0;
/// The window ends the bout this much smaller than it starts.
pub const SHRINK: f32 = 0.45;
pub const EYE_HEIGHT: f32 = 1.6;
pub const LANE_SPACING: f32 = 1.8;
pub const MARBLE_SPEED: f32 = 48.0;
pub const MARBLE_LIFE: f32 = 1.4;
pub const FIRE_COOLDOWN: f32 = 0.2;
pub const WAVE_SECONDS: f32 = 7.5;
pub const SHIFT_SECONDS: f32 = 0.9;
pub const REGROW_SECONDS: f32 = 2.2;
/// The share of a pane's width taken up by the lead around it; a marble that
/// lands on the lead clinks off without breaking anything.
pub const LEAD: f32 = 0.07;
pub const FRENZY_FROM: f32 = ROUND_SECONDS - 12.0;
/// A seat the room has stopped sending input for this long has left.
const GONE_AFTER: f32 = 1.0;

pub const PALETTE: [&str; 8] = [
    "#ff4d5e", "#3d8bff", "#ffc93d", "#3ddc84", "#b45cff", "#ff7a1a", "#2fe0e0", "#ff6fcf",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaveKind {
    Shuffle,
    Slide,
}

impl WaveKind {
    fn name(self) -> &'static str {
        match self {
            WaveKind::Shuffle => "shuffle",
            WaveKind::Slide => "slide",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pane {
    pub id: u32,
    pub owner: Option<u8>,
    /// Where the pane is, in cells from the top-left corner, before and after
    /// the current wave. A slide's `to` may run past the edge; it wraps.
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub broken: bool,
    /// Seconds until a broken pane is glazed again.
    pub regrow: f32,
}

#[derive(Clone, Debug)]
pub struct Shooter {
    pub seat: u8,
    pub x: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub score: i32,
    pub hits: u32,
    pub wrong: u32,
    pub gifts: u32,
    pub shots: u32,
    pub cooldown: f32,
    pub last_seen: f32,
    pub gone: bool,
}

#[derive(Clone, Debug)]
pub struct Marble {
    pub id: u32,
    pub by: u8,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub life: f32,
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    target: Option<u32>,
    /// Where on the target the bot thinks it is aiming, in metres off centre.
    offset: (f32, f32),
    settle: f32,
    yaw: f32,
    pitch: f32,
    /// Panes shot at recently, left alone until the marble has landed.
    fired_at: Vec<(u32, f32)>,
    trigger_happy: bool,
}

pub struct GlassCannon {
    pub cols: usize,
    pub rows: usize,
    pub center_y: f32,
    pub shooters: Vec<Shooter>,
    pub panes: Vec<Pane>,
    pub marbles: Vec<Marble>,
    pub time: f32,
    pub wave: u32,
    pub wave_kind: WaveKind,
    pub wave_at: f32,
    next_marble: u32,
    frenzy_announced: bool,
    rng: StdRng,
    brains: HashMap<u8, BotBrain>,
    skills: HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

/// Which way a marble flies for a given aim: yaw turns about the vertical
/// with yaw = pi facing the window, pitch lifts it.
pub fn aim(yaw: f32, pitch: f32) -> [f32; 3] {
    [
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        yaw.cos() * pitch.cos(),
    ]
}

/// The aim that points from `eye` at `point`.
pub fn aim_at(eye: [f32; 3], point: [f32; 3]) -> (f32, f32) {
    let (dx, dy, dz) = (point[0] - eye[0], point[1] - eye[1], point[2] - eye[2]);
    let flat = (dx * dx + dz * dz).sqrt();
    (dx.atan2(dz), dy.atan2(flat))
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lane_x(index: usize, count: usize) -> f32 {
    (index as f32 - (count as f32 - 1.0) / 2.0) * LANE_SPACING
}

impl GlassCannon {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let (cols, rows) = if contestants.len() > 4 {
            (CROWD_COLS, CROWD_ROWS)
        } else {
            (COLS, ROWS)
        };
        let center_y = WINDOW_CENTER_Y.max(SILL + rows as f32 * CELL / 2.0);
        let shooters: Vec<Shooter> = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let x = lane_x(i, contestants.len());
                let (yaw, pitch) = aim_at([x, EYE_HEIGHT, 0.0], [0.0, center_y, WINDOW_Z]);
                Shooter {
                    seat: c.seat,
                    x,
                    yaw,
                    pitch,
                    score: 0,
                    hits: 0,
                    wrong: 0,
                    gifts: 0,
                    shots: 0,
                    cooldown: 0.0,
                    last_seen: 0.0,
                    gone: false,
                }
            })
            .collect();
        let mut owners: Vec<Option<u8>> = (0..cols * rows)
            .map(|i| shooters.get(i % shooters.len().max(1)).map(|s| s.seat))
            .collect();
        owners.shuffle(&mut rng);
        let panes = owners
            .into_iter()
            .enumerate()
            .map(|(i, owner)| {
                let cell = ((i % cols) as f32, (i / cols) as f32);
                Pane {
                    id: i as u32,
                    owner,
                    from: cell,
                    to: cell,
                    broken: false,
                    regrow: 0.0,
                }
            })
            .collect();
        let brains = shooters
            .iter()
            .filter(|s| contestants.iter().any(|c| c.seat == s.seat && c.is_bot))
            .map(|s| {
                (
                    s.seat,
                    BotBrain {
                        settle: 0.6,
                        yaw: s.yaw,
                        pitch: s.pitch,
                        ..Default::default()
                    },
                )
            })
            .collect();
        GlassCannon {
            cols,
            rows,
            center_y,
            shooters,
            panes,
            marbles: Vec::new(),
            time: 0.0,
            wave: 0,
            wave_kind: WaveKind::Shuffle,
            wave_at: -SHIFT_SECONDS,
            next_marble: 0,
            frenzy_announced: false,
            rng,
            brains,
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        }
    }

    /// How big the window is right now, as (scale, width, height).
    pub fn window(&self) -> (f32, f32, f32) {
        let scale = 1.0 - SHRINK * smoothstep(self.time / ROUND_SECONDS);
        (
            scale,
            self.cols as f32 * CELL * scale,
            self.rows as f32 * CELL * scale,
        )
    }

    pub fn cell_size(&self) -> (f32, f32) {
        let (_, w, h) = self.window();
        (w / self.cols as f32, h / self.rows as f32)
    }

    fn shift_progress(&self) -> f32 {
        ((self.time - self.wave_at) / SHIFT_SECONDS).clamp(0.0, 1.0)
    }

    pub fn shifting(&self) -> bool {
        self.shift_progress() < 1.0
    }

    pub fn frenzy(&self) -> bool {
        self.time >= FRENZY_FROM
    }

    /// Where a pane sits right now in cells, its top-left corner, wrapped.
    pub fn pane_cell(&self, pane: &Pane) -> (f32, f32) {
        self.pane_cell_at(pane, self.shift_progress())
    }

    fn pane_cell_at(&self, pane: &Pane, progress: f32) -> (f32, f32) {
        let e = smoothstep(progress);
        let x = pane.from.0 + (pane.to.0 - pane.from.0) * e;
        let y = pane.from.1 + (pane.to.1 - pane.from.1) * e;
        (x.rem_euclid(self.cols as f32), y)
    }

    /// The centre of a pane in the world.
    pub fn pane_center(&self, pane: &Pane) -> [f32; 3] {
        self.cell_to_world(visible_middle(self.pane_cell(pane), self.cols))
    }

    fn cell_to_world(&self, cell: (f32, f32)) -> [f32; 3] {
        let (_, w, h) = self.window();
        let (cw, ch) = self.cell_size();
        [
            -w / 2.0 + (cell.0 + 0.5) * cw,
            self.center_y + h / 2.0 - (cell.1 + 0.5) * ch,
            WINDOW_Z,
        ]
    }

    fn shooter(&self, seat: u8) -> Option<&Shooter> {
        self.shooters.iter().find(|s| s.seat == seat)
    }

    fn eye(&self, seat: u8) -> Option<[f32; 3]> {
        self.shooter(seat).map(|s| [s.x, EYE_HEIGHT, 0.0])
    }

    fn present(&self) -> Vec<u8> {
        self.shooters
            .iter()
            .filter(|s| !s.gone)
            .map(|s| s.seat)
            .collect()
    }

    /// The live and soon-to-be-live panes each present player has.
    fn glass_counts(&self) -> HashMap<u8, usize> {
        let mut counts: HashMap<u8, usize> = self.present().into_iter().map(|s| (s, 0)).collect();
        for pane in &self.panes {
            if let Some(owner) = pane.owner
                && let Some(count) = counts.get_mut(&owner)
            {
                *count += 1;
            }
        }
        counts
    }

    /// Whoever is shortest of glass, a random one of them on a tie.
    fn neediest(&mut self) -> Option<u8> {
        let counts = self.glass_counts();
        let least = counts.values().copied().min()?;
        let mut needy: Vec<u8> = counts
            .into_iter()
            .filter(|(_, count)| *count == least)
            .map(|(seat, _)| seat)
            .collect();
        needy.sort_unstable();
        let pick = self.rng.random_range(0..needy.len());
        Some(needy[pick])
    }

    /// The pane under a point on the window, or None for empty air. Lead
    /// between panes counts as a pane-less hit.
    pub fn pane_at(&self, x: f32, y: f32) -> PaneHit {
        let (_, w, h) = self.window();
        let (cw, ch) = self.cell_size();
        let u = (x + w / 2.0) / cw;
        let v = (self.center_y + h / 2.0 - y) / ch;
        if !(0.0..self.cols as f32).contains(&u) || !(0.0..self.rows as f32).contains(&v) {
            return PaneHit::Miss;
        }
        let mut best: Option<(usize, f32)> = None;
        for (index, pane) in self.panes.iter().enumerate() {
            if pane.broken {
                continue;
            }
            let (px, py) = self.pane_cell(pane);
            let mut du = u - px;
            if du < 0.0 {
                du += self.cols as f32;
            }
            let dv = v - py;
            if (0.0..1.0).contains(&du) && (0.0..1.0).contains(&dv) {
                let edge = du.min(1.0 - du).min(dv).min(1.0 - dv);
                if best.is_none_or(|(_, e)| edge > e) {
                    best = Some((index, edge));
                }
            }
        }
        match best {
            Some((_, edge)) if edge < LEAD / 2.0 => PaneHit::Lead,
            Some((index, _)) => PaneHit::Pane(index),
            None => PaneHit::Hole,
        }
    }

    fn fire(&mut self, seat: u8, input: &Input, events: &mut Vec<Value>) {
        let Some(index) = self.shooters.iter().position(|s| s.seat == seat) else {
            return;
        };
        if self.shooters[index].cooldown > 0.0 || !input.yaw.is_finite() || !input.pitch.is_finite()
        {
            return;
        }
        let pitch = input.pitch.clamp(-1.4, 1.4);
        let dir = aim(input.yaw, pitch);
        let eye = [self.shooters[index].x, EYE_HEIGHT, 0.0];
        let id = self.next_marble;
        self.next_marble += 1;
        let shooter = &mut self.shooters[index];
        shooter.cooldown = FIRE_COOLDOWN;
        shooter.shots += 1;
        self.marbles.push(Marble {
            id,
            by: seat,
            pos: [
                eye[0] + dir[0] * 0.6,
                eye[1] - 0.15 + dir[1] * 0.6,
                eye[2] + dir[2] * 0.6,
            ],
            vel: [
                dir[0] * MARBLE_SPEED,
                dir[1] * MARBLE_SPEED,
                dir[2] * MARBLE_SPEED,
            ],
            life: MARBLE_LIFE,
        });
        events.push(json!({ "kind": "marble", "seat": seat, "id": id }));
    }

    fn step_marbles(&mut self, dt: f32, events: &mut Vec<Value>) {
        let mut landed: Vec<(usize, PaneHit, [f32; 3])> = Vec::new();
        for (index, marble) in self.marbles.iter_mut().enumerate() {
            let before = marble.pos;
            for axis in 0..3 {
                marble.pos[axis] += marble.vel[axis] * dt;
            }
            marble.life -= dt;
            if before[2] > WINDOW_Z && marble.pos[2] <= WINDOW_Z {
                let t = (before[2] - WINDOW_Z) / (before[2] - marble.pos[2]);
                let at = [
                    before[0] + (marble.pos[0] - before[0]) * t,
                    before[1] + (marble.pos[1] - before[1]) * t,
                    WINDOW_Z,
                ];
                landed.push((index, PaneHit::Miss, at));
            }
        }
        for entry in landed.iter_mut() {
            entry.1 = self.pane_at(entry.2[0], entry.2[1]);
        }
        let mut spent: Vec<usize> = Vec::new();
        for (index, hit, at) in landed {
            let by = self.marbles[index].by;
            match hit {
                PaneHit::Pane(pane) => {
                    spent.push(index);
                    self.shatter(by, pane, at, events);
                }
                PaneHit::Lead => {
                    spent.push(index);
                    events.push(
                        json!({ "kind": "clink", "seat": by, "x": at[0], "y": at[1], "z": at[2] }),
                    );
                }
                PaneHit::Hole | PaneHit::Miss => {}
            }
        }
        for (index, marble) in self.marbles.iter().enumerate() {
            if marble.life <= 0.0 || marble.pos[1] < -1.0 {
                spent.push(index);
            }
        }
        spent.sort_unstable();
        spent.dedup();
        for index in spent.into_iter().rev() {
            self.marbles.remove(index);
        }
    }

    fn shatter(&mut self, by: u8, pane: usize, at: [f32; 3], events: &mut Vec<Value>) {
        let weight = if self.frenzy() { 2 } else { 1 };
        let owner = self.panes[pane].owner;
        let own = owner == Some(by);
        let mut points = 0;
        let mut gift = 0;
        if own {
            points = weight;
        } else if let Some(owner) = owner {
            points = -weight;
            gift = weight;
            if let Some(them) = self.shooters.iter_mut().find(|s| s.seat == owner) {
                them.score += gift;
                them.gifts += 1;
            }
        }
        if let Some(me) = self.shooters.iter_mut().find(|s| s.seat == by) {
            me.score += points;
            if own {
                me.hits += 1;
            } else if owner.is_some() {
                me.wrong += 1;
            }
        }
        let center = self.pane_center(&self.panes[pane]);
        let (cw, ch) = self.cell_size();
        let id = self.panes[pane].id;
        let p = &mut self.panes[pane];
        p.broken = true;
        p.owner = None;
        p.regrow = REGROW_SECONDS;
        events.push(json!({
            "kind": "shatter",
            "seat": by,
            "pane": id,
            "owner": owner,
            "own": own,
            "points": points,
            "gift": gift,
            "x": at[0], "y": at[1], "z": at[2],
            "cx": center[0], "cy": center[1],
            "w": cw, "h": ch,
        }));
    }

    fn regrow(&mut self, dt: f32) {
        for index in 0..self.panes.len() {
            if !self.panes[index].broken {
                continue;
            }
            self.panes[index].regrow -= dt;
            if self.panes[index].regrow <= 0.0 {
                let owner = self.neediest();
                let pane = &mut self.panes[index];
                pane.broken = false;
                pane.regrow = 0.0;
                pane.owner = owner;
            }
        }
    }

    fn start_wave(&mut self, events: &mut Vec<Value>) {
        // Land every pane where the last wave left it before moving again.
        for index in 0..self.panes.len() {
            let cell = self.pane_cell_at(&self.panes[index], 1.0);
            let cell = (cell.0.round().rem_euclid(self.cols as f32), cell.1.round());
            self.panes[index].from = cell;
            self.panes[index].to = cell;
        }
        self.wave += 1;
        self.wave_at = self.time;
        self.wave_kind = if self.wave % 2 == 1 {
            WaveKind::Shuffle
        } else {
            WaveKind::Slide
        };
        match self.wave_kind {
            WaveKind::Shuffle => {
                let mut cells: Vec<(f32, f32)> = self.panes.iter().map(|p| p.from).collect();
                cells.shuffle(&mut self.rng);
                for (pane, cell) in self.panes.iter_mut().zip(cells) {
                    pane.to = cell;
                }
            }
            WaveKind::Slide => {
                let shifts: Vec<f32> = (0..self.rows)
                    .map(|row| {
                        let size = self.rng.random_range(1..=3) as f32;
                        if row % 2 == 0 { size } else { -size }
                    })
                    .collect();
                let flip = if self.rng.random_bool(0.5) { 1.0 } else { -1.0 };
                for pane in self.panes.iter_mut() {
                    let row = (pane.from.1 as usize).min(self.rows - 1);
                    pane.to = (pane.from.0 + shifts[row] * flip, pane.from.1);
                }
            }
        }
        events.push(json!({ "kind": "wave", "wave": self.wave, "pattern": self.wave_kind.name() }));
    }

    fn notice_leavers(&mut self, inputs: &[(u8, Input)]) {
        for shooter in self.shooters.iter_mut() {
            if inputs.iter().any(|(seat, _)| *seat == shooter.seat) {
                shooter.last_seen = self.time;
            }
        }
        let gone: Vec<u8> = self
            .shooters
            .iter()
            .filter(|s| !s.gone && self.time - s.last_seen > GONE_AFTER)
            .map(|s| s.seat)
            .collect();
        for seat in gone {
            if let Some(s) = self.shooters.iter_mut().find(|s| s.seat == seat) {
                s.gone = true;
            }
            // Their glass goes to whoever is still here, a pane at a time.
            for index in 0..self.panes.len() {
                if self.panes[index].owner == Some(seat) {
                    self.panes[index].owner = self.neediest();
                }
            }
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < ROUND_SECONDS {
            return;
        }
        let mut rows: Vec<(i32, i64, u8)> = self
            .shooters
            .iter()
            .map(|s| (s.score, -(s.wrong as i64), s.seat))
            .collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(i32, i64)> = None;
        for (score, wrong, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (score, wrong) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((score, wrong));
        }
        self.placements = Some(groups);
    }

    fn pick_target(&mut self, seat: u8, brain: &mut BotBrain, skill: Skill) {
        let Some(eye) = self.eye(seat) else {
            return;
        };
        let mistake = (1.0 - skill.level()) * 0.1;
        let confused = self.rng.random_bool(mistake as f64);
        let shot_at: Vec<u32> = brain.fired_at.iter().map(|(id, _)| *id).collect();
        let candidates: Vec<usize> = self
            .panes
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.broken && p.owner.is_some() && !shot_at.contains(&p.id))
            .filter(|(_, p)| (p.owner == Some(seat)) != confused)
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            brain.target = None;
            return;
        }
        // People sweep to the nearest pane of theirs, not a random one.
        let mut ranked: Vec<(usize, f32)> = candidates
            .into_iter()
            .map(|i| {
                let (yaw, pitch) = aim_at(eye, self.pane_center(&self.panes[i]));
                let turn = wrap_angle(yaw - brain.yaw).abs() + (pitch - brain.pitch).abs();
                (i, turn)
            })
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let reach = ranked.len().min(if skill.level() > 0.6 { 2 } else { 3 });
        let index = ranked[self.rng.random_range(0..reach)].0;
        let spread = 0.12 + (1.0 - skill.level()) * 0.8;
        brain.target = Some(self.panes[index].id);
        brain.offset = (
            gaussian(&mut self.rng) * spread,
            gaussian(&mut self.rng) * spread * 0.8,
        );
        brain.settle = 0.12 + skill.reaction() * 0.45 + self.rng.random_range(0.04..0.2);
        brain.trigger_happy = self.rng.random_bool(skill.hesitation() * 0.18);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneHit {
    Pane(usize),
    Lead,
    Hole,
    Miss,
}

/// A pane sliding off one edge shows on the other; this is the cell whose
/// middle is the middle of the bigger part still showing.
fn visible_middle((x, y): (f32, f32), cols: usize) -> (f32, f32) {
    let spill = x + 1.0 - cols as f32;
    if spill <= 0.0 {
        (x, y)
    } else if spill <= 0.5 {
        (x - spill / 2.0, y)
    } else {
        (spill / 2.0 - 0.5, y)
    }
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut angle = a % tau;
    if angle > std::f32::consts::PI {
        angle -= tau;
    }
    if angle < -std::f32::consts::PI {
        angle += tau;
    }
    angle
}

fn gaussian(rng: &mut StdRng) -> f32 {
    let u: f32 = rng.random_range(1e-6..1.0);
    let v: f32 = rng.random_range(0.0..std::f32::consts::TAU);
    (-2.0 * u.ln()).sqrt() * v.cos()
}

impl Bout for GlassCannon {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rule": "Break the panes in your color. Break a rival's and they get the point.",
            "cols": self.cols,
            "rows": self.rows,
            "window_z": WINDOW_Z,
            "center_y": self.center_y,
            "cell": CELL,
            "eye": EYE_HEIGHT,
            "cooldown": FIRE_COOLDOWN,
            "speed": MARBLE_SPEED,
            "lead": LEAD,
            "frenzy_from": FRENZY_FROM,
            "colors": self.shooters.iter().map(|s| json!({
                "seat": s.seat, "color": PALETTE[s.seat as usize % PALETTE.len()],
            })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.notice_leavers(inputs);
        if self.time - self.wave_at >= WAVE_SECONDS && self.time < ROUND_SECONDS - 2.0 {
            self.start_wave(events);
        }
        if self.frenzy() && !self.frenzy_announced {
            self.frenzy_announced = true;
            events.push(json!({ "kind": "frenzy" }));
        }
        for (seat, input) in inputs {
            let Some(shooter) = self.shooters.iter_mut().find(|s| s.seat == *seat) else {
                continue;
            };
            shooter.cooldown = (shooter.cooldown - dt).max(0.0);
            if input.yaw.is_finite() {
                shooter.yaw = wrap_angle(input.yaw);
            }
            if input.pitch.is_finite() {
                shooter.pitch = input.pitch.clamp(-1.4, 1.4);
            }
            if input.fire && !shooter.gone {
                self.fire(*seat, input, events);
            }
        }
        self.step_marbles(dt, events);
        self.regrow(dt);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(eye) = self.eye(seat) else {
            return Input::default();
        };
        let skill = self.skills.get(&seat).copied().unwrap_or_default();
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        let now = self.time;
        brain.fired_at.retain(|(_, at)| now - at < 0.75);
        let target = brain
            .target
            .and_then(|id| self.panes.iter().position(|p| p.id == id && !p.broken));
        // A wave scrambles everything: people lose their mark and re-find it.
        let lost = self.time - self.wave_at < dt * 1.5;
        if target.is_none() || lost {
            brain.target = None;
            brain.settle -= dt;
            if brain.settle <= 0.0 || lost {
                if lost {
                    brain.settle = skill.reaction() * 0.6;
                } else {
                    self.pick_target(seat, &mut brain, skill);
                }
            }
        }
        let mut input = Input {
            yaw: brain.yaw,
            pitch: brain.pitch,
            ..Default::default()
        };
        if let Some(index) = brain
            .target
            .and_then(|id| self.panes.iter().position(|p| p.id == id && !p.broken))
        {
            let pane = self.panes[index].clone();
            let mut point = self.pane_center(&pane);
            // Sharp players lead a sliding pane; the rest aim where it is.
            if skill.level() > 0.55 && self.shifting() {
                let flight = (WINDOW_Z.abs() / MARBLE_SPEED) / SHIFT_SECONDS;
                let ahead = self.cell_to_world(visible_middle(
                    self.pane_cell_at(&pane, self.shift_progress() + flight),
                    self.cols,
                ));
                point = ahead;
            }
            let (scale, _, _) = self.window();
            point[0] += brain.offset.0 * scale;
            point[1] += brain.offset.1 * scale;
            let (want_yaw, want_pitch) = aim_at(eye, point);
            let follow = (skill.tracking() * 1.6 * dt).min(1.0);
            brain.yaw += wrap_angle(want_yaw - brain.yaw) * follow;
            brain.pitch += (want_pitch - brain.pitch) * follow;
            brain.settle -= dt;
            let off = wrap_angle(want_yaw - brain.yaw).abs() + (want_pitch - brain.pitch).abs();
            let steady = off < 0.01 || brain.trigger_happy && off < 0.06;
            let waiting = self.shifting() && skill.level() <= 0.55;
            let ready = self.shooter(seat).is_some_and(|s| s.cooldown <= 0.0);
            if brain.settle <= 0.0 && steady && ready && !waiting {
                input.fire = true;
                brain.fired_at.push((pane.id, now));
                brain.target = None;
                brain.settle = 0.16 + skill.reaction() * 0.3 + self.rng.random_range(0.02..0.14);
            }
            input.yaw = brain.yaw;
            input.pitch = brain.pitch;
        }
        self.brains.insert(seat, brain);
        input
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        let (scale, w, h) = self.window();
        json!({
            "game": KEY,
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "wave": self.wave,
            "pattern": self.wave_kind.name(),
            "shift": round(self.shift_progress()),
            "frenzy": self.frenzy(),
            "window": {
                "scale": round(scale), "width": round(w), "height": round(h),
                "cols": self.cols, "rows": self.rows, "z": WINDOW_Z, "center_y": self.center_y,
            },
            "players": self.shooters.iter().map(|s| json!({
                "seat": s.seat,
                "x": round(s.x),
                "z": 0.0,
                "yaw": round(s.yaw),
                "pitch": round(s.pitch),
                "score": s.score,
                "hits": s.hits,
                "wrong": s.wrong,
                "gifts": s.gifts,
                "shots": s.shots,
                "cooldown": round(s.cooldown),
                "gone": s.gone,
                "color": PALETTE[s.seat as usize % PALETTE.len()],
                "panes": self.panes.iter().filter(|p| p.owner == Some(s.seat)).count(),
            })).collect::<Vec<_>>(),
            "panes": self.panes.iter().map(|p| {
                let (x, y) = self.pane_cell(p);
                json!({
                    "id": p.id,
                    "owner": p.owner,
                    "x": round(x),
                    "y": round(y),
                    "broken": p.broken,
                    "regrow": round(p.regrow.max(0.0)),
                })
            }).collect::<Vec<_>>(),
            "marbles": self.marbles.iter().map(|m| json!({
                "id": m.id,
                "by": m.by,
                "pos": [round(m.pos[0]), round(m.pos[1]), round(m.pos[2])],
                "vel": [round(m.vel[0]), round(m.vel[1]), round(m.vel[2])],
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::TICK_DT;

    fn people(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn bots(count: u8, skill: f32) -> Vec<Contestant> {
        (0..count)
            .map(|seat| Contestant::bot(seat, skill))
            .collect()
    }

    fn shot_at(game: &GlassCannon, seat: u8, point: [f32; 3]) -> Input {
        let (yaw, pitch) = aim_at(game.eye(seat).unwrap(), point);
        Input {
            yaw,
            pitch,
            fire: true,
            ..Default::default()
        }
    }

    fn idle(game: &GlassCannon) -> Vec<(u8, Input)> {
        game.shooters
            .iter()
            .map(|s| {
                (
                    s.seat,
                    Input {
                        yaw: s.yaw,
                        pitch: s.pitch,
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    /// Fires one marble from `seat` at `point` and runs until it lands.
    fn fire_and_land(game: &mut GlassCannon, seat: u8, point: [f32; 3]) -> Vec<Value> {
        let mut events = Vec::new();
        let mut inputs = idle(game);
        for (s, input) in inputs.iter_mut() {
            if *s == seat {
                *input = shot_at(game, seat, point);
            }
        }
        game.step(&inputs, TICK_DT, &mut events);
        for _ in 0..60 {
            game.step(&idle(game), TICK_DT, &mut events);
            if game.marbles.is_empty() {
                break;
            }
        }
        events
    }

    fn pane_of(game: &GlassCannon, owner: u8) -> usize {
        game.panes
            .iter()
            .position(|p| !p.broken && p.owner == Some(owner))
            .unwrap()
    }

    #[test]
    fn panes_are_shared_out_evenly() {
        for count in 2..=8u8 {
            let game = GlassCannon::new(&people(count), count as u64);
            let counts = game.glass_counts();
            let (low, high) = (
                counts.values().min().unwrap(),
                counts.values().max().unwrap(),
            );
            assert!(high - low <= 1, "{count} players: {counts:?}");
            assert!(
                *low >= 7,
                "{count} players each get a fair handful: {counts:?}"
            );
            let (_, _, h) = game.window();
            assert!(
                game.center_y - h / 2.0 >= SILL - 1e-4,
                "the window sits above the floor"
            );
        }
    }

    #[test]
    fn breaking_your_own_color_scores() {
        let mut game = GlassCannon::new(&people(3), 7);
        let pane = pane_of(&game, 0);
        let center = game.pane_center(&game.panes[pane]);
        let events = fire_and_land(&mut game, 0, center);
        assert!(game.panes[pane].broken);
        assert_eq!(game.shooters[0].score, 1);
        assert_eq!(game.shooters[0].hits, 1);
        assert!(
            events
                .iter()
                .any(|e| e["kind"] == "shatter" && e["own"] == true)
        );
    }

    #[test]
    fn breaking_a_rivals_color_gives_them_the_point() {
        let mut game = GlassCannon::new(&people(3), 7);
        let pane = pane_of(&game, 2);
        let center = game.pane_center(&game.panes[pane]);
        fire_and_land(&mut game, 0, center);
        assert_eq!(game.shooters[0].score, -1);
        assert_eq!(game.shooters[0].wrong, 1);
        assert_eq!(game.shooters[2].score, 1);
        assert_eq!(game.shooters[1].score, 0);
    }

    #[test]
    fn lead_and_empty_air_score_nothing() {
        let mut game = GlassCannon::new(&people(2), 3);
        let (cw, _) = game.cell_size();
        let first = game.cell_to_world((0.0, 0.0));
        let lead = [first[0] + cw / 2.0, first[1], WINDOW_Z];
        let events = fire_and_land(&mut game, 0, lead);
        assert!(events.iter().any(|e| e["kind"] == "clink"));
        let above = [0.0, game.center_y + CELL * game.rows as f32, WINDOW_Z];
        fire_and_land(&mut game, 1, above);
        assert!(game.shooters.iter().all(|s| s.score == 0));
        assert!(game.panes.iter().all(|p| !p.broken));
    }

    #[test]
    fn the_server_enforces_the_fire_rate_and_ignores_bad_aim() {
        let mut game = GlassCannon::new(&people(2), 3);
        let mut events = Vec::new();
        let flood = Input {
            yaw: std::f32::consts::PI,
            pitch: 0.2,
            fire: true,
            ..Default::default()
        };
        for _ in 0..60 {
            game.step(
                &[
                    (0, flood),
                    (
                        1,
                        Input {
                            yaw: f32::NAN,
                            pitch: f32::INFINITY,
                            fire: true,
                            ..Default::default()
                        },
                    ),
                ],
                TICK_DT,
                &mut events,
            );
        }
        let shots = game.shooters[0].shots;
        assert!(
            (4..=6).contains(&shots),
            "one second of held fire is about five marbles, got {shots}"
        );
        assert_eq!(game.shooters[1].shots, 0, "non-finite aim never fires");
    }

    #[test]
    fn a_broken_pane_glazes_over_for_whoever_is_short() {
        let mut game = GlassCannon::new(&people(2), 11);
        let pane = pane_of(&game, 0);
        let center = game.pane_center(&game.panes[pane]);
        fire_and_land(&mut game, 0, center);
        assert!(game.panes[pane].broken);
        for _ in 0..((REGROW_SECONDS + 0.2) * 60.0) as usize {
            game.step(&idle(&game), TICK_DT, &mut Vec::new());
        }
        assert!(!game.panes[pane].broken);
        assert_eq!(game.panes[pane].owner, Some(0), "seat 0 was a pane short");
    }

    #[test]
    fn waves_move_panes_and_the_window_shrinks() {
        let mut game = GlassCannon::new(&people(4), 5);
        let start: Vec<(f32, f32)> = game.panes.iter().map(|p| game.pane_cell(p)).collect();
        let (_, w0, _) = game.window();
        let mut events = Vec::new();
        for _ in 0..((WAVE_SECONDS * 2.0 + SHIFT_SECONDS + 0.1) * 60.0) as usize {
            game.step(&idle(&game), TICK_DT, &mut events);
        }
        let waves: Vec<&Value> = events.iter().filter(|e| e["kind"] == "wave").collect();
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0]["pattern"], "shuffle");
        assert_eq!(waves[1]["pattern"], "slide");
        let moved = game
            .panes
            .iter()
            .zip(&start)
            .filter(|(p, s)| game.pane_cell(p) != **s)
            .count();
        assert!(moved > 20, "most panes moved, {moved} did");
        let mut cells: Vec<(i32, i32)> = game
            .panes
            .iter()
            .map(|p| {
                let c = game.pane_cell(p);
                (c.0.round() as i32 % game.cols as i32, c.1.round() as i32)
            })
            .collect();
        cells.sort_unstable();
        cells.dedup();
        assert_eq!(
            cells.len(),
            game.cols * game.rows,
            "after a wave every cell holds exactly one pane"
        );
        assert!(game.window().1 < w0);
    }

    #[test]
    fn a_pane_is_hit_where_it_has_slid_to() {
        let mut game = GlassCannon::new(&people(2), 9);
        let mut events = Vec::new();
        while game.wave < 2 {
            game.step(&idle(&game), TICK_DT, &mut events);
        }
        // Mid-slide: aim where the pane will be when the marble lands.
        for _ in 0..12 {
            game.step(&idle(&game), TICK_DT, &mut events);
        }
        let pane = pane_of(&game, 1);
        let flight_ticks = (WINDOW_Z.abs() / MARBLE_SPEED * 60.0).ceil();
        let later = game.pane_cell_at(
            &game.panes[pane],
            game.shift_progress() + flight_ticks / 60.0 / SHIFT_SECONDS,
        );
        let point = game.cell_to_world(visible_middle(later, game.cols));
        fire_and_land(&mut game, 1, point);
        assert!(
            game.panes[pane].broken,
            "the marble meets the pane where it moved to"
        );
    }

    #[test]
    fn the_last_stretch_counts_double() {
        let mut game = GlassCannon::new(&people(2), 2);
        game.time = FRENZY_FROM + 0.5;
        game.wave_at = game.time;
        let own = pane_of(&game, 0);
        let center = game.pane_center(&game.panes[own]);
        fire_and_land(&mut game, 0, center);
        assert_eq!(game.shooters[0].score, 2);
        let theirs = pane_of(&game, 1);
        let center = game.pane_center(&game.panes[theirs]);
        fire_and_land(&mut game, 0, center);
        assert_eq!(game.shooters[0].score, 0);
        assert_eq!(game.shooters[1].score, 2);
    }

    #[test]
    fn ties_share_a_place_and_the_bout_ends_on_time() {
        let mut game = GlassCannon::new(&people(3), 4);
        for _ in 0..(ROUND_SECONDS * 60.0) as usize + 2 {
            game.step(&idle(&game), TICK_DT, &mut Vec::new());
        }
        assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
    }

    #[test]
    fn a_leaver_hands_their_glass_on_and_keeps_their_score() {
        let mut game = GlassCannon::new(&people(3), 6);
        let pane = pane_of(&game, 2);
        let center = game.pane_center(&game.panes[pane]);
        fire_and_land(&mut game, 2, center);
        let mut events = Vec::new();
        for _ in 0..120 {
            let inputs: Vec<(u8, Input)> =
                idle(&game).into_iter().filter(|(s, _)| *s != 2).collect();
            game.step(&inputs, TICK_DT, &mut events);
        }
        assert!(game.shooters[2].gone);
        assert!(game.panes.iter().all(|p| p.owner != Some(2)));
        let counts = game.glass_counts();
        assert!(!counts.contains_key(&2));
        while game.placements().is_none() {
            let inputs: Vec<(u8, Input)> =
                idle(&game).into_iter().filter(|(s, _)| *s != 2).collect();
            game.step(&inputs, TICK_DT, &mut events);
        }
        let places = game.placements().unwrap();
        assert_eq!(places[0], vec![2], "the leaver's point still stands");
    }

    #[test]
    fn bots_mostly_break_their_own_glass() {
        for skill in [0.15, 0.4, 1.0] {
            let mut game = GlassCannon::new(&bots(4, skill), 21);
            run(&mut game);
            let hits: u32 = game.shooters.iter().map(|s| s.hits).sum();
            let wrong: u32 = game.shooters.iter().map(|s| s.wrong).sum();
            assert!(hits > 40, "skill {skill}: {hits} hits");
            assert!(wrong > 0, "skill {skill}: even good bots slip sometimes");
            assert!(
                hits > wrong * 3,
                "skill {skill}: {hits} right, {wrong} wrong"
            );
        }
    }

    #[test]
    fn sharper_bots_beat_clumsy_ones() {
        let mut sharp_wins = 0;
        for seed in 0..12 {
            let seats = vec![
                Contestant::bot(0, 0.15),
                Contestant::bot(1, 1.0),
                Contestant::bot(2, 0.4),
            ];
            let mut game = GlassCannon::new(&seats, seed);
            run(&mut game);
            if game.placements().unwrap()[0].contains(&1) {
                sharp_wins += 1;
            }
        }
        assert!(sharp_wins >= 8, "ruthless won {sharp_wins} of 12");
    }

    #[test]
    fn bot_aim_is_not_instant() {
        let mut game = GlassCannon::new(&bots(2, 1.0), 1);
        let first = game.bot_input(0, TICK_DT);
        assert!(!first.fire, "a bot needs a moment to find its mark");
    }

    fn run(game: &mut GlassCannon) -> u64 {
        let mut ticks = 0;
        let seats: Vec<u8> = game.shooters.iter().map(|s| s.seat).collect();
        while game.placements().is_none() {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|s| (*s, game.bot_input(*s, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut Vec::new());
            ticks += 1;
            assert!(ticks < 60 * 120, "a bout must end");
        }
        ticks
    }

    /// `cargo test -p party glass::tests::balance -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn balance() {
        for (label, skill) in crate::bout::SKILLS {
            for count in [2u8, 4, 8] {
                let (mut hits, mut wrong, mut shots, mut top, mut low, mut ties) =
                    (0u32, 0u32, 0u32, 0i32, 0i32, 0u32);
                let runs = 40;
                for seed in 0..runs {
                    let mut game = GlassCannon::new(&bots(count, *skill), seed);
                    run(&mut game);
                    hits += game.shooters.iter().map(|s| s.hits).sum::<u32>();
                    wrong += game.shooters.iter().map(|s| s.wrong).sum::<u32>();
                    shots += game.shooters.iter().map(|s| s.shots).sum::<u32>();
                    top += game.shooters.iter().map(|s| s.score).max().unwrap();
                    low += game.shooters.iter().map(|s| s.score).min().unwrap();
                    ties += (game.placements().unwrap()[0].len() > 1) as u32;
                }
                let per = (runs as u32 * count as u32) as f32;
                println!(
                    "{label:>8} x{count}: per player {:.1} shots, {:.1} right, {:.1} wrong ({:.0}% accurate); winner {:.1}, last {:.1}; tied wins {ties}/{runs}",
                    shots as f32 / per,
                    hits as f32 / per,
                    wrong as f32 / per,
                    100.0 * hits as f32 / shots.max(1) as f32,
                    top as f32 / runs as f32,
                    low as f32 / runs as f32,
                );
            }
        }
        let mut wins = [0u32; 4];
        for seed in 0..200 {
            let seats: Vec<Contestant> = crate::bout::SKILLS
                .iter()
                .enumerate()
                .map(|(i, (_, s))| Contestant::bot(i as u8, *s))
                .collect();
            let mut game = GlassCannon::new(&seats, 1000 + seed);
            run(&mut game);
            for seat in &game.placements().unwrap()[0] {
                wins[*seat as usize] += 1;
            }
        }
        println!("mixed table wins by skill (relaxed, steady, sharp, ruthless): {wins:?}");
    }
}
