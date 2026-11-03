//! What makes the bots play like people: they react a beat late, steer a
//! little loose, only know what they can see or hear, lose their nerve when
//! an officer closes in, and a second officer cuts off instead of following.
use super::*;

/// Bot skill presets for lobbies and the sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BotSkill {
    Casual,
    Good,
}

impl BotSkill {
    pub fn value(self) -> f32 {
        match self {
            BotSkill::Casual => 0.3,
            BotSkill::Good => 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Mind {
    goal: ((f32, f32), bool),
    think_at: f32,
    thought: bool,
    /// Steadiness under pressure, from 0 (jumpy) to 1 (cool).
    pub nerve: f32,
    panic_until: f32,
    flee_to: (f32, f32),
}

/// How close a visible officer has to get before a robber feels the heat.
const THREAT: f32 = 7.;
/// How far bots hear a fumbled check.
const HEARING: f32 = 45.;
/// The share of an officer's beat spent at the bank or power station.
const INSPECT_SHARE: f32 = 0.25;

/// Loose hands: a bot's stick drifts a little around where it means to go.
pub(super) fn wobble(time: f32, a: &Actor, stick: Stick) -> Stick {
    let amount = (1. - a.skill) * 0.3;
    let phase = f32::from(a.seat) * 1.37;
    let angle = (time * 1.9 + phase).sin() * amount;
    let (s, c) = angle.sin_cos();
    Stick {
        x: stick.x * c - stick.z * s,
        z: stick.x * s + stick.z * c,
    }
}

impl CityGame {
    pub(super) fn mind_for(&self, seat: u8) -> Mind {
        Mind {
            nerve: 0.25 + self.roll(seat, 77, 0.) * 0.6,
            ..Mind::default()
        }
    }
    /// Each bot's goal, rethought only after its reaction time has passed.
    pub(super) fn bot_goals(&mut self) -> Vec<((f32, f32), bool)> {
        for index in 0..self.actors.len() {
            let a = &self.actors[index];
            if a.mind.thought && self.time < a.mind.think_at {
                continue;
            }
            let (seat, skill) = (a.seat, a.skill);
            self.panic(index);
            let goal = match self.actors[index].mind {
                m if m.panic_until > self.time => (m.flee_to, false),
                _ => self.ai_goal(index),
            };
            // A good player reacts in about a fifth of a second, a casual one
            // in half a second.
            let reaction = 0.55 - skill * 0.4;
            let jitter = 0.7 + self.roll(seat, 78, self.time) * 0.6;
            let mind = &mut self.actors[index].mind;
            mind.goal = goal;
            mind.thought = true;
            mind.think_at = self.time + reaction * jitter;
        }
        self.actors.iter().map(|a| a.mind.goal).collect()
    }
    /// A robber who sees an officer close in may bolt, dropping a check
    /// half done. Carriers hold their nerve better: the bag is the point.
    fn panic(&mut self, index: usize) {
        let a = &self.actors[index];
        if a.side != Side::Crew || a.jailed || !a.automated || a.mind.panic_until > self.time {
            return;
        }
        let Some(cop) = self
            .actors
            .iter()
            .filter(|o| o.side == Side::Police && !o.jailed)
            .filter(|o| self.visible(a.at, o.at, if o.in_car { 10. } else { THREAT }))
            .min_by(|x, y| distance(a.at, x.at).total_cmp(&distance(a.at, y.at)))
            .map(|o| o.at)
        else {
            return;
        };
        let carrying = self.carrying(a.seat);
        let near_van = carrying && distance(a.at, ESCAPE) < 8.;
        let calm = a.mind.nerve + if carrying { 0.35 } else { 0. };
        if near_van || self.roll(a.seat, 79, self.time) < calm {
            return;
        }
        let away = (a.at.0 - cop.0, a.at.1 - cop.1);
        let len = away.0.hypot(away.1).max(0.001);
        let base = away.1.atan2(away.0);
        let flee_to = [0., 0.9, -0.9, 1.6, -1.6]
            .into_iter()
            .find_map(|turn: f32| {
                let angle = base + turn;
                let spot = (a.at.0 + angle.cos() * 9., a.at.1 + angle.sin() * 9.);
                (walkable(spot) && len > 0.).then_some(spot)
            });
        let Some(flee_to) = flee_to else { return };
        let seconds = 1.2 + (1. - a.mind.nerve) * 2.;
        let a = &mut self.actors[index];
        a.mind.panic_until = self.time + seconds;
        a.mind.flee_to = flee_to;
        if a.check.take().is_some() {
            a.check_wait = 1.;
        }
    }
    /// A fumble police heard, if it is the freshest one this officer can reach.
    pub(super) fn heard(&self, a: &Actor) -> Option<(f32, f32)> {
        self.noises
            .iter()
            .filter(|(at, _)| distance(a.at, *at) < HEARING)
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(at, _)| *at)
    }
    /// With two officers on one carrier, the farther one heads for the van
    /// to cut it off instead of trailing behind.
    pub(super) fn cut_off(&self, a: &Actor, target: u8) -> bool {
        let Some(robber) = self.actors.iter().find(|r| r.seat == target) else {
            return false;
        };
        if !self.carrying(target) {
            return false;
        }
        self.actors.iter().any(|o| {
            o.seat != a.seat
                && o.side == Side::Police
                && !o.jailed
                && self.visible(o.at, robber.at, 25.)
                && distance(o.at, robber.at) < distance(a.at, robber.at)
        })
    }
    /// The nearest officer checks the vault; any other covers the van.
    pub(super) fn answer_alarm(&self, a: &Actor) -> (f32, f32) {
        let closer = self.actors.iter().any(|o| {
            o.seat != a.seat
                && o.side == Side::Police
                && !o.jailed
                && distance(o.at, BANK) < distance(a.at, BANK)
        });
        if closer { ESCAPE } else { BANK }
    }
    /// Where this officer's beat takes them right now, if it is inspection time.
    pub(super) fn inspecting(&self, a: &Actor) -> Option<(f32, f32)> {
        let period = self.tuning.inspect_period;
        if period <= 0. || self.time < self.tuning.inspect_delay {
            return None;
        }
        // Two officers walk opposite halves of the cycle.
        let clock = (self.time - self.tuning.inspect_delay) / period + f32::from(a.seat) * 0.5;
        (clock.fract() < INSPECT_SHARE).then(|| if (clock as u32) % 2 == 0 { BANK } else { POWER })
    }
    pub fn set_bot_skill(&mut self, skill: f32) {
        for a in &mut self.actors {
            a.skill = skill.clamp(0., 1.);
        }
    }
}
