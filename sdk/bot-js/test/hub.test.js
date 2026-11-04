import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { HubBot, HubDb, destructiveStatement } from '../dist/hub.js';

const open = () => new HubDb(new DatabaseSync(':memory:'));

test('migrations run once, in order, and record how far they got', () => {
  const db = open();
  db.migrate(['CREATE TABLE a (x)', 'CREATE TABLE b (y)']);
  db.migrate(['CREATE TABLE a (x)', 'CREATE TABLE b (y)', 'CREATE TABLE c (z)']);
  assert.equal(db.get('PRAGMA user_version').user_version, 3);
  assert.equal(db.all("SELECT name FROM sqlite_master WHERE type = 'table'").length, 3);
});

test('a file ahead of the code, after a rollback, is left alone', () => {
  const db = open();
  db.migrate(['CREATE TABLE a (x)', 'CREATE TABLE b (y)']);
  db.migrate(['CREATE TABLE a (x)']);
  assert.equal(db.get('PRAGMA user_version').user_version, 2);
  db.run('INSERT INTO b VALUES (1)');
});

test('a migration that destroys something is refused and changes nothing', () => {
  const db = open();
  db.migrate(['CREATE TABLE a (x)']);
  for (const bad of ['DROP TABLE a', 'ALTER TABLE a DROP COLUMN x', 'ALTER TABLE a RENAME TO b', 'CREATE TABLE c (z); DROP TABLE a;']) {
    assert.throws(() => db.migrate(['CREATE TABLE a (x)', bad]), /would destroy data/);
  }
  assert.equal(db.get('PRAGMA user_version').user_version, 1);
  assert.equal(destructiveStatement('ALTER TABLE a ADD COLUMN y TEXT'), null);
  assert.equal(destructiveStatement('-- DROP TABLE a\nCREATE TABLE b (c)'), null);
});

test('a table the change names may go', () => {
  process.env.TAWKS_ALLOWED_DROPS = 'old_picks';
  try {
    const db = open();
    db.migrate(['CREATE TABLE old_picks (x)', 'DROP TABLE old_picks']);
    assert.equal(db.all("SELECT name FROM sqlite_master WHERE name = 'old_picks'").length, 0);
    assert.throws(() => db.migrate(['CREATE TABLE old_picks (x)', 'DROP TABLE old_picks', 'CREATE TABLE z (a)', 'DROP TABLE z']), /would destroy data/);
  } finally {
    delete process.env.TAWKS_ALLOWED_DROPS;
  }
});

test('a transaction commits together or not at all', () => {
  const db = open();
  db.migrate(['CREATE TABLE a (x)']);
  assert.throws(() =>
    db.tx(() => {
      db.run('INSERT INTO a VALUES (1)');
      throw new Error('no');
    }),
  );
  assert.equal(db.get('SELECT count(*) AS n FROM a').n, 0);
  db.tx(() => db.run('INSERT INTO a VALUES (1)'));
  assert.equal(db.get('SELECT count(*) AS n FROM a').n, 1);
});

const refresher = (fail) => {
  const hub = new HubBot(open(), { rspcUrl: 'http://x', eventsUrl: 'ws://x', token: 't' });
  const sent = [];
  hub.bot.api.mutate = async (_procedure, body) => {
    if (fail(body)) throw Object.assign(new Error('refused'), { code: 403 });
    sent.push(body.filter.user_id);
    return 1;
  };
  return { hub, sent };
};

test('a refresh that fails for one person still reaches everyone after them', async () => {
  const { hub, sent } = refresher((body) => body.filter.user_id === 'm_a');
  await hub.refresh('p', () => [['m_a', { x: 1 }], ['m_b', { x: 1 }], ['m_c', { x: 1 }]]);
  assert.deepEqual([...new Set(sent)], ['m_b', 'm_c']);
});

test('a refresh asked for while one runs uses the newest build', async () => {
  const { hub, sent } = refresher(() => false);
  await Promise.all([hub.refresh('p', () => [['m_a', { x: 1 }]]), hub.refresh('p', () => [['m_b', { x: 2 }]])]);
  assert.ok(sent.includes('m_b'), JSON.stringify(sent));
});

test('a refresh with nobody to push to leaves the next one working', async () => {
  const { hub, sent } = refresher(() => false);
  await hub.refresh('p', () => []);
  await hub.refresh('p', () => [['m_a', { x: 1 }]]);
  assert.deepEqual(sent, ['m_a']);
});
