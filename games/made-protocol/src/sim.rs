//! Simulations: the second kind of made activity. A sim is one `sim.js`
//! against `@tawks/made-kit/sim` (open/sdk/made-kit/src/sim.js). made-simd
//! runs it in the same sandbox as games, stepping it at a fixed rate while it
//! plays; the controls, the transport (play, pause, step, reset, speed) and
//! grabbing a shape on the canvas all arrive as [`SimControl`]s. Its view
//! describes a flat 2D scene in world units with y up, readouts, and plot
//! samples, as [`SimFrame`]s; the relay diffs those into [`SimDelta`]s and
//! the kit client draws them with Tawks' own controls, plots, ruler and
//! stopwatch. Nothing the agent writes reaches a browser.
//!
//! In a lobby everyone sees the same run. One person at a time is the
//! controller, whose controls the relay passes on; anyone may take control.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::simd::Gate;

pub const MAX_SIM_ENTITIES: usize = 600;
pub const MAX_PATH_POINTS: usize = 1000;
pub const MAX_FRAME_POINTS: usize = 6000;
pub const MAX_READOUTS: usize = 12;
pub const MAX_PARAMS: usize = 12;
pub const MAX_PLOTS: usize = 3;
pub const MAX_SERIES: usize = 4;
/// Points a plot keeps; older ones scroll away.
pub const MAX_PLOT_POINTS: usize = 600;
pub const MAX_SIM_COORD: f64 = 1.0e6;
pub const MAX_SIM_FRAME_BYTES: usize = 128 * 1024;
pub const MAX_TEXT: usize = 80;
pub const SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MadeKind {
    #[default]
    Game,
    Sim,
}

impl MadeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MadeKind::Game => "game",
            MadeKind::Sim => "sim",
        }
    }

    pub fn parse(kind: &str) -> MadeKind {
        if kind == "sim" {
            MadeKind::Sim
        } else {
            MadeKind::Game
        }
    }
}

/// Palette names; the client maps each to the Tawks theme for light and dark.
#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SimColor {
    /// The text colour: black on light, white on dark.
    Ink,
    Muted,
    /// Thin rules and grid lines.
    Faint,
    /// The panel colour, for shapes that read as solid ground or walls.
    Surface,
    /// The Tawks brand colour.
    Accent,
    Blue,
    Orange,
    Green,
    Yellow,
    Pink,
    Purple,
    Red,
    Teal,
    Sky,
    Earth,
    Water,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    /// `x, y, r`.
    Circle,
    /// Centred on `x, y`, `w` by `h`, turned by `angle` radians.
    Rect,
    /// From `x, y` to `x2, y2`.
    Line,
    /// Through `points`, `closed` to fill it.
    Path,
    /// A vector from its tail `x, y` to its head `x2, y2`.
    Arrow,
    /// A coil from `x, y` to `x2, y2`, `coils` turns `r` wide.
    Spring,
    /// `text` at `x, y`.
    Text,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// One shape. Positions and sizes are world units; `width` and `size` are
