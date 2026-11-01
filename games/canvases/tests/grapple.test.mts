import { test } from 'node:test';
import assert from 'node:assert/strict';
import { grappleFlightSeconds, grappleTravelProgress, grappleResult } from '../src/party/board/grapple.ts';

const members = [{seat: 0, name: 'Alice'}, {seat: 1, name: 'Bob'}];
const hit = {kind: 'item', item: 'grapple', by: 0, target: 1, quality: 'hit', coins: 8, flight: 1};

test('a catch names the coin transfer for thrower, target, and spectators', () => {
  assert.equal(grappleResult(hit, members, 0)?.title, '+8 COINS');
  assert.equal(grappleResult(hit, members, 0)?.detail, 'You took 8 coins from Bob.');
  assert.equal(grappleResult(hit, members, 1)?.title, '−8 COINS');
  assert.equal(grappleResult(hit, members, 1)?.detail, 'Alice takes 8 coins from you.');
  assert.equal(grappleResult(hit, members, null)?.detail, 'Alice takes 8 coins from Bob.');
});

test('a landed empty hook is distinct from a miss or broken shield', () => {
  const empty = grappleResult({...hit, coins: 0, quality: 'empty'}, members, 0)!;
  assert.equal(empty.heading, 'HOOK LANDED');
  assert.equal(empty.title, 'EMPTY POCKETS');
  assert.equal(empty.detail, 'Bob has no coins to take.');
  assert.equal(grappleResult({...hit, quality: 'miss'}, members, 0)?.heading, 'HOOK MISSED');
  assert.equal(grappleResult({...hit, quality: 'blocked'}, members, 0)?.title, 'SHIELD BROKEN');
  assert.equal(grappleResult({...hit, coins: 0}, members, 0)?.title, 'EMPTY POCKETS', 'older server hits with zero coins stay readable');
  assert.equal(grappleResult({...hit, item: 'bomb'}, members, 0), null);
});

test('slow catch affects only a successful local flight, and results wait for impact', () => {
  assert.equal(grappleFlightSeconds(hit), 1.75);
  assert.equal(grappleResult(hit, members, 0)?.delay, 1.75);
  assert.equal(grappleFlightSeconds({...hit, quality: 'miss'}), 1);
  assert.equal(grappleFlightSeconds({...hit, quality: 'empty', coins: 0}), 1);
  assert.equal(grappleTravelProgress(0, 1), 0);
  assert.equal(grappleTravelProgress(1, 1), 1);
  const join = .72 / 1.75;
  assert.ok(Math.abs(grappleTravelProgress(join - 1e-6, 1) - grappleTravelProgress(join + 1e-6, 1)) < 1e-5);
  let previous = 0;
  for (let step = 1; step <= 100; step++) {
    const progress = grappleTravelProgress(step / 100, 1);
    assert.ok(progress > previous && progress <= 1);
    previous = progress;
  }
  assert.ok(grappleTravelProgress(.75, 1) < .9, 'last part visibly lingers on the catch');
});
