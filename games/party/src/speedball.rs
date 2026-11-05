//! Speedball: two teams at opposite ends of a field of inflatable bunkers.
//! One hit puts you out for the round. Hang the centre flag at the other
//! team's station, or take everyone out, to win the round; the first team to
//! three rounds wins.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use crate::fpmove::Block;
use crate::shooter::PaintBlasters;

pub const KEY: &str = "speedball";
pub const HALF_X: f32 = 28.0;
pub const HALF_Z: f32 = 17.0;
pub const ROUNDS_TO_WIN: u8 = 3;
pub const ROUND_SECONDS: f32 = 75.0;
pub const BETWEEN_SECONDS: f32 = 4.0;
pub const PICKUP_RADIUS: f32 = 1.8;
/// A station is the strip along the back of each end where the flag is hung.
pub const STATION_X: f32 = 25.5;
pub const STATION_HALF_Z: f32 = 4.0;
const RULE: &str =
    "Hang the centre flag at their station or take everyone out. Broad builds grab farther; small builds move faster. First to three rounds.";

fn flag_reach(body_width: f32) -> f32 {
    PICKUP_RADIUS * BodyProfile::new(body_width).radius
}

/// Where each team lines up at the start of a round, as offsets across the field.
const STARTS: [f32; 4] = [-3.0, 3.0, -8.0, 8.0];
const START_X: f32 = 26.0;

/// One side of the field, mirrored for the other: kind, x, z, half sizes, height.
const BUNKERS: &[(&str, f32, f32, f32, f32, f32)] = &[
    // The snake runs low along one side, the doritos sit along the other.
    ("snake", 9.0, 12.5, 5.0, 0.8, 0.9),
    ("dorito", 5.0, -11.0, 1.1, 1.1, 1.5),
    ("dorito", 13.0, -12.0, 1.1, 1.1, 1.5),
    ("dorito", 20.0, -11.5, 1.1, 1.1, 1.5),
    // Cans and temples break up the middle.
    ("can", 4.5, 4.0, 1.0, 1.0, 1.9),
    ("can", 11.0, 0.0, 1.0, 1.0, 1.9),
    ("temple", 17.0, 5.5, 1.5, 1.5, 2.3),
    ("temple", 17.0, -5.0, 1.5, 1.5, 2.3),
    ("can", 21.5, 9.0, 1.0, 1.0, 1.9),
    // The brick right out of the start, the first cover off the break.
    ("brick", 22.5, 0.0, 0.8, 2.2, 1.4),
];

/// The field: every bunker on the red side and its mirror on the blue side,
/// plus a short mini on the centre line on each side of the flag.
pub fn field() -> (Vec<Block>, Vec<&'static str>) {
    field_for(1.0)
}

/// The field spread out by `size` for a bigger crowd: bunkers move out from
/// the middle and grow across the turf, but keep their height.
pub fn field_for(size: f32) -> (Vec<Block>, Vec<&'static str>) {
    let (blocks, kinds) = laid_out();
    (crate::shooter::stretch(blocks, size), kinds)
}

