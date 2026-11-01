import { test } from 'node:test';
import assert from 'node:assert/strict';
import { headingOf, turnToward } from '../src/party/facing.ts';
import { SnapshotClock } from '../src/party/snapshot-clock.ts';

const forward = (yaw: number) => ({ x: Math.sin(yaw), z: Math.cos(yaw) });

test('a character turned by its heading faces the way it travels', () => {
  for (const [vx, vz] of [[0, 1], [1, 0], [0, -1], [-3, 2], [0.4, -5]]) {
    const face = forward(headingOf(vx, vz));
    const speed = Math.hypot(vx, vz);
    assert.ok(face.x * vx / speed + face.z * vz / speed > 0.999, `faces along (${vx}, ${vz})`);
  }
});

test('turning eases the short way round and settles on the heading', () => {
  const nearly = Math.PI - 0.1;
  const step = turnToward(nearly, -nearly, 1 / 60);
  assert.ok(step > nearly, 'crosses the back rather than spinning through the front');
  let yaw = 0;
  for (let i = 0; i < 60; i++) yaw = turnToward(yaw, 2, 1 / 60);
  assert.ok(Math.abs(yaw - 2) < 1e-3);
});

function play(rate: number, jitter: number) {
  let seed = 7;
  const random = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
  const arrivals = Array.from({ length: 1200 }, (_, k) => ({ server: k * 0.05, at: (k * 0.05) / rate + 0.03 + random() * jitter }))
    .sort((a, b) => a.at - b.at);
  const clock = new SnapshotClock();
  let newest = -Infinity;
  let next = 0;
  let frames = 0;
  let ahead = 0;
  let previous = -Infinity;
  for (let now = 0; now < 55; now += 1 / 60) {
    while (next < arrivals.length && arrivals[next].at <= now) {
      clock.arrive(arrivals[next].server, arrivals[next].at);
      newest = Math.max(newest, arrivals[next++].server);
    }
    const render = clock.renderTime(now);
    if (render === null || now < 10) continue;
    assert.ok(render >= previous, 'playback never runs backward');
    previous = render;
    frames++;
    if (render > newest) ahead++;
  }
  return ahead / frames;
}

test('playback stays between snapshots when the server clock drifts or packets bunch up', () => {
  assert.equal(play(1, 0.02), 0);
  assert.ok(play(0.95, 0.05) < 0.02, 'a server running slow is followed');
  assert.ok(play(1.05, 0.05) < 0.02, 'and so is one running fast');
  assert.ok(play(1, 0.2) < 0.05, 'uneven arrivals widen the delay');
});

test('a reset starts the clock over', () => {
  const clock = new SnapshotClock();
  clock.arrive(100, 5);
  clock.reset();
  assert.equal(clock.renderTime(6), null);
  clock.arrive(0.05, 6);
  assert.ok(Math.abs(clock.renderTime(6)! - (0.05 - 0.1)) < 1e-9);
});
