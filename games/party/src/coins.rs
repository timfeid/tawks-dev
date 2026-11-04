//! Coin Scramble: coins rain onto the floor and everyone grabs what they can
//! before the clock runs out. A dive or punch shakes part of someone's purse
//! loose for anybody to scoop up, and the last stretch rains coins worth five,
//! so a purse filled early never settles it.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Bout, Contestant, Input, Skill, Stick};
use crate::ground::{self, Runner};

pub const ARENA_RADIUS: f32 = 14.0;
pub const ROUND_SECONDS: f32 = 60.0;
pub const BRONZE: u32 = 1;
pub const GOLD: u32 = 5;
pub const DROP_HEIGHT: f32 = 9.0;
pub const BRONZE_FALL: f32 = 7.0;
/// Gold drifts down, so the whole field has time to converge on it.
pub const GOLD_FALL: f32 = 3.2;
/// Seconds between coins with a full field; a smaller field gets fewer.
pub const RAIN_PER_PLAYER: f32 = 3.0;
/// The drizzle holds off while this many coins are already lying about, for
/// four; a bigger field on a bigger floor gets room for more.
pub const FLOOR_MAX: usize = 40;
/// Seconds left on the clock when the gold starts falling.
pub const GOLD_FROM: f32 = 26.0;
pub const GOLD_EVERY: f32 = 6.0;
/// The last gold lands with time left to fight over it.
pub const LAST_GOLD_BY: f32 = ROUND_SECONDS - 4.0;
pub const PICKUP_RANGE: f32 = 1.2;
/// A coin at shoulder height is close enough to snatch out of the air.
pub const CATCH_HEIGHT: f32 = 1.4;
pub const CHARGE_SECONDS: f32 = 0.38;
pub const CHARGE_COOLDOWN: f32 = 3.0;
pub const CHARGE_SPEED: f32 = 2.6;
pub const CHARGE_RANGE: f32 = 1.35;
pub const PUNCH_SECONDS: f32 = 0.18;
pub const PUNCH_COOLDOWN: f32 = 1.25;
pub const PUNCH_RANGE: f32 = 1.45;
pub const KNOCKBACK: f32 = 11.0;
/// A breather after a charge, so one rush cannot empty a purse.
pub const SAFE_SECONDS: f32 = 2.0;
pub const SPILL_SHARE: f32 = 0.3;
pub const SPILL_MAX: u32 = 6;
/// Spilled coins bounce clear before anyone can scoop them up.
pub const SCATTER_SECONDS: f32 = 0.45;
/// How far a bot will leave the coins to go and charge someone richer.
pub const CHASE_RANGE: f32 = 6.5;

#[derive(Clone, Copy, Debug)]
pub struct Coin {
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub fall: f32,
    pub value: u32,
    pub scatter: f32,
}

impl Coin {
    pub fn loose(&self) -> bool {
        self.y <= CATCH_HEIGHT && self.scatter <= 0.0
    }
}

#[derive(Clone, Debug)]
pub struct Scrambler {
    pub seat: u8,
    pub run: Runner,
    pub purse: u32,
    pub charge: f32,
    pub cooldown: f32,
    pub safe: f32,
    pub facing: f32,
    pub punch: f32,
    pub punch_cooldown: f32,
    fire_held: bool,
    dive_held: bool,
}

impl Scrambler {
    pub fn charging(&self) -> bool {
        self.charge > 0.0
    }
}

pub struct CoinScramble {
    /// Partners, when played in pairs, who leave each other alone.
    allies: std::collections::HashMap<u8, u8>,
    pub players: Vec<Scrambler>,
    pub coins: Vec<Coin>,
    pub time: f32,
    /// How far the floor reaches, wider for a bigger field.
    pub radius: f32,
    /// How many coins can lie about before the drizzle holds off.
    pub floor_max: usize,
    /// How fast gold drifts down. A wider floor slows it, so everyone still
    /// has time to get under it.
    gold_fall: f32,
    rain: f32,
    golds: u32,
    next_coin: u32,
    rng: StdRng,
    skills: std::collections::HashMap<u8, Skill>,
    placements: Option<Vec<Vec<u8>>>,
}

