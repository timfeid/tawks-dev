import { test } from 'node:test';
import assert from 'node:assert/strict';
import http from 'node:http';
import { TawksApi, TawksBot, ServerActionContext, requireTls, result, ui } from '../dist/index.js';

test('tokens only go over plain http to private hosts', () => {
  for (const ok of ['https://rspc.tawks.app/rspc', 'http://localhost:8787/rspc', 'ws://10.0.0.5:8085/', 'http://tawks-rspc:3000/rspc']) {
    assert.doesNotThrow(() => requireTls(ok), ok);
  }
  for (const bad of ['http://rspc.tawks.app/rspc', 'ws://203.0.113.9:8085/']) assert.throws(() => requireTls(bad), bad);
});

test('number inputs accept template strings', () => {
  const ctx = new ServerActionContext({ workflow: { responses: { a: '7', b: 3, c: 'x', d: '' } }, context: {} });
  assert.equal(ctx.number('a'), 7);
  assert.equal(ctx.number('b'), 3);
  assert.equal(ctx.number('c'), undefined);
  assert.equal(ctx.number('d'), undefined);
});

test('ui helpers build the JSON tawks expects', () => {
  const def = ui.workflow('hi', 'Hi', { home: ui.node('Home', [ui.page([ui.form('go', [ui.textInput('name'), ui.submit('Go', 'go')])])]) }, { actions: [ui.actions.run('go', 'sa-hi.go-bot')] });
  const form = def.nodes.home.visuals[0].display.Block.content[0].type.Form;
  assert.equal(form.server_action_id, 'go');
  assert.equal(form.content[1].type.Button.type, 'submit');
  assert.deepEqual(def.actions[0], { id: 'go', action_type: 'RunServerAction', allow_while_loading: false, target: 'sa-hi.go-bot', progress_node: true });
});

function fakeTawks(handle) {
  const calls = [];
  const server = http.createServer(async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    const procedure = req.url.split('/').pop();
    calls.push({ procedure, input: body ? JSON.parse(body) : null, auth: req.headers.authorization });
    const out = handle(procedure);
    res.end(JSON.stringify(out instanceof Error ? { result: { type: 'error', data: { message: out.message } } } : { result: { type: 'response', data: out } }));
  });
  return new Promise((resolve) => server.listen(0, () => resolve({ server, calls, url: `http://localhost:${server.address().port}/rspc` })));
}

test('the api sends the bot token and surfaces errors', async () => {
  const { server, calls, url } = await fakeTawks((p) => (p === 'boom' ? new Error('No such thing') : 'ok'));
  const api = new TawksApi({ rspcUrl: url, token: 'tbot_x' });
  assert.equal(await api.mutate('chat_create_message', { chat_id: 'c' }), 'ok');
  assert.equal(calls[0].auth, 'Bot tbot_x');
  await assert.rejects(api.query('boom'), /boom: No such thing/);
  server.close();
});

test('a server action request runs its handler once and answers with the result', async () => {
  const { server, calls, url } = await fakeTawks((p) => (p === 'server_action_register' ? 'sa-hello.wave-bot' : null));
  const bot = new TawksBot({ rspcUrl: url, eventsUrl: 'ws://localhost:1/', token: 't', logger: null });
  let runs = 0;
  const id = await bot.serverAction('hello.wave', (ctx) => {
    runs += 1;
    return result.responses({ reply: `hi ${ctx.input('name')}` });
  });
  const request = JSON.stringify({ type: 'workflow_request_server_action', id: 'tok1', action_id: id, workflow: { workflow: { responses: { name: 'Ada' } }, context: {} } });
  await bot['receive'](request);
  await bot['receive'](request);
  assert.equal(runs, 1);
  const answer = calls.find((c) => c.procedure === 'workflow_respond_server_action');
  assert.deepEqual(answer.input, { token: 'tok1', result: { UpdateResponses: { reply: 'hi Ada' } } });
  server.close();
});

test('notify sends to the kind, and narrows only when asked', async () => {
  const { server, calls, url } = await fakeTawks(() => 2);
  const bot = new TawksBot({ rspcUrl: url, eventsUrl: 'ws://localhost:1/', token: 't', logger: null });
  await bot.notify('c1', { eventKey: 'hellobot.daily', dedupeKey: 'hellobot.daily:1', title: 'Hi' });
  await bot.notify('c1', { eventKey: 'hellobot.daily', dedupeKey: 'hellobot.daily:2', title: 'Hi', userIds: ['u1'] });
  const [all, narrowed] = calls.filter((c) => c.procedure === 'bot_notifications_send').map((c) => c.input);
  assert.equal('user_ids' in all, false);
  assert.deepEqual(narrowed.user_ids, ['u1']);
  server.close();
});
