import { describe, expect, it } from 'vitest';
import { SPEED, stepFoot, type FootEnv, type FootInput } from './foot';

const open: FootEnv = { walls: () => {}, ground: (x) => x * 0.1 };
const still: FootInput = { forward: 0, right: 0, run: false, crouch: false, facing: 0 };

function walk(input: FootInput, secs: number, env = open) {
  let f = { x: 0, z: 0, y: 0, h: 0, s: 0 };
  for (let t = 0; t < secs; t += 1 / 60) f = stepFoot(f, input, 1 / 60, env);
  return f;
}

describe('on foot', () => {
  it('walks, runs and crouches at their own speeds, relative to the camera', () => {
    const walked = walk({ ...still, forward: 1 }, 2);
    expect(walked.z).toBeLessThan(-SPEED.walk * 1.5);
    const ran = walk({ ...still, forward: 1, run: true }, 2);
    expect(-ran.z).toBeGreaterThan(-walked.z * 2.5);
    const crept = walk({ ...still, forward: 1, crouch: true }, 2);
    expect(-crept.z).toBeLessThan(-walked.z * 0.6);
    const sideways = walk({ ...still, forward: 1, facing: Math.PI / 2 }, 2);
    expect(sideways.x).toBeGreaterThan(2);
    expect(sideways.y).toBeCloseTo(sideways.x * 0.1, 5);
  });

  it('stops against a wall', () => {
    const wall: FootEnv = { ...open, walls: (_x, _z, _r, visit) => visit(-5, -3, 5, -3) };
    const f = walk({ ...still, forward: 1, run: true }, 3, wall);
    expect(f.z).toBeGreaterThan(-3);
    expect(f.z).toBeLessThan(-2.5);
  });
});