fn laid_out() -> (Vec<Block>, Vec<&'static str>) {
    let mut blocks = Vec::new();
    let mut kinds = Vec::new();
    for &(kind, x, z, hx, hz, top) in BUNKERS {
        for (bx, bz) in [(-x, z), (x, -z)] {
            blocks.push(Block {
                x: bx,
                z: bz,
                hx,
                hz,
                top,
                bottom: 0.0,
            });
            kinds.push(kind);
        }
    }
    for z in [-6.5, 6.5] {
        blocks.push(Block {
            x: 0.0,
            z,
            hx: 0.9,
            hz: 0.9,
            top: 1.2,
            bottom: 0.0,
        });
        kinds.push("mini");
    }
    (blocks, kinds)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flag {
    pub x: f32,
    pub z: f32,
    pub holder: Option<u8>,
}

impl Flag {
    fn centre() -> Self {
        Flag {
            x: 0.0,
            z: 0.0,
            holder: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoundResult {
    /// The team that took the round, or None when nobody did.
    pub winner: Option<u8>,
    pub how: &'static str,
}

#[derive(Clone)]
struct Nav {
    x: f32,
    z: f32,
    stuck: f32,
    side: f32,
}

pub struct Speedball {
    inner: PaintBlasters,
    pub round: u8,
    pub wins: [u8; 2],
    pub clock: f32,
    /// Seconds left of the pause between rounds; zero while a round is live.
    pub between: f32,
    pub flag: Flag,
    pub last: Option<RoundResult>,
    placed: Option<Vec<Vec<u8>>>,
    seats: Vec<u8>,
    started: [usize; 2],
    nav: BTreeMap<u8, Nav>,
    /// How much bigger than the field for four, for the size of the crowd.
    size: f32,
}

/// Which way a team runs: red starts at the negative end and hangs at the positive one.
fn direction(team: u8) -> f32 {
    if team == 0 { 1.0 } else { -1.0 }
}

impl Speedball {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let mut inner = PaintBlasters::new(contestants, seed);
        let size = inner.size();
        let (blocks, kinds) = field_for(size);
        inner.use_team_field(blocks, kinds, HALF_X * size, HALF_Z * size);
        let mut started = [0usize; 2];
        // The board picks the sides; with none given, alternate down the list.
        let given = contestants.iter().any(|c| c.team != contestants[0].team);
        for (index, contestant) in contestants.iter().enumerate() {
            let team = if given { contestant.team.min(1) } else { (index % 2) as u8 };
            inner.set_team(contestant.seat, team);
            started[team as usize] += 1;
        }
        let mut game = Speedball {
            inner,
            round: 1,
            wins: [0, 0],
            clock: 0.0,
            between: 0.0,
            flag: Flag::centre(),
            last: None,
            placed: None,
            seats: contestants.iter().map(|c| c.seat).collect(),
            started,
            nav: BTreeMap::new(),
            size,
        };
        game.line_up();
        game
    }

    pub fn team_of(&self, seat: u8) -> Option<u8> {
        self.inner.team_of(seat)
    }

    fn line_up(&mut self) {
        let mut count = [0usize; 2];
        for seat in self.seats.clone() {
            let team = self.team_of(seat).unwrap_or(0);
            let slot = count[team as usize];
            count[team as usize] += 1;
            let x = -direction(team) * START_X * self.size;
            self.inner.place(seat, x, STARTS[slot % STARTS.len()] * self.size);
        }
        self.flag = Flag::centre();
        self.clock = 0.0;
        self.nav.clear();
    }

    fn alive_on(&self, team: u8) -> usize {
        self.inner
            .fighters
            .iter()
            .filter(|f| f.hp > 0 && self.team_of(f.seat) == Some(team))
            .count()
    }

    /// Where the station strip starts and how far it runs either side, on
    /// this size of field.
    fn station(&self) -> (f32, f32) {
        (STATION_X * self.size, STATION_HALF_Z * self.size)
    }

    fn at_their_station(&self, team: u8, x: f32, z: f32) -> bool {
        let (station_x, half_z) = self.station();
        x * direction(team) >= station_x && z.abs() <= half_z
    }

    fn step_flag(&mut self, events: &mut Vec<Value>) -> Option<RoundResult> {
        if let Some(holder) = self.flag.holder {
            let carrier = self
                .inner
                .fighters
                .iter()
                .find(|f| f.seat == holder)
                .cloned();
            match carrier {
                Some(carrier) if carrier.hp > 0 => {
                    self.flag.x = carrier.x;
                    self.flag.z = carrier.z;
                    let team = self.team_of(holder).unwrap_or(0);
                    if self.at_their_station(team, carrier.x, carrier.z) {
                        events.push(json!({ "kind": "flag_hung", "seat": holder, "team": team }));
                        return Some(RoundResult {
                            winner: Some(team),
                            how: "flag",
                        });
                    }
                }
                _ => {
                    self.flag.holder = None;
                    events.push(json!({ "kind": "flag_dropped", "seat": holder, "x": self.flag.x, "z": self.flag.z }));
                }
            }
            return None;
        }
        let grabber = self
            .inner
            .fighters
            .iter()
            .filter(|f| f.hp > 0)
            .filter_map(|f| {
                let distance_squared =
                    (f.x - self.flag.x).powi(2) + (f.z - self.flag.z).powi(2);
                (distance_squared <= flag_reach(f.body_width).powi(2))
                    .then_some((f.seat, distance_squared))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)))
            .map(|(seat, _)| seat);
        if let Some(seat) = grabber {
            self.flag.holder = Some(seat);
            events.push(json!({ "kind": "flag_taken", "seat": seat, "team": self.team_of(seat) }));
        }
        None
    }

    fn judge(&self) -> Option<RoundResult> {
        // Taking everyone out only counts when both sides had someone to take out.
        if self.started[0] > 0 && self.started[1] > 0 {
            match (self.alive_on(0), self.alive_on(1)) {
                (0, 0) => {
                    return Some(RoundResult {
                        winner: None,
                        how: "trade",
                    });
                }
                (0, _) => {
                    return Some(RoundResult {
                        winner: Some(1),
                        how: "elimination",
                    });
                }
                (_, 0) => {
                    return Some(RoundResult {
                        winner: Some(0),
                        how: "elimination",
                    });
                }
                _ => {}
            }
        }
        if self.clock >= ROUND_SECONDS {
            let (red, blue) = (self.alive_on(0), self.alive_on(1));
            let winner = match red.cmp(&blue) {
                std::cmp::Ordering::Greater => Some(0),
                std::cmp::Ordering::Less => Some(1),
                std::cmp::Ordering::Equal => None,
            };
            return Some(RoundResult {
                winner,
                how: "time",
            });
        }
        None
    }

    fn finish_round(&mut self, result: RoundResult, events: &mut Vec<Value>) {
        if let Some(team) = result.winner {
            self.wins[team as usize] += 1;
        }
        self.last = Some(result);
        events.push(json!({
            "kind": "round_end",
            "round": self.round,
            "winner": result.winner,
            "how": result.how,
            "wins": self.wins,
        }));
        if let Some(team) = result
            .winner
            .filter(|t| self.wins[*t as usize] >= ROUNDS_TO_WIN)
        {
            let (winners, losers): (Vec<u8>, Vec<u8>) = self
                .seats
                .iter()
                .partition(|seat| self.team_of(**seat) == Some(team));
            self.placed = Some(
                [winners, losers]
                    .into_iter()
                    .filter(|side| !side.is_empty())
                    .collect(),
            );
            return;
        }
        self.between = BETWEEN_SECONDS;
    }

    /// Bots play the objective: whoever carries the flag runs it home, someone
    /// goes for a loose flag when the coast is clear, and everyone else fights.
    fn objective_input(&mut self, seat: u8, dt: f32) -> Input {
        let fighting = self.inner.bot_input(seat, dt);
        let Some(me) = self.inner.fighters.iter().find(|f| f.seat == seat).cloned() else {
            return fighting;
        };
        if me.hp <= 0 || self.between > 0.0 {
            return fighting;
        }
        let team = self.team_of(seat).unwrap_or(0);
        let enemy_near = self.inner.fighters.iter().any(|other| {
            other.hp > 0
                && self.team_of(other.seat) != Some(team)
                && (other.x - me.x).powi(2) + (other.z - me.z).powi(2) < 12.0 * 12.0
        });
        let toward = |x: f32, z: f32| Input {
            stick: Stick { x: 0.0, z: -1.0 },
            yaw: (me.x - x).atan2(me.z - z),
            pitch: 0.0,
            fire: fighting.fire,
            jump: false,
            crouch: false,
            press: None,
            arena_target: None,
        };
        if self.flag.holder == Some(seat) {
            let goal = toward(direction(team) * (self.station().0 + 1.0), 0.0);
            return self.unstick(seat, me.x, me.z, goal, dt);
        }
        if self.flag.holder.is_none() && !enemy_near {
            // The teammate nearest the flag goes for it; the rest hold their lanes.
            let distance = |x: f32, z: f32| (x - self.flag.x).powi(2) + (z - self.flag.z).powi(2);
            let mine = distance(me.x, me.z);
            let closest = self
                .inner
                .fighters
                .iter()
                .filter(|f| f.hp > 0 && self.team_of(f.seat) == Some(team))
                .all(|f| distance(f.x, f.z) >= mine);
            if closest {
                let goal = toward(self.flag.x, self.flag.z);
                return self.unstick(seat, me.x, me.z, goal, dt);
            }
        }
        fighting
    }

    /// Walking straight at a goal runs into bunkers, so a bot that stops
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
            return Input {
                stick: Stick {
                    x: memo.side,
                    z: -0.55,
                }
                .clamped(),
                ..want
            };
        }
        want
    }
}

