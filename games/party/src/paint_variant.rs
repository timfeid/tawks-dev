//! Focused paint-combat playlists.  They share the first-person arena and
//! authoritative hit detection with Paint Blasters, while exposing distinct
//! match identities to the room, HUD, and replay stream.

use std::collections::BTreeMap;

use crate::{
    bout::{Contestant, Input, Bout, Stick},
    shooter::{PaintBlasters, target_lane_centers},
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

#[derive(Clone, Copy)]
pub enum PaintVariant {
    TargetFrenzy,
    PopgunKnockout,
    RicochetRumble,
    SiteRetake,
}
impl PaintVariant {
    pub const fn key(self) -> &'static str {
        match self {
            Self::TargetFrenzy => "target",
            Self::PopgunKnockout => "popgun",
            Self::RicochetRumble => "ricochet",
            Self::SiteRetake => "retake",
        }
    }
    const fn rule(self) -> &'static str {
        match self {
            Self::TargetFrenzy => {
                "Be first to hit the shared wall light. Faster reactions score more."
            }
            Self::PopgunKnockout => "One hit knocks you out. Last player standing wins.",
            Self::RicochetRumble => "Circle the cover and win the paint rumble.",
            Self::SiteRetake => {
                "Attackers defend the planted core. Defenders retake the site and hold to defuse."
            }
        }
    }
}
pub const RETAKE_FUSE: f32 = 40.0;
pub const RETAKE_DEFUSE: f32 = 5.0;
/// The bomb sits on open floor on the defending side of the site crate, so the
/// retaking team has to push past cover to reach it.
pub const RETAKE_BOMB: (f32, f32) = (0.0, -3.6);
pub const RETAKE_ZONE: f32 = 2.6;
pub const RETAKE_BLAST: f32 = 7.5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Bomb {
    Ticking,
    Defused,
    Exploded,
}

#[derive(Clone)]
struct Target {
    id: u8,
    owner: u8,
    slot: u8,
    x: f32,
    y: f32,
    z: f32,
    active: bool,
    up_at: f32,
}

pub struct PaintPlaylist {
    kind: PaintVariant,
    inner: PaintBlasters,
    time: f32,
    targets: Vec<Target>,
    scores: BTreeMap<u8, u32>,
    placed: Option<Vec<Vec<u8>>>,
    rng: StdRng,
    target_shown_at: f32,
    target_slot: u8,
    target_expires_at: f32,
    defuse_progress: f32,
    fuse: f32,
    bomb: Bomb,
    /// How many started on each side, so an empty side is not a walkover.
    started: [usize; 2],
    nav: BTreeMap<u8, Nav>,
}

/// What a bot remembers about walking into things.
#[derive(Clone, Copy, Default)]
struct Nav {
    x: f32,
    z: f32,
    stuck: f32,
    side: f32,
}

