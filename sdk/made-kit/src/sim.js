// The sim kit: what a sim.js imports. Like the game kit it is pure and runs
// the same in the server sandbox (QuickJS), in node and in the browser; it
// never touches the DOM, the clock or Math.random.
//
// The world is a flat x/y plane in the sim's own units with y up. Shape
// positions and sizes are world units; line widths and text sizes are screen
// pixels. Time is seconds of sim time.

export const SIM_KIT_VERSION = 1;

export const SIM_COLORS = [
  'ink', 'muted', 'faint', 'surface', 'accent', 'blue', 'orange', 'green', 'yellow', 'pink', 'purple',
  'red', 'teal', 'sky', 'earth', 'water',
];
export const PARAM_KINDS = ['slider', 'number', 'toggle', 'choice', 'button'];
export const SPEEDS = [0.25, 0.5, 1, 2, 4];
export const MAX_SHAPES = 600;
export const MAX_READOUTS = 12;
export const MAX_PARAMS = 12;
export const MAX_PLOTS = 3;
export const MAX_SERIES = 4;

const fail = (where, message) => {
  throw new Error(`${where}: ${message}`);
};
const isNum = (n) => typeof n === 'number' && Number.isFinite(n);
const line = (where, s, max) => {
  if (typeof s !== 'string' || !s.trim()) fail(where, 'give it some text');
  if (s.length > max) fail(where, `keep it to ${max} characters`);
  return s.trim();
};
const keyOf = (where, k) => {
  if (!/^[a-zA-Z][a-zA-Z0-9_]{0,31}$/.test(k)) fail(where, `'${k}' must be a short name of letters, digits and _`);
  return k;
};
const colorOf = (where, c, fallback) => {
  if (c === undefined || c === null) return fallback;
  if (!SIM_COLORS.includes(c)) fail(where, `unknown color '${c}'; use one of ${SIM_COLORS.join(', ')}`);
  return c;
};

// ------------------------------------------------------------ controls

/** A slider from min to max. `restart: true` starts the run over when it changes. */
export function slider(label, { min, max, value, step, unit, restart = false, help } = {}) {
  return { kind: 'slider', label, min, max, value, step, unit, restart, help };
}

/** A number box with min and max. */
export function number(label, { min, max, value, step, unit, restart = false, help } = {}) {
  return { kind: 'number', label, min, max, value, step, unit, restart, help };
}

/** An on/off switch. */
export function toggle(label, value = false, { restart = false, help } = {}) {
  return { kind: 'toggle', label, value: !!value, restart, help };
}

/** One of a few options, given as { value: 'Label' }. */
export function choice(label, options, value, { restart = false, help } = {}) {
  return { kind: 'choice', label, options, value, restart, help };
}

/** A button. Pressing it calls the sim's `press(state, p, key, ctx)`. */
export function button(label, { help } = {}) {
  return { kind: 'button', label, value: false, restart: false, help };
}

/** A live line plot of samples over time, `seconds` wide. series: { key: ['Label', 'color'] }. */
export function plot(title, { y = '', seconds = 10, series, range } = {}) {
  return { title, y, seconds, series, range };
}

