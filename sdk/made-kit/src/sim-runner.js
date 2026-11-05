// Runs a sim: its params, the transport (play, pause, step, reset, speed),
// fixed steps at meta.rate, plot sampling and frames. made-simd drives it
// inside its QuickJS sandbox, advancing it 60 times a real second, and node
// tests drive it directly. It is kit code, not sim code.

import { createSimView, nextRandom, seededState, SPEEDS } from './sim.js';

export const ADVANCE_HZ = 60;
export const SAMPLE_DT = 1 / 20;
const MAX_STEPS_PER_ADVANCE = 400;
const MAX_POINTS = 600;
const MAX_POINTS_PER_FRAME = 200;
const ADVANCES_PER_FRAME = 3;

const isNum = (n) => typeof n === 'number' && Number.isFinite(n);
const clone = (x) => JSON.parse(JSON.stringify(x));

export function createSimRunner(sim, { seed: firstSeed = 1 } = {}) {
  let seed = firstSeed;
  const spec = sim.spec;
  if (!spec) throw new Error('sim.js must `export default defineSim({ ... })`');
  const dt = 1 / spec.rate;
  const defaults = Object.fromEntries(spec.params.map((p) => [p.key, p.value]));
  const byKey = new Map(spec.params.map((p) => [p.key, p]));

  let run = null;
  let frameN = 0;
  let draggable = new Set();
  let grabbing = null;

  const ctxFor = () => ({
    get t() { return run.t; },
    dt,
    rand: () => nextRandom(run.rng),
  });

  function fresh(params, epoch) {
    const r = { seed, params, rng: seededState(seed), epoch, t: 0, steps: 0, acc: 0, playing: run ? run.playing : true, speed: run ? run.speed : 1, nextSample: 0, plots: {}, sent: {}, state: null };
    for (const plot of spec.plots) {
      r.plots[plot.id] = { total: 0, points: [] };
      r.sent[plot.id] = 0;
    }
    return r;
  }

  // A reset is the same start every time: the random numbers start over too.
  function startOver() {
    const epoch = run ? run.epoch + 1 : 1;
    const params = run ? run.params : { ...defaults };
    run = fresh(params, epoch);
    const state = sim.setup(Object.freeze({ ...run.params }), ctxFor());
    if (state === undefined || state === null || typeof state !== 'object') throw new Error('setup: return the state object');
    run.state = state;
    sample();
  }

  function sample() {
    if (!spec.plots.length) return;
    const got = sim.sample(run.state, Object.freeze({ ...run.params }), ctxFor()) || {};
    for (const plot of spec.plots) {
      const values = got[plot.id];
      if (!values || typeof values !== 'object') continue;
      const point = [Math.round(run.t * 1000) / 1000];
      for (const series of plot.series) {
        const value = values[series.key];
        if (!isNum(value)) throw new Error(`sample: plots.${plot.id}.${series.key} must be a finite number, got ${value}`);
        point.push(Math.round(value * 10000) / 10000);
      }
      const held = run.plots[plot.id];
      held.points.push(point);
      held.total += 1;
      if (held.points.length > MAX_POINTS) held.points.splice(0, held.points.length - MAX_POINTS);
    }
  }

  function stepOnce() {
    sim.step(run.state, Object.freeze({ ...run.params }), dt, ctxFor());
    run.steps += 1;
    run.t = run.steps * dt;
    if (run.t + 1e-9 >= run.nextSample) {
      run.nextSample += SAMPLE_DT;
      sample();
    }
  }

  function steps(n) {
    for (let i = 0; i < n; i++) stepOnce();
  }

  const stepsPerAdvance = (speed) => (spec.rate * speed) / ADVANCE_HZ;

  startOver();

  return {
    spec,

    /** One real 1/60 s. While playing it runs rate x speed / 60 fixed steps. */
    advance() {
      if (!run.playing) return;
      run.acc += stepsPerAdvance(run.speed);
      const n = Math.min(MAX_STEPS_PER_ADVANCE, Math.floor(run.acc + 1e-9));
      run.acc -= n;
      steps(n);
    },

    /** Applies one control from the controller. Returns null, or why it was refused. */
    control(c) {
      if (!c || typeof c !== 'object') return 'not a control';
      switch (c.do) {
        case 'play':
          run.playing = true;
          return null;
        case 'pause':
          run.playing = false;
          return null;
        case 'step':
          run.playing = false;
          steps(Math.max(1, Math.round(stepsPerAdvance(1) * ADVANCES_PER_FRAME)));
          return null;
        case 'reset':
          startOver();
          return null;
        case 'speed':
          if (!SPEEDS.includes(c.speed)) return `speed is one of ${SPEEDS.join(', ')}`;
          run.speed = c.speed;
          return null;
        case 'set': {
          const p = byKey.get(c.key);
          if (!p) return `no control named ${c.key}`;
          if (p.kind === 'button') {
            if (typeof sim.press === 'function') sim.press(run.state, Object.freeze({ ...run.params }), p.key, ctxFor());
            return null;
          }
          let value = c.value;
          if (p.kind === 'toggle') value = !!value;
          else if (p.kind === 'choice') {
            if (!p.options.some((o) => o.value === value)) return `${p.key} is one of its options`;
          } else {
            if (!isNum(value)) return `${p.key} takes a number`;
            value = Math.min(p.max, Math.max(p.min, value));
          }
          if (run.params[p.key] === value) return null;
          run.params = { ...run.params, [p.key]: value };
          if (p.restart) startOver();
          else if (typeof sim.changed === 'function') sim.changed(run.state, Object.freeze({ ...run.params }), p.key, ctxFor());
          return null;
        }
        case 'grab': {
          if (typeof sim.grab !== 'function') return 'nothing to drag';
          if (c.phase === 'start') {
            if (!draggable.has(c.id)) return `${c.id} cannot be dragged`;
            grabbing = c.id;
          } else if (grabbing !== c.id) return null;
          if (!isNum(c.x) || !isNum(c.y)) return 'a grab needs x and y';
          sim.grab(run.state, Object.freeze({ ...run.params }), { id: c.id, x: c.x, y: c.y, phase: c.phase }, ctxFor());
          if (c.phase === 'end') grabbing = null;
          return null;
        }
        default:
          return `unknown control ${c.do}`;
      }
    },

    /** The sim now, with the plot samples taken since the last frame. */
    frame() {
      const v = createSimView({ t: run.t });
      sim.view(run.state, Object.freeze({ ...run.params }), v);
      const built = v.build();
      draggable = new Set(built.shapes.filter((s) => s.drag).map((s) => s.id));
      const plots = spec.plots.map((plot) => {
        const held = run.plots[plot.id];
        const fresh = Math.min(MAX_POINTS_PER_FRAME, held.total - run.sent[plot.id], held.points.length);
        run.sent[plot.id] = held.total;
        return { id: plot.id, epoch: run.epoch, total: held.total, points: fresh > 0 ? held.points.slice(-fresh) : [] };
      });
      const params = {};
      for (const p of spec.params) if (p.kind !== 'button') params[p.key] = run.params[p.key];
      return {
        n: frameN++,
        t: Math.round(run.t * 10000) / 10000,
        playing: run.playing,
        speed: run.speed,
        params,
        shapes: built.shapes,
        readouts: built.readouts,
        plots,
        notice: built.notice,
      };
    },

    get t() { return run.t; },
    get playing() { return run.playing; },

    /** Everything the run depends on, as JSON, so a copy carries on identically. */
    snapshot() {
      return clone({ run, grabbing, draggable: [...draggable] });
    },
    restore(json) {
      const saved = typeof json === 'string' ? JSON.parse(json) : json;
      run = saved.run;
      seed = run.seed;
      grabbing = saved.grabbing;
      draggable = new Set(saved.draggable);
    },
  };
}