/// screen pixels so lines and labels stay readable at any zoom.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub id: String,
    pub kind: ShapeKind,
    pub x: f64,
    pub y: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y2: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<Vec<[f64; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coils: Option<u32>,
    /// The outline or line colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<SimColor>,
    /// The fill, for circles, rects and closed paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<SimColor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dash: Option<bool>,
    /// 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f64>,
    /// A short label drawn beside the shape (an arrow's name, a mass).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<TextAlign>,
    /// The controller can drag it; the sim's `grab` hook gets the moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<bool>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum ParamValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    Slider,
    Number,
    Toggle,
    Choice,
    /// A press, such as Fire or Shock the market; its value stays false.
    Button,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParamOption {
    pub value: String,
    pub label: String,
}

/// One control in the panel.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParamSpec {
    pub key: String,
    pub label: String,
    pub kind: ParamKind,
    pub value: ParamValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ParamOption>,
    /// Changing it starts the run over with the new value.
    #[serde(default)]
    pub restart: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl ParamSpec {
    /// The value a control may take, or why not.
    pub fn accept(&self, value: &ParamValue) -> Result<ParamValue, String> {
        match (self.kind, value) {
            (ParamKind::Toggle, ParamValue::Bool(b)) => Ok(ParamValue::Bool(*b)),
            (ParamKind::Button, ParamValue::Bool(b)) => Ok(ParamValue::Bool(*b)),
            (ParamKind::Slider | ParamKind::Number, ParamValue::Number(n)) if n.is_finite() => {
                let lo = self.min.unwrap_or(f64::MIN);
                let hi = self.max.unwrap_or(f64::MAX);
                Ok(ParamValue::Number(n.clamp(lo, hi)))
            }
            (ParamKind::Choice, ParamValue::Text(t))
                if self.options.iter().any(|o| &o.value == t) =>
            {
                Ok(ParamValue::Text(t.clone()))
            }
            _ => Err(format!("{} does not take {value:?}", self.key)),
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SeriesSpec {
    pub key: String,
    pub label: String,
    pub color: SimColor,
}

/// A live line plot of samples against time.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlotSpec {
    pub id: String,
    pub title: String,
    pub y_label: String,
    pub series: Vec<SeriesSpec>,
    /// Seconds of sim time across the plot.
    pub window: f64,
    /// A fixed y range; absent to fit the data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<[f64; 2]>,
}

/// The part of the world the canvas shows, in world units, y up.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimWorld {
    pub x: [f64; 2],
    pub y: [f64; 2],
    /// What one world unit is, e.g. "m"; the ruler reads in it.
    pub unit: String,
    /// Grid spacing in world units; absent for no grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<f64>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimTools {
    pub ruler: bool,
    pub stopwatch: bool,
}

/// A sim's `meta`, as checked by the kit's `defineSim`.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimMeta {
    pub name: String,
    /// One line: what it shows.
    pub about: String,
    /// Two or three sentences: what to try.
    pub explain: String,
    pub world: SimWorld,
    pub params: Vec<ParamSpec>,
    pub plots: Vec<PlotSpec>,
    /// Fixed steps per second of sim time.
    pub rate: u32,
    pub tools: SimTools,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Readout {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digits: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<SimColor>,
}

/// The newest samples of one plot. `epoch` changes when the plot starts over
/// (a reset); `total` counts the samples since then, so a client that holds
/// some of them appends only the ones it lacks. Each point is `[t, v1, v2...]`
/// in the order of the plot's series.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlotChunk {
    pub id: String,
    pub epoch: u32,
    pub total: u32,
    pub points: Vec<Vec<f64>>,
}

/// The whole sim at sim time `t`.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimFrame {
    pub n: u32,
    pub t: f64,
    pub playing: bool,
    pub speed: f32,
    pub params: BTreeMap<String, ParamValue>,
    pub shapes: Vec<Shape>,
    pub readouts: Vec<Readout>,
    pub plots: Vec<PlotChunk>,
    /// A short line over the canvas, such as "Landed at 41.2 m".
    #[serde(default)]
    pub notice: Option<String>,
}

/// A frame as the relay sends it: the shapes that changed since frame
/// `base` and the ids that went away. With `base` null it is full.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SimDelta {
    pub n: u32,
    pub base: Option<u32>,
    pub t: f64,
    pub playing: bool,
    pub speed: f32,
    pub params: BTreeMap<String, ParamValue>,
    pub set: Vec<Shape>,
    pub del: Vec<String>,
    pub readouts: Vec<Readout>,
    pub plots: Vec<PlotChunk>,
    #[serde(default)]
    pub notice: Option<String>,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GrabPhase {
    Start,
    Move,
    End,
}

