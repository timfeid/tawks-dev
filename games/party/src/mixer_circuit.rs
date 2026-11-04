//! A social sprint: exchange introductions with distinct partners to finish a card.
use crate::bout::{BodyProfile, Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "mixer_circuit";
pub const LIMIT: f32 = 60.;
const RADIUS: f32 = 9.5;
const SPEED: f32 = 6.4;
const REACH: f32 = 1.65;
const PAIR_WAIT: f32 = 2.4;
const LEAVE_AFTER: f32 = 4.;
fn greeting_reach(width: f32) -> f32 {
    let body = BodyProfile::new(width);
    REACH * body.radius * body.mass.sqrt()
}
fn run_speed(width: f32) -> f32 {
    let body = BodyProfile::new(width);
    SPEED * body.speed * body.acceleration.sqrt()
}
fn mix(mut n: u64) -> u64 {
    n ^= n >> 30;
    n = n.wrapping_mul(0xbf58476d1ce4e5b9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

#[derive(Clone)]
struct Player {
    seat: u8,
    width: f32,
    skill: f32,
    x: f32,
    z: f32,
    yaw: f32,
    points: u32,
    intros: u32,
    circuits: u32,
    met: u16,
    held: bool,
    missing: f32,
    left: bool,
}
#[derive(Clone)]
struct Link {
    id: u32,
    a: u8,
    b: u8,
    life: f32,
}
pub struct MixerCircuit {
    seed: u64,
    players: Vec<Player>,
    pair_ready: [[f32; 8]; 8],
    links: Vec<Link>,
    next_link: u32,
    tick: u64,
    time: f32,
    placed: Option<Vec<Vec<u8>>>,
}
impl MixerCircuit {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let angle = TAU * ((i + seed as usize % count) % count) as f32 / count as f32;
                Player {
                    seat: c.seat,
                    width: BodyProfile::new(c.body_width).width,
                    skill: c.skill.clamp(0., 1.),
                    x: angle.sin() * 6.8,
                    z: angle.cos() * 6.8,
                    yaw: angle,
                    points: 0,
                    intros: 0,
                    circuits: 0,
                    met: 0,
                    held: false,
                    missing: 0.,
                    left: false,
                }
            })
            .collect();
        Self {
            seed,
            players,
            pair_ready: [[0.; 8]; 8],
            links: Vec::new(),
            next_link: 0,
            tick: 0,
            time: 0.,
            placed: None,
        }
    }
    fn target(&self) -> u32 {
        self.players
            .iter()
            .filter(|p| !p.left)
            .count()
            .saturating_sub(1)
            .min(3) as u32
    }
    fn complete(p: &mut Player, target: u32, events: &mut Vec<Value>) {
        if target > 0 && p.met.count_ones() >= target {
            p.points += 3;
            p.circuits += 1;
            p.met = 0;
            events.push(json!({"kind":"mixer_circuit","seat":p.seat}));
        }
    }
    fn finish(&mut self) {
        let mut ranked = self.players.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.points.cmp(&a.points))
                .then_with(|| b.circuits.cmp(&a.circuits))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left && q.points == p.points && q.circuits == p.circuits
                });
            if same {
                groups.last_mut().unwrap().push(p.seat);
            } else {
                groups.push(vec![p.seat]);
            }
        }
        self.placed = Some(groups);
    }
}
impl Bout for MixerCircuit {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"limit":LIMIT,"reach":REACH})
    }
    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placed.is_some() {
            return;
        }
        let dt = if dt.is_finite() {
            dt.clamp(0., 0.1)
        } else {
            0.
        };
        self.time += dt;
        self.tick += 1;
        for link in &mut self.links {
            link.life -= dt;
        }
        self.links.retain(|link| link.life > 0.);
        let mut pressed = Vec::new();
        for (index, p) in self.players.iter_mut().enumerate() {
            if p.left {
                continue;
            }
            let Some(input) = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, input)| *input)
            else {
                p.missing += dt;
                if p.missing >= LEAVE_AFTER {
                    p.left = true;
                    events.push(json!({"kind":"mixer_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            let stick = input.stick.clamped();
            p.x += stick.x * run_speed(p.width) * dt;
            p.z += stick.z * run_speed(p.width) * dt;
            let distance = p.x.hypot(p.z);
            let edge = RADIUS - 0.45 * BodyProfile::new(p.width).radius;
            if distance > edge {
                let scale = edge / distance;
                p.x *= scale;
                p.z *= scale;
            }
            if stick.x.hypot(stick.z) > 0.1 {
                p.yaw = stick.x.atan2(stick.z);
            }
            let fire = input.fire || input.jump;
            if fire && !p.held {
                pressed.push(index);
            }
            p.held = fire;
        }
        let target = self.target();
        let order_seed = mix(self.seed ^ self.tick.wrapping_mul(0x9e3779b97f4a7c15));
        pressed.sort_by_key(|index| mix(order_seed ^ self.players[*index].seat as u64));
        for index in pressed {
            if self.players[index].left {
                continue;
            }
            let p = &self.players[index];
            let partner = self
                .players
                .iter()
                .enumerate()
                .filter(|(j, q)| *j != index && !q.left)
                .filter(|(_, q)| self.pair_ready[p.seat as usize][q.seat as usize] <= self.time)
                .map(|(j, q)| (j, (p.x - q.x).hypot(p.z - q.z)))
                .filter(|(_, distance)| *distance <= greeting_reach(p.width))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(j, _)| j);
            let Some(j) = partner else { continue };
            let (left, right) = self.players.split_at_mut(index.max(j));
            let (a, b) = if index < j {
                (&mut left[index], &mut right[0])
            } else {
                (&mut right[0], &mut left[j])
            };
            let a_seat = a.seat;
            let b_seat = b.seat;
            self.pair_ready[a_seat as usize][b_seat as usize] = self.time + PAIR_WAIT;
            self.pair_ready[b_seat as usize][a_seat as usize] = self.time + PAIR_WAIT;
            let fresh_a = a.met & (1u16 << b_seat) == 0;
            let fresh_b = b.met & (1u16 << a_seat) == 0;
            if fresh_a {
                a.met |= 1u16 << b_seat;
                a.points += 1;
                a.intros += 1;
            }
            if fresh_b {
                b.met |= 1u16 << a_seat;
                b.points += 1;
                b.intros += 1;
            }
            if fresh_a || fresh_b {
                a.points += 1;
                self.links.push(Link {
                    id: self.next_link,
                    a: a_seat,
                    b: b_seat,
                    life: 0.75,
                });
                self.next_link += 1;
                events.push(json!({"kind":"mixer_intro","a":a_seat,"b":b_seat}));
            }
            Self::complete(a, target, events);
            Self::complete(b, target, events);
        }
        for p in &mut self.players {
            if !p.left {
                Self::complete(p, target, events);
            }
        }
        if self.time >= LIMIT
            || (self.players.len() > 1 && self.players.iter().filter(|p| !p.left).count() <= 1)
        {
            self.finish();
        }
    }
    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(p) = self.players.iter().find(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        let target = self
            .players
            .iter()
            .filter(|q| q.seat != seat && !q.left)
            .map(|q| {
                let fresh = p.met & (1u16 << q.seat) == 0;
                let ready = self.pair_ready[seat as usize][q.seat as usize] <= self.time;
                let distance = (p.x - q.x).hypot(p.z - q.z);
                (q, fresh, ready, distance)
            })
            .min_by(|a, b| {
                (!a.1)
                    .cmp(&(!b.1))
                    .then_with(|| (!a.2).cmp(&(!b.2)))
                    .then_with(|| a.3.total_cmp(&b.3))
            });
        let Some((q, _, ready, distance)) = target else {
            return Input::default();
        };
        let dx = q.x - p.x;
        let dz = q.z - p.z;
        let pace = 0.72 + p.skill * 0.28;
        let wobble = (1. - p.skill) * 0.18 * (self.time * 3.0 + seat as f32 * 1.8).sin();
        let (x, z) = if distance > 0.001 {
            (
                (dx / distance + wobble) * pace,
                (dz / distance - wobble) * pace,
            )
        } else {
            (0., 0.)
        };
        let fire =
            ready && distance <= greeting_reach(p.width) * (0.78 + p.skill * 0.12) && !p.held;
        Input {
            stick: Stick { x, z },
            fire,
            ..Default::default()
        }
    }
    fn snapshot(&self) -> Value {
        let target = self.target();
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),"target":target,
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"yaw":p.yaw,
                "points":p.points,"intros":p.intros,"circuits":p.circuits,"met":p.met,
                "met_count":p.met.count_ones(),"left":p.left,"body_width":p.width,
                "greeting_reach":greeting_reach(p.width),"move_speed":run_speed(p.width)})).collect::<Vec<_>>(),
            "links":self.links.iter().map(|l|json!({"id":l.id,"a":l.a,"b":l.b,"life":l.life})).collect::<Vec<_>>()})
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn small_build_runs_faster_broad_build_greets_farther() {
        let mut small = Contestant::person(0);
        small.body_width = 0.7;
        let mut broad = Contestant::person(1);
        broad.body_width = 1.4;
        let mut g = MixerCircuit::new(&[small, broad], 4);
        g.players[0].x = 0.;
        g.players[1].x = 0.;
        g.players[0].z = 0.;
        g.players[1].z = 0.;
        g.step(
            &[
                (0, Input::from_stick(Stick { x: 1., z: 0. })),
                (1, Input::from_stick(Stick { x: 1., z: 0. })),
            ],
            0.1,
            &mut vec![],
        );
        assert!(g.players[0].x > g.players[1].x);
        g.players[0].x = 0.;
        g.players[1].x = 1.8;
        let mut events = Vec::new();
        g.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (1, Input::default()),
            ],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].intros, 0);
        g.step(
            &[
                (0, Input::default()),
                (
                    1,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
            ],
            0.,
            &mut events,
        );
        assert_eq!(g.players[1].intros, 1);
        assert_eq!(g.players[0].intros, 1);
    }
    #[test]
    fn mixed_builds_can_all_finish_cards() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        let mut scores = [0u32; 4];
        let mut wins = [0u32; 4];
        for seed in 0..128 {
            let seats = (0..4)
                .map(|seat| {
                    let mut c = Contestant::bot(seat, 0.5);
                    c.body_width = widths[(seat as usize + seed as usize / 4) % 4];
                    c
                })
                .collect::<Vec<_>>();
            let mut g = MixerCircuit::new(&seats, seed);
            while g.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1. / 60., &mut vec![]);
            }
            for player in &g.players {
                let index = widths
                    .iter()
                    .position(|width| *width == player.width)
                    .unwrap();
                scores[index] += player.points;
            }
            for seat in &g.placements().unwrap()[0] {
                let player = g.players.iter().find(|p| p.seat == *seat).unwrap();
                let index = widths
                    .iter()
                    .position(|width| *width == player.width)
                    .unwrap();
                wins[index] += 1;
            }
        }
        println!("mixer circuit points by build: {scores:?}, wins: {wins:?}");
        assert!(scores.iter().all(|score| *score > 0));
        assert!(scores.iter().max().unwrap() < &(scores.iter().min().unwrap() * 11 / 10));
        assert!(wins.iter().all(|wins| (20..=50).contains(wins)));
    }
    #[test]
    fn eight_player_mixed_builds_keep_exchanging() {
        let widths = [0.7, 0.9, 1.2, 1.4];
        for seed in 0..16 {
            let seats = (0..8)
                .map(|seat| {
                    let mut c = Contestant::bot(seat, 0.5);
                    c.body_width = widths[(seat as usize + seed as usize / 8) % 4];
                    c
                })
                .collect::<Vec<_>>();
            let mut game = MixerCircuit::new(&seats, seed);
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut vec![]);
            }
            assert_eq!(game.placements().unwrap().iter().flatten().count(), 8);
            assert!(
                game.players.iter().all(|player| player.points > 0),
                "seed {seed}"
            );
            assert!(
                game.players
                    .iter()
                    .map(|player| player.circuits)
                    .sum::<u32>()
                    > 0
            );
        }
    }
    #[test]
    fn circuits_scale_from_two_to_four_players() {
        for count in 2..=4 {
            let seats = (0..count)
                .map(|s| Contestant::bot(s, 0.5))
                .collect::<Vec<_>>();
            let g = MixerCircuit::new(&seats, 1);
            assert_eq!(g.target(), (count - 1).min(3) as u32);
        }
    }
    #[test]
    fn a_pair_exchanges_one_credit_each_and_repeats_wait() {
        let seats = (0..2).map(|s| Contestant::person(s)).collect::<Vec<_>>();
        let mut g = MixerCircuit::new(&seats, 1);
        g.players[0].x = 0.;
        g.players[0].z = 0.;
        g.players[1].x = 1.;
        g.players[1].z = 0.;
        let mut events = Vec::new();
        g.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (1, Input::default()),
            ],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].intros, 1);
        assert_eq!(g.players[1].intros, 1);
        assert_eq!(g.players[0].circuits, 1);
        assert_eq!(g.players[1].circuits, 1);
        g.step(
            &[(0, Input::default()), (1, Input::default())],
            0.,
            &mut events,
        );
        g.step(
            &[
                (
                    0,
                    Input {
                        fire: true,
                        ..Default::default()
                    },
                ),
                (1, Input::default()),
            ],
            0.,
            &mut events,
        );
        assert_eq!(g.players[0].intros, 1);
    }
    #[test]
    fn a_leaver_reduces_the_required_card_size() {
        let seats = (0..4).map(|s| Contestant::person(s)).collect::<Vec<_>>();
        let mut g = MixerCircuit::new(&seats, 2);
        g.players[0].met = (1 << 1) | (1 << 2);
        let mut events = Vec::new();
        let inputs = seats[0..3]
            .iter()
            .map(|s| (s.seat, Input::default()))
            .collect::<Vec<_>>();
        for _ in 0..45 {
            g.step(&inputs, 0.1, &mut events);
        }
        assert!(g.players[3].left);
        assert_eq!(g.target(), 2);
        assert_eq!(g.players[0].circuits, 1);
    }
    #[test]
    fn bots_finish_and_all_players_place() {
        let seats = (0..4).map(|s| Contestant::bot(s, 0.5)).collect::<Vec<_>>();
        let mut g = MixerCircuit::new(&seats, 7);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(g.players.iter().map(|p| p.intros).sum::<u32>() > 8);
    }
    #[test]
    fn small_lobbies_complete_cards() {
        for count in 2..=3 {
            let seats = (0..count)
                .map(|s| Contestant::bot(s, 0.5))
                .collect::<Vec<_>>();
            let mut g = MixerCircuit::new(&seats, count as u64);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1. / 60., &mut events);
            }
            assert!(g.players.iter().all(|p| p.circuits > 0));
        }
    }
    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = (0..4).map(|s| Contestant::bot(s, 0.5)).collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut circuits = 0;
        for seed in 0..160 {
            let mut g = MixerCircuit::new(&seats, seed);
            let mut events = Vec::new();
            while g.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                g.step(&inputs, 1. / 60., &mut events);
            }
            for &seat in &g.placements().unwrap()[0] {
                first[seat as usize] += 1;
            }
            for p in &g.players {
                scores[p.seat as usize] += p.points;
                circuits += p.circuits;
            }
        }
        println!(
            "Mixer Circuit equal skill: first={first:?}, scores={scores:?}, circuits={circuits}"
        );
        assert!(circuits > 500);
        assert!(first.iter().max().unwrap() - first.iter().min().unwrap() <= 30);
        assert!(scores.iter().max().unwrap() - scores.iter().min().unwrap() <= 1_000);
    }
}
