//! Footed movement for the top-down party games. The stick points where the
//! camera does, there is no momentum to fight, and players push each other
//! apart rather than bouncing off like the bumper balls do.

use crate::bout::Stick;

pub const RUN_SPEED: f32 = 8.5;
pub const RUNNER_RADIUS: f32 = 0.55;
const ACCEL: f32 = 55.0;
const FRICTION: f32 = 9.0;

#[derive(Clone, Copy, Debug)]
pub struct Runner {
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub width: f32,
}

impl Default for Runner {
    fn default() -> Self { Self::at(0.0, 0.0) }
}

impl Runner {
    pub fn at(x: f32, z: f32) -> Self {
        Runner {
            x,
            z,
            vx: 0.0,
            vz: 0.0,
            width: 1.0,
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = crate::bout::BodyProfile::new(width).width;
        self
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
    let body = crate::bout::BodyProfile::new(runner.width);
    let speed = speed * body.speed;
    let stick = stick.clamped();
    let push = (stick.x * stick.x + stick.z * stick.z).sqrt();
    if push > 1e-3 {
        let (wx, wz) = (stick.x / push, stick.z / push);
        let along = runner.vx * wx + runner.vz * wz;
        let add = (speed * push - along).max(0.0);
        let step = (ACCEL * body.acceleration * dt).min(add);
        runner.vx += wx * step;
        runner.vz += wz * step;
    }
    let moving = runner.speed();
    if moving > 0.0 {
        let drop = moving * FRICTION * dt * (2.0 - body.mass).clamp(0.7, 1.2)
            * if push > 0.1 { 0.35 } else { 1.0 };
        let keep = (moving - drop).max(0.0) / moving;
        runner.vx *= keep;
        runner.vz *= keep;
    }
    runner.x += runner.vx * dt;
    runner.z += runner.vz * dt;
}

/// Keep everyone out of each other's space.
pub fn separate(runners: &mut [Runner]) {
    for i in 0..runners.len() {
        for j in (i + 1)..runners.len() {
            let a = crate::bout::BodyProfile::new(runners[i].width);
            let b = crate::bout::BodyProfile::new(runners[j].width);
            let reach = RUNNER_RADIUS * (a.radius + b.radius);
            let (dx, dz) = (runners[j].x - runners[i].x, runners[j].z - runners[i].z);
            let distance = (dx * dx + dz * dz).sqrt();
            if distance >= reach || distance < 1e-4 {
                continue;
            }
            let push = reach - distance;
            let (nx, nz) = (dx / distance, dz / distance);
            let a_share = b.mass / (a.mass + b.mass);
            let b_share = a.mass / (a.mass + b.mass);
            runners[i].x -= nx * push * a_share;
            runners[i].z -= nz * push * a_share;
            runners[j].x += nx * push * b_share;
            runners[j].z += nz * push * b_share;
        }
    }
}

/// Hold a runner inside a circular arena.
pub fn hold_inside(runner: &mut Runner, radius: f32) {
    let out = (runner.x * runner.x + runner.z * runner.z).sqrt();
    let edge = radius - RUNNER_RADIUS * crate::bout::BodyProfile::new(runner.width).radius;
    if out > edge && out > 1e-4 {
        runner.x = runner.x / out * edge;
        runner.z = runner.z / out * edge;
        runner.vx *= 0.2;
        runner.vz *= 0.2;
    }
}

/// Hold a runner inside a square arena.
pub fn hold_square(runner: &mut Runner, half: f32) {
    let edge = half - RUNNER_RADIUS * crate::bout::BodyProfile::new(runner.width).radius;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_runners_gain_speed_while_broad_runners_hold_more_space() {
        let mut narrow = Runner::at(0.0, 0.0).with_width(0.7);
        let mut broad = Runner::at(0.0, 0.0).with_width(1.4);
        for _ in 0..60 {
            step_runner(&mut narrow, Stick { x: 1.0, z: 0.0 }, 1.0 / 60.0, RUN_SPEED);
            step_runner(&mut broad, Stick { x: 1.0, z: 0.0 }, 1.0 / 60.0, RUN_SPEED);
        }
        assert!(narrow.x > broad.x * 1.08);
        assert!(broad.width > narrow.width);
        let mut pair = [Runner::at(0.0, 0.0).with_width(1.4),
            Runner::at(0.8, 0.0).with_width(0.7)];
        separate(&mut pair);
        assert!(pair[0].x.abs() < (pair[1].x - 0.8).abs(),
            "the broad runner should yield less space in contact");
    }
}
