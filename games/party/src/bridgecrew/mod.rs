//! Bridge Crew: two teams race carts over rope bridges in thick fog. Every
//! crew is a pair: the navigator sees the whole course from above and can only
//! talk in quick pings, and the driver sees a few metres of planks. Planks
//! give way, gusts shove, forks hide a broken branch and swing planks open
//! over the drop. Each crew has its own lane on the same course, the first
//! team with every crew across takes the round, and the pair swap seats
//! between rounds.

mod brain;
mod course;
#[cfg(test)]
mod tests;

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::Serialize;
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, DEFAULT_SKILL, Input, Skill};
use brain::Brain;
pub use course::{Blow, Course, Kind, Piece};

pub const KEY: &str = "bridgecrew";
pub const RULE: &str = "Navigators ping, drivers steer blind through the fog. First team across wins. Swap seats each round.";
pub const ROUNDS: usize = 2;
pub const ROUND_LIMIT: f32 = 40.0;
/// How far the driver can see through the fog. The client draws the fog to
/// match and the driver bots see no further.
pub const SIGHT: f32 = 7.0;
pub const LANE_SPACING: f32 = 12.0;
pub const PING_LIFE: f32 = 2.2;
const PING_GAP: f32 = 0.25;
const KEPT_PINGS: usize = 6;
const CHEER_SECONDS: f32 = 2.4;
const SWAP_SECONDS: f32 = 3.5;
pub const FALL_SECONDS: f32 = 1.3;
/// Someone who sends nothing for this long, in the seat that has to act,
/// gets a bot at the wheel until they touch a control again.
const DRIVER_IDLE: f32 = 5.0;
const NAVIGATOR_IDLE: f32 = 10.0;
/// Ids for the stand-ins that make up an odd crew. Never a real seat.
pub const STAND_IN: u8 = 100;

pub const MAX_SPEED: f32 = 5.2;
pub const BOOST_SPEED: f32 = 8.0;
const ACCEL: f32 = 5.5;
const BRAKE: f32 = 10.0;
const REVERSE_SPEED: f32 = 2.0;
const COAST: f32 = 1.6;
const TURN: f32 = 2.1;
pub const BOOST_FUEL: f32 = 2.5;
const BOOST_REGEN: f32 = 0.3;
/// Sideways shove at full speed. A slower cart catches less of it, which is
/// what a careful ping is for.
pub const GUST_PUSH: f32 = 1.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ping {
    Go,
    Left,
    Right,
    Stop,
    Careful,
    Boost,
}

impl Ping {
    pub const ALL: [Ping; 6] = [
        Ping::Go,
        Ping::Left,
        Ping::Right,
        Ping::Stop,
        Ping::Careful,
        Ping::Boost,
    ];

    /// A ping travels as a shot whose yaw is its number, so it rides the
    /// room's existing fire message.
    pub fn code(self) -> f32 {
        Ping::ALL.iter().position(|p| *p == self).unwrap_or(0) as f32
    }