impl PaintPlaylist {
    pub fn new(kind: PaintVariant, c: &[Contestant], seed: u64) -> Self {
        let mut inner = PaintBlasters::new(c, seed);
        if matches!(kind, PaintVariant::TargetFrenzy) {
            inner.use_target_range();
        }
        if matches!(kind, PaintVariant::PopgunKnockout) {
            inner.use_popgun_knockout();
        }
        if matches!(kind, PaintVariant::RicochetRumble) {
            inner.use_ricochet_table();
        }
        if matches!(kind, PaintVariant::SiteRetake) {
            inner.use_retake_site();
        }
        let mut rng = StdRng::seed_from_u64(seed ^ 0xA17A_2E57);
        let first_slot = rng.random_range(0..9) as u8;
        let targets = if matches!(kind, PaintVariant::TargetFrenzy) {
            let mut wall = Vec::new();
            for (lane, (&center, contestant)) in target_lane_centers(c.len())
                .iter()
                .zip(c.iter())
                .enumerate()
            {
                for (row, y) in [0.9, 2.05, 3.2].into_iter().enumerate() {
                    for (column, offset) in [-2.0, 0.0, 2.0].into_iter().enumerate() {
                        let slot = (row * 3 + column) as u8;
                        wall.push(Target {
                            id: (lane * 9 + slot as usize) as u8,
                            owner: contestant.seat,
                            slot,
                            x: center + offset,
                            y,
                            z: -31.58,
                            active: slot == first_slot,
                            up_at: f32::INFINITY,
                        });
                    }
                }
            }
            wall
        } else {
            Vec::new()
        };
        let mut started = [0usize; 2];
        for contestant in c {
            if let Some(team) = inner.team_of(contestant.seat) {
                started[team.min(1) as usize] += 1;
            }
        }
        Self {
            kind,
            inner,
            time: 0.0,
            targets,
            scores: c.iter().map(|p| (p.seat, 0)).collect(),
            placed: None,
            rng,
            target_shown_at: 0.0,
            target_slot: first_slot,
            target_expires_at: 3.0,
            defuse_progress: 0.0,
            fuse: RETAKE_FUSE,
            bomb: Bomb::Ticking,
            started,
            nav: BTreeMap::new(),
        }
    }

