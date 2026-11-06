/**
 * Arcade car handling: a kinematic bicycle with grip that lets go under the
 * handbrake, a soft speed cap off road, and circle-against-segment bumps
 * with buildings and other cars. Hills pull the car down their slope, and a
 * crest taken fast enough throws it into the air. Heading 0 faces north (-z)
 * and grows clockwise, so forward is (sin h, -cos h).
 */

import type { RoadHit } from './roads';

export type CarState = {
  x: number;
  z: number;
  /** Height of the car's underside. */
  y: number;
  /** Vertical speed, meters per second. */
  vy: number;
  airborne: boolean;
  h: number;
  vx: number;
  vz: number;
  steer: number;
};

export type Controls = { throttle: number; brake: number; steer: number; handbrake: boolean };

export type Env = {
  road(x: number, z: number): RoadHit | null;
  walls(x: number, z: number, r: number, visit: (x1: number, z1: number, x2: number, z2: number) => void): void;
  /** Height of what a car at height y stands on at a point. */
  ground(x: number, z: number, y: number): number;
  /** Other cars' centres. */
  cars: { x: number; z: number }[];
  assist: boolean;
  /** Damage: grip left, steering pull toward a flat side, and a top speed. */
  mods?: { grip: number; pull: number; top: number };
};

export type StepResult = { impact: number; bumped: boolean; offRoad: boolean; landed: number };

export const TUNING = {
  maxSpeed: 44,
  offRoadSpeed: 20,
  reverseSpeed: 10,
  accel: 13,
  brake: 28,
  drag: 0.02,
  rolling: 0.6,
  wheelbase: 2.7,
  maxSteer: 0.6,
  grip: 14,
  driftGrip: 2.2,
  carRadius: 1.15,
  carSpan: 1.25,
  carGap: 4.2,
  substep: 1 / 120,
  gravity: 13,
  /** The ground has to drop away this much faster than the car is moving down for it to leave the ground. */
  liftOff: 2.6,
};

export function newCar(x: number, z: number, h: number, y = 0): CarState {
  return { x, z, y, vy: 0, airborne: false, h, vx: 0, vz: 0, steer: 0 };
}

export function forwardSpeed(car: CarState): number {
  return car.vx * Math.sin(car.h) - car.vz * Math.cos(car.h);
}

export function wrapAngle(a: number): number {
  return Math.atan2(Math.sin(a), Math.cos(a));
}

/** The heading that drives along a direction (dx, dz). */
export function headingOf(dx: number, dz: number): number {
  return Math.atan2(dx, -dz);
}

function approach(value: number, target: number, rate: number) {
  return value < target ? Math.min(target, value + rate) : Math.max(target, value - rate);
}