/// What the controller does.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum SimControl {
    Set {
        key: String,
        value: ParamValue,
    },
    Play,
    Pause,
    /// One frame's worth of steps while paused.
    Step,
    Reset,
    Speed {
        speed: f32,
    },
    /// Dragging a shape drawn with `drag`, at world point `x, y`.
    Grab {
        id: String,
        x: f64,
        y: f64,
        phase: GrabPhase,
    },
}

impl SimControl {
    /// Drops what no honest client sends.
    pub fn sanitized(self) -> Option<SimControl> {
        match self {
            SimControl::Speed { speed } => SPEEDS
                .iter()
                .copied()
                .find(|s| (s - speed).abs() < 1e-3)
                .map(|speed| SimControl::Speed { speed }),
            SimControl::Grab { id, x, y, phase } => (x.is_finite()
                && y.is_finite()
                && x.abs() < MAX_SIM_COORD
                && y.abs() < MAX_SIM_COORD
                && id.len() <= 48)
                .then_some(SimControl::Grab { id, x, y, phase }),
            SimControl::Set { key, value } => {
                (key.len() <= 48).then_some(SimControl::Set { key, value })
            }
            other => Some(other),
        }
    }
}

/// Someone looking at a sim in a lobby.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimViewer {
    pub user_id: String,
    pub name: String,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimWelcome {
    pub user_id: String,
    /// Whoever started the test lobby; they may take control at any time.
    pub owner: bool,
    pub lobby_id: String,
    pub activity_id: String,
    pub version: i32,
    pub meta: SimMeta,
    pub controller: Option<SimViewer>,
    pub viewers: Vec<SimViewer>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimShot {
    pub label: String,
    pub frame: SimFrame,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimReport {
    /// Every gate that is not a warning passed.
    pub ok: bool,
    pub meta: Option<SimMeta>,
    pub gates: Vec<Gate>,
    pub first_error: Option<String>,
    /// One line for the card, e.g. "4 controls checked, 90 s run, p99 step 0.4 ms".
    pub summary: String,
    pub shots: Vec<SimShot>,
}

/// What the render page fetches for one sim still: `kind` is always "sim".
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SimRenderShot {
    pub kind: String,
    pub meta: SimMeta,
    pub frame: SimFrame,
    pub width: u32,
    pub height: u32,
}

fn finite(what: &str, n: f64) -> Result<(), String> {
    if !n.is_finite() {
        return Err(format!("{what} is not a finite number"));
    }
    if n.abs() > MAX_SIM_COORD {
        return Err(format!("{what} is {n}, past {MAX_SIM_COORD}"));
    }
    Ok(())
}

fn short(what: &str, s: &str, max: usize) -> Result<(), String> {
    if s.chars().count() > max {
        return Err(format!("{what} is longer than {max} characters"));
    }
    Ok(())
}

impl Shape {
    fn check(&self) -> Result<usize, String> {
        let what = |field: &str| format!("{} {field}", self.id);
        for (field, value) in [
            ("x", Some(self.x)),
            ("y", Some(self.y)),
            ("x2", self.x2),
            ("y2", self.y2),
            ("r", self.r),
            ("w", self.w),
            ("h", self.h),
            ("angle", self.angle),
            ("width", self.width),
            ("size", self.size),
            ("alpha", self.alpha),
        ] {
            if let Some(value) = value {
                finite(&what(field), value)?;
            }
        }
        for (field, value) in [("r", self.r), ("w", self.w), ("h", self.h)] {
            if value.is_some_and(|v| v < 0.0) {
                return Err(format!("{} is negative", what(field)));
            }
        }
        if self.width.is_some_and(|w| w > 40.0)
            || self.size.is_some_and(|s| !(6.0..=48.0).contains(&s))
        {
            return Err(format!(
                "{} has a line width over 40 px or a text size outside 6 to 48 px",
                self.id
            ));
        }
        let points = self.points.as_ref().map(Vec::len).unwrap_or(0);
        if points > MAX_PATH_POINTS {
            return Err(format!(
                "{} has {points} points; a path holds at most {MAX_PATH_POINTS}",
                self.id
            ));
        }
        for p in self.points.iter().flatten() {
            finite(&what("point"), p[0])?;
            finite(&what("point"), p[1])?;
        }
        for text in [&self.label, &self.text].into_iter().flatten() {
            short(&what("text"), text, MAX_TEXT)?;
        }
        match self.kind {
            ShapeKind::Circle if self.r.is_none() => {
                Err(format!("{} is a circle with no r", self.id))
            }
            ShapeKind::Rect if self.w.is_none() || self.h.is_none() => {
                Err(format!("{} is a rect with no w or h", self.id))
            }
            ShapeKind::Line | ShapeKind::Arrow | ShapeKind::Spring
                if self.x2.is_none() || self.y2.is_none() =>
            {
                Err(format!("{} needs x2 and y2", self.id))
            }
            ShapeKind::Path if points < 2 => {
                Err(format!("{} is a path with fewer than 2 points", self.id))
            }
            ShapeKind::Text if self.text.is_none() => {
                Err(format!("{} is text with no text", self.id))
            }
            _ => Ok(points),
        }
    }
}

impl SimMeta {
    pub fn param(&self, key: &str) -> Option<&ParamSpec> {
        self.params.iter().find(|p| p.key == key)
    }

    pub fn check(&self) -> Result<(), String> {
        short("meta.name", &self.name, 40)?;
        short("meta.about", &self.about, 120)?;
        short("meta.explain", &self.explain, 500)?;
        let w = &self.world;
        for n in [w.x[0], w.x[1], w.y[0], w.y[1]] {
            finite("meta.world", n)?;
        }
        if w.x[1] <= w.x[0] || w.y[1] <= w.y[0] {
            return Err("meta.world needs x and y as [low, high]".into());
        }
        if self.params.len() > MAX_PARAMS || self.plots.len() > MAX_PLOTS {
            return Err(format!(
                "at most {MAX_PARAMS} controls and {MAX_PLOTS} plots"
            ));
        }
        if !(10..=2000).contains(&self.rate) {
            return Err("meta.rate is steps per second, 10 to 2000".into());
        }
        let mut keys = HashSet::new();
        for p in &self.params {
            if !keys.insert(p.key.as_str()) {
                return Err(format!("the control {} is declared twice", p.key));
            }
            p.accept(&p.value)
                .map_err(|e| format!("its starting value: {e}"))?;
        }
        for plot in &self.plots {
            if plot.series.is_empty() || plot.series.len() > MAX_SERIES {
                return Err(format!(
                    "the plot {} needs 1 to {MAX_SERIES} series",
                    plot.id
                ));
            }
            if !(plot.window.is_finite() && plot.window > 0.0) {
                return Err(format!("the plot {} needs a positive window", plot.id));
            }
        }
        Ok(())
    }
}

impl SimFrame {
    /// The checks every frame passes before it leaves the sandbox.
    pub fn check(&self) -> Result<(), String> {
        if !self.t.is_finite() {
            return Err("the frame time is not finite".into());
        }
        if self.shapes.len() > MAX_SIM_ENTITIES {
            return Err(format!(
                "{} shapes in one frame; the most is {MAX_SIM_ENTITIES}",
                self.shapes.len()
            ));
        }
        if self.readouts.len() > MAX_READOUTS {
            return Err(format!(
                "{} readouts; the most is {MAX_READOUTS}",
                self.readouts.len()
            ));
        }
        let mut ids = HashSet::with_capacity(self.shapes.len());
        let mut points = 0;
        for s in &self.shapes {
            if s.id.is_empty() || s.id.len() > 48 {
                return Err(format!("an id must be 1 to 48 characters: {:?}", s.id));
            }
            if !ids.insert(s.id.as_str()) {
                return Err(format!("the id {} is used twice", s.id));
            }
            points += s.check()?;
        }
        if points > MAX_FRAME_POINTS {
            return Err(format!(
                "{points} path points in one frame; the most is {MAX_FRAME_POINTS}"
            ));
        }
        for r in &self.readouts {
            short("a readout label", &r.label, 40)?;
            if let Some(v) = r.value {
                if !v.is_finite() {
                    return Err(format!("the readout {} is not a finite number", r.label));
                }
            }
            if let Some(t) = &r.text {
                short("a readout", t, 40)?;
            }
        }
        for plot in &self.plots {
            for p in &plot.points {
                if p.iter().any(|v| !v.is_finite()) {
                    return Err(format!(
                        "the plot {} has a sample that is not a finite number",
                        plot.id
                    ));
                }
            }
        }
        if let Some(n) = &self.notice {
            short("the notice", n, 120)?;
        }
        Ok(())
    }

    pub fn diff(&self, prev: Option<&SimFrame>) -> SimDelta {
        let (base, set, del) = match prev {
            None => (None, self.shapes.clone(), Vec::new()),
            Some(prev) => {
                let mut before: HashMap<&str, &Shape> =
                    prev.shapes.iter().map(|s| (s.id.as_str(), s)).collect();
                let mut set = Vec::new();
                for s in &self.shapes {
                    match before.remove(s.id.as_str()) {
                        Some(old) if old == s => {}
                        _ => set.push(s.clone()),
                    }
                }
                let mut del: Vec<String> = before.into_keys().map(String::from).collect();
                del.sort();
                (Some(prev.n), set, del)
            }
        };
        SimDelta {
            n: self.n,
            base,
            t: self.t,
            playing: self.playing,
            speed: self.speed,
            params: self.params.clone(),
            set,
            del,
            readouts: self.readouts.clone(),
            plots: self.plots.clone(),
            notice: self.notice.clone(),
        }
    }
}

impl SimDelta {
    /// The full frame this delta makes from `prev`, or None when `prev` is not
    /// its base. Plot points are taken as they come; see [`PlotHistory`].
    pub fn apply(&self, prev: Option<&SimFrame>) -> Option<SimFrame> {
        let mut shapes: Vec<Shape> = match (self.base, prev) {
            (None, _) => Vec::new(),
            (Some(base), Some(prev)) if prev.n == base => prev.shapes.clone(),
            _ => return None,
        };
        shapes.retain(|s| !self.del.contains(&s.id));
        for s in &self.set {
            match shapes.iter_mut().find(|old| old.id == s.id) {
                Some(old) => *old = s.clone(),
                None => shapes.push(s.clone()),
            }
        }
        Some(SimFrame {
            n: self.n,
            t: self.t,
            playing: self.playing,
            speed: self.speed,
            params: self.params.clone(),
            shapes,
            readouts: self.readouts.clone(),
            plots: self.plots.clone(),
            notice: self.notice.clone(),
        })
    }
}

/// Every plot's samples since its last start, kept by the relay so someone
/// who joins mid-run gets the whole line.
#[derive(Default, Debug, Clone)]
pub struct PlotHistory {
    plots: BTreeMap<String, (u32, u32, Vec<Vec<f64>>)>,
}

impl PlotHistory {
    pub fn clear(&mut self) {
        self.plots.clear();
    }

    pub fn absorb(&mut self, chunks: &[PlotChunk]) {
        for chunk in chunks {
            let entry = self
                .plots
                .entry(chunk.id.clone())
                .or_insert((chunk.epoch, 0, Vec::new()));
            if entry.0 != chunk.epoch {
                *entry = (chunk.epoch, 0, Vec::new());
            }
            let have = entry.1;
            let first = chunk.total.saturating_sub(chunk.points.len() as u32);
            for (i, point) in chunk.points.iter().enumerate() {
                if first + i as u32 >= have {
                    entry.2.push(point.clone());
                }
            }
            entry.1 = entry.1.max(chunk.total);
            if entry.2.len() > MAX_PLOT_POINTS {
                let extra = entry.2.len() - MAX_PLOT_POINTS;
                entry.2.drain(..extra);
            }
        }
    }

    /// The whole of every plot, as chunks a joining client can start from.
    pub fn full(&self) -> Vec<PlotChunk> {
        self.plots
            .iter()
            .map(|(id, (epoch, total, points))| PlotChunk {
                id: id.clone(),
                epoch: *epoch,
                total: *total,
                points: points.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(id: &str, x: f64) -> Shape {
        Shape {
            id: id.into(),
            kind: ShapeKind::Circle,
            x,
            y: 0.0,
            x2: None,
            y2: None,
            r: Some(0.1),
            w: None,
            h: None,
            angle: None,
            points: None,
            closed: None,
            coils: None,
            color: None,
            fill: Some(SimColor::Blue),
            width: None,
            dash: None,
            alpha: None,
            label: None,
            text: None,
            size: None,
            align: None,
            drag: None,
        }
    }

    fn frame(n: u32, shapes: Vec<Shape>, plots: Vec<PlotChunk>) -> SimFrame {
        SimFrame {
            n,
            t: n as f64 / 20.0,
            playing: true,
            speed: 1.0,
            params: BTreeMap::new(),
            shapes,
            readouts: Vec::new(),
            plots,
            notice: None,
        }
    }

    #[test]
    fn a_delta_rebuilds_the_frame() {
        let a = frame(1, vec![shape("a", 0.0), shape("b", 1.0)], vec![]);
        let b = frame(2, vec![shape("a", 0.5), shape("c", 2.0)], vec![]);
        let delta = b.diff(Some(&a));
        assert_eq!(delta.set.len(), 2);
        assert_eq!(delta.del, vec!["b".to_string()]);
        let rebuilt = delta.apply(Some(&a)).unwrap();
        assert_eq!(rebuilt.shapes.len(), 2);
        assert!(delta.apply(Some(&b)).is_none());
    }

    #[test]
    fn bad_frames_are_refused() {
        let mut bad = shape("a", f64::NAN);
        assert!(frame(1, vec![bad.clone()], vec![]).check().is_err());
        bad.x = 0.0;
        bad.r = None;
        assert!(frame(1, vec![bad], vec![]).check().is_err());
        let json = r#"{"id":"a","kind":"circle","x":0,"y":0,"r":1,"onclick":"x"}"#;
        assert!(serde_json::from_str::<Shape>(json).is_err());
    }

    #[test]
    fn plot_history_appends_only_new_points_and_starts_over_on_a_new_epoch() {
        let mut h = PlotHistory::default();
        let chunk = |epoch, total, from: u32, to: u32| PlotChunk {
            id: "e".into(),
            epoch,
            total,
            points: (from..to).map(|i| vec![i as f64, 1.0]).collect(),
        };
        h.absorb(&[chunk(1, 3, 0, 3)]);
        h.absorb(&[chunk(1, 5, 2, 5)]);
        assert_eq!(h.full()[0].points.len(), 5);
        h.absorb(&[chunk(2, 1, 0, 1)]);
        assert_eq!(h.full()[0].points.len(), 1);
        assert_eq!(h.full()[0].epoch, 2);
    }

    #[test]
    fn controls_are_clamped_or_refused() {
        let spec = ParamSpec {
            key: "g".into(),
            label: "Gravity".into(),
            kind: ParamKind::Slider,
            value: ParamValue::Number(9.8),
            min: Some(0.0),
            max: Some(20.0),
            step: Some(0.1),
            unit: None,
            options: vec![],
            restart: false,
            help: None,
        };
        assert_eq!(
            spec.accept(&ParamValue::Number(50.0)).unwrap(),
            ParamValue::Number(20.0)
        );
        assert!(spec.accept(&ParamValue::Bool(true)).is_err());
        assert!(SimControl::Speed { speed: 3.0 }.sanitized().is_none());
        assert!(SimControl::Speed { speed: 2.0 }.sanitized().is_some());
    }
}
