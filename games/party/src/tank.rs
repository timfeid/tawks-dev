//! Tank Clash: drive a little tank around a yard of crates, swing the turret
//! wherever you are looking, and lob shells that bounce off the walls. One
//! clean hit wrecks a tank; it rolls back out a few seconds later. Most wrecks
//! when the horn goes wins.

use rand::{Rng, SeedableRng, rngs::StdRng};
use serde_json::{Value, json};

use crate::bout::{Contestant, Input, Bout, Skill, Stick};

pub const YARD_HALF: f32 = 21.0;
pub const TANK_RADIUS: f32 = 1.05;
pub const DRIVE_SPEED: f32 = 6.2;
pub const REVERSE_SPEED: f32 = 3.4;
pub const HULL_TURN: f32 = 3.4;
pub const SHELL_SPEED: f32 = 17.0;
pub const SHELL_RADIUS: f32 = 0.3;
pub const SHELL_LIFE: f32 = 2.6;
pub const SHELL_BOUNCES: u8 = 2;
pub const RELOAD_SECONDS: f32 = 1.15;
pub const SHELLS_IN_FLIGHT: usize = 2;
pub const WRECK_SECONDS: f32 = 3.2;
pub const ROUND_SECONDS: f32 = 90.0;
/// A moment after firing when your own shell cannot come back at you.
const OWN_SHELL_GRACE: f32 = 0.35;

#[derive(Clone, Copy, Debug)]
pub struct Crate {
    pub x: f32,
    pub z: f32,
    pub half: f32,
    pub solid: bool,
    pub breakable: bool,
}

#[derive(Clone, Debug)]
pub struct Shell {
    pub id: u32,
    pub by: u8,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub life: f32,
    pub age: f32,
    pub bounces: u8,
}

#[derive(Clone, Debug)]
pub struct Tank {
    pub seat: u8,
    pub x: f32,
    pub z: f32,
    pub hull: f32,
    pub turret: f32,
    pub reload: f32,
    pub wrecked_for: f32,
    pub kills: u32,
    pub deaths: u32,
}

impl Tank {
    pub fn rolling(&self) -> bool {
        self.wrecked_for <= 0.0
    }
}

#[derive(Clone, Debug, Default)]
struct BotBrain {
    think_in: f32,
    aim_error: f32,
    circle: f32,
}

pub struct TankClash {
    pub tanks: Vec<Tank>,
    pub shells: Vec<Shell>,
    pub crates: Vec<Crate>,
    pub time: f32,
    next_shell: u32,
    rng: StdRng,
    brains: std::collections::HashMap<u8, BotBrain>,
    placements: Option<Vec<Vec<u8>>>,
}

/// The yard: solid blocks in the middle, breakable crates around them.
pub fn yard() -> Vec<Crate> {
    let mut crates = Vec::new();
    crates.push(Crate {
        x: 0.0,
        z: 0.0,
        half: 2.2,
        solid: true,
        breakable: false,
    });
    for (sx, sz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        crates.push(Crate {
            x: 9.0 * sx,
            z: 9.0 * sz,
            half: 1.8,
            solid: true,
            breakable: false,
        });
        crates.push(Crate {
            x: 15.0 * sx,
            z: 4.0 * sz,
            half: 1.1,
            solid: true,
            breakable: true,
        });
        crates.push(Crate {
            x: 4.0 * sx,
            z: 15.0 * sz,
            half: 1.1,
            solid: true,
            breakable: true,
        });
        crates.push(Crate {
            x: 12.5 * sx,
            z: 12.5 * sz,
            half: 1.4,
            solid: true,
            breakable: true,
        });
        crates.push(Crate {
            x: 6.5 * sx,
            z: 0.5 * sz,
            half: 0.9,
            solid: true,
            breakable: true,
        });
    }
    crates
}

fn spawns() -> Vec<(f32, f32)> {
    vec![
        (-17.0, -17.0),
        (17.0, 17.0),
        (17.0, -17.0),
        (-17.0, 17.0),
        (0.0, -18.5),
        (0.0, 18.5),
    ]
}

fn wrap_angle(a: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let mut angle = a % tau;
    if angle > std::f32::consts::PI {
        angle -= tau;
    }
    if angle < -std::f32::consts::PI {
        angle += tau;
    }
    angle
}

/// Where a turret points, as a direction on the floor.
pub fn muzzle(yaw: f32) -> (f32, f32) {
    (-yaw.sin(), -yaw.cos())
}

