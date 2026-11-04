//! Skill checks: every crew job, and the police's hands-on jobs, is its own
//! small test of skill instead of a timer. The server runs each check from
//! the raw inputs it receives (stick flicks, dial position, trigger
//! pressure, button presses) and its own clock, so a client can only report
//! what the player did, never claim a result. Failing makes noise police
//! can hear, stuns, or costs progress.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// Power switch: flick the stick to the terminal matching each wire.
    Wires,
    /// Security desk: stop a sweeping cursor inside moving windows.
    Cameras,
    /// Vault: turn the dial and set each number by feel.
    Dial,
    /// Bags: repeat a shown button sequence to stuff a bag.
    Stuff,
    /// Getaway van: hold the trigger's pressure inside a moving band.
    Pressure,
    /// Jail: find the pick's sweet spot and turn without forcing it.
    Lockpick,
    /// Police, a dropped bag: hold the aim steady on a moving tag.
    Steady,
    /// Police, a flat by hand: spin the wheel nuts tight in time.
    LugNuts,
}

/// What a player did during a check: raw input, never a result.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum CheckInput {
    /// Stick flicked toward a terminal: 0 up, 1 right, 2 down, 3 left.
    Flick { dir: u8 },
    /// The A press that stops a cursor or sets a dial number.
    Press,
    /// The dial's position, in ticks from 0 to 40.
    Dial { at: f32 },
    /// A face button: 0 A, 1 B, 2 X, 3 Y.
    Button { b: u8 },
    /// Trigger pressure from 0 to 1.
    Trigger { value: f32 },
    /// Pick position from -1 to 1 and turning pressure from 0 to 1.
    Pick { angle: f32, turn: f32 },
    /// Aim, each axis from -1 to 1.
    Aim { x: f32, y: f32 },
    /// The stick's angle in radians, for spinning nuts.
    Spin { angle: f32 },
    /// Walk away from the check.
    Quit,
}

#[derive(Clone, Debug)]
pub(super) struct Check {
    pub kind: CheckKind,
    pub started: f32,
    pub step: u8,
    pub steps: u8,
    /// When the current step (or the whole check) runs out.
    pub deadline: f32,
    pub seq: Vec<u8>,
    pub perm: [u8; 4],
    pub value: f32,
    pub value2: f32,
    pub progress: f32,
    pub strain: f32,
    pub target: f32,
    pub width: f32,
    pub closest: f32,
    pub fails: u8,
    pub at: (f32, f32),
    /// Bots resolve after this time, with a tuned chance of success.
    pub bot_done: f32,
}