    fn target_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self
            .inner
            .fighters
            .iter()
            .find(|fighter| fighter.seat == seat && fighter.hp > 0)
            .cloned()
        else {
            return Input::default();
        };
        let Some(target) = self
            .targets
            .iter()
            .filter(|target| target.owner == seat && target.active)
            .min_by(|a, b| {
                ((a.x - me.x).powi(2) + (a.z - me.z).powi(2))
                    .partial_cmp(&((b.x - me.x).powi(2) + (b.z - me.z).powi(2)))
                    .unwrap()
            })
        else {
            return self.inner.bot_input(seat, dt);
        };
        let (dx, dz) = (target.x - me.x, target.z - me.z);
        let distance = (dx * dx + dz * dz).sqrt().max(0.1);
        // Bots should feel like party opponents, not an aim-bot benchmark.
        // Their aim drifts around the target, they periodically hesitate, and
        // their side-step can put a prop between them and a shot.
        let sharpness = self.inner.skill_of(seat);
        let wobble = (self.time * 1.65 + seat as f32 * 2.71).sin() * (1.4 - sharpness.level());
        let yaw = (-dx).atan2(-dz) + wobble * 0.006;
        let pitch = ((target.y - (me.y + 1.38)) / distance).atan()
            + (self.time * 1.07 + seat as f32).cos() * 0.004;
        // A relaxed bot hangs back before slapping the light.
        let firing =
            (self.time * 4.1 + seat as f32 * 1.93).sin() > 0.48 + (1.0 - sharpness.level()) * 0.42;
        let advance = if distance > 34.0 { -0.82 } else { -0.12 };
        Input {
            stick: Stick {
                x: wobble * 0.42,
                z: advance,
            },
            yaw,
            pitch,
            fire: firing,
            jump: false,
            crouch: false,
        }
    }

    fn score_targets(&mut self, events: &mut Vec<Value>, start: usize) {
        let shots: Vec<Value> = events[start..]
            .iter()
            .filter(|event| event["kind"] == "shot")
            .cloned()
            .collect();
        for shot in &shots {
            let Some(seat) = shot["seat"].as_u64().map(|v| v as u8) else {
                continue;
            };
            let Some(from) = point(&shot["from"]) else {
                continue;
            };
            let Some(to) = point(&shot["to"]) else {
                continue;
            };
            let direction = (to.0 - from.0, to.1 - from.1, to.2 - from.2);
            let length2 =
                direction.0 * direction.0 + direction.1 * direction.1 + direction.2 * direction.2;
            if length2 < 0.01 {
                continue;
            }
            if let Some(index) = self.targets.iter().position(|target| {
                target.owner == seat
                    && target.active
                    && segment_distance2(from, direction, (target.x, target.y, target.z), length2)
                        < 0.62 * 0.62
            }) {
                let (target_id, x, y, z) = {
                    let target = &mut self.targets[index];
                    target.active = false;
                    target.up_at = f32::INFINITY;
                    (target.id, target.x, target.y, target.z)
                };
                let reaction = (self.time - self.target_shown_at).max(0.0);
                let points = (1000.0 - reaction * 350.0).clamp(100.0, 1000.0).round() as u32;
                *self.scores.entry(seat).or_default() += points;
                events.push(json!({ "kind": "target_hit", "seat": seat, "target": target_id, "x": x, "y": y, "z": z, "points": points, "reaction": reaction }));
                if !self.targets.iter().any(|target| target.active) {
                    self.queue_wall_round();
                }
            }
        }
    }

    fn queue_wall_round(&mut self) {
        let mut slot = self.rng.random_range(0..8) as u8;
        if slot >= self.target_slot {
            slot += 1;
        }
        self.target_slot = slot;
        let ready = self.time + self.rng.random_range(0.35..0.7);
        for target in &mut self.targets {
            target.active = false;
            target.up_at = if target.slot == slot {
                ready
            } else {
                f32::INFINITY
            };
        }
        self.target_expires_at = f32::INFINITY;
    }

    fn raise_wall_targets(&mut self) {
        if self.targets.iter().any(|target| target.active) && self.time >= self.target_expires_at {
            self.queue_wall_round();
        }
        let mut raised = false;
        for target in &mut self.targets {
            if !target.active && target.up_at <= self.time {
                target.active = true;
                target.up_at = f32::INFINITY;
                raised = true;
            }
        }
        if raised {
            self.target_shown_at = self.time;
            self.target_expires_at = self.time + 3.0;
        }
    }

    fn settle_targets(&mut self) {
        if self.placed.is_some() || self.time < 60.0 {
            return;
        }
        let mut rows: Vec<(u32, u8)> = self
            .scores
            .iter()
            .map(|(&seat, &score)| (score, seat))
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        let mut groups = Vec::<Vec<u8>>::new();
        let mut score = None;
        for (next, seat) in rows {
            if score == Some(next) {
                groups.last_mut().unwrap().push(seat)
            } else {
                groups.push(vec![seat]);
                score = Some(next);
            }
        }
        self.placed = Some(groups);
    }

    fn settle_popgun(&mut self) {
        if self.placed.is_some() {
            return;
        }
        let standing = self
            .inner
            .fighters
            .iter()
            .filter(|fighter| fighter.hp > 0)
            .count();
        if standing > 1 && self.time < 45.0 {
            return;
        }
        // A survivor always wins. If the timer expires with more than one
        // contestant up, direct knockouts break the tie before deaths do.
        let mut rows: Vec<(bool, u32, i64, u8)> = self
            .inner
            .fighters
            .iter()
            .map(|fighter| {
                (
                    fighter.hp > 0,
                    fighter.kills,
                    -(fighter.deaths as i64),
                    fighter.seat,
                )
            })
            .collect();
        rows.sort_by(|a, b| (b.0, b.1, b.2).cmp(&(a.0, a.1, a.2)));
        let mut groups = Vec::<Vec<u8>>::new();
        let mut score = None;
        for (alive, kills, deaths, seat) in rows {
            let next = (alive, kills, deaths);
            if score == Some(next) {
                groups.last_mut().unwrap().push(seat);
            } else {
                groups.push(vec![seat]);
                score = Some(next);
            }
        }
        self.placed = Some(groups);
    }

    fn alive_on(&self, team: u8) -> usize {
        self.inner
            .fighters
            .iter()
            .filter(|fighter| fighter.hp > 0 && self.inner.team_of(fighter.seat) == Some(team))
            .count()
    }

    /// Everyone on the retaking side standing on the bomb.
    fn on_the_bomb(&self) -> Vec<u8> {
        self.inner
            .fighters
            .iter()
            .filter(|fighter| {
                fighter.hp > 0
                    && self.inner.team_of(fighter.seat) == Some(1)
                    && at_bomb(fighter.x, fighter.z) <= RETAKE_ZONE * RETAKE_ZONE
            })
            .map(|fighter| fighter.seat)
            .collect()
    }

    /// The bomb ticks down whatever else is happening. Killing the other side
    /// never ends it on its own: the retake is only won by defusing in time.
    fn step_retake(&mut self, dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() || self.bomb != Bomb::Ticking {
            return;
        }
        self.fuse = (self.fuse - dt).max(0.0);

        let defusers = self.on_the_bomb();
        let before = self.defuse_progress;
        if defusers.is_empty() {
            // Step off the bomb and the wire goes back in.
            self.defuse_progress = 0.0;
        } else {
            self.defuse_progress = (self.defuse_progress + dt).min(RETAKE_DEFUSE);
        }
        if before <= 0.0 && self.defuse_progress > 0.0 {
            events.push(json!({ "kind": "defuse_start", "seat": defusers.first().copied() }));
        } else if before > 0.0 && self.defuse_progress <= 0.0 {
            events.push(json!({ "kind": "defuse_stop" }));
        }

        let retakers_left = self.started[1] == 0 || self.alive_on(1) > 0;
        let winner = if self.defuse_progress >= RETAKE_DEFUSE {
            self.bomb = Bomb::Defused;
            events.push(json!({
                "kind": "defused",
                "seat": defusers.first().copied(),
                "fuse": self.fuse,
            }));
            Some(1)
        } else if self.fuse <= 0.0 {
            self.bomb = Bomb::Exploded;
            let caught: Vec<u8> = self
                .inner
                .fighters
                .iter()
                .filter(|fighter| {
                    fighter.hp > 0 && at_bomb(fighter.x, fighter.z) <= RETAKE_BLAST * RETAKE_BLAST
                })
                .map(|fighter| fighter.seat)
                .collect();
            for fighter in self.inner.fighters.iter_mut() {
                if caught.contains(&fighter.seat) {
                    fighter.hp = 0;
                }
            }
            events.push(json!({
                "kind": "exploded",
                "x": RETAKE_BOMB.0,
                "z": RETAKE_BOMB.1,
                "caught": caught,
            }));
            Some(0)
        } else if !retakers_left {
            // Nobody left to defuse, so the bomb may as well have gone off.
            events.push(json!({ "kind": "retake_failed" }));
            Some(0)
        } else {
            None
        };

        if let Some(winner) = winner {
            let winners = self
                .inner
                .fighters
                .iter()
                .filter(|fighter| self.inner.team_of(fighter.seat) == Some(winner))
                .map(|fighter| fighter.seat)
                .collect::<Vec<u8>>();
            let losers = self
                .inner
                .fighters
                .iter()
                .filter(|fighter| self.inner.team_of(fighter.seat) != Some(winner))
                .map(|fighter| fighter.seat)
                .collect::<Vec<u8>>();
            self.placed = Some(
                vec![winners, losers]
                    .into_iter()
                    .filter(|side| !side.is_empty())
                    .collect(),
            );
        }
    }

    /// Bots play the objective: the retaking side pushes on and stands on the
    /// bomb when the site is clear, and the defending side rotates back when
    /// someone is on it.
    fn retake_input(&mut self, seat: u8, dt: f32) -> Input {
        let fighting = self.inner.bot_input(seat, dt);
        let Some(me) = self.inner.fighters.iter().find(|f| f.seat == seat).cloned() else {
            return fighting;
        };
        if me.hp <= 0 {
            return fighting;
        }
        let team = self.inner.team_of(seat).unwrap_or(0);
        let distance2 = at_bomb(me.x, me.z);
        let enemy_near = self.inner.fighters.iter().any(|other| {
            other.hp > 0
                && self.inner.team_of(other.seat) != Some(team)
                && (other.x - me.x).powi(2) + (other.z - me.z).powi(2) < 13.0 * 13.0
        });
        let toward_site = Input {
            stick: Stick { x: 0.0, z: -1.0 },
            yaw: (me.x - RETAKE_BOMB.0).atan2(me.z - RETAKE_BOMB.1),
            pitch: 0.0,
            fire: fighting.fire,
            jump: false,
            crouch: false,
        };
        if team == 1 {
            if distance2 <= RETAKE_ZONE * RETAKE_ZONE {
                // Standing on it already: hold still and watch the lanes.
                return Input {
                    stick: Stick::default(),
                    ..fighting
                };
            }
            if !enemy_near {
                return self.unstick(seat, me.x, me.z, toward_site, dt);
            }
        } else if !enemy_near && distance2 > 8.0 * 8.0 {
            // Guarding the bomb means staying near it, not wandering the map.
            return self.unstick(seat, me.x, me.z, toward_site, dt);
        }
        fighting
    }

    /// Walking straight at the site runs into walls, so a bot that stops
    /// making ground slides along whatever it is pressed against.
    fn unstick(&mut self, seat: u8, x: f32, z: f32, want: Input, dt: f32) -> Input {
        let memo = self.nav.entry(seat).or_insert(Nav {
            x,
            z,
            stuck: 0.0,
            side: 1.0,
        });
        let moved = ((x - memo.x).powi(2) + (z - memo.z).powi(2)).sqrt();
        memo.x = x;
        memo.z = z;
        if moved < dt * 1.5 {
            memo.stuck += dt;
        } else {
            memo.stuck = (memo.stuck - dt * 2.0).max(0.0);
        }
        if memo.stuck > 1.6 {
            memo.side = -memo.side;
            memo.stuck = 0.4;
        }
        if memo.stuck > 0.25 {
            let side = memo.side;
            return Input {
                stick: Stick { x: side, z: -0.55 }.clamped(),
                ..want
            };
        }
        want
    }
}