    pub fn from_code(yaw: f32) -> Option<Ping> {
        let index = yaw.round();
        if !yaw.is_finite() || (yaw - index).abs() > 0.01 || !(0.0..=5.0).contains(&index) {
            return None;
        }
        Some(Ping::ALL[index as usize])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Navigator,
    Driver,
}

#[derive(Clone, Debug)]
pub struct Hand {
    pub id: u8,
    pub seat: Option<u8>,
    pub bot: bool,
    pub skill: Skill,
    idle: f32,
    pub autopilot: bool,
}

impl Hand {
    fn thinks_for_itself(&self) -> bool {
        self.seat.is_none() || self.autopilot
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fall {
    Edge,
    Weak,
    Lift,
    Dead,
}

#[derive(Clone, Debug)]
pub struct Cart {
    pub x: f32,
    pub z: f32,
    pub heading: f32,
    pub speed: f32,
    pub fuel: f32,
    pub boosting: bool,
    pub steer: f32,
    pub falling: Option<f32>,
    pub cause: Option<Fall>,
    pub checkpoint: usize,
    pub falls: u32,
    pub finished_at: Option<f32>,
    pub span: Option<usize>,
    pub weak_since: Vec<Option<f32>>,
    pub collapsed: Vec<bool>,
    pub gusted: f32,
}

impl Cart {
    fn at_start(course: &Course) -> Cart {
        let start = course.checkpoints[0];
        Cart {
            x: start.x,
            z: start.z + 2.0,
            heading: 0.0,
            speed: 0.0,
            fuel: BOOST_FUEL,
            boosting: false,
            steer: 0.0,
            falling: None,
            cause: None,
            checkpoint: 0,
            falls: 0,
            finished_at: None,
            span: Some(0),
            weak_since: vec![None; course.spans.len()],
            collapsed: vec![false; course.spans.len()],
            gusted: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SentPing {
    pub id: u32,
    pub kind: Ping,
    pub at: f32,
}

#[derive(Clone, Debug)]
pub struct Crew {
    pub team: u8,
    pub lane: f32,
    pub hands: [Hand; 2],
    pub cart: Cart,
    pub pings: Vec<SentPing>,
    last_ping: f32,
    next_ping_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stage {
    Racing,
    Cheer { left: f32, winner: Option<u8> },
    Swap { left: f32 },
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundResult {
    pub winner: Option<u8>,
    pub times: [f32; 2],
}

pub struct BridgeCrew {
    pub crews: Vec<Crew>,
    pub courses: Vec<Course>,
    pub round: usize,
    pub time: f32,
    pub stage: Stage,
    pub results: Vec<RoundResult>,
    brains: Vec<[Brain; 2]>,
    finishers: u32,
    rng: StdRng,
    placed: Option<Vec<Vec<u8>>>,
}

fn wrap(angle: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    (angle + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI
}

impl BridgeCrew {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let given = contestants.iter().any(|c| c.team != contestants[0].team);
        let mut sides: [Vec<&Contestant>; 2] = [Vec::new(), Vec::new()];
        for (index, contestant) in contestants.iter().enumerate() {
            let team = if given {
                contestant.team.min(1)
            } else {
                (index % 2) as u8
            };
            sides[team as usize].push(contestant);
        }
        // Partners from a pairs party share a cart when they share a side.
        for side in &mut sides {
            side.sort_by_key(|c| (c.ally.map_or(u16::MAX, u16::from), c.seat));
        }
        let stand_in_skill = contestants
            .iter()
            .find(|c| c.is_bot)
            .map_or(DEFAULT_SKILL, |c| c.skill);
        let per_team = sides
            .iter()
            .map(|s| s.len().div_ceil(2))
            .max()
            .unwrap_or(1)
            .max(1);
        let mut stand_ins = 0u8;
        let mut teams: [Vec<[Hand; 2]>; 2] = [Vec::new(), Vec::new()];
        for (team, side) in sides.iter().enumerate() {
            let mut hands: Vec<Hand> = side
                .iter()
                .map(|c| Hand {
                    id: c.seat,
                    seat: Some(c.seat),
                    bot: c.is_bot,
                    skill: Skill(if c.is_bot { c.skill } else { DEFAULT_SKILL }),
                    idle: 0.0,
                    autopilot: false,
                })
                .collect();
            // An odd one out leads their pair: they navigate the first round.
            while hands.len() < per_team * 2 {
                hands.push(Hand {
                    id: STAND_IN + stand_ins,
                    seat: None,
                    bot: true,
                    skill: Skill(stand_in_skill),
                    idle: 0.0,
                    autopilot: false,
                });
                stand_ins += 1;
            }
            let mut hands = hands.into_iter();
            while let (Some(a), Some(b)) = (hands.next(), hands.next()) {
                teams[team].push([a, b]);
            }
        }
        let courses: Vec<Course> = (0..ROUNDS)
            .map(|_| Course::generate(rng.random()))
            .collect();
        let total = per_team * 2;
        let mut crews = Vec::new();
        // Lanes alternate sides so both teams sit side by side on screen.
        for slot in 0..per_team {
            for team in 0..2 {
                let hands = teams[team][slot].clone();
                let lane = (crews.len() as f32 - (total as f32 - 1.0) / 2.0) * LANE_SPACING;
                crews.push(Crew {
                    team: team as u8,
                    lane,
                    hands,
                    cart: Cart::at_start(&courses[0]),
                    pings: Vec::new(),
                    last_ping: -10.0,
                    next_ping_id: 1,
                });
            }
        }
        let brains = crews
            .iter()
            .map(|_| [Brain::default(), Brain::default()])
            .collect();
        BridgeCrew {
            crews,
            courses,
            round: 0,
            time: 0.0,
            stage: Stage::Racing,
            results: Vec::new(),
            brains,
            finishers: 0,
            rng,
            placed: None,
        }
    }

    pub fn course(&self) -> &Course {
        &self.courses[self.round.min(self.courses.len() - 1)]
    }

    /// Which slot navigates this round; the pair swap seats every round.
    pub fn navigator_slot(&self) -> usize {
        self.round % 2
    }

    pub fn role(&self, slot: usize) -> Role {
        if slot == self.navigator_slot() {
            Role::Navigator
        } else {
            Role::Driver
        }
    }

    pub fn find(&self, seat: u8) -> Option<(usize, usize)> {
        self.crews.iter().enumerate().find_map(|(c, crew)| {
            crew.hands
                .iter()
                .position(|h| h.seat == Some(seat))
                .map(|slot| (c, slot))
        })
    }

    pub fn progress(&self, crew: &Crew) -> f32 {
        if crew.cart.finished_at.is_some() {
            return 1.0;
        }
        let course = self.course();
        ((crew.cart.z - course.start_z) / course.length()).clamp(0.0, 0.999)
    }

    fn team_seats(&self, team: u8) -> Vec<u8> {
        let mut seats: Vec<u8> = self
            .crews
            .iter()
            .filter(|c| c.team == team)
            .flat_map(|c| c.hands.iter().filter_map(|h| h.seat))
            .collect();
        seats.sort();
        seats
    }

    pub fn wins(&self) -> [u32; 2] {
        let mut wins = [0; 2];
        for result in &self.results {
            if let Some(team) = result.winner {
                wins[team as usize] += 1;
            }
        }
        wins
    }

    /// Lifts and collapsed planks stop holding a cart; everything else does.
    fn supports(course: &Course, cart: &Cart, time: f32, x: f32, z: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (index, span) in course.spans.iter().enumerate() {
            if cart.collapsed[index] {
                continue;
            }
            if span.kind == Kind::Lift && course.lifts.iter().any(|l| l.span == index && l.up(time))
            {
                continue;
            }
            let (_, off) = span.project(x, z);
            let slack = span.half + course::EDGE - off;
            if slack >= 0.0 && best.is_none_or(|(_, s)| slack > s) {
                best = Some((index, slack));
            }
        }
        best.map(|(index, _)| index)
    }

    fn why_fell(course: &Course, cart: &Cart) -> Fall {
        let near = |index: usize| {
            let (_, off) = course.spans[index].project(cart.x, cart.z);
            off <= course.spans[index].half + course::EDGE + 0.3
        };
        if (0..course.spans.len()).any(|i| cart.collapsed[i] && near(i)) {
            return Fall::Weak;
        }
        if course.lifts.iter().any(|l| near(l.span)) {
            return Fall::Lift;
        }
        if cart
            .span
            .is_some_and(|s| course.spans[s].kind == Kind::Dead)
        {
            return Fall::Dead;
        }
        Fall::Edge
    }

    fn respawn(course: &Course, cart: &mut Cart) {
        let point = course.checkpoints[cart.checkpoint.min(course.checkpoints.len() - 1)];
        cart.x = point.x;
        cart.z = point.z + 0.4;
        cart.heading = point.heading;
        cart.speed = 0.0;
        cart.falling = None;
        cart.cause = None;
        cart.span = None;
        cart.weak_since.iter_mut().for_each(|w| *w = None);
        cart.collapsed.iter_mut().for_each(|c| *c = false);
    }

    fn drive(&mut self, crew: usize, input: Input, dt: f32, events: &mut Vec<Value>) {
        let time = self.time;
        let course = &self.courses[self.round];
        let lane = self.crews[crew].lane;
        let team = self.crews[crew].team;
        let cart = &mut self.crews[crew].cart;
        if let Some(t) = cart.falling {
            let t = t + dt;
            cart.falling = Some(t);
            cart.z += cart.speed.max(0.0) * dt * 0.5;
            if t >= FALL_SECONDS {
                Self::respawn(course, cart);
                events.push(
                    json!({ "kind": "respawn", "crew": crew, "x": lane + cart.x, "z": cart.z }),
                );
            }
            return;
        }
        if cart.finished_at.is_some() {
            cart.speed = (cart.speed - 6.0 * dt).max(0.0);
            cart.boosting = false;
            let end = course.spans.last().map_or(cart.z, |s| s.b.1 - 1.0);
            cart.z = (cart.z + cart.speed * dt).min(end);
            return;
        }
        let throttle = input.stick.z.clamp(-1.0, 1.0);
        let steer = input.stick.x.clamp(-1.0, 1.0);
        cart.steer = steer;
        let wants_boost = (input.jump || input.fire) && throttle >= 0.0;
        cart.boosting = wants_boost && cart.fuel > 0.0;
        if cart.boosting {
            cart.fuel = (cart.fuel - dt).max(0.0);
            cart.speed = (cart.speed + ACCEL * 1.6 * dt).min(BOOST_SPEED);
        } else {
            cart.fuel = (cart.fuel + BOOST_REGEN * dt).min(BOOST_FUEL);
            if throttle > 0.05 {
                let target = MAX_SPEED * throttle;
                if cart.speed < target {
                    cart.speed = (cart.speed + ACCEL * dt).min(target);
                } else {
                    cart.speed = (cart.speed - COAST * 1.5 * dt).max(target);
                }
            } else if throttle < -0.05 {
                if cart.speed > 0.0 {
                    cart.speed = (cart.speed - BRAKE * -throttle * dt).max(0.0);
                } else {
                    cart.speed = (cart.speed - ACCEL * 0.6 * dt).max(-REVERSE_SPEED * -throttle);
                }
            } else {
                let drag = COAST * dt;
                cart.speed = if cart.speed > 0.0 {
                    (cart.speed - drag).max(0.0)
                } else {
                    (cart.speed + drag).min(0.0)
                };
            }
        }
        let grip = 0.35 + 0.65 * (cart.speed.abs() / 2.0).min(1.0);
        let reverse = if cart.speed < -0.1 { -1.0 } else { 1.0 };
        cart.heading = wrap(cart.heading - steer * TURN * grip * reverse * dt);
        cart.x += cart.heading.sin() * cart.speed * dt;
        cart.z += cart.heading.cos() * cart.speed * dt;
        cart.gusted = 0.0;
        for gust in &course.gusts {
            if cart.z >= gust.z0 && cart.z <= gust.z1 {
                let (blow, dir) = gust.state(time);
                if blow == Blow::Blow {
                    cart.x += dir
                        * GUST_PUSH
                        * (0.35 + 0.65 * (cart.speed.abs() / MAX_SPEED).min(1.0))
                        * dt;
                    cart.gusted = dir;
                }
            }
        }
        for (index, span) in course.spans.iter().enumerate() {
            if let Some(since) = cart.weak_since[index] {
                if !cart.collapsed[index] && time - since >= span.weak_delay() {
                    cart.collapsed[index] = true;
                    events.push(json!({ "kind": "collapse", "crew": crew, "span": index }));
                }
            }
        }
        match Self::supports(course, cart, time, cart.x, cart.z) {
            Some(index) => {
                cart.span = Some(index);
                let span = &course.spans[index];
                cart.checkpoint = cart.checkpoint.max(span.piece);
                if span.kind == Kind::Weak && cart.weak_since[index].is_none() {
                    cart.weak_since[index] = Some(time);
                    events.push(json!({ "kind": "creak", "crew": crew, "span": index }));
                }
            }
            None => {
                let cause = Self::why_fell(course, cart);
                cart.falling = Some(0.0);
                cart.cause = Some(cause);
                cart.falls += 1;
                cart.boosting = false;
                events.push(json!({
                    "kind": "fall", "crew": crew, "team": team, "cause": cause,
                    "x": lane + cart.x, "z": cart.z,
                }));
                return;
            }
        }
        if cart.z >= course.finish_z {
            cart.finished_at = Some(time);
            self.finishers += 1;
            events.push(json!({
                "kind": "finish", "crew": crew, "team": team, "place": self.finishers,
                "time": (time * 100.0).round() / 100.0,
            }));
        }
    }

    fn ping(&mut self, crew: usize, kind: Ping, events: &mut Vec<Value>) {
        let time = self.time;
        let crew_state = &mut self.crews[crew];
        if time - crew_state.last_ping < PING_GAP || crew_state.cart.finished_at.is_some() {
            return;
        }
        crew_state.last_ping = time;
        let id = crew_state.next_ping_id;
        crew_state.next_ping_id += 1;
        crew_state.pings.push(SentPing { id, kind, at: time });
        if crew_state.pings.len() > KEPT_PINGS {
            crew_state.pings.remove(0);
        }
        events.push(json!({
            "kind": "ping", "crew": crew, "team": crew_state.team, "ping": kind,
            "x": crew_state.lane + crew_state.cart.x, "z": crew_state.cart.z,
        }));
    }

    fn think(&mut self, crew: usize, slot: usize, dt: f32) -> Input {
        let role = self.role(slot);
        let course = &self.courses[self.round];
        let state = &self.crews[crew];
        let skill = state.hands[slot].skill;
        let brain = &mut self.brains[crew][slot];
        match role {
            Role::Driver => brain.drive(course, state, self.time, skill, &mut self.rng, dt),
            Role::Navigator => brain.navigate(course, state, self.time, skill, &mut self.rng, dt),
        }
    }

    fn race(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        self.time += dt;
        let nav = self.navigator_slot();
        for crew in 0..self.crews.len() {
            for slot in 0..2 {
                let hand = &self.crews[crew].hands[slot];
                let given = hand
                    .seat
                    .and_then(|seat| inputs.iter().find(|(s, _)| *s == seat))
                    .map(|(_, input)| *input)
                    .unwrap_or_default();
                if hand.seat.is_some() && !hand.bot {
                    let active = given.stick.x.abs() > 0.2
                        || given.stick.z.abs() > 0.2
                        || given.fire
                        || given.jump
                        || given.press.is_some();
                    let limit = if slot == nav {
                        NAVIGATOR_IDLE
                    } else {
                        DRIVER_IDLE
                    };
                    let hand = &mut self.crews[crew].hands[slot];
                    if active {
                        if hand.autopilot {
                            events.push(json!({ "kind": "autopilot", "crew": crew, "seat": hand.seat, "on": false }));
                        }
                        hand.idle = 0.0;
                        hand.autopilot = false;
                    } else {
                        hand.idle += dt;
                        if !hand.autopilot && hand.idle >= limit {
                            hand.autopilot = true;
                            self.brains[crew][slot] = Brain::default();
                            events.push(json!({ "kind": "autopilot", "crew": crew, "seat": hand.seat, "on": true }));
                        }
                    }
                }
                let input = if self.crews[crew].hands[slot].thinks_for_itself() {
                    self.think(crew, slot, dt)
                } else {
                    given
                };
                if slot == nav {
                    if input.fire {
                        if let Some(kind) = Ping::from_code(input.yaw) {
                            self.ping(crew, kind, events);
                        }
                    }
                } else {
                    self.drive(crew, input, dt, events);
                }
            }
        }
        let across: Vec<u8> = (0..2u8)
            .filter(|&team| {
                let mut crews = self.crews.iter().filter(|c| c.team == team).peekable();
                crews.peek().is_some() && crews.all(|c| c.cart.finished_at.is_some())
            })
            .collect();
        if !across.is_empty() || self.time >= ROUND_LIMIT {
            let times = self.team_times();
            let winner = match across.as_slice() {
                [team] => Some(*team),
                [_, _] => {
                    if (times[0] - times[1]).abs() < 1e-4 {
                        None
                    } else {
                        Some(if times[0] < times[1] { 0 } else { 1 })
                    }
                }
                _ => {
                    let progress = self.team_progress();
                    if (progress[0] - progress[1]).abs() < 1e-4 {
                        None
                    } else {
                        Some(if progress[0] > progress[1] { 0 } else { 1 })
                    }
                }
            };
            self.results.push(RoundResult { winner, times });
            self.stage = Stage::Cheer {
                left: CHEER_SECONDS,
                winner,
            };
            events.push(json!({
                "kind": "across", "round": self.round, "team": winner,
                "timeout": across.is_empty(),
            }));
        }
    }

    fn team_progress(&self) -> [f32; 2] {
        let mut sum = [0.0; 2];
        for crew in &self.crews {
            sum[crew.team as usize] += self.progress(crew);
        }
        sum
    }

    /// A team's time for the round: when its last cart crossed, or for a
    /// team still out there, the clock plus what is left at full pelt.
    fn team_times(&self) -> [f32; 2] {
        let course = self.course();
        let mut times = [0.0f32; 2];
        for crew in &self.crews {
            let time = crew.cart.finished_at.unwrap_or_else(|| {
                let left = (course.finish_z - crew.cart.z).max(0.0);
                self.time
                    + left / (MAX_SPEED * 0.8)
                    + if crew.cart.falling.is_some() {
                        FALL_SECONDS
                    } else {
                        0.0
                    }
            });
            let team = &mut times[crew.team as usize];
            *team = team.max(time);
        }
        times.map(|t| (t * 100.0).round() / 100.0)
    }

    fn next_round(&mut self, events: &mut Vec<Value>) {
        if self.round + 1 >= ROUNDS {
            self.stage = Stage::Done;
            self.settle(events);
            return;
        }
        self.round += 1;
        self.time = 0.0;
        self.finishers = 0;
        let course = &self.courses[self.round];
        for crew in &mut self.crews {
            crew.cart = Cart::at_start(course);
            crew.pings.clear();
            crew.last_ping = -10.0;
        }
        for brains in &mut self.brains {
            *brains = [Brain::default(), Brain::default()];
        }
        self.stage = Stage::Swap { left: SWAP_SECONDS };
        events.push(json!({ "kind": "swap", "round": self.round }));
    }

    fn settle(&mut self, events: &mut Vec<Value>) {
        let wins = self.wins();
        let totals = self.results.iter().fold([0.0f32; 2], |acc, r| {
            [acc[0] + r.times[0], acc[1] + r.times[1]]
        });
        let winner = if wins[0] != wins[1] {
            Some(if wins[0] > wins[1] { 0 } else { 1 })
        } else if (totals[0] - totals[1]).abs() > 1e-3 {
            Some(if totals[0] < totals[1] { 0 } else { 1 })
        } else {
            None
        };
        let placed: Vec<Vec<u8>> = match winner {
            Some(team) => vec![self.team_seats(team), self.team_seats(1 - team)],
            None => {
                let mut all = self.team_seats(0);
                all.extend(self.team_seats(1));
                all.sort();
                vec![all]
            }
        }
        .into_iter()
        .filter(|group| !group.is_empty())
        .collect();
        events.push(json!({ "kind": "winner", "team": winner, "wins": wins, "totals": totals }));
        self.placed = Some(placed);
    }

    fn hand_json(&self, slot: usize, hand: &Hand) -> Value {
        json!({
            "id": hand.id,
            "seat": hand.seat,
            "bot": hand.bot,
            "stand_in": hand.seat.is_none(),
            "autopilot": hand.autopilot,
            "role": self.role(slot),
        })
    }
}

impl Bout for BridgeCrew {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        json!({
            "game": KEY,
            "rule": RULE,
            "rounds": ROUNDS,
            "sight": SIGHT,
            "lanes": self.crews.iter().map(|c| c.lane).collect::<Vec<_>>(),
            "courses": self.courses.iter().map(Course::json).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        match self.stage {
            Stage::Done => {}
            Stage::Racing => self.race(inputs, dt, events),
            Stage::Cheer { left, winner } => {
                let left = left - dt;
                if left <= 0.0 {
                    self.next_round(events);
                } else {
                    self.stage = Stage::Cheer { left, winner };
                }
                for crew in 0..self.crews.len() {
                    let cart = &mut self.crews[crew].cart;
                    cart.speed *= (-3.0 * dt).exp();
                    cart.boosting = false;
                }
            }
            Stage::Swap { left } => {
                let left = left - dt;
                self.stage = if left <= 0.0 {
                    events.push(json!({ "kind": "go", "round": self.round }));
                    Stage::Racing
                } else {
                    Stage::Swap { left }
                };
            }
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        if self.stage != Stage::Racing {
            return Input::default();
        }
        match self.find(seat) {
            Some((crew, slot)) => self.think(crew, slot, dt),
            None => Input::default(),
        }
    }

    fn snapshot(&self) -> Value {
        let r = |v: f32| (v * 100.0).round() / 100.0;
        let course = self.course();
        let nav = self.navigator_slot();
        let (stage, left, winner) = match self.stage {
            Stage::Racing => ("racing", 0.0, None),
            Stage::Cheer { left, winner } => ("cheer", left, winner),
            Stage::Swap { left } => ("swap", left, None),
            Stage::Done => ("done", 0.0, self.results.last().and_then(|r| r.winner)),
        };
        let mut players = Vec::new();
        let crews: Vec<Value> = self
            .crews
            .iter()
            .enumerate()
            .map(|(index, crew)| {
                let cart = &crew.cart;
                let drop = cart.falling.map_or(0.0, |t| -4.0 * t * t);
                for (slot, hand) in crew.hands.iter().enumerate() {
                    let Some(seat) = hand.seat else { continue };
                    let navigating = slot == nav;
                    players.push(json!({
                        "seat": seat,
                        "x": r(crew.lane + cart.x),
                        "z": r(cart.z - if navigating { 1.2 } else { 0.0 }),
                        "y": r(if navigating { 6.2 } else { 0.42 + drop }),
                        "yaw": r(cart.heading),
                        "crew": index,
                        "team": crew.team,
                        "role": self.role(slot),
                    }));
                }
                json!({
                    "crew": index,
                    "team": crew.team,
                    "lane": crew.lane,
                    "navigator": crew.hands[nav].id,
                    "driver": crew.hands[1 - nav].id,
                    "hands": crew.hands.iter().enumerate().map(|(slot, h)| self.hand_json(slot, h)).collect::<Vec<_>>(),
                    "x": r(crew.lane + cart.x),
                    "local_x": r(cart.x),
                    "z": r(cart.z),
                    "y": r(drop),
                    "heading": r(cart.heading),
                    "speed": r(cart.speed),
                    "steer": r(cart.steer),
                    "fuel": r(cart.fuel / BOOST_FUEL),
                    "boosting": cart.boosting,
                    "falling": cart.falling.map(r),
                    "cause": cart.cause,
                    "falls": cart.falls,
                    "finished": cart.finished_at.map(r),
                    "progress": r(self.progress(crew)),
                    "gusted": cart.gusted,
                    "collapsed": cart.collapsed.iter().enumerate().filter(|(_, c)| **c).map(|(i, _)| i).collect::<Vec<_>>(),
                    "creaking": cart.weak_since.iter().enumerate()
                        .filter_map(|(i, since)| since.filter(|_| !cart.collapsed[i]).map(|s| json!([i, r((self.time - s) / course.spans[i].weak_delay())])))
                        .collect::<Vec<_>>(),
                    "pings": crew.pings.iter().filter(|p| self.time - p.at < PING_LIFE * 2.0).map(|p| json!({
                        "id": p.id, "kind": p.kind, "age": r(self.time - p.at),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "game": KEY,
            "time": r(self.time),
            "limit": ROUND_LIMIT,
            "round": self.round,
            "rounds": ROUNDS,
            "stage": stage,
            "stage_left": r(left),
            "round_winner": winner,
            "wins": self.wins(),
            "results": self.results.iter().map(|res| json!({ "winner": res.winner, "times": res.times })).collect::<Vec<_>>(),
            "lifts": course.lifts.iter().map(|l| {
                let (until, up) = l.until_change(self.time);
                json!({ "span": l.span, "up": up, "until": r(until) })
            }).collect::<Vec<_>>(),
            "gusts": course.gusts.iter().map(|g| {
                let (blow, dir) = g.state(self.time);
                json!({ "state": blow, "dir": dir })
            }).collect::<Vec<_>>(),
            "crews": crews,
            "players": players,
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}
