import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { bracket, nextRound, rankBy, roundRobin, snakeOrder } from './compete.ts';

describe('roundRobin', () => {
  it('has everyone play once a week and everyone meet in a cycle', () => {
    const teams = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j'];
    const weeks = roundRobin(teams, 9);
    const met = new Set<string>();
    for (const week of weeks) {
      const playing = week.flatMap((pair) => [pair.home, pair.away]);
      assert.equal(new Set(playing).size, 10);
      for (const pair of week) met.add([pair.home, pair.away].sort().join());
    }
    assert.equal(met.size, 45);
  });

  it('gives an odd league one bye a week, spread evenly', () => {
    const weeks = roundRobin(['a', 'b', 'c', 'd', 'e'], 5);
    const byes: Record<string, number> = {};
    for (const week of weeks) {
      const bye = week.filter((pair) => pair.away === null);
      assert.equal(bye.length, 1);
      byes[bye[0].home] = (byes[bye[0].home] ?? 0) + 1;
    }
    assert.deepEqual(Object.values(byes), [1, 1, 1, 1, 1]);
  });

  it('keeps going past a full cycle', () => {
    assert.equal(roundRobin(['a', 'b', 'c', 'd'], 14).length, 14);
  });
});

describe('snakeOrder', () => {
  it('reverses every other round', () => {
    assert.deepEqual(snakeOrder(['a', 'b', 'c'], 3), ['a', 'b', 'c', 'c', 'b', 'a', 'a', 'b', 'c']);
  });
});

describe('rankBy', () => {
  it('breaks ties key by key and never by input order', () => {
    const rows = [
      { id: 'x', w: 5, p: 100 },
      { id: 'y', w: 5, p: 120 },
      { id: 'z', w: 6, p: 90 },
      { id: 'w', w: 5, p: 100 },
    ];
    const ranked = rankBy(rows, [(r) => r.w, (r) => r.p], (r) => r.id).map((r) => r.id);
    assert.deepEqual(ranked, ['z', 'y', 'w', 'x']);
  });
});

describe('bracket', () => {
  it('gives the top seeds the byes and keeps 1 and 2 apart', () => {
    const first = bracket(['s1', 's2', 's3', 's4', 's5', 's6']);
    assert.deepEqual(first, [
      { high: 's1', low: null },
      { high: 's4', low: 's5' },
      { high: 's2', low: null },
      { high: 's3', low: 's6' },
    ]);
    assert.deepEqual(nextRound(['s1', 's4', 's2', 's3']), [
      { high: 's1', low: 's4' },
      { high: 's2', low: 's3' },
    ]);
  });
});
