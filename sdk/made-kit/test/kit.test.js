import test from 'node:test';
import assert from 'node:assert/strict';
import { drift, ring, rankOuts, rankBy, collide, body, steer, createView, diffFrame, applyFrame, seeded, defineGame } from '../src/core.js';
import { createRunner } from '../src/runner.js';
import thinIce from '../examples/thin-ice.js';

test('ring spaces points on the circle', () => {
  const pts = ring(4, 5);
  assert.equal(pts.length, 4);
  for (const p of pts) assert.ok(Math.abs(Math.hypot(p.x, p.z) - 5) < 1e-9);
});

test('rankOuts puts survivors first and later outs higher, with ties', () => {
  const players = [
    { seat: 1, out: 3 },
    { seat: 2, out: null },
    { seat: 3, out: 7 },
    { seat: 4, out: 3 },
  ];
  assert.deepEqual(rankOuts(players), [[2], [3], [1, 4]]);
  assert.deepEqual(rankBy([{ seat: 1, s: 2 }, { seat: 2, s: 5 }], 's'), [[2], [1]]);
});

test('collide separates overlapping bodies', () => {
  const a = body(0, 0, { r: 1 });
  const b = body(0.5, 0, { r: 1 });
  collide([a, b]);
  assert.ok(b.x - a.x >= 2 - 1e-9);
});

test('steer moves towards the stick and stops without it', () => {
  const b = body(0, 0);
  for (let i = 0; i < 60; i++) steer(b, { x: 1, z: 0 }, 1 / 60, { speed: 5, accel: 50 });
  assert.ok(Math.abs(b.vx - 5) < 1e-9);
  for (let i = 0; i < 120; i++) steer(b, null, 1 / 60, { speed: 5, accel: 50 });
  assert.equal(b.vx, 0);
});

test('the view refuses unknown colors and duplicate ids', () => {
  const v = createView();
  assert.throws(() => v.disc('a', 0, 0, 1, { color: 'chartreuse' }), /unknown color/);
  v.disc('a', 0, 0, 1);
  assert.throws(() => v.disc('a', 0, 0, 1), /used twice/);
  assert.throws(() => v.box('b', NaN, 0, 1, 1), /finite/);
});

test('a delta rebuilds the next frame', () => {
  const a = createView({ n: 1 });
  a.disc('x', 0, 0, 1);
  a.disc('y', 1, 1, 1);
  const f1 = a.build();
  const b = createView({ n: 2 });
  b.disc('x', 0, 0, 1);
  b.disc('z', 2, 2, 1);
  const f2 = b.build();
  const d = diffFrame(f1, f2);
  assert.deepEqual(d.del, ['y']);
  assert.deepEqual(d.set.map((e) => e.id), ['z']);
  const rebuilt = applyFrame(f1, d);
  assert.deepEqual(rebuilt.entities.map((e) => e.id).sort(), ['x', 'z']);
  assert.equal(applyFrame({ ...f1, n: 9 }, d), null);
});

test('seeded random is repeatable', () => {
  const a = seeded(5);
  const b = seeded(5);
  for (let i = 0; i < 10; i++) assert.equal(a(), b());
});

test('defineGame checks the shape', () => {
  assert.throws(() => defineGame({ meta: { name: 'x' } }), /meta.rules/);
});

test('thin ice finishes with bots at every seat count', () => {
  for (let n = 2; n <= 8; n++) {
    const seats = Array.from({ length: n }, (_, i) => ({ seat: i + 1, name: `Bot ${i + 1}`, bot: true, skill: 0.7 }));
    const run = createRunner(thinIce, { seats, seed: n });
    let placements = null;
    for (let i = 0; i < 150 * 60 && !placements; i++) {
      placements = run.step();
      if (i % 3 === 0) run.frame();
    }
    assert.ok(placements, `no end at ${n} seats`);
    assert.deepEqual(placements.flat().sort((a, b) => a - b), seats.map((s) => s.seat));
  }
});

test('the sample frames the Rust side checks are current', async () => {
  const fs = await import('node:fs');
  const path = new URL('./fixtures/frames.json', import.meta.url);
  const seats = [1, 2, 3, 4].map((seat) => ({ seat, name: `Seat ${seat}`, bot: true, skill: 0.7 }));
  const run = createRunner(thinIce, { seats, seed: 7 });
  const frames = [];
  for (let i = 0; i < 60 * 6; i++) {
    run.step();
    if (i % 60 === 0) frames.push(run.frame());
  }
  const v = createView({ n: 99, t: 1 });
  v.ground('snow');
  v.camera.follow(2, { half: 6 });
  v.box('wall', 1, 2, 3, 0.5, { color: 'stone', h: 2, yaw: 0.5 });
  v.ball('snowball', 0, 0, 2, { spin: 1.5, seat: 1, color: 'seat' });
  v.marker('goal', 3, 3, 1, { color: 'gold' });
  v.player(2, { x: 1, z: 1, r: 0.6, yaw: 0 }, { pose: 'cheer' });
  v.hud({ objective: 'All of it', alert: 'Now', timer: 12.5, scores: [{ seat: 2, score: 3, label: 'Ice' }], meter: { label: 'Power', value: 0.5 } });
  v.sound('win');
  v.burst(1, 1, 'gold');
  frames.push(v.build());
  frames.push(diffFrame(frames[0], frames[1]));
  const json = JSON.stringify(frames, null, 1) + '\n';
  if (process.env.MADE_WRITE_FIXTURE) fs.writeFileSync(path, json);
  assert.equal(fs.readFileSync(path, 'utf8'), json, 'run MADE_WRITE_FIXTURE=1 node --test test/');
});

test('meta may say the goal, how to win and lose, and which inputs it ignores', () => {
  const base = { ...thinIce, meta: { ...thinIce.meta } };
  assert.doesNotThrow(() => defineGame({ ...base, meta: { ...base.meta, unused: ['up', 'down'] } }));
  assert.throws(() => defineGame({ ...base, meta: { ...base.meta, unused: ['jump'] } }), /meta.unused/);
  assert.throws(() => defineGame({ ...base, meta: { ...base.meta, goal: '' } }), /meta.goal/);
  assert.throws(() => defineGame({ ...base, meta: { ...base.meta, win: 'x'.repeat(91) } }), /meta.win/);
});

test('the runner notes when each bot first pressed a button', () => {
  const seats = [1, 2, 3].map((seat) => ({ seat, name: `Seat ${seat}`, bot: seat > 1, skill: 1 }));
  const run = createRunner(thinIce, { seats, seed: 3 });
  for (let i = 0; i < 60 * 15; i++) run.step();
  const { acted } = run.stats();
  assert.equal(acted[1], undefined);
  assert.ok(Object.keys(acted).length >= 1);
});

test('drift turns right clockwise on screen and slides wide', () => {
  const b = body(0, 0, { yaw: Math.PI });
  for (let i = 0; i < 120; i++) drift(b, { x: 0, z: -1 }, 1 / 60);
  assert.ok(b.vz < -5, 'up drives along the heading');
  for (let i = 0; i < 30; i++) drift(b, { x: 1, z: -1 }, 1 / 60);
  assert.ok(b.yaw < Math.PI - 0.5, 'right lowers the yaw');
  const heading = { x: Math.sin(b.yaw), z: Math.cos(b.yaw) };
  const speed = Math.hypot(b.vx, b.vz);
  assert.ok((b.vx * heading.x + b.vz * heading.z) / speed < 0.97, 'the velocity lags the heading');
});
