import { test } from 'node:test';
import assert from 'node:assert/strict';
import { figureEight, LOOP, TraceCursor, strugglePhase, inBand } from '../src/party/city/arrest.ts';

// Mirrors the server judge in city_arrest.rs: progress follows the nearest
// curve point ahead of the guide while the cursor stays within tolerance.
function judge(t: number, cursor: [number, number], dt: number) {
  let best = t, distance = Infinity;
  for (let s = Math.max(0, t - 0.25); s <= t + 0.7; s += 0.02) {
    const [x, y] = figureEight(s);
    const d = Math.hypot(x - cursor[0], y - cursor[1]);
    if (d < distance) { distance = d; best = s; }
  }
  if (distance <= 0.24) return { t: Math.max(t, Math.min(best, t + (LOOP / 1.5) * dt)), miss: false };
  return { t, miss: distance > 0.42 };
}

test('eight-way keys held roughly along the curve can finish the loop without a miss', () => {
  const cursor = new TraceCursor();
  let t = 0, misses = 0;
  const dt = 1 / 60;
  for (let frame = 0; frame < 60 * 8 && t < LOOP; frame++) {
    const [x0, y0] = figureEight(t), [x1, y1] = figureEight(t + 0.05);
    const angle = Math.round(Math.atan2(y1 - y0, x1 - x0) / (Math.PI / 4)) * (Math.PI / 4);
    cursor.keys(Math.round(Math.cos(angle)), Math.round(Math.sin(angle)), dt, t);
    const result = judge(t, [cursor.x, cursor.y], dt);
    t = result.t;
    if (result.miss) misses++;
  }
  assert.ok(t >= LOOP, `reached ${t.toFixed(2)} of ${LOOP.toFixed(2)}`);
  assert.equal(misses, 0);
});

test('keys pointing away from the line do not follow it', () => {
  const cursor = new TraceCursor();
  for (let frame = 0; frame < 30; frame++) cursor.keys(0, -1, 1 / 60, 0);
  assert.ok(judge(0, [cursor.x, cursor.y], 1 / 60).miss);
});

test('the struggle ring is extrapolated from the last server clock', () => {
  const arrest = { clock: 0.5, period: 1.1, window: 0.12 } as Parameters<typeof strugglePhase>[0];
  assert.ok(inBand(arrest, strugglePhase(arrest, 0.05)));
  assert.ok(!inBand(arrest, strugglePhase(arrest, 0.5)));
});
