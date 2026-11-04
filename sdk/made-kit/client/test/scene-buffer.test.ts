import { test } from 'node:test';
import assert from 'node:assert/strict';
import { applyDelta, blendEntity, diffFrames, SceneBuffer, toFrame, toState } from '../src/scene-buffer.ts';
import type { Entity, FrameDelta, SceneFrame } from '../src/contract.ts';

const camera = { half_x: 10, half_z: 10, pitch: 55, x: 0, z: 0, follow: null };
const hud = { objective: 'Stay on', alert: null, timer: null, scores: [], meter: null };
const stage = { water: true, ground: null };
const player = (seat: number, x: number, z: number, extra: Partial<Entity> = {}): Entity => ({ id: `p${seat}`, kind: 'player', seat, x, y: 0, z, r: 0.6, yaw: 0, pose: 'idle', ...extra });
const frame = (n: number, t: number, entities: Entity[], effects: SceneFrame['effects'] = []): SceneFrame => ({ n, t, stage, camera, hud, entities, effects });

test('a diff applied to its base rebuilds the frame', () => {
  const a = frame(0, 0, [player(1, 0, 0), player(2, 3, 0), { id: 'floe', kind: 'disc', x: 0, y: 0, z: 0, r: 12, h: 0.4, color: 'ice' }]);
  const b = frame(1, 0.05, [player(1, 0.5, 0), player(2, 3, 0), { id: 'crack0', kind: 'disc', x: 1, y: 0.41, z: 1, r: 1.2, h: 0.02, color: 'danger' }]);
  const delta = diffFrames(a, b);
  assert.equal(delta.base, 0);
  assert.deepEqual(delta.set.map((e) => e.id), ['p1', 'crack0']);
  assert.deepEqual(delta.del, ['floe']);
  const rebuilt = applyDelta(toState(a), delta);
  assert.ok(rebuilt);
  assert.deepEqual(toFrame(rebuilt).entities.sort((x, y) => x.id.localeCompare(y.id)), [...b.entities].sort((x, y) => x.id.localeCompare(y.id)));
});

test('an unchanged entity is not sent again', () => {
  const a = frame(0, 0, [player(1, 0, 0)]);
  assert.deepEqual(diffFrames(a, frame(1, 0.05, [player(1, 0, 0)])).set, []);
});

test('a whole frame needs no base and drops what came before', () => {
  const before = toState(frame(4, 0.2, [player(1, 0, 0), player(2, 1, 1)]));
  const whole = diffFrames(null, frame(9, 0.45, [player(3, 2, 2)]));
  assert.equal(whole.base, null);
  const state = applyDelta(before, whole)!;
  assert.deepEqual([...state.entities.keys()], ['p3']);
});

test('a diff against the wrong base is refused', () => {
  const base = toState(frame(4, 0.2, [player(1, 0, 0)]));
  const delta: FrameDelta = { ...diffFrames(frame(5, 0.25, [player(1, 0, 0)]), frame(6, 0.3, [player(1, 1, 0)])) };
  assert.equal(applyDelta(base, delta), null);
  assert.equal(applyDelta(null, delta), null);
});

test('the buffer waits for a whole frame after a gap, then carries on', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 0, [player(1, 0, 0)]);
  const f1 = frame(1, 0.05, [player(1, 1, 0)]);
  const f2 = frame(2, 0.1, [player(1, 2, 0)]);
  assert.ok(buffer.push(diffFrames(null, f0)));
  assert.equal(buffer.push(diffFrames(f1, f2)), false);
  assert.ok(buffer.stalled);
  assert.ok(buffer.push(diffFrames(null, f2)));
  assert.equal(buffer.stalled, false);
  assert.equal(buffer.current!.entities.get('p1')!.x, 2);
});

test('sampling between two frames interpolates by entity id', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 0, [player(1, 0, 0), player(2, 4, 4)]);
  const f1 = frame(1, 0.1, [player(2, 6, 4), player(1, 2, -2)]);
  buffer.push(diffFrames(null, f0));
  buffer.push(diffFrames(f0, f1));
  const mid = buffer.sample(0.05)!;
  const byId = new Map(mid.entities.map((e) => [e.id, e]));
  assert.equal(byId.get('p1')!.x, 1);
  assert.equal(byId.get('p1')!.z, -1);
  assert.equal(byId.get('p2')!.x, 5);
  assert.ok(Math.abs(mid.speeds.get('p1')! - Math.hypot(2, 2) / 0.1) < 1e-9);
});

