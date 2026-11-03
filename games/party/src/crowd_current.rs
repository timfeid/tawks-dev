//! Everyone can move one heavy parade float. A moving arch scores a delivery;
//! recent push contribution determines who earns it.
use crate::bout::{Bout, Contestant, Input, Stick};
use serde_json::{Value, json};
use std::f32::consts::TAU;

pub const KEY: &str = "crowd_current";
pub const LIMIT: f32 = 65.0;
pub const RADIUS: f32 = 10.5;
pub const GOAL_RADIUS: f32 = 8.45;
pub const FLOAT_RADIUS: f32 = 1.25;
pub const ARCH_SPEED: f32 = 0.20;
const WALK_SPEED: f32 = 5.6;
const SHOVE_COOLDOWN: f32 = 1.15;
const LEAVE_SECONDS: f32 = 4.0;
const ARCH_WIDTH: f32 = 0.62;

fn norm(x: f32, z: f32) -> (f32, f32) {
    let l = x.hypot(z).max(0.001);
    (x / l, z / l)
}
fn angle_delta(a: f32, b: f32) -> f32 {
    (a - b + std::f32::consts::PI).rem_euclid(TAU) - std::f32::consts::PI
}

#[derive(Clone)]
struct Player {
    seat: u8,
    skill: f32,
    x: f32,
    z: f32,
    vx: f32,
    vz: f32,
    yaw: f32,
    points: u32,
    deliveries: u32,
    credit: f32,
    missing: f32,
    left: bool,
    held_fire: bool,
    last_shove: f32,
    plan_angle: f32,
    plan_at: f32,
}

pub struct CrowdCurrent {
    players: Vec<Player>,
    seed: u64,
    time: f32,
    float_x: f32,
    float_z: f32,
    float_vx: f32,
    float_vz: f32,
    arch_base: f32,
    deliveries: u32,
    placed: Option<Vec<Vec<u8>>>,
}