function substep(car: CarState, input: Controls, dt: number, env: Env, hit: RoadHit | null, out: StepResult) {
  const t = TUNING;
  const fx = Math.sin(car.h);
  const fz = -Math.cos(car.h);
  const rx = Math.cos(car.h);
  const rz = Math.sin(car.h);
  let vf = car.vx * fx + car.vz * fz;
  let vl = car.vx * rx + car.vz * rz;

  const offRoad = !!hit && hit.dist > hit.half + 1.5;
  out.offRoad = offRoad;
  const mods = env.mods;
  const cap = Math.min(offRoad ? t.offRoadSpeed : t.maxSpeed, mods ? Math.max(0, mods.top) : Infinity);
  const grip = (car.airborne ? 0.15 : 1) * (mods?.grip ?? 1);

  if (!car.airborne) {
    const ahead = env.ground(car.x + fx * t.carSpan, car.z + fz * t.carSpan, car.y + 1);
    const behind = env.ground(car.x - fx * t.carSpan, car.z - fz * t.carSpan, car.y + 1);
    const grade = (ahead - behind) / (2 * t.carSpan);
    vf -= t.gravity * (grade / Math.sqrt(1 + grade * grade)) * dt;
  }

  if (input.throttle > 0 && !car.airborne) {
    if (vf < -0.5) vf += t.brake * input.throttle * dt;
    else if (cap > 0) vf += t.accel * input.throttle * Math.max(0.05, 1 - (Math.max(0, vf) / cap) ** 4) * dt;
  }
  if (input.brake > 0 && !car.airborne) {
    if (vf > 0.5) vf -= t.brake * input.brake * dt;
    else vf -= t.accel * 0.7 * input.brake * Math.max(0, 1 + vf / t.reverseSpeed) * dt;
  }
  const coast = input.throttle === 0 && input.brake === 0;
  const drag = t.drag + (offRoad ? 0.2 : 0) + (coast ? 0.1 : 0);
  vf -= vf * drag * dt;
  vf = approach(vf, 0, t.rolling * dt);
  if (vf > cap) vf = approach(vf, cap, t.brake * 0.4 * dt);
  if (input.handbrake) vf = approach(vf, 0, 6 * dt);

  const steerRate = Math.abs(input.steer) > Math.abs(car.steer) ? 4.5 : 7;
  const pull = mods && Math.abs(vf) > 2 ? mods.pull * 0.4 : 0;
  car.steer = approach(car.steer, Math.max(-1, Math.min(1, input.steer + pull)), steerRate * dt);
  const angle = car.steer * (t.maxSteer / (1 + Math.abs(vf) / 22));
  let yaw = ((vf / t.wheelbase) * Math.tan(angle)) * grip;
  if (input.handbrake) yaw *= 1.35;
  car.h = wrapAngle(car.h + yaw * dt);

  if (env.assist && hit && Math.abs(input.steer) < 0.3 && Math.abs(vf) > 3 && hit.dist < hit.half + 4) {
    let road = headingOf(hit.dx, hit.dz);
    if (Math.cos(road - car.h) < 0) road += Math.PI;
    const diff = wrapAngle(road - car.h);
    if (Math.abs(diff) < 0.6) car.h = wrapAngle(car.h + diff * Math.min(1, 2.5 * dt));
    const keep = hit.half * 0.6;
    if (hit.dist > keep) {
      const pull = Math.min(1, 1.5 * dt) * (hit.dist - keep);
      car.x += ((hit.px - car.x) / hit.dist) * pull;
      car.z += ((hit.pz - car.z) / hit.dist) * pull;
    }
  }

  // Velocity stays in the old heading's frame; the next step sees the turn as
  // sideways slip, which grip then takes out, or the handbrake lets run.
  vl *= Math.exp(-(input.handbrake ? t.driftGrip : t.grip) * grip * dt);
  car.vx = fx * vf + rx * vl;
  car.vz = fz * vf + rz * vl;
  car.x += car.vx * dt;
  car.z += car.vz * dt;

  const floor = env.ground(car.x, car.z, car.y);
  if (car.airborne) {
    car.vy -= t.gravity * dt;
    car.y += car.vy * dt;
    if (car.y <= floor) {
      out.landed = Math.max(out.landed, -car.vy);
      car.y = floor;
      car.vy = 0;
      car.airborne = false;
    }
  } else {
    const follow = (floor - car.y) / dt;
    if (car.vy - follow > t.liftOff && Math.abs(vf) > 8) {
      car.airborne = true;
      car.vy -= t.gravity * dt;
      car.y += car.vy * dt;
    } else {
      car.vy = Math.max(-40, Math.min(40, follow));
      car.y = floor;
    }
  }

  collide(car, env, out);
}

/** Pushes the car's two body circles out of walls and other cars. */
function collide(car: CarState, env: Env, out: StepResult) {
  const t = TUNING;
  const fx = Math.sin(car.h);
  const fz = -Math.cos(car.h);
  for (const along of [t.carSpan, -t.carSpan]) {
    const cx = car.x + fx * along;
    const cz = car.z + fz * along;
    let pushX = 0;
    let pushZ = 0;
    env.walls(cx, cz, t.carRadius + 0.5, (x1, z1, x2, z2) => {
      const dx = x2 - x1;
      const dz = z2 - z1;
      const len2 = dx * dx + dz * dz;
      const u = len2 > 0 ? Math.max(0, Math.min(1, ((cx + pushX - x1) * dx + (cz + pushZ - z1) * dz) / len2)) : 0;
      const px = x1 + dx * u;
      const pz = z1 + dz * u;
      const ox = cx + pushX - px;
      const oz = cz + pushZ - pz;
      const d = Math.hypot(ox, oz);
      if (d >= t.carRadius || d < 1e-6) return;
      const nx = ox / d;
      const nz = oz / d;
      pushX += nx * (t.carRadius - d);
      pushZ += nz * (t.carRadius - d);
      const vn = car.vx * nx + car.vz * nz;
      if (vn < 0) {
        out.impact = Math.max(out.impact, -vn);
        car.vx -= nx * vn * 1.3;
        car.vz -= nz * vn * 1.3;
        car.vx *= 0.96;
        car.vz *= 0.96;
      }
    });
    car.x += pushX;
    car.z += pushZ;
  }
  for (const other of env.cars) {
    const ox = car.x - other.x;
    const oz = car.z - other.z;
    const d = Math.hypot(ox, oz);
    if (d >= t.carGap || d < 1e-6) continue;
    const nx = ox / d;
    const nz = oz / d;
    car.x += nx * (t.carGap - d) * 0.5;
    car.z += nz * (t.carGap - d) * 0.5;
    const vn = car.vx * nx + car.vz * nz;
    if (vn < 0) {
      car.vx -= nx * vn * 1.4;
      car.vz -= nz * vn * 1.4;
    }
    out.bumped = true;
  }
}

export function step(car: CarState, input: Controls, dt: number, env: Env): StepResult {
  const out: StepResult = { impact: 0, bumped: false, offRoad: false, landed: 0 };
  const hit = env.road(car.x, car.z);
  let left = Math.min(dt, 0.1);
  while (left > 1e-6) {
    const h = Math.min(TUNING.substep, left);
    substep(car, input, h, env, hit, out);
    left -= h;
  }
  return out;
}
