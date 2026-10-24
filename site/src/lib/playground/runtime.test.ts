import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { act as run, applyTransforms, begin, evalCondition, fromMocks, nextNode, start } from './runtime.ts';

const data = JSON.parse(readFileSync(new URL('./generated.json', import.meta.url), 'utf8'));
const template = (key: string) => data.templates.find((t: { key: string }) => t.key === key);
const user = { id: 'player-1', username: 'player1', display_name: 'Player 1' };
const act = (def: any, s: any, id: string, inputs: any, mocks: any) => run(def, s, id, inputs, fromMocks(mocks));

test('a page starts on its first root node', async () => {
  const s = start(template('steps').definition, user);
  assert.equal(s.nodeId, 'a_start');
  assert.equal((s.context.$user as { id: string }).id, 'player-1');
});

test('next, back and submit move through nodes like the engine', async () => {
  const def = template('steps').definition;
  let s = start(def, user);
  s = (await act(def, s, 'next', { name: 'Ada' }, {})).state;
  assert.equal(s.nodeId, 'b_details');
  assert.equal(s.responses.name, 'Ada');
  s = (await act(def, s, 'back', {}, {})).state;
  assert.equal(s.nodeId, 'a_start');
  s = (await act(def, s, 'next', {}, {})).state;
  s = (await act(def, s, 'finish', { favorite: 7 }, {})).state;
  assert.equal(s.status, 'completed');
});

test('transforms set what conditions read', async () => {
  const def = template('mood').definition;
  const s = (await act(def, start(def, user), 'good', {}, {})).state;
  assert.equal(s.responses.mood, 'good');
  assert.ok(evalCondition({ ResponseEquals: { field: 'mood', value: 'good' } }, s.responses, s.context));
});

test('server actions answer from mocks, and say so when one is missing', async () => {
  const t = template('hello');
  const { state, log } = await act(t.definition, start(t.definition, user), 'wave', { name: 'Ada' }, t.mocks);
  assert.equal(state.responses.reply, 'The bot waves back.');
  const missing = await act(t.definition, start(t.definition, user), 'wave', {}, {});
  assert.ok(missing.log.some((l) => l.kind === 'warn' && l.text.includes('hello.wave')));
  assert.ok(log.some((l) => l.kind === 'server'));
});

test('reserved $ inputs are ignored', async () => {
  const t = template('hello');
  const { state } = await act(t.definition, start(t.definition, user), 'wave', { $user: 'spoof' }, t.mocks);
  assert.notEqual(state.context.$user, 'spoof');
  assert.equal(state.responses.$user, undefined);
});

test('transform ops match the engine', async () => {
  const s = { responses: { items: [{ n: 1 }, { n: 2 }], a: { b: 1 } } as Record<string, any>, context: { who: 'x' } as Record<string, any> };
  const effects = applyTransforms(s, [
    { Set: { path: 'deep.list.0.v', value: 5 } },
    { Append: { path: 'log', value: 'one' } },
    { AppendUnique: { path: 'log', value: 'one' } },
    { MergeObject: { path: 'a', value: { c: 2 } } },
    { FilterArray: { path: 'items', as_key: 'it', predicate: { Equals: { path: 'it.n', value: 2 } } } },
    { MapArray: { path: 'items', as_key: 'it', template: { label: 'n is {it.n} for {$context.who}' } } },
    { Copy: { from: 'a.c', to: 'copied' } },
    { Move: { from: 'copied', to: 'moved' } },
    { Set: { path: '$context.flag', value: true } },
    { When: { cond: { Exists: { path: 'moved' } }, ops: ['EmitNextNode'], else_ops: ['EmitCancel'] } },
  ]);
  assert.deepEqual(s.responses.deep, { list: [{ v: 5 }] });
  assert.deepEqual(s.responses.log, ['one']);
  assert.deepEqual(s.responses.a, { b: 1, c: 2 });
  assert.deepEqual(s.responses.items, [{ label: 'n is 2 for x' }]);
  assert.equal(s.responses.copied, undefined);
  assert.equal(s.responses.moved, 2);
  assert.equal(s.context.flag, true);
  assert.deepEqual(effects, [{ kind: 'next' }]);
});

test('next node prefers a matching conditioned child', async () => {
  const node = (parent_id: string | null, condition: unknown = null) => ({ title: '', description: null, visuals: [], layout: null, condition, parent_id });
  const def = {
    id: 'x', name: 'x', description: null, responses: {}, context: {}, actions: [], starting_server_action: null, multi_instance_per_user: false,
    nodes: { a: node(null), b_plain: node('a'), c_vip: node('a', { ResponseEquals: { field: 'vip', value: true } }) },
  } as never;
  assert.equal(nextNode(def, 'a', { responses: {}, context: {} }), 'b_plain');
  assert.equal(nextNode(def, 'a', { responses: { vip: true }, context: {} }), 'c_vip');
});

test('an update from a server action re-picks the node within its group', async () => {
  const node = (parent_id: string | null, condition: unknown = null) => ({ title: '', description: null, visuals: [], layout: null, condition, parent_id });
  const def = {
    id: 'x', name: 'x', description: null, responses: {}, context: {}, starting_server_action: 'load', multi_instance_per_user: false,
    actions: [{ id: 'finish', action_type: 'RunServerAction', target: 'finish', allow_while_loading: false, progress_node: true }],
    nodes: {
      lobby: node(null),
      lobby_playing: node('lobby', { ResponseEquals: { field: 'game.phase', value: 'playing' } }),
      lobby_waiting: node('lobby', { ResponseEquals: { field: 'game.phase', value: 'waiting' } }),
    },
  } as never;
  const mocks = { load: { UpdateResponses: { game: { phase: 'waiting' } } }, finish: { UpdateResponses: { game: { phase: 'playing' } } } };
  const started = await begin(def, user, fromMocks(mocks));
  assert.equal(started.state.nodeId, 'lobby_waiting');
  const after = await act(def, started.state, 'finish', {}, mocks);
  assert.equal(after.state.nodeId, 'lobby_playing');
  assert.deepEqual(after.state.history, ['lobby', 'lobby_waiting']);
});

test('mock rules answer by condition, and the right guess switches the page', async () => {
  const t = template('guess');
  let s = start(t.definition, user);
  assert.equal(s.nodeId, 'play');
  s = (await act(t.definition, s, 'guess', { gn_guess: 80 }, t.mocks)).state;
  assert.equal(s.responses.gn_message, 'Too high. Try lower.');
  assert.equal(s.nodeId, 'play');
  s = (await act(t.definition, s, 'guess', { gn_guess: 10 }, t.mocks)).state;
  assert.equal(s.responses.gn_message, 'Too low. Try higher.');
  s = (await act(t.definition, s, 'guess', { gn_guess: 42 }, t.mocks)).state;
  assert.equal(s.nodeId, 'won');
  assert.equal(s.responses.gn_done, true);
});