impl CrowdCurrent {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let count = contestants.len().max(1);
        let players = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let slot = (i + seed as usize % count) % count;
                let angle = TAU * slot as f32 / count as f32;
                Player {
                    seat: c.seat,
                    skill: c.skill.clamp(0., 1.),
                    x: angle.sin() * 5.1,
                    z: angle.cos() * 5.1,
                    vx: 0.,
                    vz: 0.,
                    yaw: angle,
                    points: 0,
                    deliveries: 0,
                    credit: 0.,
                    missing: 0.,
                    left: false,
                    held_fire: false,
                    last_shove: -10.,
                    plan_angle: 0.,
                    plan_at: -1.,
                }
            })
            .collect();
        Self {
            players,
            seed,
            time: 0.,
            float_x: 0.,
            float_z: 0.,
            float_vx: 0.,
            float_vz: 0.,
            arch_base: (seed.wrapping_mul(0x9E3779B97F4A7C15) >> 32) as f32 / u32::MAX as f32 * TAU,
            deliveries: 0,
            placed: None,
        }
    }

    fn arch(&self) -> f32 {
        self.arch_base + self.time * ARCH_SPEED
    }

    fn finish(&mut self) {
        let mut ranked = self.players.iter().collect::<Vec<_>>();
        ranked.sort_by(|a, b| {
            a.left
                .cmp(&b.left)
                .then_with(|| b.points.cmp(&a.points))
                .then_with(|| b.deliveries.cmp(&a.deliveries))
        });
        let mut groups: Vec<Vec<u8>> = Vec::new();
        for p in ranked {
            let same = groups
                .last()
                .and_then(|g| g.first())
                .and_then(|seat| self.players.iter().find(|q| q.seat == *seat))
                .is_some_and(|q| {
                    q.left == p.left && q.points == p.points && q.deliveries == p.deliveries
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

impl Bout for CrowdCurrent {
    fn key(&self) -> &'static str {
        KEY
    }
    fn setup(&self) -> Value {
        json!({"game":KEY,"radius":RADIUS,"goal_radius":GOAL_RADIUS,"float_radius":FLOAT_RADIUS,"limit":LIMIT})
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
        let mut impulse = (0., 0.);
        for p in &mut self.players {
            if p.left {
                continue;
            }
            p.credit *= (-0.36 * dt).exp();
            let Some(input) = inputs
                .iter()
                .find(|(seat, _)| *seat == p.seat)
                .map(|(_, i)| *i)
            else {
                p.missing += dt;
                p.vx = 0.;
                p.vz = 0.;
                if p.missing >= LEAVE_SECONDS {
                    p.left = true;
                    events.push(json!({"kind":"crowd_left","seat":p.seat}));
                }
                continue;
            };
            p.missing = 0.;
            let stick = input.stick.clamped();
            p.vx = stick.x * WALK_SPEED;
            p.vz = stick.z * WALK_SPEED;
            p.x += p.vx * dt;
            p.z += p.vz * dt;
            let distance = p.x.hypot(p.z);
            if distance > RADIUS - 0.5 {
                let s = (RADIUS - 0.5) / distance;
                p.x *= s;
                p.z *= s;
            }
            if p.vx.hypot(p.vz) > 0.1 {
                p.yaw = p.vx.atan2(p.vz);
            }
            let dx = self.float_x - p.x;
            let dz = self.float_z - p.z;
            let distance = dx.hypot(dz);
            let (nx, nz) = norm(dx, dz);
            let contact = FLOAT_RADIUS + 0.52;
            if distance < contact {
                p.x = self.float_x - nx * contact;
                p.z = self.float_z - nz * contact;
                let drive = (p.vx * nx + p.vz * nz).max(0.);
                let force = drive * dt * 3.6;
                impulse.0 += nx * force;
                impulse.1 += nz * force;
                p.credit += force;
            }
            let fire = input.fire || input.jump;
            if fire
                && !p.held_fire
                && self.time - p.last_shove >= SHOVE_COOLDOWN
                && distance < contact + 0.55
            {
                let force = 3.2;
                impulse.0 += nx * force;
                impulse.1 += nz * force;
                p.credit += force;
                p.last_shove = self.time;
                events.push(json!({"kind":"crowd_shove","seat":p.seat}));
            }
            p.held_fire = fire;
        }
        self.float_vx = (self.float_vx + impulse.0) * (-0.8 * dt).exp();
        self.float_vz = (self.float_vz + impulse.1) * (-0.8 * dt).exp();
        let speed = self.float_vx.hypot(self.float_vz);
        if speed > 8.5 {
            self.float_vx *= 8.5 / speed;
            self.float_vz *= 8.5 / speed;
        }
        self.float_x += self.float_vx * dt;
        self.float_z += self.float_vz * dt;
        let radius = self.float_x.hypot(self.float_z);
        if radius >= GOAL_RADIUS {
            let angle = self.float_x.atan2(self.float_z);
            let arch = self.arch();
            if angle_delta(angle, arch).abs() <= ARCH_WIDTH {
                let best = self
                    .players
                    .iter()
                    .filter(|p| !p.left)
                    .map(|p| p.credit)
                    .fold(0., f32::max);
                let mut winners = Vec::new();
                for p in &mut self.players {
                    if p.left || p.credit < 1.0 {
                        continue;
                    }
                    let lead = p.credit >= best - 0.03;
                    p.points += if lead { 3 } else { 1 };
                    p.deliveries += u32::from(lead);
                    if lead {
                        winners.push(p.seat);
                    }
                }
                self.deliveries += 1;
                events.push(
                    json!({"kind":"crowd_delivery","seats":winners,"number":self.deliveries}),
                );
                self.float_x = 0.;
                self.float_z = 0.;
                self.float_vx = 0.;
                self.float_vz = 0.;
                for p in &mut self.players {
                    p.credit = 0.;
                }
            } else {
                let (nx, nz) = norm(self.float_x, self.float_z);
                self.float_x = nx * GOAL_RADIUS;
                self.float_z = nz * GOAL_RADIUS;
                let outward = (self.float_vx * nx + self.float_vz * nz).max(0.);
                self.float_vx -= nx * outward * 1.55;
                self.float_vz -= nz * outward * 1.55;
                events.push(json!({"kind":"crowd_rim"}));
            }
        }
        if self.time >= LIMIT || self.players.iter().all(|p| p.left) {
            self.finish();
        }
    }

    fn bot_input(&mut self, seat: u8, _dt: f32) -> Input {
        let Some(index) = self.players.iter().position(|p| p.seat == seat && !p.left) else {
            return Input::default();
        };
        if self.time >= self.players[index].plan_at {
            let p = &mut self.players[index];
            let error = ((self
                .seed
                .wrapping_add(seat as u64 * 37)
                .wrapping_add(self.deliveries as u64 * 23)
                % 97) as f32
                / 97.
                - 0.5)
                * (1. - p.skill)
                * 0.36;
            p.plan_angle = self.arch_base + self.time * ARCH_SPEED + error;
            p.plan_at = self.time + 0.14 + (1. - p.skill) * 0.42;
        }
        let p = &self.players[index];
        let (gx, gz) = (p.plan_angle.sin(), p.plan_angle.cos());
        let behind = (
            self.float_x - gx * (FLOAT_RADIUS + 1.25),
            self.float_z - gz * (FLOAT_RADIUS + 1.25),
        );
        let distance = (p.x - behind.0).hypot(p.z - behind.1);
        let target = if distance > 0.52 {
            behind
        } else {
            (self.float_x + gx * 1.4, self.float_z + gz * 1.4)
        };
        let (x, z) = norm(target.0 - p.x, target.1 - p.z);
        let near = (p.x - self.float_x).hypot(p.z - self.float_z) < FLOAT_RADIUS + 1.0;
        let fire = near && self.time - p.last_shove >= SHOVE_COOLDOWN + 0.12 && !p.held_fire;
        Input {
            stick: Stick { x, z },
            fire,
            ..Default::default()
        }
    }

    fn snapshot(&self) -> Value {
        json!({"game":KEY,"time":self.time,"limit":LIMIT,"done":self.placed.is_some(),
            "arch_angle":self.arch(),"deliveries":self.deliveries,
            "float":{"x":self.float_x,"z":self.float_z,"vx":self.float_vx,"vz":self.float_vz},
            "players":self.players.iter().map(|p|json!({"seat":p.seat,"x":p.x,"z":p.z,"vx":p.vx,"vz":p.vz,"yaw":p.yaw,"points":p.points,"deliveries":p.deliveries,"credit":p.credit,"left":p.left})).collect::<Vec<_>>()
        })
    }
    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> CrowdCurrent {
        CrowdCurrent::new(&[Contestant::person(0), Contestant::person(1)], 7)
    }
    #[test]
    fn untrusted_motion_is_clamped_and_leaver_places_last() {
        let mut g = game();
        let mut events = Vec::new();
        for _ in 0..250 {
            g.step(
                &[(
                    0,
                    Input::from_stick(Stick {
                        x: f32::NAN,
                        z: f32::INFINITY,
                    }),
                )],
                1. / 60.,
                &mut events,
            );
        }
        assert!(g.players[0].x.is_finite() && g.players[0].z.is_finite());
        assert!(g.players[1].left);
        while g.placements().is_none() {
            g.step(&[(0, Input::default())], 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().last(), Some(&vec![1]));
    }
    #[test]
    fn contact_and_shove_make_server_owned_float_move() {
        let mut g = game();
        let mut events = Vec::new();
        g.players[0].x = -1.6;
        g.players[0].z = 0.;
        g.step(
            &[
                (
                    0,
                    Input {
                        stick: Stick { x: 1., z: 0. },
                        fire: true,
                        ..Default::default()
                    },
                ),
                (1, Input::default()),
            ],
            1. / 60.,
            &mut events,
        );
        assert!(g.float_vx > 0.);
        assert!(g.players[0].credit > 0.);
        assert!(events.iter().any(|e| e["kind"] == "crowd_shove"));
    }
    #[test]
    fn delivery_rewards_recent_contributors_and_resets_float() {
        let mut g = game();
        let mut events = Vec::new();
        g.arch_base = 0.;
        g.float_x = 0.;
        g.float_z = GOAL_RADIUS - 0.05;
        g.float_vz = 8.;
        g.players[0].credit = 4.;
        g.players[1].credit = 2.;
        g.step(
            &[(0, Input::default()), (1, Input::default())],
            1. / 60.,
            &mut events,
        );
        assert_eq!(g.players[0].points, 3);
        assert_eq!(g.players[1].points, 1);
        assert_eq!(g.float_z, 0.);
    }
    #[test]
    fn bots_finish_and_somebody_delivers() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.6))
            .collect::<Vec<_>>();
        let mut g = CrowdCurrent::new(&seats, 3);
        let mut events = Vec::new();
        while g.placements().is_none() {
            let inputs = seats
                .iter()
                .map(|s| (s.seat, g.bot_input(s.seat, 1. / 60.)))
                .collect::<Vec<_>>();
            g.step(&inputs, 1. / 60., &mut events);
        }
        assert_eq!(g.placements().unwrap().iter().flatten().count(), 4);
        assert!(
            g.deliveries > 0,
            "bots should move the float through an arch"
        );
    }

    #[test]
    #[ignore]
    fn seeded_balance_report() {
        let seats = (0..4)
            .map(|seat| Contestant::bot(seat, 0.5))
            .collect::<Vec<_>>();
        let mut first = [0u32; 4];
        let mut scores = [0u32; 4];
        let mut deliveries = 0u32;
        for seed in 0..160 {
            let mut g = CrowdCurrent::new(&seats, seed);
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
            }
            deliveries += g.deliveries;
        }
        println!(
            "Crowd Current equal skill: first={first:?}, scores={scores:?}, deliveries={deliveries}"
        );
        assert!(deliveries > 160);
        assert!(*first.iter().max().unwrap() - *first.iter().min().unwrap() <= 25);
        assert!(*scores.iter().max().unwrap() - *scores.iter().min().unwrap() <= 300);
    }
}