function normalizeParam(key, spec) {
  const where = `params.${key}`;
  if (!spec || typeof spec !== 'object' || !PARAM_KINDS.includes(spec.kind))
    fail(where, 'make it with slider(), number(), toggle(), choice() or button()');
  const out = { key, label: line(`${where} label`, spec.label, 40), kind: spec.kind, restart: !!spec.restart };
  if (spec.help !== undefined) out.help = line(`${where} help`, spec.help, 120);
  if (spec.kind === 'slider' || spec.kind === 'number') {
    if (!isNum(spec.min) || !isNum(spec.max) || spec.min >= spec.max) fail(where, 'needs min and max, min first');
    if (!isNum(spec.value) || spec.value < spec.min || spec.value > spec.max) fail(where, 'value must be a number from min to max');
    out.min = spec.min;
    out.max = spec.max;
    out.value = spec.value;
    out.step = isNum(spec.step) && spec.step > 0 ? spec.step : (spec.max - spec.min) / 100;
    if (spec.unit !== undefined) out.unit = line(`${where} unit`, spec.unit, 16);
  } else if (spec.kind === 'choice') {
    const options = spec.options && typeof spec.options === 'object' ? Object.entries(spec.options) : [];
    if (options.length < 2 || options.length > 8) fail(where, 'give 2 to 8 options as { value: "Label" }');
    out.options = options.map(([value, label]) => ({ value, label: line(`${where} option`, label, 30) }));
    if (!out.options.some((o) => o.value === spec.value)) fail(where, 'value must be one of the option keys');
    out.value = spec.value;
  } else {
    out.value = !!spec.value && spec.kind === 'toggle';
  }
  return out;
}

function normalizePlot(id, spec) {
  const where = `plots.${id}`;
  if (!spec || typeof spec !== 'object') fail(where, "make it with plot('Title', { y, seconds, series })");
  const series = spec.series && typeof spec.series === 'object' ? Object.entries(spec.series) : [];
  if (series.length < 1 || series.length > MAX_SERIES) fail(where, `give 1 to ${MAX_SERIES} series as { key: ['Label', 'color'] }`);
  const span = spec.seconds === undefined ? 10 : spec.seconds;
  if (!isNum(span) || span <= 0 || span > 600) fail(where, 'seconds is how much time the plot shows, 0 to 600');
  const out = {
    id,
    title: line(`${where} title`, spec.title, 40),
    y_label: typeof spec.y === 'string' ? spec.y.slice(0, 30) : '',
    series: series.map(([key, value]) => {
      const [label, color] = Array.isArray(value) ? value : [value, 'blue'];
      return { key: keyOf(`${where} series`, key), label: line(`${where} series label`, label, 30), color: colorOf(`${where} series ${key}`, color, 'blue') };
    }),
    window: span,
  };
  if (spec.range !== undefined) {
    if (!Array.isArray(spec.range) || spec.range.length !== 2 || !spec.range.every(isNum) || spec.range[0] >= spec.range[1])
      fail(where, 'range is [low, high]');
    out.y = spec.range;
  }
  return out;
}

/** Declares a sim. Checks its shape so a broken sim fails at load, not mid-run. */
export function defineSim(sim) {
  if (!sim || typeof sim !== 'object') fail('defineSim', 'pass one object: { meta, params, plots, setup, step, view }');
  const meta = sim.meta;
  if (!meta || typeof meta !== 'object') fail('defineSim', 'meta is missing');
  const world = meta.world;
  if (!world || !Array.isArray(world.x) || !Array.isArray(world.y) || world.x.length !== 2 || world.y.length !== 2)
    fail('meta.world', 'give x: [left, right] and y: [bottom, top] in world units');
  if (![...world.x, ...world.y].every(isNum) || world.x[0] >= world.x[1] || world.y[0] >= world.y[1])
    fail('meta.world', 'x and y are [low, high] numbers');
  const rate = meta.rate === undefined ? 120 : meta.rate;
  if (!Number.isInteger(rate) || rate < 10 || rate > 2000) fail('meta.rate', 'fixed steps per second of sim time, 10 to 2000');
  const params = sim.params || {};
  const keys = Object.keys(params);
  if (keys.length > MAX_PARAMS) fail('params', `at most ${MAX_PARAMS} controls`);
  const plots = sim.plots || {};
  if (Object.keys(plots).length > MAX_PLOTS) fail('plots', `at most ${MAX_PLOTS} plots`);
  for (const hook of ['setup', 'step', 'view'])
    if (typeof sim[hook] !== 'function') fail('defineSim', `${hook} must be a function`);
  for (const hook of ['sample', 'grab', 'press', 'changed'])
    if (sim[hook] !== undefined && typeof sim[hook] !== 'function') fail('defineSim', `${hook} must be a function when given`);
  if (Object.keys(plots).length && typeof sim.sample !== 'function') fail('defineSim', 'a sim with plots needs sample(state, p) returning { plotId: { seriesKey: number } }');
  const tools = meta.tools || {};
  const spec = {
    name: line('meta.name', meta.name, 40),
    about: line('meta.about', meta.about, 120),
    explain: line('meta.explain', meta.explain, 500),
    world: {
      x: [world.x[0], world.x[1]],
      y: [world.y[0], world.y[1]],
      unit: typeof world.unit === 'string' && world.unit.trim() ? world.unit.trim().slice(0, 12) : 'm',
      ...(isNum(world.grid) && world.grid > 0 ? { grid: world.grid } : {}),
    },
    params: keys.map((key) => normalizeParam(keyOf('params', key), params[key])),
    plots: Object.keys(plots).map((id) => normalizePlot(keyOf('plots', id), plots[id])),
    rate,
    tools: { ruler: tools.ruler !== false, stopwatch: tools.stopwatch !== false },
  };
  return { ...sim, spec };
}

