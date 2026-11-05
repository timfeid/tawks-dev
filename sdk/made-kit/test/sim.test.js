import test from 'node:test';
import assert from 'node:assert/strict';
import { defineSim, slider, toggle, plot, createSimView, rk4, trail } from '../src/sim.js';
import { createSimRunner } from '../src/sim-runner.js';
import springLab from '../examples/spring-lab.js';

const tiny = (extra = {}) =>
  defineSim({
    meta: { name: 'Tiny', about: 'A dot that drifts.', explain: 'It drifts right at the speed you set.', world: { x: [0, 10], y: [0, 5] } },
    params: { speed: slider('Speed', { min: 0, max: 5, value: 1 }), big: toggle('Big', false) },
    plots: { where: plot('Where', { series: { x: ['x', 'blue'] } }) },
    setup: () => ({ x: 0 }),
    step: (s, p, dt) => {
      s.x += p.speed * dt;
    },
    sample: (s) => ({ where: { x: s.x } }),
    view: (s, p, v) => {
      v.circle('dot', s.x, 2, p.big ? 1 : 0.2, { drag: true });
      v.readout('x', s.x, { unit: 'm' });
    },
    grab: (s, p, g) => {
      s.x = g.x;
    },
    ...extra,
  });

test('defineSim normalizes meta, params and plots', () => {
  const sim = tiny();
  assert.equal(sim.spec.rate, 120);
  assert.deepEqual(sim.spec.tools, { ruler: true, stopwatch: true });
  assert.equal(sim.spec.params[0].key, 'speed');
  assert.equal(sim.spec.plots[0].series[0].color, 'blue');
  assert.throws(() => tiny({ meta: { name: 'x' } }), /meta.about|meta.world/);
  assert.throws(() => defineSim({ ...tiny(), params: { s: slider('S', { min: 2, max: 1, value: 1 }) } }), /min first/);
});

test('the runner steps only while playing, at rate times speed', () => {
  const runner = createSimRunner(tiny());
  for (let i = 0; i < 60; i++) runner.advance();
  assert.ok(Math.abs(runner.t - 1) < 1e-9);
  runner.control({ do: 'pause' });
  for (let i = 0; i < 60; i++) runner.advance();
  assert.ok(Math.abs(runner.t - 1) < 1e-9);
  runner.control({ do: 'speed', speed: 2 });
  runner.control({ do: 'play' });
  for (let i = 0; i < 60; i++) runner.advance();
  assert.ok(Math.abs(runner.t - 3) < 1e-9);
  assert.match(runner.control({ do: 'speed', speed: 3 }), /speed/);
});

test('controls change params, clamp them and refuse unknown ones', () => {
  const runner = createSimRunner(tiny());
  runner.control({ do: 'set', key: 'speed', value: 50 });
  assert.equal(runner.frame().params.speed, 5);
  assert.match(runner.control({ do: 'set', key: 'nope', value: 1 }), /no control/);
  runner.control({ do: 'set', key: 'big', value: true });
  assert.equal(runner.frame().shapes[0].r, 1);
});

test('frames carry only new plot samples, and reset starts a new epoch', () => {
  const runner = createSimRunner(tiny());
  for (let i = 0; i < 30; i++) runner.advance();
  const first = runner.frame();
  assert.ok(first.plots[0].points.length >= 10);
  for (let i = 0; i < 3; i++) runner.advance();
  const second = runner.frame();
  assert.equal(second.plots[0].total - first.plots[0].total, second.plots[0].points.length);
  runner.control({ do: 'reset' });
  const third = runner.frame();
  assert.equal(third.plots[0].epoch, first.plots[0].epoch + 1);
  assert.equal(third.t, 0);
});

test('grabbing works only on a shape drawn with drag', () => {
  const runner = createSimRunner(tiny());
  runner.frame();
  assert.equal(runner.control({ do: 'grab', id: 'dot', x: 7, y: 2, phase: 'start' }), null);
  assert.equal(runner.frame().shapes[0].x, 7);
  assert.match(runner.control({ do: 'grab', id: 'other', x: 1, y: 1, phase: 'start' }), /cannot be dragged/);
});

test('a snapshot carries on identically', () => {
  const a = createSimRunner(springLab, { seed: 4 });
  for (let i = 0; i < 120; i++) a.advance();
  const b = createSimRunner(springLab, { seed: 4 });
  b.restore(JSON.stringify(a.snapshot()));
  for (let i = 0; i < 120; i++) {
    a.advance();
    b.advance();
  }
  assert.deepEqual(a.frame().shapes, b.frame().shapes);
});

test('the view checks its shapes', () => {
  const v = createSimView();
  assert.throws(() => v.circle('a', NaN, 0, 1), /finite/);
  assert.throws(() => v.line('a', 0, 0, 1, 1, { color: 'chartreuse' }), /unknown color/);
  v.arrow('zero', 0, 0, 0, 0);
  v.axes('ax', { x: [0, 10], y: [0, 5] });
  assert.ok(v.build().shapes.length > 4);
  assert.ok(!v.build().shapes.some((s) => s.id === 'zero'));
});

test('rk4 and trail behave', () => {
  const [y] = rk4([1], 0, 0.1, ([y]) => [-y]);
  assert.ok(Math.abs(y - Math.exp(-0.1)) < 1e-6);
  const list = [];
  for (let i = 0; i < 10; i++) trail(list, [i, 0], { max: 4 });
  assert.equal(list.length, 4);
  assert.equal(list[0][0], 6);
});

test('Spring Lab runs with every control and stays finite', () => {
  const runner = createSimRunner(springLab);
  runner.control({ do: 'set', key: 'world', value: 'moon' });
  runner.control({ do: 'set', key: 'pull', value: true });
  for (let i = 0; i < 600; i++) runner.advance();
  const frame = runner.frame();
  assert.ok(frame.shapes.find((s) => s.id === 'block').drag);
  assert.ok(frame.readouts.every((r) => r.value === undefined || Number.isFinite(r.value)));
});