impl Bout for Speedball {
    fn key(&self) -> &'static str {
        KEY
    }

    fn setup(&self) -> Value {
        let mut value = self.inner.setup();
        if let Some(o) = value.as_object_mut() {
            o.insert("game".into(), json!(KEY));
            o.insert("rule".into(), json!(RULE));
            let (station_x, half_z) = self.station();
            o.insert(
                "stations".into(),
                json!([
                    { "team": 0, "x": station_x + 1.0, "z": 0.0, "half_z": half_z },
                    { "team": 1, "x": -(station_x + 1.0), "z": 0.0, "half_z": half_z },
                ]),
            );
        }
        value
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        if self.between > 0.0 {
            self.between = (self.between - dt).max(0.0);
            if self.between == 0.0 {
                self.round += 1;
                self.line_up();
                events.push(json!({ "kind": "round_start", "round": self.round }));
            }
            return;
        }
        self.clock += dt;
        self.inner.step(inputs, dt, events);
        let result = self.step_flag(events).or_else(|| self.judge());
        if let Some(result) = result {
            self.finish_round(result, events);
        }
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        self.objective_input(seat, dt)
    }

    fn snapshot(&self) -> Value {
        let mut value = self.inner.snapshot();
        if let Some(o) = value.as_object_mut() {
            o.insert("game".into(), json!(KEY));
            o.insert("time".into(), json!(self.clock));
            o.insert("limit".into(), json!(ROUND_SECONDS));
            o.insert(
                "flag".into(),
                json!({ "x": self.flag.x, "z": self.flag.z, "holder": self.flag.holder }),
            );
            o.insert(
                "round".into(),
                json!({
                    "number": self.round,
                    "wins": self.wins,
                    "to_win": ROUNDS_TO_WIN,
                    "between": self.between,
                    "last": self.last.map(|r| json!({ "winner": r.winner, "how": r.how })),
                }),
            );
            if let Some(players) = o.get_mut("players").and_then(Value::as_array_mut) {
                for player in players {
                    let seat = player["seat"].as_u64().unwrap_or_default() as u8;
                    if let Some(view) = player.as_object_mut() {
                        view.insert("team".into(), json!(self.team_of(seat).unwrap_or_default()));
                    }
                }
            }
        }
        value
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
impl Speedball {
    pub fn put(&mut self, seat: u8, x: f32, z: f32) {
        self.inner.place(seat, x, z);
    }

    pub fn knock_out(&mut self, seat: u8) {
        if let Some(fighter) = self.inner.fighters.iter_mut().find(|f| f.seat == seat) {
            fighter.hp = 0;
            fighter.respawn_in = f32::INFINITY;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn players(count: u8) -> Vec<Contestant> {
        (0..count).map(Contestant::person).collect()
    }

    fn idle(game: &mut Speedball, seconds: f32) -> Vec<Value> {
        let mut events = Vec::new();
        let steps = (seconds * 60.0) as usize;
        for _ in 0..steps {
            game.step(&[], 1.0 / 60.0, &mut events);
        }
        events
    }

    #[test]
    fn the_field_is_mirrored_and_teams_start_at_opposite_ends() {
        let (blocks, kinds) = field();
        assert_eq!(blocks.len(), kinds.len());
        for block in &blocks {
            assert!(
                blocks
                    .iter()
                    .any(|b| (b.x + block.x).abs() < 0.01 && (b.z + block.z).abs() < 0.01)
            );
        }
        let game = Speedball::new(&players(4), 1);
        assert_eq!(game.team_of(0), Some(0));
        assert_eq!(game.team_of(1), Some(1));
        let red = game.inner.fighters.iter().find(|f| f.seat == 0).unwrap();
        let blue = game.inner.fighters.iter().find(|f| f.seat == 1).unwrap();
        assert!(red.x < -20.0 && blue.x > 20.0);
    }

    #[test]
    fn hanging_the_flag_wins_the_round_and_a_carrier_down_drops_it() {
        let mut game = Speedball::new(&players(2), 1);
        game.put(0, 0.5, 0.0);
        let events = idle(&mut game, 0.1);
        assert_eq!(game.flag.holder, Some(0));
        assert!(events.iter().any(|e| e["kind"] == "flag_taken"));

        game.knock_out(0);
        idle(&mut game, 0.1);
        assert_eq!(game.flag.holder, None);
        assert_eq!(game.last.map(|r| r.how), Some("elimination"));
        assert_eq!(game.wins, [0, 1]);

        idle(&mut game, BETWEEN_SECONDS + 0.1);
        assert_eq!(game.round, 2);
        assert_eq!(game.flag, Flag::centre());
        game.put(0, 0.5, 0.0);
        idle(&mut game, 0.1);
        game.put(0, STATION_X + 0.5, 1.0);
        let events = idle(&mut game, 0.1);
        assert!(events.iter().any(|e| e["kind"] == "flag_hung"));
        assert_eq!(game.wins, [1, 1]);
    }

    #[test]
    fn broad_player_can_reach_a_loose_flag_and_closest_player_wins_a_contest() {
        let mut contestants = players(2);
        contestants[0].body_width = 0.7;
        contestants[1].body_width = 1.4;
        let mut game = Speedball::new(&contestants, 13);
        game.put(0, 2.0, 0.0);
        game.put(1, 1.9, 0.0);
        let events = idle(&mut game, 0.1);
        assert_eq!(game.flag.holder, Some(1));
        assert_eq!(events.iter().filter(|event| event["kind"] == "flag_taken").count(), 1);

        game.flag = Flag::centre();
        game.put(0, 1.0, 0.0);
        game.put(1, 0.8, 0.0);
        idle(&mut game, 0.1);
        assert_eq!(game.flag.holder, Some(1), "the closer player gets the contested flag");
    }

    #[test]
    fn three_rounds_win_the_match() {
        let mut game = Speedball::new(&players(2), 1);
        for _ in 0..ROUNDS_TO_WIN {
            game.knock_out(1);
            idle(&mut game, BETWEEN_SECONDS + 0.2);
        }
        assert_eq!(game.wins, [ROUNDS_TO_WIN, 0]);
        assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
    }

    #[test]
    fn a_bigger_crowd_gets_a_bigger_field() {
        let four = Speedball::new(&players(4), 1);
        assert_eq!(four.setup()["half"], json!(HALF_X), "four play it as built");
        assert_eq!(four.setup()["stations"][0]["half_z"], json!(STATION_HALF_Z));
        let red = four.inner.fighters.iter().find(|f| f.seat == 0).unwrap();
        assert_eq!((red.x, red.z), (-START_X, STARTS[0]));

        let eight = Speedball::new(&players(8), 1);
        let grow = 2f32.sqrt();
        let half = eight.setup()["half"].as_f64().unwrap() as f32;
        assert!((half / HALF_X - grow).abs() < 1e-4, "eight get the room of two fours");
        let station = eight.setup()["stations"][0]["x"].as_f64().unwrap() as f32;
        assert!((station - (STATION_X * grow + 1.0)).abs() < 1e-4, "and the client is told");
        let (blocks, _) = field_for(grow);
        assert!((blocks[0].hx / field().0[0].hx - grow).abs() < 1e-4, "the bunkers grow too");
        // The flag still hangs at the far end of the bigger field.
        let mut eight = eight;
        eight.put(0, 0.5, 0.0);
        idle(&mut eight, 0.1);
        eight.put(0, STATION_X + 0.5, 1.0);
        assert!(!idle(&mut eight, 0.1).iter().any(|e| e["kind"] == "flag_hung"), "the old line is short of it now");
        eight.put(0, STATION_X * grow + 0.5, 1.0);
        assert!(idle(&mut eight, 0.1).iter().any(|e| e["kind"] == "flag_hung"));
    }

    #[test]
    fn a_tied_clock_is_nobodys_round() {
        let mut game = Speedball::new(&players(2), 1);
        idle(&mut game, ROUND_SECONDS + 0.1);
        assert_eq!(
            game.last,
            Some(RoundResult {
                winner: None,
                how: "time"
            })
        );
        assert_eq!(game.wins, [0, 0]);
    }
}