// ------------------------------------------------------------- helpers

export const clamp = (n, lo, hi) => Math.min(hi, Math.max(lo, n));
export const lerp = (a, b, t) => a + (b - a) * t;
export const hypot = (x, y) => Math.hypot(x, y);
export const deg = (radians) => (radians * 180) / Math.PI;
export const rad = (degrees) => (degrees * Math.PI) / 180;

/**
 * One classic Runge-Kutta step. `y` is an array of numbers, `f(y, t)` returns
 * its derivative as an array of the same length. Returns the new array.
 */
export function rk4(y, t, dt, f) {
  const n = y.length;
  const add = (a, b, k) => {
    const out = new Array(n);
    for (let i = 0; i < n; i++) out[i] = a[i] + b[i] * k;
    return out;
  };
  const k1 = f(y, t);
  const k2 = f(add(y, k1, dt / 2), t + dt / 2);
  const k3 = f(add(y, k2, dt / 2), t + dt / 2);
  const k4 = f(add(y, k3, dt), t + dt);
  const out = new Array(n);
  for (let i = 0; i < n; i++) out[i] = y[i] + (dt / 6) * (k1[i] + 2 * k2[i] + 2 * k3[i] + k4[i]);
  return out;
}

/** Adds `point` ([x, y]) to `list` when it is at least `gap` from the last one, keeping the newest `max`. */
export function trail(list, point, { max = 300, gap = 0 } = {}) {
  const last = list[list.length - 1];
  if (!last || Math.hypot(point[0] - last[0], point[1] - last[1]) >= gap) list.push([point[0], point[1]]);
  if (list.length > max) list.splice(0, list.length - max);
  return list;
}