impl TankClash {
    pub fn new(contestants: &[Contestant], seed: u64) -> Self {
        let places = spawns();
        let tanks = contestants
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let (x, z) = places[i % places.len()];
                Tank {
                    seat: c.seat,
                    x,
                    z,
                    hull: x.atan2(z),
                    turret: x.atan2(z),
                    reload: 0.0,
                    wrecked_for: 0.0,
                    kills: 0,
                    deaths: 0,
                }
            })
            .collect();
        let brains = contestants
            .iter()
            .filter(|c| c.is_bot)
            .map(|c| {
                (
                    c.seat,
                    BotBrain {
                        think_in: 0.3,
                        ..Default::default()
                    },
                )
            })
            .collect();
        TankClash {
            tanks,
            shells: Vec::new(),
            crates: yard(),
            time: 0.0,
            next_shell: 0,
            rng: StdRng::seed_from_u64(seed),
            brains,
            placements: None,
        }
    }

    fn tank(&self, seat: u8) -> Option<&Tank> {
        self.tanks.iter().find(|t| t.seat == seat)
    }

    fn blocked(&self, x: f32, z: f32, radius: f32) -> bool {
        self.crates.iter().any(|block| {
            block.solid
                && (x - block.x).abs() <= block.half + radius
                && (z - block.z).abs() <= block.half + radius
        })
    }

    /// Whether a shell fired from here would reach a mark without clipping.
    fn clear_shot(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        let (dx, dz) = (to.0 - from.0, to.1 - from.1);
        let distance = (dx * dx + dz * dz).sqrt();
        if distance < 0.01 {
            return true;
        }
        let steps = (distance / 0.7).ceil() as i32;
        for step in 1..steps {
            let t = step as f32 / steps as f32;
            if self.blocked(from.0 + dx * t, from.1 + dz * t, SHELL_RADIUS) {
                return false;
            }
        }
        true
    }

    fn drive(&mut self, index: usize, input: Input, dt: f32) {
        let stick = input.stick.clamped();
        let push = (stick.x * stick.x + stick.z * stick.z).sqrt();
        let tank = &mut self.tanks[index];
        tank.turret = input.yaw;
        if push < 0.08 {
            return;
        }
        // The hull swings around to the way you are pushing, then rolls.
        let want = stick.x.atan2(stick.z);
        let turn = wrap_angle(want - tank.hull);
        let step = (HULL_TURN * dt).min(turn.abs());
        tank.hull += step * turn.signum();
        let lined_up = wrap_angle(want - tank.hull).abs();
        let speed = if lined_up < 0.6 {
            DRIVE_SPEED
        } else if lined_up > 2.4 {
            -REVERSE_SPEED
        } else {
            DRIVE_SPEED * 0.45
        };
        let (dx, dz) = (tank.hull.sin(), tank.hull.cos());
        let (here_x, here_z) = (tank.x, tank.z);
        let (nx, nz) = (
            here_x + dx * speed * push * dt,
            here_z + dz * speed * push * dt,
        );
        let edge = YARD_HALF - TANK_RADIUS;
        if !self.blocked(nx, here_z, TANK_RADIUS) && nx.abs() < edge {
            self.tanks[index].x = nx;
        }
        let settled_x = self.tanks[index].x;
        if !self.blocked(settled_x, nz, TANK_RADIUS) && nz.abs() < edge {
            self.tanks[index].z = nz;
        }
    }

    fn fire(&mut self, seat: u8, events: &mut Vec<Value>) {
        let Some(tank) = self.tank(seat).cloned() else {
            return;
        };
        if !tank.rolling() || tank.reload > 0.0 {
            return;
        }
        if self.shells.iter().filter(|s| s.by == seat).count() >= SHELLS_IN_FLIGHT {
            return;
        }
        let (dx, dz) = muzzle(tank.turret);
        let id = self.next_shell;
        self.next_shell += 1;
        self.shells.push(Shell {
            id,
            by: seat,
            x: tank.x + dx * (TANK_RADIUS + 0.5),
            z: tank.z + dz * (TANK_RADIUS + 0.5),
            vx: dx * SHELL_SPEED,
            vz: dz * SHELL_SPEED,
            life: SHELL_LIFE,
            age: 0.0,
            bounces: 0,
        });
        if let Some(tank) = self.tanks.iter_mut().find(|t| t.seat == seat) {
            tank.reload = RELOAD_SECONDS;
        }
        events.push(json!({ "kind": "shell", "seat": seat, "id": id, "x": tank.x, "z": tank.z }));
    }

    fn step_shells(&mut self, dt: f32, events: &mut Vec<Value>) {
        let mut spent: Vec<usize> = Vec::new();
        let mut wrecks: Vec<(u8, u8, f32, f32)> = Vec::new();
        let mut broken: Vec<usize> = Vec::new();
        for index in 0..self.shells.len() {
            let shell = self.shells[index].clone();
            let (nx, nz) = (shell.x + shell.vx * dt, shell.z + shell.vz * dt);

            // Walls and crates turn a shell around until it runs out of bounces.
            let mut vx = shell.vx;
            let mut vz = shell.vz;
            let mut bounced = false;
            let edge = YARD_HALF - SHELL_RADIUS;
            if nx.abs() > edge {
                vx = -vx;
                bounced = true;
            }
            if nz.abs() > edge {
                vz = -vz;
                bounced = true;
            }
            if let Some(hit) = self.crates.iter().position(|block| {
                block.solid
                    && (nx - block.x).abs() <= block.half + SHELL_RADIUS
                    && (nz - block.z).abs() <= block.half + SHELL_RADIUS
            }) {
                let block = self.crates[hit];
                if block.breakable {
                    broken.push(hit);
                    spent.push(index);
                    events.push(json!({ "kind": "splinter", "x": block.x, "z": block.z }));
                    continue;
                }
                // Bounce off the face the shell came through.
                let from_side = (shell.x - block.x).abs() > block.half + SHELL_RADIUS - 0.05;
                if from_side {
                    vx = -vx;
                } else {
                    vz = -vz;
                }
                bounced = true;
            }

            let shell = &mut self.shells[index];
            if bounced {
                shell.bounces += 1;
                shell.vx = vx;
                shell.vz = vz;
                events.push(
                    json!({ "kind": "ricochet", "id": shell.id, "x": shell.x, "z": shell.z }),
                );
                if shell.bounces > SHELL_BOUNCES {
                    spent.push(index);
                    continue;
                }
            } else {
                shell.x = nx;
                shell.z = nz;
            }
            shell.age += dt;
            shell.life -= dt;
            if shell.life <= 0.0 {
                spent.push(index);
                continue;
            }

            let shell = self.shells[index].clone();
            let reach = SHELL_RADIUS + TANK_RADIUS;
            if let Some(mark) = self.tanks.iter().find(|tank| {
                tank.rolling()
                    && (tank.seat != shell.by || shell.age > OWN_SHELL_GRACE)
                    && (tank.x - shell.x).abs() <= reach
                    && (tank.z - shell.z).abs() <= reach
            }) {
                wrecks.push((mark.seat, shell.by, shell.x, shell.z));
                spent.push(index);
            }
        }

        for (seat, by, x, z) in wrecks {
            if let Some(tank) = self.tanks.iter_mut().find(|t| t.seat == seat) {
                tank.wrecked_for = WRECK_SECONDS;
                tank.deaths += 1;
            }
            if seat != by
                && let Some(shooter) = self.tanks.iter_mut().find(|t| t.seat == by)
            {
                shooter.kills += 1;
            }
            events.push(json!({ "kind": "wreck", "seat": seat, "by": by, "x": x, "z": z }));
        }
        broken.sort_unstable();
        broken.dedup();
        for hit in broken.into_iter().rev() {
            self.crates[hit].solid = false;
        }
        spent.sort_unstable();
        spent.dedup();
        for index in spent.into_iter().rev() {
            self.shells.remove(index);
        }
    }

    /// Roll a wrecked tank back out somewhere quiet.
    fn respawn(&mut self, seat: u8, events: &mut Vec<Value>) {
        let live: Vec<(f32, f32)> = self
            .tanks
            .iter()
            .filter(|t| t.seat != seat && t.rolling())
            .map(|t| (t.x, t.z))
            .collect();
        let spot = spawns()
            .into_iter()
            .max_by(|a, b| {
                let score = |p: (f32, f32)| {
                    live.iter()
                        .map(|o| (o.0 - p.0).powi(2) + (o.1 - p.1).powi(2))
                        .fold(f32::INFINITY, f32::min)
                };
                score(*a)
                    .partial_cmp(&score(*b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or((0.0, 0.0));
        if let Some(tank) = self.tanks.iter_mut().find(|t| t.seat == seat) {
            tank.x = spot.0;
            tank.z = spot.1;
            tank.hull = spot.0.atan2(spot.1);
            tank.turret = tank.hull;
            tank.wrecked_for = 0.0;
            tank.reload = 0.0;
        }
        events.push(json!({ "kind": "rollout", "seat": seat }));
    }

    fn settle(&mut self) {
        if self.placements.is_some() || self.time < ROUND_SECONDS {
            return;
        }
        let mut rows: Vec<(u32, i64, u8)> = self
            .tanks
            .iter()
            .map(|t| (t.kills, -(t.deaths as i64), t.seat))
            .collect();
        rows.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)));
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut last: Option<(u32, i64)> = None;
        for (kills, deaths, seat) in rows {
            match (last, groups.last_mut()) {
                (Some(prev), Some(group)) if prev == (kills, deaths) => group.push(seat),
                _ => groups.push(vec![seat]),
            }
            last = Some((kills, deaths));
        }
        self.placements = Some(groups);
    }
}

