import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PRESENCE_TEXT_MAX, readPresence } from './protocol.ts';

test('a game presence is trimmed, capped and checked', () => {
  assert.deepEqual(readPresence({ details: '  Bully by Shinedown ', state: 'Drums, Expert', started_at: 1793875345000.4 }), {
    details: 'Bully by Shinedown',
    state: 'Drums, Expert',
    started_at: 1793875345000,
  });
  assert.deepEqual(readPresence({ details: 'Picking a song' }), { details: 'Picking a song', state: null, started_at: null });
  assert.equal(readPresence({ details: 'x'.repeat(500) })?.details?.length, PRESENCE_TEXT_MAX);
  assert.equal(readPresence(null), null);
  assert.equal(readPresence({ details: ' ', state: '' }), null);
  assert.equal(readPresence({ details: 5 }), undefined);
  assert.equal(readPresence({ details: 'a', started_at: 'soon' }), undefined);
  assert.equal(readPresence({ details: 'a', started_at: -1 }), undefined);
  assert.equal(readPresence(['a']), undefined);
  assert.equal(readPresence('a'), undefined);
});