test('sampling past the newest frame holds it, and before the oldest holds that', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 1, [player(1, 0, 0)]);
  const f1 = frame(1, 1.05, [player(1, 1, 0)]);
  buffer.push(diffFrames(null, f0));
  buffer.push(diffFrames(f0, f1));
  assert.equal(buffer.sample(9)!.entities[0].x, 1);
  assert.equal(buffer.sample(0)!.entities[0].x, 0);
});

test('a respawn jumps instead of sliding across the arena', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 0, [player(1, -10, 0)]);
  const f1 = frame(1, 0.05, [player(1, 10, 0)]);
  buffer.push(diffFrames(null, f0));
  buffer.push(diffFrames(f0, f1));
  assert.equal(buffer.sample(0.01)!.entities[0].x, -10);
  assert.equal(buffer.sample(0.04)!.entities[0].x, 10);
});

test('facings turn the short way round; a rolling ball does not wrap', () => {
  const a = player(1, 0, 0, { yaw: Math.PI - 0.1 });
  const b = player(1, 0, 0, { yaw: -Math.PI + 0.1 });
  const half = blendEntity(a, b, 0.5).yaw!;
  assert.ok(Math.abs(Math.abs(half) - Math.PI) < 1e-9);
  const ballA: Entity = { id: 'ball', kind: 'ball', x: 0, y: 0, z: 0, r: 2, yaw: 3 };
  const ballB: Entity = { ...ballA, yaw: 9 };
  assert.equal(blendEntity(ballA, ballB, 0.5).yaw, 6);
});

test('poses, colours and hidden snap to the newer frame', () => {
  const a = player(1, 0, 0, { pose: 'ride' });
  const b = player(1, 1, 0, { pose: 'fall', hidden: true });
  const out = blendEntity(a, b, 0.2);
  assert.equal(out.pose, 'fall');
  assert.equal(out.hidden, true);
  assert.equal(out.x, 0.2);
});

test('effects are handed out once, when the drawn moment reaches their frame', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 0, [player(1, 0, 0)]);
  const f1 = frame(1, 0.05, [player(1, 0, 0)], [{ kind: 'sound', name: 'splash' }]);
  buffer.push(diffFrames(null, f0));
  buffer.push(diffFrames(f0, f1));
  assert.deepEqual(buffer.takeEffects(0.02), []);
  assert.deepEqual(buffer.takeEffects(0.06), [{ kind: 'sound', name: 'splash' }]);
  assert.deepEqual(buffer.takeEffects(0.1), []);
});

test('a new round with its clock back at zero starts the buffer over', () => {
  const buffer = new SceneBuffer();
  const f0 = frame(0, 30, [player(1, 5, 0)]);
  buffer.push(diffFrames(null, f0));
  buffer.push(diffFrames(null, frame(0, 0, [player(1, -5, 0)])));
  assert.equal(buffer.sample(0)!.entities[0].x, -5);
  assert.equal(buffer.sample(30)!.entities[0].x, -5);
});

test('the renderer rebuilds and diffs frames the way the kit core does', async () => {
  const { readFile } = await import('node:fs/promises');
  const all = JSON.parse(await readFile(new URL('../../test/fixtures/frames.json', import.meta.url), 'utf8'));
  const delta = all.pop() as FrameDelta;
  const frames = all as SceneFrame[];
  const mine = diffFrames(frames[0], frames[1]);
  assert.deepEqual({ ...mine, del: [...mine.del].sort() }, { ...delta, del: [...delta.del].sort() });
  const rebuilt = applyDelta(toState(frames[0]), delta)!;
  assert.equal(rebuilt.entities.size, frames[1].entities.length);
  for (const entity of frames[1].entities) assert.deepEqual(rebuilt.entities.get(entity.id), entity);
});
