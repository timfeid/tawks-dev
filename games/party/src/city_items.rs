//! Pickups, hazards and the moves that go with them: crew pads that roll an
//! item, caltrops that flatten a patrol car's tyre, banana peels anyone can
//! slip on, decoy bags that look like loot to police, the crew's dodge roll
//! and the officer's takedown. One ruleset: the mature setting changes only
//! how these look and are named, never what they do.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Caltrops,
    Banana,
    Decoy,
    Nightstick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemAct {
    /// At your feet.
    Place,
    /// Ahead, along `yaw`.
    Throw,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Pad {
    pub at: (f32, f32),
    pub side: Side,
    pub item: Option<ItemKind>,
    pub respawn: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardKind {
    Caltrops,
    Banana,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Hazard {
    pub id: u32,
    pub kind: HazardKind,
    pub at: (f32, f32),
    /// Caltrops lie across this heading.
    pub yaw: f32,
    pub owner: u8,
    pub left: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Decoy {
    pub at: (f32, f32),
    pub left: f32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Items {
    pub pads: Vec<Pad>,
    pub hazards: Vec<Hazard>,
    pub decoys: Vec<Decoy>,
    pub seq: u32,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct PadView {
    pub x: f32,
    pub z: f32,
    pub ready: bool,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct HazardView {
    pub id: u32,
    pub kind: HazardKind,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub mine: bool,
}
/// A bag on the ground as this seat may see it. Police cannot tell a decoy
/// from the real loot; the crew can.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct BagView {
    pub x: f32,
    pub z: f32,
    pub decoy: bool,
}

/// How long a pad waits before rolling a new item, around this.
pub const RESPAWN_SECONDS: f32 = 30.;
const PICKUP_RANGE: f32 = 1.3;
pub const CALTROPS_SECONDS: f32 = 30.;
pub const CALTROPS_HALF: f32 = 1.5;
pub const BANANA_SECONDS: f32 = 20.;
pub const DECOY_SECONDS: f32 = 25.;
/// How long a carried decoy passes for the real bag.
pub const FAKE_CARRY_SECONDS: f32 = 15.;
pub const FLAT_SECONDS: f32 = 15.;
pub const FLAT_TOP_SPEED: f32 = 8.;
pub const REPAIR_SECONDS: f32 = 3.;
/// The motor-pool bay beside the jail lot: park a flat car in it to fix it.
pub const BAY: (f32, f32) = (35.5, -28.);
pub const BAY_RANGE: f32 = 2.5;
pub const BAY_SECONDS: f32 = 2.;
/// Close enough to the car to work on its wheel.
pub const REPAIR_RANGE: f32 = 3.5;
pub const SLIP_SECONDS: f32 = 1.;
pub const STUMBLE_SECONDS: f32 = 0.7;
pub const DUD_SECONDS: f32 = 1.;
pub const ROLL_SECONDS: f32 = 0.4;
pub const ROLL_DISTANCE: f32 = 4.;
pub const ROLL_COOLDOWN: f32 = 3.;
pub const LUNGE_SECONDS: f32 = 0.35;
pub const DIVE_VIEW_SECONDS: f32 = 0.65;
pub const LUNGE_DISTANCE: f32 = 3.5;
pub const TAKEDOWN_STUN: f32 = 1.8;
pub const NIGHTSTICK_STUN: f32 = 2.6;
pub const FLOP_SECONDS: f32 = 0.8;
pub const TAKEDOWN_COOLDOWN: f32 = 4.;
/// A stun starting this soon after the last one ended lasts half as long.
pub const STUN_STACK_WINDOW: f32 = 4.;
const THROW_DISTANCE: f32 = 6.;

/// Crew item pads: alleys and sidewalks, none close to a heist building.
pub(super) fn pad_spots() -> Vec<(f32, f32)> {
    let landmarks = [BANK, POWER, ESCAPE, JAIL, JAIL_EXIT, CREW_SPAWN];
    let mut spots = Vec::new();
    for s in [-40., -16., 16., 40.] {
        for t in [-52., -28., 4., 28., 52.] {
            for p in [(s + 3.2, t), (t, s - 3.2)] {
                if walkable(p)
                    && city_buildings::room_at(p).is_none()
                    && landmarks.iter().all(|l| distance(*l, p) > 10.)
                    && spots.iter().all(|q: &(f32, f32)| distance(*q, p) > 12.)
                {
                    spots.push(p);
                }
            }
        }
    }
    spots.truncate(20);
    spots
}

fn segment_point_distance(p: (f32, f32), center: (f32, f32), yaw: f32, half: f32) -> f32 {
    // Caltrops lie across the heading: along its right vector.
    let along = (yaw.cos(), -yaw.sin());
    let a = (center.0 - along.0 * half, center.1 - along.1 * half);
    let b = (center.0 + along.0 * half, center.1 + along.1 * half);
    segment_distance(p, a, b)
}

impl CityGame {
    fn roll_item(&self, salt: u32) -> ItemKind {
        let r = self.roll(
            salt as u8,
            (salt >> 8) as u8,
            self.time + salt as f32 * 0.37,
        );
        if r < 0.4 {
            ItemKind::Caltrops
        } else if r < 0.75 {
            ItemKind::Banana
        } else {
            ItemKind::Decoy
        }
    }
    pub(super) fn seed_items(&mut self) {
        let spots = pad_spots();
        self.items.pads = spots
            .into_iter()
            .enumerate()
            .map(|(i, at)| Pad {
                at,
                side: Side::Crew,
                item: None,
                respawn: 1. + i as f32 * 0.05,
            })
            .collect();
        self.items.pads.push(Pad {
            at: JAIL_EXIT,
            side: Side::Police,
            item: Some(ItemKind::Nightstick),
            respawn: 45.,
        });
    }
    fn live_items(&self) -> usize {
        let crew = self.actors.iter().filter(|a| a.side == Side::Crew).count();
        6 + 2 * crew
    }
    /// A stun that lands soon after the last one ended is halved, so nobody
    /// can be chain-stunned for long.
    pub(super) fn stun_for(&mut self, index: usize, seconds: f32) {
        let time = self.time;
        let a = &mut self.actors[index];
        let soon = time - a.stun_ended < STUN_STACK_WINDOW && a.stun_ended > 0.;
        let seconds = if soon && a.stun <= 0. {
            seconds * 0.5
        } else {
            seconds
        };
        a.stun = a.stun.max(seconds);
        a.stun_ended = time + a.stun;
    }
    /// Uses the held item; refused while jailed, dazed or in an arrest.
    pub fn use_item(&mut self, seat: u8, act: ItemAct, yaw: f32) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        if self.role(seat).is_some() {
            return false;
        }
        let a = &self.actors[index];
        if a.jailed || a.stun > 0. || a.in_car {
            return false;
        }
        let Some(kind) = a.item else { return false };
        if kind == ItemKind::Nightstick { return false; }
        let yaw = if yaw.is_finite() { yaw } else { a.yaw };
        let at = a.at;
        let spot = match act {
            ItemAct::Place => at,
            ItemAct::Throw => {
                let mut spot = at;
                for step in 1..=12 {
                    let d = THROW_DISTANCE * step as f32 / 12.;
                    let p = (at.0 + yaw.sin() * d, at.1 + yaw.cos() * d);
                    if !walkable(p) && !drivable(p) {
                        break;
                    }
                    if !walkable(p) || city_buildings::sight_blocked(at, p) {
                        break;
                    }
                    spot = p;
                }
                spot
            }
        };
        self.items.seq = self.items.seq.wrapping_add(1);
        let id = self.items.seq;
        match kind {
            ItemKind::Caltrops => {
                self.items
                    .hazards
                    .retain(|h| !(h.kind == HazardKind::Caltrops && h.owner == seat));
                self.items.hazards.push(Hazard {
                    id,
                    kind: HazardKind::Caltrops,
                    at: spot,
                    yaw,
                    owner: seat,
                    left: CALTROPS_SECONDS,
                });
            }
            ItemKind::Banana => self.items.hazards.push(Hazard {
                id,
                kind: HazardKind::Banana,
                at: spot,
                yaw,
                owner: seat,
                left: BANANA_SECONDS,
            }),
            ItemKind::Decoy => self.items.decoys.push(Decoy {
                at: spot,
                left: DECOY_SECONDS,
            }),
            ItemKind::Nightstick => unreachable!(),
        }
        self.actors[index].item = None;
        // Bots wait a while before taking another, so the street is not
        // carpeted with peels.
        self.actors[index].item_guard = if self.actors[index].automated {
            12.
        } else {
            0.6
        };
        self.impact("item", spot);
        true
    }
    /// The crew's dodge roll: a short burst with a moment no grab or
    /// takedown can land in.
    pub fn dodge(&mut self, seat: u8, yaw: f32) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        if self.role(seat).is_some() {
            return false;
        }
        let carrying = self.carrying(seat);
        let a = &mut self.actors[index];
        if a.side != Side::Crew
            || a.jailed
            || a.stun > 0.
            || a.in_car
            || a.height > 0.
            || a.roll_cooldown > 0.
        {
            return false;
        }
        let yaw = if yaw.is_finite() { yaw } else { a.yaw };
        a.roll = ROLL_SECONDS;
        a.roll_yaw = yaw;
        a.roll_speed = ROLL_DISTANCE / ROLL_SECONDS * if carrying { 0.5 } else { 1. };
        a.roll_cooldown = ROLL_COOLDOWN;
        a.yaw = yaw;
        let at = a.at;
        self.impact("dodge", at);
        true
    }
    /// The officer's takedown: a lunge that knocks a robber down on contact.
    pub fn takedown(&mut self, seat: u8, yaw: f32) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        if self.role(seat).is_some() {
            return false;
        }
        let automated = self.actors[index].automated;
        let target = if automated { None } else {
            if self.slow_owner != Some(seat) || self.slow_left <= 0. {
                self.actors[index].notice = Some(("Call out a suspect, then hold Clarity to dive.", NOTICE_SECONDS));
                return false;
            }
            let Some(callout) = self.callout.filter(|c| c.officer == seat && self.callout_visible(*c)) else {
                self.actors[index].notice = Some(("Lost sight of the suspect.", NOTICE_SECONDS));
                return false;
            };
            Some(callout.suspect)
        };
        let a = &mut self.actors[index];
        if a.side != Side::Police || a.jailed || a.stun > 0. || a.in_car || a.lunge_cooldown > 0. {
            return false;
        }
        let yaw = if yaw.is_finite() { yaw } else { a.yaw };
        a.lunge = LUNGE_SECONDS;
        a.dive_view = DIVE_VIEW_SECONDS;
        a.lunge_yaw = yaw;
        a.lunge_hit = false;
        a.lunge_target = target;
        a.lunge_baton = a.item == Some(ItemKind::Nightstick);
        a.lunge_cooldown = TAKEDOWN_COOLDOWN;
        a.yaw = yaw;
        a.notice = Some(("DIVE!", NOTICE_SECONDS));
        let at = a.at;
        self.impact("lunge", at);
        true
    }
    /// Makes a seat the loot carrier, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_carry(&mut self, seat: u8) {
        self.job_wait = 0.;
        self.job = Job::Carry;
        let at = self
            .actors
            .iter()
            .find(|a| a.seat == seat)
            .map_or(BANK, |a| a.at);
        self.bags = vec![bags::Bag {
            at,
            carrier: Some(seat),
        }];
    }
    /// Has a seat set down an item at a spot, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_drop_item(&mut self, seat: u8, item: ItemKind, at: (f32, f32), yaw: f32) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        let a = &mut self.actors[index];
        a.at = at;
        a.item = Some(item);
        a.stun = 0.;
        a.in_car = false;
        self.use_item(seat, ItemAct::Place, yaw)
    }
    /// Starts a seat's takedown, for scripted reviews.
    #[cfg(any(test, feature = "qa"))]
    pub fn qa_takedown(&mut self, seat: u8, yaw: f32) -> bool {
        if let Some(index) = self.index_of(seat) {
            self.actors[index].lunge_cooldown = 0.;
            self.actors[index].in_car = false;
        }
        self.takedown(seat, yaw)
    }
    /// Sandbox: puts an item in this seat's hand.
    pub fn sandbox_give(&mut self, seat: u8, item: ItemKind) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        self.actors[index].item = Some(item);
        self.actors[index].item_guard = 1.;
        true
    }
    /// Sandbox: flattens this seat's patrol car tyre.
    pub fn sandbox_flat(&mut self, seat: u8) -> bool {
        let Some(index) = self.index_of(seat) else {
            return false;
        };
        let a = &mut self.actors[index];
        if a.side != Side::Police {
            return false;
        }
        a.tire = FLAT_SECONDS;
        true
    }
    /// Moves a rolling robber or lunging officer; returns true when it did.
    pub(super) fn forced_move(a: &mut Actor, dt: f32) -> bool {
        let (left, yaw, speed) = if a.roll > 0. {
            (&mut a.roll, a.roll_yaw, a.roll_speed)
        } else if a.lunge > 0. {
            (&mut a.lunge, a.lunge_yaw, LUNGE_DISTANCE / LUNGE_SECONDS)
        } else {
            return false;
        };
        let step = dt.min(*left);
        *left -= dt;
        let ended = *left <= 0.;
        let old = a.at;
        let next = (
            old.0 + yaw.sin() * speed * step,
            old.1 + yaw.cos() * speed * step,
        );
        if walkable(next) && !city_buildings::body_blocked(old, next) {
            a.at = next;
        }
        a.velocity = ((a.at.0 - old.0) / dt, (a.at.1 - old.1) / dt);
        if ended && a.roll <= 0. && a.side == Side::Police && !a.lunge_hit {
            a.flop_pending = true;
        }
        true
    }
    fn knock_down(&mut self, index: usize, from: (f32, f32), stun: f32, bounce: f32) {
        self.stun_for(index, stun);
        let a = &mut self.actors[index];
        let away = (a.at.0 - from.0, a.at.1 - from.1);
        let d = away.0.hypot(away.1).max(0.001);
        a.up = bounce;
        a.height = a.height.max(0.01);
        a.knock = (away.0 / d * 3., away.1 / d * 3.);
        a.interact = false;
        a.suction = false;
        let seat = a.seat;
        let at = a.at;
        if self.carrying(seat) {
            self.drop_bag(seat, at);
        } else if self.actors[index].fake_bag > 0. {
            self.actors[index].fake_bag = 0.;
            self.items.decoys.push(Decoy {
                at,
                left: DECOY_SECONDS,
            });
        }
    }
    /// Crew bots use what they carry, each item on one plain condition, and
    /// only on what they can see.
    fn bot_items(&mut self) {
        let mut uses = Vec::new();
        for a in self
            .actors
            .iter()
            .filter(|a| a.automated && a.side == Side::Crew && !a.jailed)
        {
            let Some(kind) = a.item else { continue };
            let police = || {
                self.actors
                    .iter()
                    .filter(|p| p.side == Side::Police && self.visible(a.at, p.at, 25.))
            };
            match kind {
                ItemKind::Nightstick => {},
                ItemKind::Caltrops => {
                    if on_road(a.at) && police().any(|p| p.in_car && distance(p.at, a.at) < 20.) {
                        uses.push((a.seat, ItemAct::Place, a.yaw));
                    }
                }
                ItemKind::Banana => {
                    // Only at an officer actually closing in, not one passing by.
                    let closing = |p: &&Actor| {
                        let to_me = (a.at.0 - p.at.0, a.at.1 - p.at.1);
                        to_me.0 * p.velocity.0 + to_me.1 * p.velocity.1 > 0.
                    };
                    if let Some(p) = police()
                        .filter(closing)
                        .find(|p| !p.in_car && distance(p.at, a.at) < 6.)
                    {
                        uses.push((
                            a.seat,
                            ItemAct::Throw,
                            (p.at.0 - a.at.0).atan2(p.at.1 - a.at.1),
                        ));
                    }
                }
                ItemKind::Decoy => {
                    let near_bag = self
                        .actors
                        .iter()
                        .find(|c| self.carrying(c.seat))
                        .is_some_and(|c| distance(c.at, a.at) < 10.);
                    if near_bag && police().next().is_some() {
                        uses.push((a.seat, ItemAct::Place, a.yaw));
                    }
                }
            }
        }
        for (seat, act, yaw) in uses {
            self.use_item(seat, act, yaw);
        }
    }
    /// Pickups, hazards, decoys, takedown contacts, flat tyres and timers.
    pub(super) fn tick_items(&mut self, dt: f32) {
        if self.briefing() || self.resolved.is_some() {
            return;
        }
        self.bot_items();
        let limit = self.live_items();
        let mut live = self.items.pads.iter().filter(|p| p.side == Side::Crew && p.item.is_some()).count()
            + self.actors.iter().filter(|a| a.item.is_some()).count()
            + self.items.hazards.len()
            + self.items.decoys.len();
        for i in 0..self.items.pads.len() {
            if self.items.pads[i].side == Side::Police {
                if self.items.pads[i].item.is_none() {
                    self.items.pads[i].respawn -= dt;
                    if self.items.pads[i].respawn <= 0. {
                        self.items.pads[i].item = Some(ItemKind::Nightstick);
                    }
                }
                continue;
            }
            if self.items.pads[i].item.is_some() {
                continue;
            }
            self.items.pads[i].respawn -= dt;
            if self.items.pads[i].respawn <= 0. && live < limit {
                let kind = self.roll_item(i as u32 + self.items.seq * 31);
                self.items.pads[i].item = Some(kind);
                live += 1;
            }
        }
        for index in 0..self.actors.len() {
            let a = &mut self.actors[index];
            a.item_guard = (a.item_guard - dt).max(0.);
            a.roll_cooldown = (a.roll_cooldown - dt).max(0.);
            a.lunge_cooldown = (a.lunge_cooldown - dt).max(0.);
            a.dive_view = (a.dive_view - dt).max(0.);
            if a.tire > 0. {
                a.tire = (a.tire - dt).max(0.);
            }
            a.fake_bag = (a.fake_bag - dt).max(0.);
            if a.stun <= 0. && a.stun_ended > self.time {
                a.stun_ended = self.time;
            }
            if a.jailed || a.in_car || a.item.is_some() || a.item_guard > 0.
            {
                continue;
            }
            let at = a.at;
            if let Some(pad) = self
                .items
                .pads
                .iter_mut()
                .find(|p| p.side == a.side && p.item.is_some() && distance(p.at, at) < PICKUP_RANGE)
            {
                let rest =
                    RESPAWN_SECONDS - 10. + (pad.at.0.abs() * 7.3 + self.time).rem_euclid(20.);
                self.actors[index].item = pad.item.take();
                pad.respawn = rest;
                self.impact("pickup", at);
            }
        }
        // A robber who picks a decoy back up carries it to pose as the carrier.
        for index in 0..self.actors.len() {
            let a = &self.actors[index];
            if a.side != Side::Crew
                || a.jailed
                || !a.interact
                || a.fake_bag > 0.
                || self.carrying(a.seat)
                || self.role(a.seat).is_some()
            {
                continue;
            }
            let at = a.at;
            if let Some(pos) = self.items.decoys.iter().position(|d| reach(at, d.at, 2.3)) {
                self.items.decoys.remove(pos);
                self.actors[index].fake_bag = FAKE_CARRY_SECONDS;
                self.impact("pickup", at);
            }
        }
        self.items.hazards.iter_mut().for_each(|h| h.left -= dt);
        self.items.decoys.iter_mut().for_each(|d| d.left -= dt);
        self.items.hazards.retain(|h| h.left > 0.);
        self.items.decoys.retain(|d| d.left > 0.);
        let mut sprung = Vec::new();
        for (hi, h) in self.items.hazards.iter().enumerate() {
            for (ai, a) in self.actors.iter().enumerate() {
                if a.jailed || a.height > 0. || a.roll > 0. || self.role(a.seat).is_some() {
                    continue;
                }
                let hit = match (h.kind, a.in_car) {
                    (HazardKind::Caltrops, true) => {
                        a.drive_speed > 0.5
                            && segment_point_distance(a.at, h.at, h.yaw, CALTROPS_HALF) < 1.4
                    }
                    (HazardKind::Caltrops, false) => {
                        a.side == Side::Police
                            && a.stun <= 0.
                            && segment_point_distance(a.at, h.at, h.yaw, CALTROPS_HALF) < 0.5
                    }
                    (HazardKind::Banana, true) => distance(a.at, h.at) < 1.5,
                    (HazardKind::Banana, false) => {
                        a.stun <= 0. && a.seat != h.owner && distance(a.at, h.at) < 0.8
                    }
                };
                if hit {
                    sprung.push((hi, ai));
                    break;
                }
            }
        }
        let mut gone = Vec::new();
        for (hi, ai) in sprung {
            let h = self.items.hazards[hi];
            let in_car = self.actors[ai].in_car;
            match (h.kind, in_car) {
                (HazardKind::Caltrops, true) => {
                    let a = &mut self.actors[ai];
                    a.tire = FLAT_SECONDS;
                    a.repair = 0.;
                    gone.push(h.id);
                    let at = a.at;
                    self.impact("flat", at);
                }
                (HazardKind::Caltrops, false) => {
                    self.stun_for(ai, STUMBLE_SECONDS);
                    let at = self.actors[ai].at;
                    let victim = self.actors[ai].seat;
                    self.impact_on("stumble", at, Some(victim), 2.);
                }
                (HazardKind::Banana, true) => {
                    let a = &mut self.actors[ai];
                    a.car_yaw = wrap(a.car_yaw + std::f32::consts::PI);
                    a.spin = 1.2;
                    a.drive_speed *= 0.5;
                    gone.push(h.id);
                    let at = a.at;
                    self.impact("spin", at);
                }
                (HazardKind::Banana, false) => {
                    let from = h.at;
                    let back = (
                        from.0 - self.actors[ai].velocity.0.signum() * 0.1,
                        from.1 - self.actors[ai].velocity.1.signum() * 0.1,
                    );
                    self.knock_down(ai, back, SLIP_SECONDS, 4.);
                    gone.push(h.id);
                    let at = self.actors[ai].at;
                    let victim = self.actors[ai].seat;
                    self.impact_on("slip", at, Some(victim), 4.);
                }
            }
        }
        self.items.hazards.retain(|h| !gone.contains(&h.id));
        // Takedown contact during the lunge; a lunge that ends empty flops.
        for oi in 0..self.actors.len() {
            let o = &self.actors[oi];
            if o.flop_pending {
                let at = o.at;
                self.actors[oi].flop_pending = false;
                self.stun_for(oi, FLOP_SECONDS);
                self.actors[oi].notice = Some(("MISSED DIVE", NOTICE_SECONDS));
                self.impact("flop", at);
                continue;
            }
            if o.lunge <= 0. || o.lunge_hit {
                continue;
            }
            let at = o.at;
            let target_seat = o.lunge_target;
            let baton = o.lunge_baton;
            let target = self
                .actors
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    r.side == Side::Crew
                        && target_seat.is_none_or(|seat| r.seat == seat)
                        && !r.jailed
                        && r.immune <= 0.
                        && r.roll <= 0.
                        && r.height <= 0.
                        && self.role(r.seat).is_none()
                        && distance(r.at, at) < if baton { 1.9 } else { 1.6 }
                        && !city_buildings::sight_blocked(r.at, at)
                })
                .map(|(i, _)| i)
                .next();
            if let Some(ri) = target {
                self.actors[oi].lunge_hit = true;
                self.actors[oi].lunge = 0.;
                self.actors[oi].notice = Some(("HIT! Click again to pin them.", NOTICE_SECONDS));
                let graze = !baton && distance(self.actors[ri].at, at) >= 1.2;
                self.knock_down(ri, at, if baton { NIGHTSTICK_STUN } else if graze { 1.25 } else { TAKEDOWN_STUN }, 3.);
                if baton { self.actors[oi].item = None; }
                let hit = self.actors[ri].at;
                let victim = self.actors[ri].seat;
                self.actors[ri].notice = Some(("TACKLED!", NOTICE_SECONDS));
                self.impact_on(
                    "takedown",
                    hit,
                    Some(victim),
                    LUNGE_DISTANCE / LUNGE_SECONDS,
                );
            }
        }
        for a in &mut self.actors {
            a.spin = (a.spin - dt).max(0.);
        }
    }
    /// Parking a flat car in the bay; by hand it is the wheel-nut check.
    pub(super) fn repair_flats(&mut self, dt: f32) {
        for a in &mut self.actors {
            if a.tire <= 0. {
                a.repair = 0.;
                continue;
            }
            let in_bay = a.in_car && a.drive_speed < 2. && distance(a.at, BAY) < BAY_RANGE;
            if in_bay {
                a.repair += dt;
                if a.repair >= BAY_SECONDS {
                    a.tire = 0.;
                    a.repair = 0.;
                }
            } else {
                a.repair = 0.;
            }
        }
    }
    /// The hazards a bot can see that would hurt it, with how wide a berth
    /// to give each. Bots never route around what they cannot see.
    pub(super) fn hazards_to_avoid(&self, a: &Actor) -> Vec<((f32, f32), f32)> {
        if !a.automated {
            return Vec::new();
        }
        self.items
            .hazards
            .iter()
            .filter(|h| match h.kind {
                HazardKind::Caltrops => a.side == Side::Police,
                HazardKind::Banana => h.owner != a.seat,
            })
            .filter(|h| {
                let ours = self
                    .actors
                    .iter()
                    .any(|o| o.seat == h.owner && o.side == a.side);
                ours || distance(a.at, h.at) < 8. && !city_buildings::sight_blocked(a.at, h.at)
            })
            .map(|h| (h.at, if a.in_car { CALTROPS_HALF + 1.6 } else { 1.2 }))
            .collect()
    }
    pub(super) fn pad_views(&self, side: Side) -> Vec<PadView> {
        self.items
            .pads
            .iter()
            .filter(|p| p.side == side)
            .map(|p| PadView {
                x: p.at.0,
                z: p.at.1,
                ready: p.item.is_some(),
            })
            .collect()
    }
    /// Your own team's hazards always show; the other team's only up close.
    pub(super) fn hazard_views(&self, me: Option<&Actor>) -> Vec<HazardView> {
        let Some(me) = me else { return Vec::new() };
        self.items
            .hazards
            .iter()
            .filter(|h| {
                let mine = self
                    .actors
                    .iter()
                    .any(|o| o.seat == h.owner && o.side == me.side);
                mine || distance(me.at, h.at) < 8. && !city_buildings::sight_blocked(me.at, h.at)
            })
            .map(|h| HazardView {
                id: h.id,
                kind: h.kind,
                x: h.at.0,
                z: h.at.1,
                yaw: h.yaw,
                mine: self
                    .actors
                    .iter()
                    .any(|o| o.seat == h.owner && o.side == me.side),
            })
            .collect()
    }
    /// Bags on the ground: police see real and decoy alike, the crew knows.
    pub(super) fn bag_views(&self, me: Option<&Actor>) -> Vec<BagView> {
        let Some(me) = me else { return Vec::new() };
        let crew = me.side == Side::Crew;
        let mut bags: Vec<BagView> = self
            .items
            .decoys
            .iter()
            .filter(|d| crew || self.visible(me.at, d.at, 25.))
            .map(|d| BagView {
                x: d.at.0,
                z: d.at.1,
                decoy: crew,
            })
            .collect();
        for loot in self.loose_bags() {
            if crew || self.visible(me.at, loot, 25.) {
                bags.push(BagView {
                    x: loot.0,
                    z: loot.1,
                    decoy: false,
                });
            }
        }
        bags.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.z.total_cmp(&b.z)));
        bags
    }
}

#[cfg(test)]
#[path = "city_items_tests.rs"]
mod tests;