impl CoinScramble {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let size = crate::bout::crowd_scale(contestants.len());
        let radius = ARENA_RADIUS * size;
        let spread = ground::ring(contestants.len().max(1), radius * 0.5, 0.4);
        let players: Vec<Scrambler> = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| Scrambler {
                seat: c.seat,
                run: Runner::at(spread[i].0, spread[i].1).with_width(c.body_width),
                purse: 0,
                charge: 0.0,
                cooldown: 0.0,
                safe: 0.0,
                facing: 0.0,
                punch: 0.0,
                punch_cooldown: 0.0,
                fire_held: false,
                dive_held: false,
            })
            .collect();
        CoinScramble {
            allies: contestants
                .iter()
                .filter_map(|c| c.ally.map(|ally| (c.seat, ally)))
                .collect(),
            players,
            coins: Vec::new(),
            time: 0.0,
            radius,
            floor_max: crate::bout::crowd_count(FLOOR_MAX, contestants.len()),
            gold_fall: GOLD_FALL / size,
            rain: 0.0,
            golds: 0,
            next_coin: 0,
            rng: StdRng::seed_from_u64(seed),
            skills: contestants
                .iter()
                .map(|c| (c.seat, Skill(c.skill)))
                .collect(),
            placements: None,
        }
    }

    fn skill(&self, seat: u8) -> Skill {
        self.skills.get(&seat).copied().unwrap_or_default()
    }

    fn player(&self, seat: u8) -> Option<&Scrambler> {
        self.players.iter().find(|p| p.seat == seat)
    }

    pub fn purse(&self, seat: u8) -> u32 {
        self.player(seat).map(|p| p.purse).unwrap_or(0)
    }

    /// Whoever is ahead on their own, or nobody while the top is shared.
    pub fn leader(&self) -> Option<u8> {
        let mut order: Vec<&Scrambler> = self.players.iter().collect();
        order.sort_by(|a, b| b.purse.cmp(&a.purse));
        match (order.first(), order.get(1)) {
            (Some(top), Some(next)) if top.purse == next.purse => None,
            (Some(top), _) => Some(top.seat),
            _ => None,
        }
    }

    fn rain_every(&self) -> f32 {
        RAIN_PER_PLAYER / self.players.len().max(1) as f32
    }

    /// When the next gold is due, or `None` once they have all fallen.
    fn gold_due(&self) -> Option<f32> {
        let at = ROUND_SECONDS - GOLD_FROM + self.golds as f32 * GOLD_EVERY;
        (at <= LAST_GOLD_BY).then_some(at)
    }

    fn rain(&mut self, dt: f32, events: &mut Vec<Value>) {
        if let Some(at) = self.gold_due() {
            if self.time >= at {
                self.golds += 1;
                self.drop_coin(GOLD, events);
            }
        }
        self.rain -= dt;
        if self.rain <= 0.0 && self.coins.len() < self.floor_max {
            self.rain = self.rain_every() * self.rng.random_range(0.75..1.25);
            self.drop_coin(BRONZE, events);
        }
    }

    fn drop_coin(&mut self, value: u32, events: &mut Vec<Value>) {
        let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
        let out = self.radius * 0.86 * self.rng.random_range(0.0f32..1.0).sqrt();
        let coin = Coin {
            id: self.next_coin,
            x: angle.cos() * out,
            z: angle.sin() * out,
            y: DROP_HEIGHT,
            fall: if value > BRONZE {
                self.gold_fall
            } else {
                BRONZE_FALL
            },
            value,
            scatter: 0.0,
        };
        self.next_coin += 1;
        self.coins.push(coin);
        if value > BRONZE {
            events.push(json!({ "kind": "gold", "x": coin.x, "z": coin.z, "value": value }));
        }
    }

    /// A charge that connects shakes coins out of the purse it hits.
    fn resolve_charges(&mut self, events: &mut Vec<Value>) {
        let chargers: Vec<(u8, f32, f32, f32, f32)> = self
            .players
            .iter()
            .filter(|p| p.charging())
            .map(|p| (p.seat, p.run.x, p.run.z, p.run.vx, p.run.vz))
            .collect();
        for (seat, x, z, vx, vz) in chargers {
            let attacker = self.players.iter().find(|p| p.seat == seat).unwrap();
            let reach = CHARGE_RANGE * crate::bout::BodyProfile::new(attacker.run.width).reach;
            let hit = self
                .players
                .iter()
                .filter(|p| p.seat != seat && p.safe <= 0.0)
                .filter(|p| {
                    !crate::bout::allied(
                        self.allies.get(&seat).copied(),
                        self.allies.get(&p.seat).copied(),
                    )
                })
                .map(|p| {
                    (
                        p.seat,
                        ((p.run.x - x).powi(2) + (p.run.z - z).powi(2)).sqrt(),
                    )
                })
                .filter(|(seat, away)| {
                    let victim = self.players.iter().find(|p| p.seat == *seat).unwrap();
                    *away <= reach + ground::RUNNER_RADIUS * crate::bout::BodyProfile::new(victim.run.width).radius
                })
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let Some((mark, _)) = hit else { continue };
            let mut loose = 0;
            let mut at = (0.0, 0.0);
            if let Some(target) = self.players.iter_mut().find(|p| p.seat == mark) {
                loose = ((target.purse as f32 * SPILL_SHARE).ceil() as u32)
                    .min(SPILL_MAX)
                    .min(target.purse);
                target.purse -= loose;
                let body = crate::bout::BodyProfile::new(target.run.width);
                target.safe = SAFE_SECONDS * body.stun;
                let push = (vx * vx + vz * vz).sqrt().max(1.0);
                target.run.vx += vx / push * KNOCKBACK / body.mass;
                target.run.vz += vz / push * KNOCKBACK / body.mass;
                at = (target.run.x, target.run.z);
            }
            if let Some(charger) = self.players.iter_mut().find(|p| p.seat == seat) {
                charger.charge = 0.0;
            }
            self.scatter(at, loose);
            events.push(json!({ "kind": "spill", "seat": mark, "by": seat, "coins": loose }));
        }
    }

    fn resolve_punches(&mut self, events: &mut Vec<Value>) {
        let punches: Vec<_> = self
            .players
            .iter()
            .filter(|p| p.punch > 0.0)
            .map(|p| (p.seat, p.run.x, p.run.z, p.facing))
            .collect();
        for (seat, x, z, facing) in punches {
            let reach = self.players.iter().find(|p| p.seat == seat)
                .map(|p| crate::bout::BodyProfile::new(p.run.width).reach).unwrap_or(1.0);
            let forward = (facing.sin(), facing.cos());
            let target = self
                .players
                .iter()
                .filter(|p| p.seat != seat && p.safe <= 0.0)
                .filter(|p| {
                    !crate::bout::allied(
                        self.allies.get(&seat).copied(),
                        self.allies.get(&p.seat).copied(),
                    )
                })
                .filter_map(|p| {
                    let dx = p.run.x - x;
                    let dz = p.run.z - z;
                    let distance = dx.hypot(dz);
                    (distance <= PUNCH_RANGE * reach + ground::RUNNER_RADIUS * crate::bout::BodyProfile::new(p.run.width).radius
                        && dx * forward.0 + dz * forward.1 > distance * 0.45)
                        .then_some((p.seat, distance))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let Some((mark, _)) = target else { continue };
            let mut loose = 0;
            let mut at = (0.0, 0.0);
            if let Some(victim) = self.players.iter_mut().find(|p| p.seat == mark) {
                loose = ((victim.purse as f32 * 0.18).ceil() as u32)
                    .min(3)
                    .min(victim.purse);
                victim.purse -= loose;
                let body = crate::bout::BodyProfile::new(victim.run.width);
                victim.safe = body.stun;
                victim.run.vx += forward.0 * 6.0 / body.mass;
                victim.run.vz += forward.1 * 6.0 / body.mass;
                at = (victim.run.x, victim.run.z);
            }
            if let Some(attacker) = self.players.iter_mut().find(|p| p.seat == seat) {
                attacker.punch = 0.0;
            }
            self.scatter(at, loose);
            events
                .push(json!({"kind":"spill","seat":mark,"by":seat,"coins":loose,"attack":"punch"}));
        }
    }

    /// Knocked-loose coins land around whoever dropped them.
    fn scatter(&mut self, at: (f32, f32), coins: u32) {
        for _ in 0..coins {
            let angle = self.rng.random_range(0.0..std::f32::consts::TAU);
            let out = self.rng.random_range(1.3..2.7);
            let (mut x, mut z) = (at.0 + angle.cos() * out, at.1 + angle.sin() * out);
            let away = (x * x + z * z).sqrt();
            let edge = self.radius - 1.0;
            if away > edge && away > 1e-4 {
                x = x / away * edge;
                z = z / away * edge;
            }
            self.coins.push(Coin {
                id: self.next_coin,
                x,
                z,
                y: 1.3,
                fall: 3.0,
                value: BRONZE,
                scatter: SCATTER_SECONDS,
            });
            self.next_coin += 1;
        }
    }

    /// The nearest player takes any loose coin at their feet.
    fn collect(&mut self, events: &mut Vec<Value>) {
        for index in (0..self.coins.len()).rev() {
            let coin = self.coins[index];
            if !coin.loose() {
                continue;
            }
            let near = self
                .players
                .iter()
                .map(|p| (p.seat, p.run.distance_to(coin.x, coin.z)))
                .filter(|(_, away)| *away <= PICKUP_RANGE)
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            let Some((seat, _)) = near else { continue };
            self.coins.remove(index);
            let mut purse = 0;
            if let Some(player) = self.players.iter_mut().find(|p| p.seat == seat) {
                player.purse += coin.value;
                purse = player.purse;
            }
            events.push(json!({
                "kind": "grab",
                "seat": seat,
                "value": coin.value,
                "purse": purse,
                "x": coin.x,
                "z": coin.z,
            }));
        }
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < ROUND_SECONDS {
            return;
        }
        let mut order: Vec<(u32, u8)> = self.players.iter().map(|p| (p.purse, p.seat)).collect();
        order.sort_by(|a, b| b.0.cmp(&a.0));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<u32> = None;
        for (purse, seat) in order {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == purse => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some(purse);
        }
        self.placements = Some(groups);
    }
}

impl Bout for CoinScramble {
    fn key(&self) -> &'static str {
        "coins"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "coins",
            "rule": "Grab falling coins. Punch nearby rivals or dive to knock coins loose.",
            "radius": self.radius,
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;
        self.rain(dt, events);

        for index in 0..self.players.len() {
            let seat = self.players[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let player = &mut self.players[index];
            player.cooldown = (player.cooldown - dt).max(0.0);
            player.punch_cooldown = (player.punch_cooldown - dt).max(0.0);
            player.safe = (player.safe - dt).max(0.0);
            player.charge = (player.charge - dt).max(0.0);
            player.punch = (player.punch - dt).max(0.0);
            if input.fire && !player.fire_held && player.punch_cooldown <= 0.0 && !player.charging()
            {
                player.punch = PUNCH_SECONDS;
                player.punch_cooldown = PUNCH_COOLDOWN;
                events.push(json!({"kind":"punch","seat":seat}));
            }
            player.fire_held = input.fire;
            if input.jump && !player.dive_held && player.cooldown <= 0.0 && !player.charging() {
                player.charge = CHARGE_SECONDS;
                player.cooldown = CHARGE_COOLDOWN;
                events.push(json!({"kind":"dive","seat":seat}));
                let aim = if input.stick.x.abs() + input.stick.z.abs() > 0.1 {
                    input.stick
                } else {
                    Stick {
                        x: player.run.vx,
                        z: player.run.vz,
                    }
                }
                .clamped();
                let length = (aim.x * aim.x + aim.z * aim.z).sqrt().max(0.01);
                let speed = crate::bout::BodyProfile::new(player.run.width).speed;
                player.run.vx = aim.x / length * ground::RUN_SPEED * CHARGE_SPEED * speed;
                player.run.vz = aim.z / length * ground::RUN_SPEED * CHARGE_SPEED * speed;
            }
            player.dive_held = input.jump;
            let speed = if player.charging() {
                ground::RUN_SPEED * CHARGE_SPEED
            } else {
                ground::RUN_SPEED
            };
            let stick = if player.charging() {
                Stick::default()
            } else {
                input.stick
            };
            ground::step_runner(&mut player.run, stick, dt, speed);
            ground::hold_inside(&mut player.run, self.radius);
            if player.run.speed() > 0.2 {
                player.facing = player.run.facing();
            }
        }
        let mut runners: Vec<Runner> = self.players.iter().map(|p| p.run).collect();
        ground::separate(&mut runners);
        for (player, run) in self.players.iter_mut().zip(runners) {
            player.run = run;
            ground::hold_inside(&mut player.run, self.radius);
        }

        for coin in &mut self.coins {
            coin.scatter = (coin.scatter - dt).max(0.0);
            coin.y = (coin.y - coin.fall * dt).max(0.0);
        }

        self.resolve_charges(events);
        self.resolve_punches(events);
        self.collect(events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(me) = self.player(seat).cloned() else {
            return Input::default();
        };
        let here = (me.run.x, me.run.z);
        // Somebody richer within reach is worth more than the next coin.
        let mark = self
            .players
            .iter()
            .filter(|p| {
                p.seat != seat
                    && p.safe <= 0.0
                    && p.purse >= me.purse + 4
                    && me.run.distance_to(p.run.x, p.run.z) <= CHASE_RANGE
            })
            .min_by(|a, b| {
                me.run
                    .distance_to(a.run.x, a.run.z)
                    .partial_cmp(&me.run.distance_to(b.run.x, b.run.z))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();
        let skill = self.skill(seat);
        if let Some(mark) = mark.filter(|_| me.cooldown <= 0.0) {
            let away = me.run.distance_to(mark.run.x, mark.run.z);
            let stick = ground::steer(here, (mark.run.x, mark.run.z));
            return Input {
                stick: Stick {
                    x: stick.x * skill.pace(),
                    z: stick.z * skill.pace(),
                }
                .clamped(),
                jump: away <= CHARGE_RANGE + 0.8
                    && self.rng.random_bool(0.01 + skill.level() as f64 * 0.06),
                ..Default::default()
            };
        }
        // Otherwise sweep up whatever is worth the run.
        let worth = |coin: &Coin| coin.value as f32 / (me.run.distance_to(coin.x, coin.z) + 2.0);
        let target = self
            .coins
            .iter()
            .filter(|coin| coin.scatter <= 0.0)
            .max_by(|a, b| {
                worth(a)
                    .partial_cmp(&worth(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|coin| (coin.x, coin.z));
        let stick = match target {
            Some(at) => ground::steer(here, at),
            // Nothing on the floor: wait near the middle for the next drop.
            None => ground::steer(here, (0.0, 0.0)),
        };
        Input {
            stick: Stick {
                x: stick.x * skill.pace(),
                z: stick.z * skill.pace(),
            }
            .clamped(),
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "coins",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "next_gold": self.gold_due().map(|at| round((at - self.time).max(0.0))),
            "coins": self.coins.iter().map(|coin| json!({
                "id": coin.id,
                "x": round(coin.x),
                "y": round(coin.y),
                "z": round(coin.z),
                "value": coin.value,
            })).collect::<Vec<_>>(),
            "players": self.players.iter().map(|p| json!({
                "seat": p.seat,
                "x": round(p.run.x),
                "z": round(p.run.z),
                "vx": round(p.run.vx),
                "vz": round(p.run.vz),
                "yaw": round(p.facing),
                "purse": p.purse,
                "dashing": p.charging(),
                "ready": p.cooldown <= 0.0,
                "dive_cd": round(p.cooldown),
                "punching": p.punch > 0.0,
                "punch_ready": p.punch_cooldown <= 0.0,
                "punch_cd": round(p.punch_cooldown),
                "safe": p.safe > 0.0,
                "out": false,
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bout::Contestant;

    fn field(count: u8) -> Vec<Contestant> {
        (0..count).map(|seat| Contestant::bot(seat, 0.5)).collect()
    }

    #[test]
    fn a_bigger_field_gets_a_wider_floor_and_room_for_more_coins() {
        let four = CoinScramble::new(&field(4), 1);
        assert_eq!(four.radius, ARENA_RADIUS, "four play it as built");
        assert_eq!(four.floor_max, FLOOR_MAX);
        assert_eq!(four.gold_fall, GOLD_FALL);
        let eight = CoinScramble::new(&field(8), 1);
        assert!(
            (eight.radius / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4,
            "eight get the room of two fours"
        );
        assert_eq!(
            eight.floor_max,
            FLOOR_MAX * 2,
            "and twice the coins can lie about"
        );
        assert!(
            eight.gold_fall < GOLD_FALL,
            "gold drifts down slower over the wider floor"
        );
        assert_eq!(
            eight.setup()["radius"],
            json!(eight.radius),
            "and the client is told"
        );
    }

    #[test]
    fn punch_spills_nearby_rival_and_dive_requires_a_new_press() {
        let mut game = CoinScramble::new(&field(2), 9);
        game.players[0].run = Runner::at(0.0, 0.0);
        game.players[1].run = Runner::at(0.0, 1.4);
        game.players[1].purse = 10;
        let mut events = Vec::new();
        let punch = Input {
            fire: true,
            stick: Stick { x: 0.0, z: 1.0 },
            ..Default::default()
        };
        game.step(&[(0, punch)], 0.016, &mut events);
        assert!(events.iter().any(|event| event["kind"] == "punch"));
        assert!(
            events
                .iter()
                .any(|event| event["kind"] == "spill" && event["attack"] == "punch")
        );
        assert_eq!(game.players[1].purse, 8);

        let dive = Input {
            jump: true,
            stick: Stick { x: 0.0, z: 1.0 },
            ..Default::default()
        };
        events.clear();
        game.step(&[(0, dive)], 0.016, &mut events);
        assert_eq!(
            events
                .iter()
                .filter(|event| event["kind"] == "dive")
                .count(),
            1
        );
        events.clear();
        game.step(&[(0, dive)], CHARGE_COOLDOWN + 0.1, &mut events);
        assert!(
            !events.iter().any(|event| event["kind"] == "dive"),
            "holding dive cannot repeat it"
        );
    }
}
