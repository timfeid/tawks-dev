//! One slow, persistent board hazard. Waiting players are never damaged.
use super::*;
use std::collections::VecDeque;
const SPEED: f32 = 0.35;
const PRESSURE_SPEED: f32 = 0.65;
const CONTACT: f32 = 0.75;
const COOLDOWN: f32 = 12.0;
const GRACE: f32 = 2.0;
#[derive(Clone, Debug, Serialize)]
pub struct CrawlerView {
    pub controller: Option<u8>,
    pub controller_round: Option<u32>,
    pub pulled_by: Option<u8>,
    pub blowing: bool,
    pub strength: f32,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub target: Option<u8>,
    pub speed: f32,
    pub contact_radius: f32,
    pub grace: f32,
    pub cooldowns: Vec<(u8, f32)>,
}
pub(super) struct Crawler {
    controller: Option<u8>,
    controller_round: Option<u32>,
    controller_bot: bool,
    input: (f32, f32),
    input_age: f32,
    pulled_by: Option<u8>,
    blowing: bool,
    strength: f32,
    x: f32,
    z: f32,
    yaw: f32,
    target: Option<u8>,
    turn: Option<(u32, usize)>,
    grace: f32,
    cooldowns: HashMap<u8, f32>,
    wander: u8,
}
impl Crawler {
    pub(super) fn new(layout: &[Space]) -> Self {
        let at = layout
            .iter()
            .filter(|s| !s.under && !s.gated)
            .min_by(|a, b| {
                ((a.x - layout[0].x).hypot(a.z - layout[0].z) - 9.0)
                    .abs()
                    .total_cmp(&((b.x - layout[0].x).hypot(b.z - layout[0].z) - 9.0).abs())
            })
            .unwrap_or(&layout[0]);
        Self {
            controller: None,
            controller_round: None,
            controller_bot: false,
            input: (0., 0.),
            input_age: 1.,
            pulled_by: None,
            blowing: false,
            strength: 0.,
            x: at.x,
            z: at.z,
            yaw: 0.,
            target: None,
            turn: None,
            grace: GRACE,
            cooldowns: HashMap::new(),
            wander: at.id,
        }
    }
}
impl Board {
    /// Choose after randomized pawn placement, maximizing the distance to
    /// the nearest of *all* players rather than just the old start tile.
    pub(super) fn place_crawler_far_from_players(&mut self) {
        let starts: Vec<_> = self
            .tokens
            .iter()
            .map(|token| {
                let at = &self.layout[token.space as usize];
                (at.x, at.z)
            })
            .collect();
        let nearest = |space: &Space| {
            starts
                .iter()
                .map(|p| (space.x - p.0).hypot(space.z - p.1))
                .fold(f32::INFINITY, f32::min)
        };
        let Some(at) = self
            .layout
            .iter()
            .filter(|space| !space.under && !space.gated)
            .max_by(|a, b| nearest(a).total_cmp(&nearest(b)).then(a.id.cmp(&b.id)))
        else {
            return;
        };
        self.crawler.x = at.x;
        self.crawler.z = at.z;
        self.crawler.wander = at.id;
        self.crawler.yaw = 0.;
        self.crawler.target = None;
        self.crawler.turn = None;
        self.crawler.grace = GRACE;
        self.crawler.cooldowns.clear();
        self.crawler.input = (0., 0.);
        self.crawler.input_age = 1.;
        self.crawler.pulled_by = None;
        self.crawler.strength = 0.;
    }
    pub fn begin_crawler_tick(&mut self) {
        self.crawler.pulled_by = None;
        self.crawler.blowing = false;
        self.crawler.strength = 0.;
    }
    pub(super) fn assign_crawler_controller(&mut self, seat: Option<u8>) {
        self.crawler.controller = seat.filter(|seat| *seat != self.turn_seat());
        self.crawler.controller_round = self.crawler.controller.map(|_| self.round);
        self.crawler.controller_bot = false;
        self.crawler.input = (0., 0.);
        self.crawler.input_age = 1.;
    }
    pub(super) fn crawler_turn_arrived(&mut self, seat: u8) {
        if self.crawler.controller == Some(seat) {
            self.assign_crawler_controller(None);
        }
    }
    pub fn crawler_control_presence(&mut self, eligible: &[u8], bots: &[u8]) {
        if let Some(seat) = self.crawler.controller {
            if !eligible.contains(&seat)
                || self.crawler.controller_round != Some(self.round)
                || seat == self.turn_seat()
            {
                self.assign_crawler_controller(None);
            } else {
                self.crawler.controller_bot = bots.contains(&seat);
            }
        }
    }
    pub fn crawler_input(&mut self, seat: u8, x: f32, z: f32) -> Result<(), BoardError> {
        if self.crawler.controller != Some(seat)
            || self.crawler.controller_bot
            || seat == self.turn_seat()
            || self.crawler.controller_round != Some(self.round)
        {
            return Err(BoardError::WrongStep);
        }
        if !x.is_finite() || !z.is_finite() {
            return Err(BoardError::OutOfReach);
        }
        let n = x.hypot(z).max(1.);
        self.crawler.input = (x / n, z / n);
        self.crawler.input_age = 0.;
        Ok(())
    }
    pub fn end_crawler_preview(&mut self, seat: u8) {
        self.assign_crawler_controller(None);
        self.dev_first(seat);
    }
    pub fn dev_crawler_controller(&mut self, seat: u8) {
        self.end_drive("crawler_preview");
        self.clear_raids("crawler_preview");
        self.finish_position();
        self.walk = None;
        self.vacuum = Vacuum::default();
        self.order.retain(|other| *other != seat);
        self.order.push(seat);
        self.turn_index = 0;
        self.step = TurnStep::Items;
        self.assign_crawler_controller(Some(seat));
    }
    /// Sandbox: the crawler lunges onto a seat and finishes them off, so a knockout can be seen.
    pub fn dev_zombie_kill(&mut self, seat: u8) {
        let Some(health) = self.tokens.iter().find(|t| t.seat == seat).map(|t| t.health) else {
            return;
        };
        let at = self.actual_spot_of(seat);
        self.crawler.x = at.0;
        self.crawler.z = at.1;
        self.crawler.cooldowns.insert(seat, COOLDOWN);
        self.hurt(seat, health.max(1));
        self.say(json!({"kind":"crawler_hit","seat":seat,"damage":health.max(1),"x":self.crawler.x,"z":self.crawler.z}));
    }
    pub fn crawler_view(&self) -> CrawlerView {
        let c = &self.crawler;
        let mut cooldowns: Vec<_> = c
            .cooldowns
            .iter()
            .filter(|(_, v)| **v > 0.)
            .map(|(k, v)| (*k, *v))
            .collect();
        cooldowns.sort_by_key(|v| v.0);
        CrawlerView {
            controller: c.controller,
            controller_round: c.controller_round,
            pulled_by: c.pulled_by,
            blowing: c.blowing,
            strength: c.strength,
            x: c.x,
            z: c.z,
            yaw: c.yaw,
            target: c.target,
            speed: SPEED,
            contact_radius: CONTACT,
            grace: c.grace,
            cooldowns,
        }
    }
    fn crawler_victim(&self) -> Option<u8> {
        let seat = self.turn_seat();
        let token = self
            .tokens
            .iter()
            .find(|t| t.seat == seat && t.resting == 0)?;
        let _ = token;
        let available = match self.step {
            TurnStep::Items
            | TurnStep::Roll
            | TurnStep::Position { .. }
            | TurnStep::Raiding
            | TurnStep::Vacuum { .. } => true,
            TurnStep::Walking => self.walk_view().is_some_and(|w| w.elapsed >= w.delay),
            _ => false,
        };
        available.then_some(seat)
    }
    fn crawler_nearest(&self, point: (f32, f32)) -> u8 {
        self.layout
            .iter()
            .filter(|s| !s.under && (!s.gated || self.gates_open()))
            .min_by(|a, b| {
                (a.x - point.0)
                    .hypot(a.z - point.1)
                    .total_cmp(&(b.x - point.0).hypot(b.z - point.1))
            })
            .map(|s| s.id)
            .unwrap_or(0)
    }
    fn crawler_waypoint(&self, goal: (f32, f32)) -> (f32, f32) {
        let start = self.crawler_nearest((self.crawler.x, self.crawler.z));
        let end = self.crawler_nearest(goal);
        if start == end {
            return goal;
        }
        let mut queue = VecDeque::from([start]);
        let mut previous = HashMap::from([(start, start)]);
        while let Some(node) = queue.pop_front() {
            if node == end {
                break;
            }
            for next in self.open_ways(node) {
                if !self.layout[next as usize].under && !previous.contains_key(&next) {
                    previous.insert(next, node);
                    queue.push_back(next);
                }
            }
        }
        if !previous.contains_key(&end) {
            let at = &self.layout[start as usize];
            return (at.x, at.z);
        }
        let mut next = end;
        while previous[&next] != start {
            next = previous[&next];
        }
        let at = &self.layout[next as usize];
        (at.x, at.z)
    }
    fn move_crawler(&mut self, delta: (f32, f32)) -> bool {
        let old = (self.crawler.x, self.crawler.z);
        let home = self.crawler_nearest(old);
        let at = (old.0 + delta.0, old.1 + delta.1);
        if self.position_ground(at, home) {
            self.crawler.x = at.0;
            self.crawler.z = at.1;
        } else if self.position_ground((at.0, old.1), home) {
            self.crawler.x = at.0;
        } else if self.position_ground((old.0, at.1), home) {
            self.crawler.z = at.1;
        }
        old != (self.crawler.x, self.crawler.z)
    }
    /// Uses the exact displayed cone, origin and motor for walking and standing.
    pub(super) fn crawler_pressure(&mut self, seat: u8, dt: f32, power: f32, blow: bool) -> bool {
        if !dt.is_finite() || dt <= 0. || !power.is_finite() || power <= 0. {
            return false;
        }
        let point = (self.crawler.x, self.crawler.z);
        let Some(strength) = self.reach_of(seat)(point, 1.) else {
            return false;
        };
        let origin = self
            .vacuum
            .origin
            .unwrap_or_else(|| self.actual_spot_of(seat));
        let (dx, dz) = (point.0 - origin.0, point.1 - origin.1);
        let length = dx.hypot(dz);
        if length < 0.01 {
            return false;
        }
        let sign = if blow { 1. } else { -1. };
        let distance = PRESSURE_SPEED
            * (strength * power.clamp(0., 1.) * self.pull_rate(seat)).min(1.)
            * dt.min(0.05);
        let moved =
            self.move_crawler((dx / length * distance * sign, dz / length * distance * sign));
        if moved {
            self.crawler.pulled_by = Some(seat);
            self.crawler.blowing = blow;
            self.crawler.strength = ((self.crawler.x - point.0).hypot(self.crawler.z - point.1)
                / (PRESSURE_SPEED * dt.min(0.05)))
            .min(1.);
        }
        moved
    }
    /// Returns true only for discrete damage; room broadcasts movement at 10Hz.
    pub fn crawler_tick(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt <= 0. {
            return false;
        }
        let dt = dt.min(0.05);
        self.crawler.input_age += dt;
        if self
            .crawler
            .controller_round
            .is_some_and(|round| round != self.round)
            || self.crawler.controller == Some(self.turn_seat())
        {
            self.assign_crawler_controller(None);
        }
        let key = (self.round, self.turn_index);
        if self.crawler.turn != Some(key) {
            self.crawler.turn = Some(key);
            self.crawler.grace = GRACE;
        }
        self.crawler.grace = (self.crawler.grace - dt).max(0.);
        for value in self.crawler.cooldowns.values_mut() {
            *value = (*value - dt).max(0.);
        }
        let target = self.crawler_victim();
        // A menu/die could last beyond turn-start grace. Give control back
        // before contact is allowed, rather than hitting on its first frame.
        if target.is_some() && target != self.crawler.target {
            self.crawler.grace = self.crawler.grace.max(1.0);
        }
        self.crawler.target = target;
        let goal = if let Some(seat) = target {
            self.actual_spot_of(seat)
        } else {
            let at = &self.layout[self.crawler.wander as usize];
            if (at.x - self.crawler.x).hypot(at.z - self.crawler.z) < 0.2 {
                self.crawler.wander = self
                    .open_ways(at.id)
                    .into_iter()
                    .filter(|node| !self.layout[*node as usize].under)
                    .min()
                    .unwrap_or(at.id);
            }
            let at = &self.layout[self.crawler.wander as usize];
            (at.x, at.z)
        };
        let manual = self.crawler.controller.is_some() && !self.crawler.controller_bot;
        let waypoint = self.crawler_waypoint(goal);
        let (dx, dz) = if manual {
            if self.crawler.input_age <= 0.35 {
                self.crawler.input
            } else {
                (0., 0.)
            }
        } else {
            (waypoint.0 - self.crawler.x, waypoint.1 - self.crawler.z)
        };
        let length = dx.hypot(dz);
        if length > 0.02 {
            let distance = if manual {
                SPEED * dt * length.min(1.)
            } else {
                (SPEED * dt).min(length)
            };
            self.crawler.yaw = dx.atan2(dz);
            self.move_crawler((dx / length * distance, dz / length * distance));
        }
        if let Some(seat) = target {
            let at = self.actual_spot_of(seat);
            if self.crawler.grace <= 0.
                && self.crawler.cooldowns.get(&seat).copied().unwrap_or(0.) <= 0.
                && (at.0 - self.crawler.x).hypot(at.1 - self.crawler.z) <= CONTACT
            {
                self.crawler.cooldowns.insert(seat, COOLDOWN);
                self.hurt(seat, 1);
                self.say(json!({"kind":"crawler_hit","seat":seat,"damage":1,"x":self.crawler.x,"z":self.crawler.z}));
                return true;
            }
        }
        false
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn board() -> Board {
        let mut b = Board::new(&[0, 1], 10, 71);
        let p = b.actual_spot_of(0);
        b.crawler.x = p.0;
        b.crawler.z = p.1;
        b
    }
    fn ticks(b: &mut Board, n: usize) {
        for _ in 0..n {
            b.crawler_tick(0.05);
        }
    }
    #[test]
    fn grace_contact_cooldown_and_waiting_players_are_safe() {
        let mut b = board();
        ticks(&mut b, 39);
        assert_eq!(b.tokens[0].health, 3);
        ticks(&mut b, 2);
        assert_eq!(b.tokens[0].health, 2);
        ticks(&mut b, 100);
        assert_eq!(b.tokens[0].health, 2);
        assert_eq!(b.tokens[1].health, 3);
        assert_eq!(b.tokens[0].coins, START_COINS);
    }
    #[test]
    fn menus_dice_and_resting_never_damage() {
        for step in [
            TurnStep::Done,
            TurnStep::Check {
                item: Item::Shield,
                period: 1.,
                fuse: 10.,
                number: None,
            },
            TurnStep::Branch {
                options: vec![],
                left: 1,
            },
        ] {
            let mut b = board();
            b.step = step;
            ticks(&mut b, 300);
            assert_eq!(b.tokens[0].health, 3);
        }
        let mut b = board();
        b.tokens[0].resting = 1;
        ticks(&mut b, 300);
        assert_eq!(b.tokens[0].health, 3);
    }
    #[test]
    fn pressure_is_bounded_uses_visible_cone_and_never_rewards() {
        let mut b = board();
        let p = b.actual_spot_of(0);
        b.crawler.x = p.0;
        b.crawler.z = p.1 + 3.;
        b.vacuum.origin = Some(p);
        b.vacuum.yaw = 0.;
        let before = (b.crawler.x, b.crawler.z);
        assert!(b.crawler_pressure(0, 0.05, 1., false));
        assert!(b.crawler.z < before.1);
        assert!(
            (b.crawler.x - before.0).hypot(b.crawler.z - before.1)
                <= PRESSURE_SPEED * 0.05 + 0.0001
        );
        let pulled = b.crawler.z;
        assert!(b.crawler_pressure(0, 0.05, 1., true));
        assert!(b.crawler.z > pulled);
        b.vacuum.yaw = std::f32::consts::PI;
        assert!(!b.crawler_pressure(0, 0.05, 1., false));
        assert_eq!(b.tokens[0].coins, START_COINS);
        assert!(!b.crawler_pressure(0, f32::NAN, 1., false));
    }
    #[test]
    fn persists_between_turns_and_new_actor_has_grace() {
        let mut b = board();
        ticks(&mut b, 42);
        let old = (b.crawler.x, b.crawler.z);
        b.turn_index = 1;
        ticks(&mut b, 1);
        assert!((b.crawler.x - old.0).hypot(b.crawler.z - old.1) <= SPEED * 0.05 + 0.0001);
        assert!(b.crawler.grace > 1.9);
        assert_eq!(b.crawler.target, Some(1));
    }
    #[test]
    fn safe_slow_pursuit_across_all_maps() {
        for map in Map::ALL {
            let mut b = Board::on(map, &[0, 1], 10, 20);
            for _ in 0..1000 {
                let old = (b.crawler.x, b.crawler.z);
                b.crawler_tick(0.05);
                let now = (b.crawler.x, b.crawler.z);
                assert!((now.0 - old.0).hypot(now.1 - old.1) <= SPEED * 0.05 + 0.0002);
                assert!(b.position_ground(now, b.crawler_nearest(now)));
            }
            assert!(b.crawler.x.is_finite() && b.crawler.z.is_finite());
        }
    }
    #[test]
    fn leaving_a_protected_menu_has_a_full_reaction_grace() {
        let mut b = board();
        b.step = TurnStep::Check {
            item: Item::Shield,
            period: 1.,
            fuse: 10.,
            number: None,
        };
        ticks(&mut b, 100);
        let p = b.actual_spot_of(0);
        b.crawler.x = p.0;
        b.crawler.z = p.1;
        b.step = TurnStep::Items;
        ticks(&mut b, 19);
        assert_eq!(b.tokens[0].health, MAX_HEALTH);
        assert!(b.crawler.grace > 0.);
        ticks(&mut b, 3);
        assert_eq!(b.tokens[0].health, MAX_HEALTH - 1);
    }

    #[test]
    fn previous_minigame_last_place_controls_until_their_own_turn_with_ttl_and_no_idle_ai() {
        let mut b = Board::new(&[0, 1, 2], 10, 8);
        b.finish_round(&[vec![2], vec![0], vec![1]]);
        assert_eq!(b.view().order, vec![2, 0, 1]);
        assert_eq!(b.crawler_view().controller, Some(1));
        assert_eq!(b.crawler_view().controller_round, Some(2));
        assert_eq!(b.crawler_input(0, 1., 0.), Err(BoardError::WrongStep));
        assert_eq!(
            b.crawler_input(1, f32::NAN, 0.),
            Err(BoardError::OutOfReach)
        );
        b.crawler_input(1, 100., 0.).unwrap();
        let before = (b.crawler.x, b.crawler.z);
        b.crawler_tick(0.05);
        assert!((b.crawler.x - before.0).hypot(b.crawler.z - before.1) <= SPEED * 0.05 + 0.0001);
        for _ in 0..10 {
            b.crawler_tick(0.05);
        }
        let stopped = (b.crawler.x, b.crawler.z);
        b.crawler_tick(0.05);
        assert_eq!(
            stopped,
            (b.crawler.x, b.crawler.z),
            "expired manual input does not turn into autopursuit"
        );
        b.end_turn();
        assert_eq!(b.turn_seat(), 0);
        assert_eq!(b.crawler_view().controller, Some(1));
        b.end_turn();
        assert_eq!(b.turn_seat(), 1);
        assert_eq!(b.crawler_view().controller, None);
    }

    #[test]
    fn controller_disconnect_falls_back_to_ai_and_bots_keep_ai_control() {
        let mut b = Board::new(&[0, 1], 10, 8);
        b.assign_crawler_controller(Some(1));
        b.crawler_control_presence(&[0, 1], &[1]);
        assert_eq!(b.crawler_view().controller, Some(1));
        assert_eq!(b.crawler_input(1, 1., 0.), Err(BoardError::WrongStep));
        b.crawler_control_presence(&[0], &[]);
        assert_eq!(b.crawler_view().controller, None);
        let before = (b.crawler.x, b.crawler.z);
        b.crawler_tick(0.05);
        assert_ne!(before, (b.crawler.x, b.crawler.z));
    }

    #[test]
    fn pressure_indicator_reports_real_force_and_resets_without_collecting_the_body() {
        let mut b = board();
        let origin = b.actual_spot_of(0);
        b.crawler.x = origin.0;
        b.crawler.z = origin.1 + 3.;
        b.vacuum.origin = Some(origin);
        b.vacuum.yaw = 0.;
        b.begin_crawler_tick();
        assert_eq!(b.crawler_view().pulled_by, None);
        assert!(b.crawler_pressure(0, 0.05, 1., false));
        let view = b.crawler_view();
        assert_eq!(view.pulled_by, Some(0));
        assert!(!view.blowing);
        assert!(view.strength > 0.);
        b.begin_crawler_tick();
        assert_eq!(b.crawler_view().strength, 0.);
        assert!(b.crawler_pressure(0, 0.05, 1., true));
        assert!(b.crawler_view().blowing);
        assert_eq!(b.tokens[0].coins, START_COINS);
    }
    #[test]
    fn randomized_spawns_put_crawler_far_from_every_pawn_on_every_map() {
        let mut smallest = f32::INFINITY;
        for map in Map::ALL {
            for count in [2u8, 4, 8] {
                for seed in 0..32 {
                    let seats: Vec<_> = (0..count).collect();
                    let mut b = Board::on(map, &seats, 10, seed);
                    b.scatter_starts();
                    let c = b.crawler_view();
                    let distance = b
                        .tokens
                        .iter()
                        .map(|t| {
                            let at = &b.layout[t.space as usize];
                            (c.x - at.x).hypot(c.z - at.z)
                        })
                        .fold(f32::INFINITY, f32::min);
                    smallest = smallest.min(distance);
                    assert!(
                        distance >= 18.,
                        "{map:?} {count} players seed{seed}: nearest pawn {distance}"
                    );
                    assert!(b.position_ground((c.x, c.z), b.crawler.wander));
                }
            }
        }
        assert!(smallest.is_finite());
    }
}