/** A seeded random number generator returning [0, 1), whose state is a plain number. */
export function seededState(seed) {
  return { s: seed >>> 0 };
}
export function nextRandom(r) {
  r.s = (r.s + 0x6d2b79f5) >>> 0;
  let t = r.s;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

// ------------------------------------------------------------ the view

const round = (n) => Math.round(n * 10000) / 10000;

function num(where, name, n, fallback) {
  if (n === undefined || n === null) {
    if (fallback === undefined) fail(where, `${name} is missing`);
    return fallback;
  }
  if (!isNum(n)) fail(where, `${name} must be a finite number, got ${n}`);
  return round(n);
}

function pointList(where, points) {
  if (!Array.isArray(points)) fail(where, 'points is a list of [x, y] (or {x, y})');
  return points.map((p, i) => {
    const x = Array.isArray(p) ? p[0] : p && p.x;
    const y = Array.isArray(p) ? p[1] : p && p.y;
    return [num(where, `point ${i} x`, x), num(where, `point ${i} y`, y)];
  });
}

function styleOf(where, opts, defaults = {}) {
  const out = {};
  const color = colorOf(`${where} color`, opts.color, defaults.color);
  const fill = colorOf(`${where} fill`, opts.fill, defaults.fill);
  if (color) out.color = color;
  if (fill) out.fill = fill;
  if (opts.width !== undefined) out.width = clamp(num(where, 'width', opts.width), 0.5, 40);
  if (opts.dash) out.dash = true;
  if (opts.alpha !== undefined) out.alpha = clamp(num(where, 'alpha', opts.alpha), 0, 1);
  if (opts.label !== undefined && opts.label !== null && opts.label !== '') out.label = String(opts.label).slice(0, 80);
  if (opts.drag) out.drag = true;
  return out;
}

/**
 * The scene builder a sim's `view(state, p, v)` draws with. The runner makes
 * one per frame; sims never call this themselves.
 */
export function createSimView({ t = 0 } = {}) {
  const shapes = [];
  const ids = new Set();
  const readouts = [];
  let notice = null;

  const add = (where, s) => {
    if (typeof s.id !== 'string' || !s.id || s.id.length > 48) fail(where, 'the id must be text of 1 to 48 characters');
    if (ids.has(s.id)) fail(where, `the id '${s.id}' is used twice in one frame`);
    if (shapes.length >= MAX_SHAPES) fail(where, `a frame can hold at most ${MAX_SHAPES} shapes`);
    ids.add(s.id);
    shapes.push(s);
  };

  const v = {
    t,
    circle(id, x, y, r, opts = {}) {
      const where = `v.circle('${id}')`;
      add(where, { id: String(id), kind: 'circle', x: num(where, 'x', x), y: num(where, 'y', y), r: Math.abs(num(where, 'r', r)), ...styleOf(where, opts, { fill: 'blue' }) });
    },
    rect(id, x, y, w, h, opts = {}) {
      const where = `v.rect('${id}')`;
      const s = { id: String(id), kind: 'rect', x: num(where, 'x', x), y: num(where, 'y', y), w: Math.abs(num(where, 'w', w)), h: Math.abs(num(where, 'h', h)), ...styleOf(where, opts, { fill: 'surface' }) };
      if (opts.angle) s.angle = num(where, 'angle', opts.angle);
      add(where, s);
    },
    line(id, x1, y1, x2, y2, opts = {}) {
      const where = `v.line('${id}')`;
      add(where, { id: String(id), kind: 'line', x: num(where, 'x1', x1), y: num(where, 'y1', y1), x2: num(where, 'x2', x2), y2: num(where, 'y2', y2), ...styleOf(where, opts, { color: 'ink' }) });
    },
    path(id, points, opts = {}) {
      const where = `v.path('${id}')`;
      const pts = pointList(where, points);
      if (pts.length < 2) return;
      const s = { id: String(id), kind: 'path', x: pts[0][0], y: pts[0][1], points: pts, ...styleOf(where, opts, { color: opts.fill ? undefined : 'ink' }) };
      if (opts.closed) s.closed = true;
      add(where, s);
    },
    /** A vector drawn from (x, y) along (dx, dy), times `scale`. A zero vector draws nothing. */
    arrow(id, x, y, dx, dy, opts = {}) {
      const where = `v.arrow('${id}')`;
      const k = opts.scale === undefined ? 1 : num(where, 'scale', opts.scale);
      const ax = num(where, 'dx', dx) * k;
      const ay = num(where, 'dy', dy) * k;
      if (Math.hypot(ax, ay) < 1e-9) return;
      add(where, { id: String(id), kind: 'arrow', x: num(where, 'x', x), y: num(where, 'y', y), x2: round(x + ax), y2: round(y + ay), ...styleOf(where, opts, { color: 'orange' }) });
    },
    spring(id, x1, y1, x2, y2, opts = {}) {
      const where = `v.spring('${id}')`;
      const coils = opts.coils === undefined ? 10 : Math.round(clamp(num(where, 'coils', opts.coils), 2, 40));
      add(where, {
        id: String(id), kind: 'spring', x: num(where, 'x1', x1), y: num(where, 'y1', y1), x2: num(where, 'x2', x2), y2: num(where, 'y2', y2),
        coils, r: Math.abs(num(where, 'r', opts.r, 0.1)), ...styleOf(where, opts, { color: 'muted' }),
      });
    },
    text(id, x, y, text, opts = {}) {
      const where = `v.text('${id}')`;
      const s = { id: String(id), kind: 'text', x: num(where, 'x', x), y: num(where, 'y', y), text: String(text).slice(0, 80), ...styleOf(where, opts, { color: 'ink' }) };
      if (opts.size !== undefined) s.size = clamp(num(where, 'size', opts.size), 6, 48);
      if (opts.align) {
        if (!['left', 'center', 'right'].includes(opts.align)) fail(where, "align is 'left', 'center' or 'right'");
        s.align = opts.align;
      }
      add(where, s);
    },
    /**
     * Axes for a graph drawn in the scene: lines along x and y through
     * `origin` with `ticks` labelled every `every` units and axis names.
     */
    axes(id, { x, y, origin = [x[0], y[0]], every = [(x[1] - x[0]) / 5, (y[1] - y[0]) / 5], xLabel, yLabel, color = 'muted' }) {
      const where = `v.axes('${id}')`;
      if (!Array.isArray(x) || !Array.isArray(y)) fail(where, 'give x: [low, high] and y: [low, high]');
      const [ox, oy] = origin;
      v.line(`${id}:x`, x[0], oy, x[1], oy, { color, width: 1.5 });
      v.line(`${id}:y`, ox, y[0], ox, y[1], { color, width: 1.5 });
      const ex = num(where, 'every x', every[0]);
      const ey = num(where, 'every y', every[1]);
      const tickX = (y[1] - y[0]) * 0.015;
      const tickY = (x[1] - x[0]) * 0.015;
      let i = 0;
      for (let tx = Math.ceil(x[0] / ex) * ex; tx <= x[1] + 1e-9 && i < 40; tx += ex, i++) {
        v.line(`${id}:tx${i}`, tx, oy - tickX, tx, oy + tickX, { color, width: 1 });
        v.text(`${id}:lx${i}`, tx, oy - tickX * 4, String(round(tx)), { size: 11, color, align: 'center' });
      }
      i = 0;
      for (let ty = Math.ceil(y[0] / ey) * ey; ty <= y[1] + 1e-9 && i < 40; ty += ey, i++) {
        v.line(`${id}:ty${i}`, ox - tickY, ty, ox + tickY, ty, { color, width: 1 });
        v.text(`${id}:ly${i}`, ox - tickY * 2, ty, String(round(ty)), { size: 11, color, align: 'right' });
      }
      if (xLabel) v.text(`${id}:xl`, x[1], oy + tickX * 3, xLabel, { size: 12, color, align: 'right' });
      if (yLabel) v.text(`${id}:yl`, ox + tickY * 2, y[1], yLabel, { size: 12, color, align: 'left' });
    },
    /** A number (or short text) in the readouts panel. */
    readout(label, value, opts = {}) {
      const where = `v.readout('${label}')`;
      if (readouts.length >= MAX_READOUTS) fail(where, `at most ${MAX_READOUTS} readouts`);
      const r = { label: line(where, label, 40) };
      if (typeof value === 'number') r.value = num(where, 'value', value);
      else r.text = String(value).slice(0, 40);
      if (opts.unit) r.unit = String(opts.unit).slice(0, 16);
      if (opts.digits !== undefined) r.digits = Math.round(clamp(num(where, 'digits', opts.digits), 0, 6));
      if (opts.color) r.color = colorOf(where, opts.color);
      readouts.push(r);
    },
    /** A short line over the canvas, such as 'Landed at 41.2 m'. */
    notice(text) {
      notice = text === null || text === undefined ? null : String(text).slice(0, 120);
    },
    build() {
      return { shapes, readouts, notice };
    },
  };
  return v;
}