#[cfg(test)]
impl PaintPlaylist {
    pub fn team_for_test(&self, seat: u8) -> Option<u8> {
        self.inner.team_of(seat)
    }

    pub fn place_for_test(&mut self, seat: u8, x: f32, z: f32) {
        if let Some(fighter) = self.inner.fighters.iter_mut().find(|f| f.seat == seat) {
            fighter.x = x;
            fighter.z = z;
            fighter.y = 0.0;
            fighter.vx = 0.0;
            fighter.vz = 0.0;
        }
    }

    pub fn drop_for_test(&mut self, seat: u8) {
        if let Some(fighter) = self.inner.fighters.iter_mut().find(|f| f.seat == seat) {
            fighter.hp = 0;
            // Nobody comes back in a retake, so the timer never runs out.
            fighter.respawn_in = f32::INFINITY;
        }
    }

    pub fn set_fuse_for_test(&mut self, seconds: f32) {
        self.fuse = seconds;
    }

    pub fn defuse_progress_for_test(&self) -> f32 {
        self.defuse_progress
    }
}

/// How far a spot is from the bomb, squared.
fn at_bomb(x: f32, z: f32) -> f32 {
    (x - RETAKE_BOMB.0).powi(2) + (z - RETAKE_BOMB.1).powi(2)
}

