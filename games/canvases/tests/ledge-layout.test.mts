import {test} from 'node:test';
import assert from 'node:assert/strict';
import {ledgeHint, ledgeReach, ledgeSectionAt, ledgeSections} from '../src/party/arena/ledge-layout.ts';

const rings = [
  {inner: 2.8, outer: 5.2, count: 6, turn: 0},
  {inner: 5.2, outer: 7.6, count: 9, turn: 0.37},
  {inner: 7.6, outer: 10, count: 12, turn: 0.74},
];

test('sections are numbered ring by ring, innermost first, as the server numbers them', () => {
  const sections = ledgeSections(rings);
  assert.equal(sections.length, 27);
  assert.deepEqual(sections.map((s) => s.index), sections.map((_, i) => i));
  assert.equal(sections[6].ring, 1);
  for (const section of sections) {
    const mid = (section.from + section.to) / 2;
    const out = (section.inner + section.outer) / 2;
    assert.equal(ledgeSectionAt(rings, 2.8, Math.cos(mid) * out, Math.sin(mid) * out), section.index);
  }
  assert.equal(ledgeSectionAt(rings, 2.8, 1, -1), -1, 'the core');
  assert.equal(ledgeSectionAt(rings, 2.8, 10.2, 0), null, 'over the edge');
  assert.equal(ledgeSectionAt(rings, 1.5, 2, 0), null, 'where the worn core used to be');
});

test('the framed reach closes in only once a whole ring has gone', () => {
  const footing = new Array(27).fill(0);
  assert.equal(ledgeReach(rings, 2.8, footing), 10);
  for (let i = 15; i < 26; i++) footing[i] = 3;
  assert.equal(ledgeReach(rings, 2.8, footing), 10, 'one rim section is still up');
  footing[26] = 2;
  assert.equal(ledgeReach(rings, 2.8, footing), 7.6);
  for (let i = 0; i < 15; i++) footing[i] = 3;
  assert.equal(ledgeReach(rings, 2.8, footing), 2.8);
});

test('the how-to-play line names the keys and buttons the player has bound', () => {
  const keys = ledgeHint(['I', 'J', 'K', 'L'], 'F', null);
  assert.match(keys, /I J K L to move/);
  assert.match(keys, /F or click to shove/);
  assert.match(keys, /brace/);
  const pad = ledgeHint(['W', 'A', 'S', 'D'], 'Space', 'X');
  assert.match(pad, /X to shove/);
  assert.doesNotMatch(pad, /Space/);
});
