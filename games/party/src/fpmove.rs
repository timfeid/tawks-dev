//! Movement and sight lines shared by the first person minigames: running and
//! jumping around a floor of blocks, and the ray tests they need for shots and
//! for working out who can see whom.

use crate::bout::Input;

pub const PLAYER_RADIUS: f32 = 0.45;
pub const PLAYER_HEIGHT: f32 = 1.7;
pub const EYE_HEIGHT: f32 = 1.45;

pub const MAX_SPEED: f32 = 7.0;
/// Crouching: slower, shorter, and quiet enough that nobody hears you coming.
pub const CROUCH_SPEED: f32 = 0.42;
pub const CROUCH_EYE: f32 = 0.95;
pub const CROUCH_HEIGHT: f32 = 1.15;
pub const ACCEL: f32 = 60.0;
pub const FRICTION: f32 = 8.0;
pub const AIR_CONTROL: f32 = 0.35;
pub const JUMP_SPEED: f32 = 5.4;
pub const GRAVITY: f32 = 16.0;
const STEP_FORGIVENESS: f32 = 0.2;

/// A box of cover, wall or platform. Most rest on the floor; one with a
/// `bottom` above zero floats, like a bridge you can walk under or a canopy.
#[derive(Clone, Copy, Debug, Default)]
pub struct Block {
    pub x: f32,
    pub z: f32,
    pub hx: f32,
    pub hz: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Block {
    pub fn floating(x: f32, z: f32, hx: f32, hz: f32, bottom: f32, top: f32) -> Self {
        Self { x, z, hx, hz, top, bottom }
    }

    pub fn contains_xz(&self, x: f32, z: f32, margin: f32) -> bool {
        (x - self.x).abs() <= self.hx + margin && (z - self.z).abs() <= self.hz + margin
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Body {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub on_ground: bool,
}

/// How tall a player stands, and where their eyes sit.
pub fn stance(crouching: bool) -> (f32, f32) {
    if crouching {
        (CROUCH_HEIGHT, CROUCH_EYE)
    } else {
        (PLAYER_HEIGHT, EYE_HEIGHT)
    }
}

/// Where a player is looking, as a unit vector.
pub fn look(yaw: f32, pitch: f32) -> (f32, f32, f32) {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    (-sy * cp, sp, -cy * cp)
}

pub fn step_body(
    body: &mut Body,
    input: Input,
    dt: f32,
    blocks: &[Block],
    half_x: f32,
    half_z: f32,
) {
    let stick = input.stick.clamped();
    let (sy, cy) = input.yaw.sin_cos();
    // Screen up is forward, and the stick's x strafes.
    let wish_x = -sy * -stick.z + cy * stick.x;
    let wish_z = -cy * -stick.z - sy * stick.x;
    let wish_len = (wish_x * wish_x + wish_z * wish_z).sqrt();
    let control = if body.on_ground { 1.0 } else { AIR_CONTROL };
    let top_speed = if input.crouch && body.on_ground {
        MAX_SPEED * CROUCH_SPEED
    } else {
        MAX_SPEED
    };
    if wish_len > 1e-3 {
        let (wx, wz) = (wish_x / wish_len, wish_z / wish_len);
        let speed_along = body.vx * wx + body.vz * wz;
        let add = (top_speed * wish_len - speed_along).max(0.0);
        let step = (ACCEL * control * dt).min(add);
        body.vx += wx * step;
        body.vz += wz * step;
    }
    if body.on_ground {
        let speed = (body.vx * body.vx + body.vz * body.vz).sqrt();
        if speed > 0.0 {
            let drop = speed * FRICTION * dt;
            let keep = (speed - drop).max(0.0) / speed;
            body.vx *= keep;
            body.vz *= keep;
        }
        if input.jump && !input.crouch {
            body.vy = JUMP_SPEED;
            body.on_ground = false;
        }
    }

    body.vy -= GRAVITY * dt;
    let feet_before = body.y;
    let (height, _) = stance(input.crouch);
    body.x += body.vx * dt;
    body.z += body.vz * dt;
    body.y += body.vy * dt;

    // Land on the floor, or on whatever block is under your feet. Anything
    // under your full width holds you up, the same width the push below uses,
    // so you walk up stairs instead of being shoved off their edge.
    let mut floor = 0.0f32;
    for block in blocks {
        if block.contains_xz(body.x, body.z, PLAYER_RADIUS)
            && feet_before + 1e-3 >= block.top - STEP_FORGIVENESS
            && block.top > floor
        {
            floor = block.top;
        }
    }
    if body.y <= floor {
        body.y = floor;
        body.vy = 0.0;
        body.on_ground = true;
    } else {
        body.on_ground = false;
    }

    // Bump your head on the underside of a bridge or canopy.
    for block in blocks {
        if block.bottom > 0.0
            && body.vy > 0.0
            && block.contains_xz(body.x, body.z, PLAYER_RADIUS * 0.5)
            && feet_before + height <= block.bottom + 0.05
            && body.y + height > block.bottom
        {
            body.y = block.bottom - height;
            body.vy = 0.0;
        }
    }

    // Push out of anything the body overlaps, shortest way first.
    for block in blocks {
        if body.y >= block.top - 0.01 || body.y + height <= block.bottom {
            continue;
        }
        let nearest_x = body.x.clamp(block.x - block.hx, block.x + block.hx);
        let nearest_z = body.z.clamp(block.z - block.hz, block.z + block.hz);
        let (dx, dz) = (body.x - nearest_x, body.z - nearest_z);
        let dist = (dx * dx + dz * dz).sqrt();
        if dist > PLAYER_RADIUS {
            continue;
        }
        if dist > 1e-4 {
            let push = PLAYER_RADIUS - dist;
            body.x += dx / dist * push;
            body.z += dz / dist * push;
            let into = body.vx * (dx / dist) + body.vz * (dz / dist);
            if into < 0.0 {
                body.vx -= dx / dist * into;
                body.vz -= dz / dist * into;
            }
        } else {
            // Dead centre: leave by the closest face.
            let left = (body.x - (block.x - block.hx)).abs();
            let right = ((block.x + block.hx) - body.x).abs();
            let back = (body.z - (block.z - block.hz)).abs();
            let front = ((block.z + block.hz) - body.z).abs();
            let least = left.min(right).min(back).min(front);
            if least == left {
                body.x = block.x - block.hx - PLAYER_RADIUS;
            } else if least == right {
                body.x = block.x + block.hx + PLAYER_RADIUS;
            } else if least == back {
                body.z = block.z - block.hz - PLAYER_RADIUS;
            } else {
                body.z = block.z + block.hz + PLAYER_RADIUS;
            }
        }
    }

    let bound_x = half_x - PLAYER_RADIUS;
    let bound_z = half_z - PLAYER_RADIUS;
    if body.x.abs() > bound_x {
        body.x = body.x.clamp(-bound_x, bound_x);
        body.vx = 0.0;
    }
    if body.z.abs() > bound_z {
        body.z = body.z.clamp(-bound_z, bound_z);
        body.vz = 0.0;
    }
}

/// Distance along the ray to an axis-aligned box, if it hits one.
pub fn ray_box(
    origin: (f32, f32, f32),
    dir: (f32, f32, f32),
    min: (f32, f32, f32),
    max: (f32, f32, f32),
) -> Option<f32> {
    let axes = [
        (origin.0, dir.0, min.0, max.0),
        (origin.1, dir.1, min.1, max.1),
        (origin.2, dir.2, min.2, max.2),
    ];
    let mut near = 0.0f32;
    let mut far = f32::INFINITY;
    for (o, d, lo, hi) in axes {
        if d.abs() < 1e-6 {
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let (mut t0, mut t1) = ((lo - o) / d, (hi - o) / d);
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        near = near.max(t0);
        far = far.min(t1);
        if near > far {
            return None;
        }
    }
    if far < 0.0 { None } else { Some(near) }
}

/// How far a ray travels before a block, a wall or the floor stops it.
pub fn clear_distance(
    origin: (f32, f32, f32),
    dir: (f32, f32, f32),
    blocks: &[Block],
    half_x: f32,
    half_z: f32,
    range: f32,
) -> f32 {
    let mut nearest = range;
    for block in blocks {
        if let Some(t) = ray_box(
            origin,
            dir,
            (block.x - block.hx, block.bottom, block.z - block.hz),
            (block.x + block.hx, block.top, block.z + block.hz),
        ) {
            nearest = nearest.min(t);
        }
    }
    for (o, d, half) in [(origin.0, dir.0, half_x), (origin.2, dir.2, half_z)] {
        if d.abs() > 1e-4 {
            let edge = if d > 0.0 { half } else { -half };
            let t = (edge - o) / d;
            if t > 0.0 {
                nearest = nearest.min(t);
            }
        }
    }
    if dir.1.abs() > 1e-4 {
        let ceiling = if dir.1 > 0.0 { 8.0 } else { 0.0 };
        let t = (ceiling - origin.1) / dir.1;
        if t > 0.0 {
            nearest = nearest.min(t);
        }
    }
    nearest
}

/// Distance along the ray to a standing player, treated as a vertical cylinder.
pub fn ray_player(
    origin: (f32, f32, f32),
    dir: (f32, f32, f32),
    at: (f32, f32, f32),
    radius: f32,
    height: f32,
) -> Option<f32> {
    let (ox, oz) = (origin.0 - at.0, origin.2 - at.2);
    let a = dir.0 * dir.0 + dir.2 * dir.2;
    if a < 1e-6 {
        return None;
    }
    let b = 2.0 * (ox * dir.0 + oz * dir.2);
    let c = ox * ox + oz * oz - radius * radius;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return None;
    }
    let root = disc.sqrt();
    for t in [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)] {
        if t < 0.0 {
            continue;
        }
        let y = origin.1 + dir.1 * t;
        if y >= at.1 && y <= at.1 + height {
            return Some(t);
        }
    }
    None
}

/// Whether one player can see another, with blocks in the way counting.
pub fn can_see(
    from: (f32, f32, f32),
    to: (f32, f32, f32),
    blocks: &[Block],
    half_x: f32,
    half_z: f32,
) -> bool {
    let (mut dx, mut dy, mut dz) = (to.0 - from.0, to.1 - from.1, to.2 - from.2);
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if distance < 1e-3 {
        return true;
    }
    dx /= distance;
    dy /= distance;
    dz /= distance;
    clear_distance(from, (dx, dy, dz), blocks, half_x, half_z, distance + 1.0) >= distance - 0.2
}
