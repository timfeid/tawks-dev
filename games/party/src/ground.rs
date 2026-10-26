//! Footed movement for the top-down party games. The stick points where the
//! camera does, there is no momentum to fight, and players push each other
//! apart rather than bouncing off like the bumper balls do.

use crate::bout::Stick;

pub const RUN_SPEED: f32 = 8.5;
pub const RUNNER_RADIUS: f32 = 0.55;
const ACCEL: f32 = 55.0;
const FRICTION: f32 = 9.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Runner {
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
}

impl Runner {
    pub fn at(x: f32, z: f32) -> Self {
        Runner {
            x,
            z,
            vx: 0.0,
            vz: 0.0,
        }
    }

    pub fn speed(&self) -> f32 {
        (self.vx * self.vx + self.vz * self.vz).sqrt()
    }

    pub fn facing(&self) -> f32 {
        if self.speed() < 0.05 {
            0.0
        } else {
            self.vx.atan2(self.vz)
        }
    }

    pub fn distance_to(&self, x: f32, z: f32) -> f32 {
        ((self.x - x).powi(2) + (self.z - z).powi(2)).sqrt()
    }
}

/// One step of running. `speed` is the top speed for this player right now, so
/// a game can slow someone down or hurry them along.
pub fn step_runner(runner: &mut Runner, stick: Stick, dt: f32, speed: f32) {
    let stick = stick.clamped();
    let push = (stick.x * stick.x + stick.z * stick.z).sqrt();
    if push > 1e-3 {
        let (wx, wz) = (stick.x / push, stick.z / push);
        let along = runner.vx * wx + runner.vz * wz;
        let add = (speed * push - along).max(0.0);
        let step = (ACCEL * dt).min(add);
        runner.vx += wx * step;
        runner.vz += wz * step;
    }
    let moving = runner.speed();
    if moving > 0.0 {
        let drop = moving * FRICTION * dt * if push > 0.1 { 0.35 } else { 1.0 };
        let keep = (moving - drop).max(0.0) / moving;
        runner.vx *= keep;
        runner.vz *= keep;
    }
    runner.x += runner.vx * dt;
    runner.z += runner.vz * dt;
}

/// Keep everyone out of each other's space.
pub fn separate(runners: &mut [Runner]) {
    let reach = RUNNER_RADIUS * 2.0;
    for i in 0..runners.len() {
        for j in (i + 1)..runners.len() {
            let (dx, dz) = (runners[j].x - runners[i].x, runners[j].z - runners[i].z);
            let distance = (dx * dx + dz * dz).sqrt();
            if distance >= reach || distance < 1e-4 {
                continue;
            }
            let push = (reach - distance) / 2.0;
            let (nx, nz) = (dx / distance, dz / distance);
            runners[i].x -= nx * push;
            runners[i].z -= nz * push;
            runners[j].x += nx * push;
            runners[j].z += nz * push;
        }
    }
}

/// Hold a runner inside a circular arena.
pub fn hold_inside(runner: &mut Runner, radius: f32) {
    let out = (runner.x * runner.x + runner.z * runner.z).sqrt();
    let edge = radius - RUNNER_RADIUS;
    if out > edge && out > 1e-4 {
        runner.x = runner.x / out * edge;
        runner.z = runner.z / out * edge;
        runner.vx *= 0.2;
        runner.vz *= 0.2;
    }
}

/// Hold a runner inside a square arena.
pub fn hold_square(runner: &mut Runner, half: f32) {
    let edge = half - RUNNER_RADIUS;
    if runner.x.abs() > edge {
        runner.x = runner.x.clamp(-edge, edge);
        runner.vx = 0.0;
    }
    if runner.z.abs() > edge {
        runner.z = runner.z.clamp(-edge, edge);
        runner.vz = 0.0;
    }
}

/// A stick that walks from one point toward another.
pub fn steer(from: (f32, f32), to: (f32, f32)) -> Stick {
    let (dx, dz) = (to.0 - from.0, to.1 - from.1);
    let length = (dx * dx + dz * dz).sqrt();
    if length < 1e-3 {
        return Stick::default();
    }
    Stick {
        x: dx / length,
        z: dz / length,
    }
}

/// Places evenly spaced around a circle.
pub fn ring(count: usize, radius: f32, turn: f32) -> Vec<(f32, f32)> {
    (0..count)
        .map(|i| {
            let angle = std::f32::consts::TAU * i as f32 / count.max(1) as f32 + turn;
            (angle.cos() * radius, angle.sin() * radius)
        })
        .collect()
}
