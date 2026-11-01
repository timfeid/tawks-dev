import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PressSource, padHands, type Hand } from '../src/party/bouts/presses.ts';

const pad = (x: number, pressed: number[] = []) => ({
  axes: [x, 0],
  buttons: Array.from({ length: 16 }, (_, index) => ({ pressed: pressed.includes(index) })),
});

test('each new key press is sent at once and held keys repeat nothing', () => {
  const sent: Hand[] = [];
  const source = new PressSource((hand) => sent.push(hand));
  assert.equal(source.key('left', false), false, 'nothing is sent outside the bout');
  source.active = true;
  source.key('left', false);
  source.key('left', true);
  source.key('right', false);
  source.key('left', false);
  source.key(null, false);
  assert.deepEqual(sent, ['left', 'right', 'left']);
});

test('a controller sends a press on each push, not while held', () => {
  const sent: Hand[] = [];
  const source = new PressSource((hand) => sent.push(hand));
  source.active = true;
  for (const x of [0, -0.9, -0.9, -0.4, 0, 0.9, 0.35, 0.9, 0]) source.poll(pad(x), 0);
  assert.deepEqual(sent, ['left', 'right'], 'the stick lets go only well inside the edge');
  source.poll(pad(0, [4]), 0);
  source.poll(pad(0, [4]), 0);
  source.poll(pad(0, [5, 0]), 0);
  assert.deepEqual(sent, ['left', 'right', 'left', 'right', 'both']);
});

test('the d-pad and bumpers count as sides', () => {
  assert.deepEqual([...padHands(pad(0, [14]), 0, new Set())], ['left']);
  assert.deepEqual([...padHands(pad(0, [15]), 0, new Set())], ['right']);
  assert.deepEqual([...padHands(pad(0.4, []), 0, new Set())], []);
  assert.deepEqual([...padHands(pad(0.4, []), 0, new Set(['right']))], ['right']);
});
