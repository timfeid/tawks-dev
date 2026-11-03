//! What each seat should do next, said as data: a step, the one action a
//! held A (or another button) performs right now, the targets with the
//! next one marked, and short-lived stamps for big moments. The client
//! turns these into icons, glyphs and a few words; no text here names a key.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Briefing,
    CutPower,
    HoldPower,
    CrackVault,
    GrabBag,
    CarryToVan,
    CoverCarrier,
    Rescue,
    Struggle,
    Jailed,
    Patrol,
    RespondAlarm,
    ChaseBag,
    RecoverBag,
    Cuff,
    Escort,
    Repair,
    /// Holding the security desk in a big crew.
    Cameras,
    /// Drawing police away from the job.
    Lure,
    Wait,
    Resolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verb {
    Work,
    Rescue,
    Escape,
    GrabBag,
    Struggle,
    Grab,
    Recover,
    Trace,
    Drag,
    EnterCar,
    ResetCar,
    Repair,
    /// Holding still in the repair bay.
    Bay,
}

/// The one action a button does here and now.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ContextAction {
    pub verb: Verb,
    /// "a", "x", "b" or "ls" (the left stick).
    pub button: &'static str,
    /// Seconds it must be held; zero for a tap or a stick motion.
    pub hold: f32,
    pub progress: f32,
    pub at: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Power,
    Vault,
    Van,
    Jail,
    Bag,
    Carrier,
    LastSeen,
    Alarm,
    Car,
    Patrol,
    /// A radio ping's coarse bearing, shared with every officer.
    Ping,
    /// The motor-pool repair bay.
    Bay,
    /// The security desk big crews must hold.
    Cameras,
    /// Where police heard a fumble.
    Noise,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Stamp {
    pub id: u32,
    pub key: &'static str,
    /// Which side it is good news for.
    pub side: Side,
    pub age: f32,
    /// Who may see it; None for everyone.
    #[serde(skip)]
    pub audience: Option<Side>,
}

/// How long a stamp stays in views.
pub const STAMP_SECONDS: f32 = 1.6;
/// How long police keep a last-seen ghost of the carrier.
pub const GHOST_SECONDS: f32 = 6.;

/// A robber's job while the crew works the bank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Post {
    Power,
    Cameras,
    Vault,
    Lure,
}

pub(super) fn push_stamp(stamps: &mut Vec<Stamp>, seq: &mut u32, key: &'static str, side: Side) {
    *seq = seq.wrapping_add(1);
    // The crew's own progress stays secret; police learn of the vault by its alarm.
    let audience = match key {
        "power_cut" | "vault_open" => Some(Side::Crew),
        "alarm" => Some(Side::Police),
        _ => None,
    };
    stamps.push(Stamp {
        id: *seq,
        key,
        side,
        age: 0.,
        audience,
    });
}

