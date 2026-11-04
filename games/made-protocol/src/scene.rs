//! Scene frames: what a game's `view` describes and the kit draws. The kit's
//! `createView` (open/sdk/made-kit/src/core.js) builds these, the sandbox
//! checks them with [`SceneFrame::check`], and the relay diffs them into
//! [`FrameDelta`]s by entity id. `scene.ts` is generated from this file.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use specta::Type;

pub const MAX_ENTITIES: usize = 400;
pub const MAX_EFFECTS: usize = 32;
pub const MAX_ID_LEN: usize = 48;
pub const MAX_COORD: f32 = 1000.0;
/// The largest full frame the sandbox will pass on, as JSON.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    /// A seat's Tawkathon character in its costume. Id `p<seat>`.
    Player,
    /// A flat cylinder: a floe, a platform, a hole. `r` and `h`.
    Disc,
    /// An upright box: a wall, a crate. `w`, `d`, `h` and `yaw`.
    Box,
    /// A sphere resting on `y`: a snowball, a bomb. `r`, and `yaw` as its roll.
    Ball,
    /// A flat ring on the floor: a target, a zone. `r`.
    Marker,
}

/// Palette names; the renderer owns the actual colours. `seat` takes the
/// colour of the entity's `seat`.
#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Color {
    Ice,
    Water,
    Danger,
    Seat,
    Gold,
    Wood,
    Snow,
    Stone,
    Grass,
    Sand,
    Lava,
    Light,
    Dark,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Pose {
    Idle,
    Ride,
    Cheer,
    Sad,
    Fall,
    Sit,
    Wave,
    Play,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Ground {
    Grass,
    Stone,
    Snow,
    Wood,
    Sand,
}

/// Semantic sound names; the renderer maps them to its sound files.
#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Bump,
    Hit,
    Splash,
    Crack,
    Coin,
    Win,
    Lose,
    Tick,
    Bell,
    Power,
    Thud,
    Pop,
    Blocked,
    Confirm,
}

/// One thing in the scene. Positions are metres on the x/z floor with y up;
/// a yaw of 0 faces +z. Fields a kind does not use are absent.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: String,
    pub kind: EntityKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yaw: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seat: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pose: Option<Pose>,
    /// True to keep it in the scene but not draw it; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    /// The Tawkathon sea around everything.
    pub water: bool,
    /// A floor under the play area, or none.
    pub ground: Option<Ground>,
}

/// What the camera shows: a `half_x` by `half_z` box around (`x`, `z`),
/// looked down on at `pitch` degrees, or the same box around `follow`'s player.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub half_x: f32,
    pub half_z: f32,
    pub pitch: f32,
    pub x: f32,
    pub z: f32,
    pub follow: Option<u8>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            half_x: 10.0,
            half_z: 10.0,
            pitch: 55.0,
            x: 0.0,
            z: 0.0,
            follow: None,
        }
    }
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HudScore {
    pub seat: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Still in (true) or out (false), for elimination games.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub up: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HudMeter {
    pub label: String,
    /// 0 to 1.
    pub value: f32,
    pub color: Color,
}

/// The heads-up display, drawn with @tawks/ui over the stage.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Hud {
    pub objective: Option<String>,
    pub alert: Option<String>,
    /// Seconds left, when the game shows a clock.
    pub timer: Option<f32>,
    pub scores: Vec<HudScore>,
    pub meter: Option<HudMeter>,
}

#[derive(Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Sound,
    Burst,
}

/// A one-off: a sound, or a puff of particles at (`x`, `z`). Played once by
/// whoever receives the frame that carries it.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub kind: EffectKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Sound>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}

/// A whole scene at simulated time `t`. `n` counts frames from 0 each round.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SceneFrame {
    pub n: u32,
    pub t: f32,
    pub stage: Stage,
    pub camera: Camera,
    pub hud: Hud,
    pub entities: Vec<Entity>,
    pub effects: Vec<Effect>,
}

/// A frame as sent to players: the entities that changed since frame `base`
/// (`set`) and the ids that went away (`del`). With `base` null it is a full
/// frame and `set` holds everything. The kit's `applyFrame` rebuilds it.
#[derive(Type, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrameDelta {
    pub n: u32,
    pub t: f32,
    pub base: Option<u32>,
    pub stage: Stage,
    pub camera: Camera,
    pub hud: Hud,
    pub set: Vec<Entity>,
    pub del: Vec<String>,
    pub effects: Vec<Effect>,
}

fn finite(what: &str, n: f32) -> Result<(), String> {
    if !n.is_finite() {
        return Err(format!("{what} is not a finite number"));
    }
    if n.abs() > MAX_COORD {
        return Err(format!(
            "{what} is {n}, further than {MAX_COORD} from the centre"
        ));
    }
    Ok(())
}