fn point(value: &Value) -> Option<(f32, f32, f32)> {
    let point = value.as_array()?;
    Some((
        point.first()?.as_f64()? as f32,
        point.get(1)?.as_f64()? as f32,
        point.get(2)?.as_f64()? as f32,
    ))
}

fn segment_distance2(
    from: (f32, f32, f32),
    direction: (f32, f32, f32),
    target: (f32, f32, f32),
    length2: f32,
) -> f32 {
    let offset = (target.0 - from.0, target.1 - from.1, target.2 - from.2);
    let t = ((offset.0 * direction.0 + offset.1 * direction.1 + offset.2 * direction.2) / length2)
        .clamp(0.0, 1.0);
    let (x, y, z) = (
        from.0 + direction.0 * t - target.0,
        from.1 + direction.1 * t - target.1,
        from.2 + direction.2 * t - target.2,
    );
    x * x + y * y + z * z
}

impl Bout for PaintPlaylist {
    fn key(&self) -> &'static str {
        self.kind.key()
    }
    fn setup(&self) -> Value {
        let mut value = self.inner.setup();
        if let Some(o) = value.as_object_mut() {
            o.insert("game".into(), Value::String(self.kind.key().into()));
            o.insert("rule".into(), json!(self.kind.rule()));
        }
        value
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        self.time += dt;
        if matches!(self.kind, PaintVariant::TargetFrenzy) {
            self.raise_wall_targets();
        }
        let before = events.len();
        self.inner.step(inputs, dt, events);
        match self.kind {
            PaintVariant::TargetFrenzy => {
                self.score_targets(events, before);
                self.settle_targets();
            }
            PaintVariant::PopgunKnockout => self.settle_popgun(),
            PaintVariant::SiteRetake => self.step_retake(dt, events),
            PaintVariant::RicochetRumble => {}
        }
    }
    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        match self.kind {
            PaintVariant::TargetFrenzy => self.target_input(seat, dt),
            PaintVariant::SiteRetake => self.retake_input(seat, dt),
            _ => self.inner.bot_input(seat, dt),
        }
    }
    fn snapshot(&self) -> Value {
        let mut value = self.inner.snapshot();
        if let Some(o) = value.as_object_mut() {
            o.insert("game".into(), Value::String(self.kind.key().into()));
            if matches!(self.kind, PaintVariant::TargetFrenzy) {
                o.insert("time".into(), json!(self.time));
                o.insert("limit".into(), json!(60.0));
                o.insert("targets".into(), json!(self.targets.iter().map(|target| json!({ "id": target.id, "owner": target.owner, "x": target.x, "y": target.y, "z": target.z, "up": target.active })).collect::<Vec<_>>()));
                o.insert("scores".into(), json!(self.scores));
            }
            if matches!(self.kind, PaintVariant::PopgunKnockout) {
                o.insert("time".into(), json!(self.time));
                o.insert("limit".into(), json!(45.0));
                o.insert("sudden_death".into(), Value::Bool(true));
            }
            if matches!(self.kind, PaintVariant::SiteRetake) {
                // The round clock is the fuse, so the HUD counts the bomb down.
                o.insert("time".into(), json!(RETAKE_FUSE - self.fuse));
                o.insert("limit".into(), json!(RETAKE_FUSE));
                o.insert(
                    "objective".into(),
                    json!({
                        "x": RETAKE_BOMB.0,
                        "z": RETAKE_BOMB.1,
                        "progress": self.defuse_progress,
                        "needed": RETAKE_DEFUSE,
                        "fuse": self.fuse,
                        "fuse_max": RETAKE_FUSE,
                        "zone": RETAKE_ZONE,
                        "defusing": !self.on_the_bomb().is_empty(),
                        "state": match self.bomb {
                            Bomb::Ticking => "ticking",
                            Bomb::Defused => "defused",
                            Bomb::Exploded => "exploded",
                        },
                        "active": self.placed.is_none(),
                    }),
                );
                if let Some(players) = o.get_mut("players").and_then(Value::as_array_mut) {
                    for player in players {
                        let seat = player["seat"].as_u64().unwrap_or_default() as u8;
                        if let Some(view) = player.as_object_mut() {
                            view.insert(
                                "team".into(),
                                json!(self.inner.team_of(seat).unwrap_or_default()),
                            );
                        }
                    }
                }
            }
        }
        value
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone().or_else(|| {
            if matches!(self.kind, PaintVariant::TargetFrenzy) {
                None
            } else {
                self.inner.placements()
            }
        })
    }
}