impl CityGame {
    pub(super) fn stamp(&mut self, key: &'static str, side: Side) {
        push_stamp(&mut self.stamps, &mut self.stamp_seq, key, side);
    }
    pub(super) fn age_stamps(&mut self, dt: f32) {
        for s in &mut self.stamps {
            s.age += dt;
        }
        self.stamps.retain(|s| s.age < STAMP_SECONDS);
    }
    fn carrier_seen_by(&self, me: &Actor) -> Option<(f32, f32)> {
        // Police cannot tell a decoy carrier from the real one.
        self.actors
            .iter()
            .filter(|a| self.carrying(a.seat) || me.side == Side::Police && a.fake_bag > 0.)
            .filter(|a| self.visible(me.at, a.at, 25.))
            .min_by(|a, b| distance(me.at, a.at).total_cmp(&distance(me.at, b.at)))
            .map(|a| a.at)
    }
    /// The carrier's last known spot for police, while it is fresh.
    pub(super) fn ghost_for(&self, me: &Actor) -> Option<((f32, f32), f32)> {
        if !self.any_carrier() || self.carrier_seen_by(me).is_some() {
            return None;
        }
        let (at, when, _, carrying) = me.last_seen?;
        let age = self.time - when;
        (carrying && age < GHOST_SECONDS).then_some((at, age / GHOST_SECONDS))
    }
    pub(super) fn step_for(&self, me: &Actor) -> Step {
        if self.resolved.is_some() {
            return Step::Resolved;
        }
        if self.briefing() {
            return Step::Briefing;
        }
        if self.job_wait > 0. {
            return Step::Wait;
        }
        if me.jailed {
            return Step::Jailed;
        }
        match self.role(me.seat).map(|r| r.1) {
            Some(Role::Robber) => return Step::Struggle,
            Some(Role::Officer(Stage::Ground)) => return Step::Cuff,
            Some(Role::Officer(Stage::Cuffing)) => return Step::Cuff,
            Some(Role::Officer(Stage::Escort)) => return Step::Escort,
            None => {}
        }
        if me.side == Side::Police {
            if !self.bag_views(Some(me)).is_empty() {
                return Step::RecoverBag;
            }
            if self.carrier_seen_by(me).is_some() || self.ghost_for(me).is_some() {
                return Step::ChaseBag;
            }
            if me.tire > 0. {
                return Step::Repair;
            }
            return if self.alarm {
                Step::RespondAlarm
            } else {
                Step::Patrol
            };
        }
        if self.carrying(me.seat) {
            return Step::CarryToVan;
        }
        if !self.loose_bags().is_empty() {
            return Step::GrabBag;
        }
        if self.any_carrier() {
            return Step::CoverCarrier;
        }
        let jailed = self.actors.iter().any(|a| a.side == Side::Crew && a.jailed);
        let rescuer = self
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew && !a.jailed)
            .min_by(|a, b| distance(a.at, JAIL).total_cmp(&distance(b.at, JAIL)))
            .map(|a| a.seat);
        if jailed && rescuer == Some(me.seat) {
            return Step::Rescue;
        }
        let low = |t: f32| t < 8.;
        match (self.job, self.crew_post(me.seat)) {
            (Job::Carry, _) => Step::CoverCarrier,
            (_, Post::Power) => {
                if self.power <= 0. {
                    Step::CutPower
                } else {
                    Step::HoldPower
                }
            }
            (_, Post::Cameras) => Step::Cameras,
            (_, Post::Vault) => {
                let alone = self.crew_free() <= 1;
                let power_alone = alone || self.post_uncovered(Post::Power, POWER);
                let desk_alone = alone || self.post_uncovered(Post::Cameras, CAMERAS);
                if power_alone && low(self.power) {
                    Step::CutPower
                } else if desk_alone && self.tuning.second_post && low(self.cameras) {
                    Step::Cameras
                } else {
                    Step::CrackVault
                }
            }
            (_, Post::Lure) => Step::Lure,
        }
    }
    fn crew_free(&self) -> usize {
        self.actors
            .iter()
            .filter(|a| a.side == Side::Crew && !a.jailed)
            .count()
    }
    /// Each free robber's post, so a crew spreads over the jobs without
    /// talking: players crack the vault first, bots hold the switches.
    pub(super) fn crew_post(&self, seat: u8) -> Post {
        let mut crew: Vec<&Actor> = self
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .collect();
        crew.sort_by_key(|a| (a.automated, a.seat));
        let mut posts = vec![Post::Vault, Post::Power];
        if self.tuning.second_post {
            posts.push(Post::Cameras);
        }
        // Replacing one jailed post should not make every other bot cross town.
        if self.config.police == 1
            && crew.len() == posts.len() + 1
            && crew.iter().filter(|a| !a.jailed).count() == posts.len()
        {
            let original = crew.iter().position(|a| a.seat == seat);
            if original == Some(posts.len()) {
                return crew
                    .iter()
                    .take(posts.len())
                    .position(|a| a.jailed)
                    .map_or(Post::Lure, |i| posts[i]);
            }
            if let Some(i) = original {
                return posts[i];
            }
        }
        crew.retain(|a| !a.jailed);
        crew.iter()
            .position(|a| a.seat == seat)
            .map_or(Post::Lure, |i| posts.get(i).copied().unwrap_or(Post::Lure))
    }
    /// Nobody holding this post is anywhere near its spot, so the vault
    /// post should cover it.
    pub(super) fn post_uncovered(&self, post: Post, spot: (f32, f32)) -> bool {
        !self.actors.iter().any(|a| {
            a.side == Side::Crew
                && !a.jailed
                && self.crew_post(a.seat) == post
                && distance(a.at, spot) < 20.
        })
    }
    pub(super) fn context_for(&self, me: &Actor) -> Option<ContextAction> {
        // A running skill check has its own panel and inputs.
        if me.check.is_some() {
            return None;
        }
        let at = |p: (f32, f32)| [p.0, p.1];
        let act = |verb, button, hold, progress, spot: [f32; 2]| ContextAction {
            verb,
            button,
            hold,
            progress,
            at: spot,
        };
        if me.jailed || self.resolved.is_some() {
            return None;
        }
        if let Some(x) = self
            .arrests
            .iter()
            .find(|x| x.officer == me.seat || x.robber == me.seat)
        {
            // Led away, the slip-the-cuffs panel says what to do.
            if x.robber == me.seat && x.stage == Stage::Escort {
                return None;
            }
            return Some(if x.robber == me.seat {
                act(Verb::Struggle, "a", 0., x.escape, at(me.at))
            } else if x.stage == Stage::Cuffing {
                act(Verb::Trace, "ls", 0., x.progress(), at(me.at))
            } else {
                let goal = if distance(me.at, me.car_at) <= distance(me.at, JAIL) {
                    me.car_at
                } else {
                    JAIL
                };
                act(Verb::Drag, "ls", 0., 0., at(goal))
            });
        }
        if me.in_car {
            if me.tire > 0. && distance(me.at, items::BAY) < items::BAY_RANGE {
                return Some(act(
                    Verb::Bay,
                    "ls",
                    items::BAY_SECONDS,
                    me.repair / items::BAY_SECONDS,
                    at(items::BAY),
                ));
            }
            return (me.stuck_for >= STUCK_SECONDS)
                .then(|| act(Verb::ResetCar, "b", 1., 0., at(me.at)));
        }
        if me.stun > 0. {
            return None;
        }
        if me.side == Side::Police {
            if let Some((target, held)) = me.grab {
                let spot = self.actors.iter().find(|a| a.seat == target).map(|a| a.at);
                return Some(act(
                    Verb::Grab,
                    "a",
                    arrest::GRAB_SECONDS,
                    (held / arrest::GRAB_SECONDS).min(1.),
                    at(spot.unwrap_or(me.at)),
                ));
            }
            let robber = self
                .actors
                .iter()
                .filter(|r| {
                    r.side == Side::Crew
                        && !r.jailed
                        && r.immune <= 0.
                        && r.height <= 0.
                        && (me.automated || me.dummy == Some(Behavior::Fight) || r.stun > 0.)
                        && self.role(r.seat).is_none()
                        && self.visible(me.at, r.at, 2.25)
                })
                .min_by(|a, b| distance(me.at, a.at).total_cmp(&distance(me.at, b.at)));
            if let Some(r) = robber.filter(|_| me.cuff_cooldown <= 0.) {
                return Some(act(Verb::Grab, "a", arrest::GRAB_SECONDS, 0., at(r.at)));
            }
            // A decoy reads exactly like the real bag until it pops.
            if let Some((station, spot)) = self.station_for(me) {
                let verb = if station == Station::Repair {
                    Verb::Repair
                } else {
                    Verb::Recover
                };
                return Some(act(verb, "a", 0., 0., at(spot)));
            }
            if distance(me.at, me.car_at) < 4. {
                return Some(act(Verb::EnterCar, "x", 0., 0., at(me.car_at)));
            }
            return None;
        }
        if me.fake_bag <= 0. && !self.carrying(me.seat) {
            if let Some(d) = self.items.decoys.iter().find(|d| reach(me.at, d.at, 2.3)) {
                return Some(act(Verb::GrabBag, "a", 0., 0., at(d.at)));
            }
        }
        let (station, spot) = self.station_for(me)?;
        Some(match station {
            Station::Power | Station::Cameras | Station::Vault => {
                act(Verb::Work, "a", 0., 0., at(spot))
            }
            Station::Bag => act(Verb::GrabBag, "a", 0., 0., at(spot)),
            Station::Jail => act(Verb::Rescue, "a", 0., 0., at(spot)),
            Station::Van => act(Verb::Escape, "a", 0., 0., at(spot)),
            Station::Loot => act(Verb::Recover, "a", 0., 0., at(spot)),
            Station::Repair => act(Verb::Repair, "a", 0., 0., at(spot)),
        })
    }
    /// What this seat should head for, the next one marked primary.
    pub(super) fn targets_for(&self, me: Option<&Actor>) -> Vec<Target> {
        let Some(me) = me else { return Vec::new() };
        let step = self.step_for(me);
        let mut out = Vec::new();
        let mut push =
            |kind: TargetKind, label: &'static str, p: (f32, f32), progress: f32, primary: bool| {
                out.push(Target {
                    id: kind as u32,
                    kind,
                    label,
                    x: p.0,
                    z: p.1,
                    progress,
                    primary,
                })
            };
        if me.side == Side::Crew {
            if self.actors.iter().any(|a| a.side == Side::Crew && a.jailed) {
                push(
                    TargetKind::Jail,
                    "JAIL",
                    JAIL,
                    self.rescue,
                    step == Step::Rescue,
                );
            }
            for loot in self.loose_bags() {
                push(TargetKind::Bag, "BAG", loot, 0., step == Step::GrabBag);
            }
            if self.job != Job::Carry {
                push(
                    TargetKind::Power,
                    "POWER SWITCH",
                    POWER,
                    (self.power / self.tuning.power_seconds).min(1.),
                    matches!(step, Step::CutPower | Step::HoldPower),
                );
                if self.tuning.second_post {
                    push(
                        TargetKind::Cameras,
                        "SECURITY DESK",
                        CAMERAS,
                        (self.cameras / self.tuning.power_seconds).min(1.),
                        step == Step::Cameras,
                    );
                }
                push(
                    TargetKind::Vault,
                    "BANK VAULT",
                    BANK,
                    self.vault,
                    step == Step::CrackVault,
                );
            } else {
                push(
                    TargetKind::Van,
                    "ESCAPE VAN",
                    ESCAPE,
                    self.escape_progress,
                    step == Step::CarryToVan,
                );
                for c in self
                    .actors
                    .iter()
                    .filter(|a| self.carrying(a.seat) && a.seat != me.seat)
                {
                    push(
                        TargetKind::Carrier,
                        "CARRIER",
                        c.at,
                        0.,
                        step == Step::CoverCarrier,
                    );
                }
            }
            return out;
        }
        if let Some(bag) = self
            .bag_views(Some(me))
            .into_iter()
            .min_by(|a, b| distance(me.at, (a.x, a.z)).total_cmp(&distance(me.at, (b.x, b.z))))
        {
            push(
                TargetKind::Bag,
                "BAG",
                (bag.x, bag.z),
                0.,
                step == Step::RecoverBag,
            );
        }
        if let Some(at) = self.carrier_seen_by(me) {
            push(TargetKind::Carrier, "BAG", at, 0., step == Step::ChaseBag);
        } else if let Some((at, faded)) = self.ghost_for(me) {
            push(
                TargetKind::LastSeen,
                "LAST SEEN",
                at,
                faded,
                step == Step::ChaseBag,
            );
        }
        if self.alarm {
            push(
                TargetKind::Alarm,
                "BANK ALARM",
                BANK,
                0.,
                step == Step::RespondAlarm,
            );
        } else {
            let post = PATROL[me.patrol];
            if me.in_car {
                push(
                    TargetKind::Patrol,
                    "PATROL ROUTE",
                    post,
                    0.,
                    step == Step::Patrol,
                );
            } else {
                push(
                    TargetKind::Car,
                    "PATROL CAR",
                    me.car_at,
                    0.,
                    step == Step::Patrol,
                );
            }
        }
        for (at, _) in &self.noises {
            push(TargetKind::Noise, "NOISE", *at, 0., false);
        }
        // Every officer's radio ping shows on every officer's compass: the
        // same eight-way bearing and rounded distance the pinger gets.
        for o in self.actors.iter().filter(|o| o.side == Side::Police) {
            if let Some((heading, d)) = self.scan_signal(o) {
                let d = (d / 5.).round().max(1.) * 5.;
                let spot = (o.at.0 + heading.sin() * d, o.at.1 + heading.cos() * d);
                push(TargetKind::Ping, "PING", spot, 0., false);
            }
        }
        if step == Step::Repair && me.in_car {
            push(
                TargetKind::Bay,
                "REPAIR BAY",
                items::BAY,
                me.repair / items::BAY_SECONDS,
                true,
            );
        }
        if step == Step::Repair && !me.in_car {
            push(
                TargetKind::Car,
                "PATROL CAR",
                me.car_at,
                me.repair / items::REPAIR_SECONDS,
                true,
            );
        }
        if step == Step::Escort {
            let goal = if distance(me.at, me.car_at) <= distance(me.at, JAIL) {
                (TargetKind::Car, "PATROL CAR", me.car_at)
            } else {
                (TargetKind::Jail, "JAIL", JAIL)
            };
            push(goal.0, goal.1, goal.2, 0., true);
        }
        out
    }
}

#[cfg(test)]
#[path = "city_hud_tests.rs"]
mod tests;