impl Bout for TankClash {
    fn key(&self) -> &'static str {
        "tank"
    }

    fn setup(&self) -> Value {
        json!({
            "game": "tank",
            "rule": "Drive with the keys, aim with the mouse, and bounce shells off the walls.",
            "half": YARD_HALF,
            "radius": TANK_RADIUS,
            "crates": self.crates.iter().map(|block| json!({
                "x": block.x, "z": block.z, "half": block.half, "breakable": block.breakable,
            })).collect::<Vec<_>>(),
        })
    }

    fn step(&mut self, inputs: &[(u8, Input)], dt: f32, events: &mut Vec<Value>) {
        if self.placements.is_some() {
            return;
        }
        self.time += dt;

        for index in 0..self.tanks.len() {
            let seat = self.tanks[index].seat;
            let input = inputs
                .iter()
                .find(|(s, _)| *s == seat)
                .map(|(_, input)| *input)
                .unwrap_or_default();
            let tank = &mut self.tanks[index];
            tank.reload = (tank.reload - dt).max(0.0);
            if !tank.rolling() {
                tank.wrecked_for -= dt;
                if tank.wrecked_for <= 0.0 {
                    self.respawn(seat, events);
                }
                continue;
            }
            self.drive(index, input, dt);
            if input.fire {
                self.fire(seat, events);
            }
        }

        self.step_shells(dt, events);
        self.settle();
    }

    fn bot_input(&mut self, seat: u8, dt: f32) -> Input {
        let Some(me) = self.tank(seat).cloned() else {
            return Input::default();
        };
        if !me.rolling() {
            return Input::default();
        }
        let mut brain = self.brains.remove(&seat).unwrap_or_default();
        brain.think_in -= dt;
        if brain.think_in <= 0.0 {
            brain.think_in = self.rng.random_range(0.4..1.0);
            brain.aim_error = self.rng.random_range(-0.12..0.12);
            brain.circle = [-1.0, 1.0][self.rng.random_range(0..2)];
        }
        let mark = self
            .tanks
            .iter()
            .filter(|t| t.seat != seat && t.rolling())
            .min_by(|a, b| {
                let near = |t: &Tank| (t.x - me.x).powi(2) + (t.z - me.z).powi(2);
                near(a)
                    .partial_cmp(&near(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned();
        let mut input = Input {
            yaw: me.turret,
            ..Default::default()
        };
        if let Some(mark) = mark {
            let (dx, dz) = (mark.x - me.x, mark.z - me.z);
            let distance = (dx * dx + dz * dz).sqrt().max(0.01);
            let want = (-dx).atan2(-dz) + brain.aim_error;
            input.yaw = me.turret + wrap_angle(want - me.turret) * (5.0 * dt).min(1.0);
            let sighted = self.clear_shot((me.x, me.z), (mark.x, mark.z));
            let (fx, fz) = muzzle(input.yaw);
            let lined_up = (fx * dx + fz * dz) / distance;
            input.fire = sighted && lined_up > 0.985 && me.reload <= 0.0;
            // Keep the range open and circle rather than ramming.
            let toward = if distance > 15.0 {
                1.0
            } else if distance < 7.0 {
                -0.8
            } else {
                0.0
            };
            let (tx, tz) = (dx / distance, dz / distance);
            input.stick = Stick {
                x: tx * toward - tz * brain.circle * 0.7,
                z: tz * toward + tx * brain.circle * 0.7,
            }
            .clamped();
        } else {
            input.stick = Stick { x: 0.0, z: 0.0 };
        }
        self.brains.insert(seat, brain);
        input
    }

    fn snapshot(&self) -> Value {
        let round = |v: f32| (v * 100.0).round() / 100.0;
        json!({
            "game": "tank",
            "time": round(self.time),
            "limit": ROUND_SECONDS,
            "players": self.tanks.iter().map(|t| json!({
                "seat": t.seat,
                "x": round(t.x),
                "z": round(t.z),
                "hull": round(t.hull),
                "turret": round(t.turret),
                "kills": t.kills,
                "deaths": t.deaths,
                "out": !t.rolling(),
                "back_in": round(t.wrecked_for.max(0.0)),
                "reload": round(t.reload.max(0.0)),
            })).collect::<Vec<_>>(),
            "shells": self.shells.iter().map(|s| json!({
                "id": s.id,
                "x": round(s.x),
                "z": round(s.z),
                "vx": round(s.vx),
                "vz": round(s.vz),
                "by": s.by,
            })).collect::<Vec<_>>(),
            "crates": self.crates.iter().map(|c| json!({
                "x": c.x, "z": c.z, "half": c.half, "solid": c.solid,
            })).collect::<Vec<_>>(),
        })
    }

    fn placements(&self) -> Option<Vec<Vec<u8>>> {
        self.placements.clone()
    }
}