impl Entity {
    fn check(&self) -> Result<(), String> {
        let what = |field: &str| format!("{} {field}", self.id);
        for (field, value) in [
            ("x", Some(self.x)),
            ("y", Some(self.y)),
            ("z", Some(self.z)),
            ("r", self.r),
            ("w", self.w),
            ("d", self.d),
            ("h", self.h),
            ("yaw", self.yaw),
        ] {
            if let Some(value) = value {
                finite(&what(field), value)?;
            }
        }
        for (field, value) in [("r", self.r), ("w", self.w), ("d", self.d), ("h", self.h)] {
            if value.is_some_and(|v| v < 0.0) {
                return Err(format!("{} is negative", what(field)));
            }
        }
        if let Some(seat) = self.seat {
            if !(1..=8).contains(&seat) {
                return Err(format!("{} is not a seat from 1 to 8", what("seat")));
            }
        }
        if self.kind == EntityKind::Player && self.seat.is_none() {
            return Err(format!("{} is a player with no seat", self.id));
        }
        Ok(())
    }
}

impl SceneFrame {
    /// The checks every frame passes before it leaves the sandbox.
    pub fn check(&self) -> Result<(), String> {
        if !self.t.is_finite() {
            return Err("the frame time is not finite".into());
        }
        if self.entities.len() > MAX_ENTITIES {
            return Err(format!(
                "{} things in one frame; the most is {MAX_ENTITIES}",
                self.entities.len()
            ));
        }
        if self.effects.len() > MAX_EFFECTS {
            return Err(format!(
                "{} effects in one frame; the most is {MAX_EFFECTS}",
                self.effects.len()
            ));
        }
        let mut ids = HashSet::with_capacity(self.entities.len());
        for e in &self.entities {
            if e.id.is_empty() || e.id.len() > MAX_ID_LEN {
                return Err(format!(
                    "an id must be 1 to {MAX_ID_LEN} characters: {:?}",
                    e.id
                ));
            }
            if !ids.insert(e.id.as_str()) {
                return Err(format!("the id {} is used twice", e.id));
            }
            e.check()?;
        }
        let c = &self.camera;
        for (what, n) in [
            ("camera half_x", c.half_x),
            ("camera half_z", c.half_z),
            ("camera pitch", c.pitch),
            ("camera x", c.x),
            ("camera z", c.z),
        ] {
            finite(what, n)?;
        }
        if c.half_x <= 0.0 || c.half_z <= 0.0 {
            return Err("the camera frame must have a positive size".into());
        }
        for text in [&self.hud.objective, &self.hud.alert].into_iter().flatten() {
            if text.chars().count() > 120 {
                return Err("a HUD line is longer than 120 characters".into());
            }
        }
        if let Some(t) = self.hud.timer {
            finite("hud timer", t)?;
        }
        if let Some(m) = &self.hud.meter {
            if !m.value.is_finite() || !(0.0..=1.0).contains(&m.value) {
                return Err("the HUD meter must be between 0 and 1".into());
            }
        }
        for e in &self.effects {
            for n in [e.x, e.z].into_iter().flatten() {
                finite("an effect position", n)?;
            }
        }
        Ok(())
    }

    /// The live player entities, by seat.
    pub fn players(&self) -> impl Iterator<Item = &Entity> {
        self.entities
            .iter()
            .filter(|e| e.kind == EntityKind::Player && e.hidden != Some(true))
    }

    /// The delta from `prev` (None for a full frame).
    pub fn diff(&self, prev: Option<&SceneFrame>) -> FrameDelta {
        let (base, set, del) = match prev {
            None => (None, self.entities.clone(), Vec::new()),
            Some(prev) => {
                let mut before: HashMap<&str, &Entity> =
                    prev.entities.iter().map(|e| (e.id.as_str(), e)).collect();
                let mut set = Vec::new();
                for e in &self.entities {
                    match before.remove(e.id.as_str()) {
                        Some(old) if old == e => {}
                        _ => set.push(e.clone()),
                    }
                }
                let mut del: Vec<String> = before.into_keys().map(String::from).collect();
                del.sort();
                (Some(prev.n), set, del)
            }
        };
        FrameDelta {
            n: self.n,
            t: self.t,
            base,
            stage: self.stage.clone(),
            camera: self.camera.clone(),
            hud: self.hud.clone(),
            set,
            del,
            effects: self.effects.clone(),
        }
    }
}

impl FrameDelta {
    /// The full frame this delta makes from `prev`, or None when `prev` is not its base.
    pub fn apply(&self, prev: Option<&SceneFrame>) -> Option<SceneFrame> {
        let mut entities: Vec<Entity> = match (self.base, prev) {
            (None, _) => Vec::new(),
            (Some(base), Some(prev)) if prev.n == base => prev.entities.clone(),
            _ => return None,
        };
        entities.retain(|e| !self.del.contains(&e.id));
        for e in &self.set {
            match entities.iter_mut().find(|old| old.id == e.id) {
                Some(old) => *old = e.clone(),
                None => entities.push(e.clone()),
            }
        }
        Some(SceneFrame {
            n: self.n,
            t: self.t,
            stage: self.stage.clone(),
            camera: self.camera.clone(),
            hud: self.hud.clone(),
            entities,
            effects: self.effects.clone(),
        })
    }
}
