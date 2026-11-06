import { describe, expect, it } from 'vitest';
import { TUNING, forwardSpeed, newCar, step, type Controls, type Env } from './physics';
import type { RoadHit } from './roads';

const none: Controls = { throttle: 0, brake: 0, steer: 0, handbrake: false };
const gas: Controls = { ...none, throttle: 1 };

/** A straight road running north along x = 0, as wide as asked. */
function env(over: Partial<Env> = {}, half = 6): Env {
  return {
    road: (x, z): RoadHit => ({ dist: Math.abs(x), px: 0, pz: z, dx: 0, dz: -1, half }),
    walls: () => {},
    cars: [],
    assist: false,
    ...over,
  };
}

function drive(car = newCar(0, 0, 0), controls: Controls, seconds: number, e = env()) {
  for (let t = 0; t < seconds; t += 1 / 60) step(car, controls, 1 / 60, e);
  return car;
}

describe('car physics', () => {
  it('pulls away briskly and tops out at the speed cap', () => {
    const car = drive(undefined, gas, 2);
    expect(forwardSpeed(car)).toBeGreaterThan(15);
    expect(car.z).toBeLessThan(-10);
    drive(car, gas, 30);
    expect(forwardSpeed(car)).toBeLessThanOrEqual(TUNING.maxSpeed + 0.01);
    expect(forwardSpeed(car)).toBeGreaterThan(TUNING.maxSpeed * 0.9);
  });

  it('brakes to a stop and then reverses', () => {
    const car = drive(undefined, gas, 3);
    const before = forwardSpeed(car);
    drive(car, { ...none, brake: 1 }, 1);
    expect(before - forwardSpeed(car)).toBeGreaterThan(24);
    drive(car, { ...none, brake: 1 }, 3);
    expect(forwardSpeed(car)).toBeLessThan(-3);
    expect(forwardSpeed(car)).toBeGreaterThanOrEqual(-TUNING.reverseSpeed - 0.01);
  });

  it('coasts down on its own', () => {
    const car = drive(undefined, gas, 3);
    drive(car, none, 20);
    expect(Math.abs(forwardSpeed(car))).toBeLessThan(0.5);
  });

  it('steers right with positive steer and turns tighter when slow', () => {
    const slow = drive(newCar(0, 0, 0), { ...gas, throttle: 0.3, steer: 1 }, 1.5);
    expect(slow.h).toBeGreaterThan(0.2);
    expect(slow.x).toBeGreaterThan(0);
  });

  it('lets the tail out under the handbrake', () => {
    const grip = drive(undefined, gas, 3);
    const drift = { ...grip };
    drive(grip, { ...none, steer: 1 }, 0.5);
    drive(drift, { ...none, steer: 1, handbrake: true }, 0.5);
    const lateral = (c: typeof grip) => Math.abs(c.vx * Math.cos(c.h) + c.vz * Math.sin(c.h));
    expect(lateral(drift)).toBeGreaterThan(lateral(grip));
  });

  it('runs slower off the road', () => {
    const offRoad = env({ road: (x, z) => ({ dist: 50, px: x - 50, pz: z, dx: 0, dz: -1, half: 5 }) });
    const car = drive(undefined, gas, 20, offRoad);
    expect(forwardSpeed(car)).toBeLessThanOrEqual(TUNING.offRoadSpeed + 0.5);
  });

  it('stops at a wall instead of passing through it', () => {
    const wall = env({ walls: (_x, _z, _r, visit) => visit(-20, -30, 20, -30) });
    const car = drive(undefined, gas, 6, wall);
    expect(car.z).toBeGreaterThan(-30);
    expect(forwardSpeed(car)).toBeLessThan(10);
  });

  it('bumps off another car', () => {
    const car = newCar(0, 0, 0);
    car.vz = -10;
    const result = step(car, none, 1 / 60, env({ cars: [{ x: 0, z: -3 }] }));
    expect(result.bumped).toBe(true);
    expect(car.vz).toBeGreaterThan(-10);
    expect(Math.hypot(car.x, car.z + 3)).toBeGreaterThan(3);
  });

  it('with assist on, settles onto the road heading and back toward it', () => {
    const car = newCar(4, 0, 0.3);
    car.vx = Math.sin(0.3) * 15;
    car.vz = -Math.cos(0.3) * 15;
    drive(car, { ...gas, throttle: 0.4 }, 3, env({ assist: true }, 4));
    expect(Math.abs(car.h)).toBeLessThan(0.05);
    expect(Math.abs(car.x)).toBeLessThan(4);
  });
});