/// The check as its player sees it; fields that do not apply are left out.
#[derive(Clone, Debug, Default, Serialize)]
pub struct CheckView {
    pub kind: Option<CheckKind>,
    pub step: u8,
    pub steps: u8,
    pub time_left: f32,
    pub elapsed: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wire: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminals: Option<[u8; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tremor: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shown: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub showing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strain: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<[f32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picks: Option<u8>,
}

pub const DIAL_TICKS: f32 = 40.;
// The first wire stays up while the controls lesson is shown. Later wires
// still need enough time to find the matching terminal and make a deliberate flick.
const WIRE_SECONDS: [f32; 4] = [4.5, 4.2, 3.8, 3.4];
const WIRE_INTRO_SECONDS: f32 = 5.0;
const CAMERA_WIDTHS: [f32; 3] = [0.14, 0.11, 0.08];
const DIAL_TOLERANCE: [f32; 4] = [1.6, 1.1, 0.75, 0.6];
const SHOW_SECONDS: f32 = 0.6;
const PRESS_SECONDS: f32 = 1.3;
const PICKS: u8 = 3;
const NUT_SECONDS: f32 = 2.6;
/// How long noise from a failed check stays on police compasses.
pub const NOISE_SECONDS: f32 = 4.;

impl CheckKind {
    /// How long a bot takes and how often it succeeds, before skill.
    fn bot_tuning(self) -> (f32, f32) {
        match self {
            CheckKind::Wires => (3.5, 0.72),
            CheckKind::Cameras => (4.5, 0.66),
            CheckKind::Dial => (10., 0.6),
            CheckKind::Stuff => (4.5, 0.74),
            CheckKind::Pressure => (4., 0.72),
            CheckKind::Lockpick => (6., 0.62),
            CheckKind::Steady => (3., 0.76),
            CheckKind::LugNuts => (6., 0.78),
        }
    }
    /// Whether failing it makes noise police can hear.
    fn noisy(self) -> bool {
        !matches!(
            self,
            CheckKind::Steady | CheckKind::LugNuts | CheckKind::Stuff
        )
    }
}

fn circular(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(DIAL_TICKS);
    d.min(DIAL_TICKS - d)
}

impl CityGame {
    /// A hashed number in [0, 1) from the seed and a few salts.
    pub(super) fn chance(&self, a: u8, b: u8, c: u32) -> f32 {
        let mut h = self.seed
            ^ (u64::from(a) << 40)
            ^ (u64::from(b) << 32)
            ^ u64::from(c).wrapping_mul(0x9E37_79B9);
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 32;
        (h % 100_000) as f32 / 100_000.
    }
    /// The check this station uses, if any.
    pub(super) fn check_for(station: Station) -> Option<CheckKind> {
        Some(match station {
            Station::Power => CheckKind::Wires,
            Station::Cameras => CheckKind::Cameras,
            Station::Vault => CheckKind::Dial,
            Station::Bag => CheckKind::Stuff,
            Station::Van => CheckKind::Pressure,
            Station::Jail => CheckKind::Lockpick,
            Station::Loot => CheckKind::Steady,
            Station::Repair => CheckKind::LugNuts,
        })
    }
    pub(super) fn start_check(&mut self, index: usize, station: Station, at: (f32, f32)) -> bool {
        let Some(kind) = Self::check_for(station) else {
            return false;
        };
        let a = &self.actors[index];
        if a.check.is_some() || a.check_wait > 0. {
            return false;
        }
        let seat = a.seat;
        let salt = self.check_seq;
        self.check_seq = self.check_seq.wrapping_add(1);
        let hard = self.tuning.check_difficulty;
        let roll = |i: u32| self.chance(seat, 77, salt.wrapping_mul(31).wrapping_add(i));
        let pick = |i: u32, n: u8| ((roll(i) * f32::from(n)) as u8).min(n - 1);
        let mut c = Check {
            kind,
            started: self.time,
            step: 0,
            steps: 1,
            deadline: self.time + 30.,
            seq: Vec::new(),
            perm: [0, 1, 2, 3],
            value: 0.,
            value2: 0.,
            progress: 0.,
            strain: 0.,
            target: 0.,
            width: 0.,
            closest: f32::MAX,
            fails: 0,
            at,
            bot_done: 0.,
        };
        match kind {
            CheckKind::Wires => {
                c.steps = 4;
                c.seq = (0..4).map(|i| pick(i, 4)).collect();
                c.perm = self.shuffle(seat, salt, 0);
                c.value = WIRE_INTRO_SECONDS + WIRE_SECONDS[0] / hard;
                c.deadline = self.time + c.value;
            }
            CheckKind::Cameras => {
                c.steps = 3;
                c.target = 0.15 + roll(1) * 0.7;
                c.width = CAMERA_WIDTHS[0] / hard;
                c.deadline = self.time + 15.;
            }
            CheckKind::Dial => {
                c.steps = self.tuning.dial_numbers;
                c.seq = (0..c.steps)
                    .map(|i| (roll(10 + u32::from(i)) * DIAL_TICKS) as u8)
                    .collect();
                c.value = (roll(9) * DIAL_TICKS).floor();
                c.deadline = self.time + 45.;
            }
            CheckKind::Stuff => {
                let fresh = distance(at, (BANK.0, BANK.1 - 1.4)) < 3.;
                c.steps = if fresh { 5 } else { 4 };
                c.seq = (0..c.steps).map(|i| pick(20 + u32::from(i), 4)).collect();
                c.deadline = self.time + f32::from(c.steps) * SHOW_SECONDS / hard + PRESS_SECONDS;
            }
            CheckKind::Pressure => {
                c.width = 0.2 / hard;
                c.deadline = self.time + 18.;
            }
            CheckKind::Lockpick => {
                c.target = -0.8 + roll(30) * 1.6;
                c.width = 0.07 / hard;
                c.deadline = self.time + 30.;
            }
            CheckKind::Steady => {
                c.width = 0.16 / hard;
                c.deadline = self.time + 9.;
            }
            CheckKind::LugNuts => {
                c.steps = 4;
                c.deadline = self.time + NUT_SECONDS / hard;
            }
        }
        let a = &mut self.actors[index];
        let (bot_time, _) = kind.bot_tuning();
        c.bot_done = self.time + bot_time * (1.3 - a.skill * 0.5) * hard;
        a.check = Some(c);
        a.interact = false;
        if kind == CheckKind::Dial && !self.alarm {
            self.alarm_at = self.time;
            self.alarm = true;
            hud::push_stamp(&mut self.stamps, &mut self.stamp_seq, "alarm", Side::Police);
        }
        true
    }
    fn shuffle(&self, seat: u8, salt: u32, step: u32) -> [u8; 4] {
        let mut perm = [0u8, 1, 2, 3];
        for i in (1..4).rev() {
            let j = (self.chance(
                seat,
                91,
                salt.wrapping_mul(17).wrapping_add(step * 4 + i as u32),
            ) * (i + 1) as f32) as usize;
            perm.swap(i, j.min(i));
        }
        perm
    }
    /// Somebody fumbled: police hear roughly where.
    pub(super) fn noise(&mut self, at: (f32, f32)) {
        let coarse = ((at.0 / 4.).round() * 4., (at.1 / 4.).round() * 4.);
        self.noises.push((coarse, self.time + NOISE_SECONDS));
        self.impact("noise", at);
    }
    fn fail_check(&mut self, index: usize, wait: f32) {
        let Some(c) = self.actors[index].check.take() else {
            return;
        };
        self.check_fails[c.kind as usize] += 1;
        self.actors[index].check_wait = wait;
        if c.kind.noisy() {
            self.noise(c.at);
        }
        if c.kind == CheckKind::Wires {
            self.stun_for(index, 0.8);
            self.impact("spark", c.at);
        }
        self.impact("fumble", c.at);
    }
    fn pass_check(&mut self, index: usize) {
        let Some(c) = self.actors[index].check.take() else {
            return;
        };
        self.check_passes[c.kind as usize] += 1;
        let seat = self.actors[index].seat;
        self.impact("perfect", c.at);
        match c.kind {
            CheckKind::Wires => {
                self.switch = 1.;
                self.power = self.tuning.power_seconds;
                self.advance_power_job();
            }
            CheckKind::Cameras => {
                self.cameras = self.tuning.power_seconds;
                self.advance_power_job();
            }
            CheckKind::Dial => {
                if let Some(job) = self.jobs.last_mut() {
                    job.reach(stats::Phase::Carry);
                }
                self.vault = 1.;
                self.job = Job::Carry;
                self.spill_bags();
                hud::push_stamp(
                    &mut self.stamps,
                    &mut self.stamp_seq,
                    "vault_open",
                    Side::Crew,
                );
            }
            CheckKind::Stuff => {
                if let Some(job) = self.jobs.last_mut() {
                    job.bags_picked += 1;
                }
                let at = self.actors[index].at;
                self.take_bag(seat, at, 2.6);
            }
            CheckKind::Pressure => {
                self.bag_scored(
                    Some(seat),
                    ESCAPE,
                    Side::Crew,
                    "Crew delivered the loot · next job in 8s",
                );
                self.stamp("bag_delivered", Side::Crew);
            }
            CheckKind::Lockpick => {
                for a in &mut self.actors {
                    if a.jailed {
                        a.jailed = false;
                        a.jail_left = 0.;
                        a.cuff = 0.;
                        a.at = JAIL_EXIT;
                        a.immune = 4.;
                        a.path.clear();
                    }
                }
                self.stamp("rescued", Side::Crew);
            }
            CheckKind::Steady => {
                let at = self.actors[index].at;
                if let Some(pos) = self.items.decoys.iter().position(|d| reach(at, d.at, 2.6)) {
                    self.items.decoys.remove(pos);
                    self.stun_for(index, items::DUD_SECONDS);
                    self.impact("dud", at);
                } else {
                    self.bag_scored(
                        None,
                        at,
                        Side::Police,
                        "Police recovered the loot · next job in 8s",
                    );
                    self.stamp("bag_recovered", Side::Police);
                }
            }
            CheckKind::LugNuts => {
                self.actors[index].tire = 0.;
                self.actors[index].repair = 0.;
            }
        }
    }
    /// Power, and the security desk in big crews, open the vault job.
    pub(super) fn advance_power_job(&mut self) {
        if self.job == Job::Power
            && self.power > 0.
            && (!self.tuning.second_post || self.cameras > 0.)
        {
            self.job = Job::Vault;
            if let Some(job) = self.jobs.last_mut() {
                job.reach(stats::Phase::Vault);
            }
            self.stamp("power_cut", Side::Crew);
        }
    }
    /// The vault can be worked while power, and the desk if needed, hold.
    pub(super) fn vault_open_for_work(&self) -> bool {
        self.job == Job::Vault && self.power > 0. && (!self.tuning.second_post || self.cameras > 0.)
    }
    /// Passes this seat's running check, for scripted reviews and tests of
    /// what a check unlocks.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_pass_check(&mut self, seat: u8) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        if self.actors[index].check.is_none() {
            return false;
        }
        self.pass_check(index);
        true
    }
    /// Opens the vault and spills its bags, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_open_vault(&mut self) {
        self.job_wait = 0.;
        self.job = Job::Carry;
        self.vault = 1.;
        self.spill_bags();
    }
    /// Puts a seat in the cell, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_jail(&mut self, seat: u8) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.seat == seat) {
            a.jailed = true;
            a.jail_left = 600.;
            a.at = JAIL_CELL;
        }
    }
    /// Leaves a loose bag on the ground, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_drop_bag(&mut self, at: (f32, f32)) {
        self.job_wait = 0.;
        self.job = Job::Carry;
        self.bags = vec![bags::Bag { at, carrier: None }];
    }
    /// Keeps the power and the desk on, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_hold_posts(&mut self) {
        self.power = 600.;
        self.cameras = 600.;
    }
    /// Where a lock's sweet spot is, for scripted reviews that test the
    /// turn rather than the search.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_check_target(&self, seat: u8) -> Option<f32> {
        let a = self.actors.iter().find(|a| a.seat == seat)?;
        a.check.as_ref().map(|c| c.target)
    }
    /// The running check's kind for this seat, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_check_kind(&self, seat: u8) -> Option<CheckKind> {
        self.actors
            .iter()
            .find(|a| a.seat == seat)?
            .check
            .as_ref()
            .map(|c| c.kind)
    }
    /// A player's raw input to their running check.
    pub fn check_input(&mut self, seat: u8, input: CheckInput) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        if self.actors[index].automated {
            return false;
        }
        let now = self.time;
        let hard = self.tuning.check_difficulty;
        let Some(c) = self.actors[index].check.as_mut() else {
            return false;
        };
        enum Outcome {
            Stay,
            Pass,
            Fail(f32),
        }
        let mut outcome = Outcome::Stay;
        let mut noise = None;
        match (c.kind, input) {
            (_, CheckInput::Quit) => outcome = Outcome::Fail(0.5),
            (CheckKind::Wires, CheckInput::Flick { dir }) => {
                let want = c
                    .perm
                    .iter()
                    .position(|&t| t == c.seq[usize::from(c.step)])
                    .unwrap_or(0) as u8;
                if dir == want {
                    c.step += 1;
                    if c.step >= c.steps {
                        outcome = Outcome::Pass;
                    } else {
                        c.value = WIRE_SECONDS[usize::from(c.step)] / hard;
                        c.deadline = now + c.value;
                    }
                } else {
                    outcome = Outcome::Fail(1.5);
                }
            }
            (CheckKind::Cameras, CheckInput::Press) => {
                let period = 1.4 / hard;
                let cursor = |t: f32| (((t - c.started) / period).rem_euclid(1.) * 2. - 1.).abs();
                let hit = [now, now - 0.06]
                    .iter()
                    .any(|t| (cursor(*t) - c.target).abs() <= c.width / 2.);
                if hit {
                    c.step += 1;
                    if c.step >= c.steps {
                        outcome = Outcome::Pass;
                    } else {
                        c.target = 0.15 + ((c.target * 7.31 + 0.37).fract()) * 0.7;
                        c.width = CAMERA_WIDTHS[usize::from(c.step)] / hard;
                    }
                } else {
                    c.step = 0;
                    c.width = CAMERA_WIDTHS[0] / hard;
                    c.fails += 1;
                    noise = Some(c.at);
                }
            }
            (CheckKind::Dial, CheckInput::Dial { at }) => {
                if at.is_finite() {
                    c.value = at.rem_euclid(DIAL_TICKS);
                    let target = f32::from(c.seq[usize::from(c.step)]);
                    let gap = circular(c.value, target);
                    c.closest = c.closest.min(gap);
                    // Spinning well past the number scrapes the tumblers.
                    if c.closest < 1. && gap > 2.5 {
                        c.closest = f32::MAX;
                        c.fails += 1;
                        noise = Some(c.at);
                    }
                }
            }
            (CheckKind::Dial, CheckInput::Press) => {
                let target = f32::from(c.seq[usize::from(c.step)]);
                let tolerance = DIAL_TOLERANCE[usize::from(c.step).min(3)] / hard;
                if circular(c.value, target) <= tolerance {
                    c.step += 1;
                    c.closest = f32::MAX;
                    if c.step >= c.steps {
                        outcome = Outcome::Pass;
                    }
                } else {
                    c.step = 0;
                    c.closest = f32::MAX;
                    c.fails += 1;
                    noise = Some(c.at);
                }
            }
            (CheckKind::Stuff, CheckInput::Button { b }) => {
                let show_end = c.started + f32::from(c.steps) * SHOW_SECONDS / hard;
                if now < show_end || b != c.seq[usize::from(c.step)] {
                    outcome = Outcome::Fail(1.);
                } else {
                    c.step += 1;
                    c.deadline = now + PRESS_SECONDS;
                    if c.step >= c.steps {
                        outcome = Outcome::Pass;
                    }
                }
            }
            (CheckKind::Pressure, CheckInput::Trigger { value }) => {
                if value.is_finite() {
                    c.value = value.clamp(0., 1.);
                }
            }
            (CheckKind::Lockpick, CheckInput::Pick { angle, turn }) => {
                if angle.is_finite() && turn.is_finite() {
                    c.value = angle.clamp(-1., 1.);
                    c.value2 = turn.clamp(0., 1.);
                }
            }
            (CheckKind::Steady, CheckInput::Aim { x, y }) => {
                if x.is_finite() && y.is_finite() {
                    c.value = x.clamp(-1., 1.);
                    c.value2 = y.clamp(-1., 1.);
                }
            }
            (CheckKind::LugNuts, CheckInput::Spin { angle }) => {
                if angle.is_finite() {
                    if c.strain != 0. {
                        let delta = (angle - c.value).sin().atan2((angle - c.value).cos());
                        // Clockwise on screen is a falling angle; big jumps are
                        // a stick let go, not a turn.
                        if delta.abs() < 1.5 {
                            c.progress = (c.progress - delta).max(0.);
                        }
                    }
                    c.value = angle;
                    c.strain = 1.;
                    if c.progress >= std::f32::consts::TAU * 2. {
                        c.step += 1;
                        c.progress = 0.;
                        if c.step >= c.steps {
                            outcome = Outcome::Pass;
                        } else {
                            c.deadline = now + NUT_SECONDS / hard;
                        }
                    }
                }
            }
            _ => {}
        }
        if let Some(at) = noise {
            self.noise(at);
        }
        match outcome {
            Outcome::Stay => {}
            Outcome::Pass => self.pass_check(index),
            Outcome::Fail(wait) => self.fail_check(index, wait),
        }
        true
    }
    /// Clocks for every running check, continuous checks' physics, and
    /// bots' simulated attempts.
    pub(super) fn tick_checks(&mut self, dt: f32) {
        self.noises.retain(|(_, until)| *until > self.time);
        self.cameras = (self.cameras - dt).max(0.);
        self.power = (self.power - dt).max(0.);
        let hard = self.tuning.check_difficulty;
        // Bots resolve their attempt after a while, at a tuned success rate.
        for index in 0..self.actors.len() {
            let now = self.time;
            let a = &mut self.actors[index];
            a.check_wait = (a.check_wait - dt).max(0.);
            let interrupted = a.jailed || a.stun > 0. || a.in_car || a.height > 0.;
            let moved_off = a.check.as_ref().is_some_and(|c| distance(a.at, c.at) > 3.2);
            if interrupted || moved_off {
                a.check = None;
                continue;
            }
            let Some(c) = a.check.as_ref() else { continue };
            if !a.automated || now < c.bot_done {
                continue;
            }
            let (kind, seat, skill) = (c.kind, a.seat, a.skill);
            let (_, base) = kind.bot_tuning();
            // Bots feel a format's difficulty the way players do.
            let p = (base + (skill - 0.5) * 0.4 - (hard - 1.) * 0.6).clamp(0.05, 0.97);
            let salt = (now * 10.) as u32;
            if self.chance(seat, 140, kind as u32 * 1000 + salt) < p {
                self.pass_check(index);
            } else {
                self.fail_check(index, 1.5);
            }
        }
        for index in 0..self.actors.len() {
            let now = self.time;
            let a = &mut self.actors[index];
            if a.automated {
                continue;
            }
            let Some(c) = a.check.as_mut() else { continue };
            let mut outcome = None;
            let mut noise = None;
            match c.kind {
                CheckKind::Pressure => {
                    let since = now - c.started;
                    let center =
                        0.5 + 0.28 * (since * 1.7 * hard).sin() + 0.08 * (since * 4.1).sin();
                    c.target = center;
                    if (c.value - center).abs() <= c.width / 2. {
                        c.progress += dt / 2.5;
                    } else {
                        c.progress = (c.progress - dt * 0.35).max(0.);
                    }
                    if c.value > 0.94 {
                        c.strain += dt;
                        if c.strain > 0.35 {
                            c.strain = 0.;
                            c.progress = (c.progress - 0.35).max(0.);
                            c.fails += 1;
                            noise = Some(c.at);
                        }
                    } else {
                        c.strain = 0.;
                    }
                    if c.progress >= 1. {
                        outcome = Some(true);
                    }
                }
                CheckKind::Lockpick => {
                    let allowed =
                        (1. - ((c.value - c.target).abs() - c.width).max(0.) / 0.35).clamp(0., 1.);
                    c.progress = c.value2.min(allowed);
                    if c.value2 > allowed + 0.1 {
                        c.strain += dt;
                        if c.strain > 0.7 {
                            c.strain = 0.;
                            c.fails += 1;
                            noise = Some(c.at);
                            if c.fails >= PICKS {
                                outcome = Some(false);
                            }
                        }
                    } else {
                        c.strain = (c.strain - dt).max(0.);
                    }
                    if c.progress >= 0.97 {
                        c.closest = if c.closest == f32::MAX {
                            dt
                        } else {
                            c.closest + dt
                        };
                        if c.closest >= 0.4 {
                            outcome = Some(true);
                        }
                    } else {
                        c.closest = f32::MAX;
                    }
                }
                CheckKind::Steady => {
                    let t = (now - c.started) * hard;
                    let aim = (0.55 * (t * 1.3).sin(), 0.45 * (t * 1.9 + 0.7).sin());
                    c.target = aim.0;
                    c.closest = aim.1;
                    let off = (c.value - aim.0).hypot(c.value2 - aim.1);
                    if off <= c.width {
                        c.progress += dt / 2.2;
                    } else {
                        c.progress = (c.progress - dt * 0.5).max(0.);
                    }
                    if c.progress >= 1. {
                        outcome = Some(true);
                    }
                }
                CheckKind::LugNuts => {
                    if now > c.deadline {
                        // A nut not tightened in time slips back loose.
                        c.progress = 0.;
                        c.fails += 1;
                        c.deadline = now + NUT_SECONDS / hard;
                        if c.fails >= 3 {
                            outcome = Some(false);
                        }
                    }
                }
                _ => {}
            }
            if outcome.is_none() && c.kind != CheckKind::LugNuts && now > c.deadline {
                outcome = Some(false);
            }
            if let Some(at) = noise {
                self.noise(at);
            }
            match outcome {
                Some(true) => self.pass_check(index),
                Some(false) => self.fail_check(index, 1.5),
                None => {}
            }
        }
        // The vault's dial stops working the moment power or the desk drops.
        if !self.vault_open_for_work() {
            for a in &mut self.actors {
                if a.check.as_ref().is_some_and(|c| c.kind == CheckKind::Dial) {
                    a.check = None;
                }
            }
        }
    }
    pub(super) fn check_view(&self, me: &Actor) -> Option<CheckView> {
        let c = me.check.as_ref()?;
        let hard = self.tuning.check_difficulty;
        let mut v = CheckView {
            kind: Some(c.kind),
            step: c.step,
            steps: c.steps,
            time_left: (c.deadline - self.time).max(0.),
            elapsed: self.time - c.started,
            ..Default::default()
        };
        match c.kind {
            CheckKind::Wires => {
                v.wire = c.seq.get(usize::from(c.step)).copied();
                v.terminals = Some(c.perm);
                v.duration = Some(c.value);
            }
            CheckKind::Cameras => {
                v.period = Some(1.4 / hard);
                v.target = Some(c.target);
                v.width = Some(c.width);
            }
            CheckKind::Dial => {
                let target = f32::from(c.seq[usize::from(c.step).min(c.seq.len() - 1)]);
                // Only the feel of the tumblers, never the number itself.
                v.tremor = Some((1. - circular(c.value, target) / 2.5).clamp(0., 1.));
                v.value = Some(c.value);
            }
            CheckKind::Stuff => {
                v.period = Some(SHOW_SECONDS / hard);
                let into = (self.time - c.started) / (SHOW_SECONDS / hard);
                let showing = into < f32::from(c.steps);
                v.showing = Some(showing);
                if showing && into.fract() < 0.8 {
                    v.shown = c.seq.get(into as usize).copied();
                }
            }
            CheckKind::Pressure => {
                v.target = Some(c.target);
                v.width = Some(c.width);
                v.progress = Some(c.progress);
                v.value = Some(c.value);
                v.strain = Some(c.strain);
            }
            CheckKind::Lockpick => {
                v.progress = Some(c.progress);
                v.strain = Some((c.strain / 0.7).min(1.));
                v.picks = Some(PICKS.saturating_sub(c.fails));
                v.value = Some(c.value);
            }
            CheckKind::Steady => {
                v.aim = Some([c.target, c.closest]);
                v.width = Some(c.width);
                v.progress = Some(c.progress);
            }
            CheckKind::LugNuts => {
                v.progress = Some((c.progress / (std::f32::consts::TAU * 2.)).min(1.));
            }
        }
        Some(v)
    }
}

#[cfg(test)]
#[path = "city_checks_tests.rs"]
mod tests;
