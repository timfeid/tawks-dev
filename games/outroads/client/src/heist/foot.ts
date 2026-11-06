/** Walking, running and crouching on foot: speeds, turning, and bumping off walls. */

import type { Stance } from './protocol';

export type FootState = { x: number; z: number; y: number; h: number; s: number };
export type FootInput = { forward: number; right: number; run: boolean; crouch: boolean; facing: number };
export type FootEnv = {
  walls(x: number, z: number, r: number, visit: (x1: number, z1: number, x2: number, z2: number) => void): void;
  ground(x: number, z: number): number;
};

export const SPEED: Record<Stance, number> = { walk: 1.7, run: 5.2, crouch: 0.9 };
const RADIUS = 0.35;

export function stance(input: FootInput): Stance {
  return input.crouch ? 'crouch' : input.run ? 'run' : 'walk';
}

/** Moves relative to `facing` (the camera's heading), turns toward travel, and stays out of walls. */
export function stepFoot(f: FootState, input: FootInput, dt: number, env: FootEnv): FootState {
  const len = Math.hypot(input.forward, input.right);
  const top = SPEED[stance(input)];
  const want = len > 0.05 ? top * Math.min(1, len) : 0;
  const s = f.s + (want - f.s) * Math.min(1, dt * 10);
  let { x, z, h } = f;
  if (len > 0.05) {
    const fx = Math.sin(input.facing);
    const fz = -Math.cos(input.facing);
    const rx = Math.cos(input.facing);
    const rz = Math.sin(input.facing);
    const dx = (fx * input.forward + rx * input.right) / len;
    const dz = (fz * input.forward + rz * input.right) / len;
    const target = Math.atan2(dx, -dz);
    const dh = Math.atan2(Math.sin(target - h), Math.cos(target - h));
    h += dh * Math.min(1, dt * 12);
    x += dx * s * dt;
    z += dz * s * dt;
  }
  let pushX = 0;
  let pushZ = 0;
  env.walls(x, z, RADIUS + 0.5, (x1, z1, x2, z2) => {
    const ex = x2 - x1;
    const ez = z2 - z1;
    const l2 = ex * ex + ez * ez;
    const t = l2 > 0 ? Math.max(0, Math.min(1, ((x + pushX - x1) * ex + (z + pushZ - z1) * ez) / l2)) : 0;
    const ox = x + pushX - (x1 + ex * t);
    const oz = z + pushZ - (z1 + ez * t);
    const d = Math.hypot(ox, oz);
    if (d < RADIUS && d > 1e-6) {
      pushX += (ox / d) * (RADIUS - d);
      pushZ += (oz / d) * (RADIUS - d);
    }
  });
  x += pushX;
  z += pushZ;
  return { x, z, y: env.ground(x, z), h, s };
}
